//! # DDC Verification Pallet
//!
//! The DDC Verification pallet is used to validate zk-SNARK Proof and Signature
//!
//! - [`Call`]
//! - [`Pallet`]

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use ddc_primitives::{
	traits::ClusterManager, ClusterId, DdcEra, MmrRootHash, NodePubKey, NodeUsage, StorageNodeMode,
	StorageNodeParams,
};
use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::{offchain::SubmitTransaction, pallet_prelude::*};
pub use pallet::*;
use scale_info::prelude::format;
use serde::Deserialize;
use sp_runtime::{offchain as rt_offchain, offchain::http};
use sp_std::{prelude::*, str};

pub mod weights;
use crate::weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::PalletId;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
		frame_support::traits::StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type MaxVerificationKeyLimit: Get<u32>;
		type WeightInfo: WeightInfo;
		type ClusterManager: ClusterManager<Self>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		BillingReportCreated { cluster_id: ClusterId, era: DdcEra },
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		BillingReportAlreadyExist,
		BadVerificationKey,
		BadRequest,
	}

	#[pallet::storage]
	#[pallet::getter(fn active_billing_reports)]
	pub type ActiveBillingReports<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		DdcEra,
		ReceiptParams<T::MaxVerificationKeyLimit>,
	>;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(MaxVerificationKeyLimit))]
	pub struct ReceiptParams<MaxVerificationKeyLimit: Get<u32>> {
		pub verification_key: BoundedVec<u8, MaxVerificationKeyLimit>,
		pub merkle_root_hash: MmrRootHash,
	}

	#[derive(Deserialize, Debug)]
	struct NodeActivity {
		totalBytesStored: u64,
		totalBytesDelivered: u64,
		totalPutRequests: u64,
		totalGetRequests: u64,
		proof: Vec<u8>,
	}

	impl NodeActivity {
		fn into_node_usage(self) -> NodeUsage {
			NodeUsage {
				transferred_bytes: self.totalBytesDelivered,
				stored_bytes: self.totalBytesStored,
				number_of_puts: self.totalPutRequests,
				number_of_gets: self.totalGetRequests,
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: BlockNumberFor<T>) {
			let era_id: DdcEra = 12345; // Replace with actual logic to get era_id

			// let usage = Self::fetch_nodes_usage(dac_nodes, era_id);

			// for (node_pub_key, result) in usage {
			// 	match result {
			// 		Ok(node_usage) => {
			// 			log::info!("Node {}: {:?}", node_pub_key, node_usage);
			// 		},
			// 		Err(e) => {
			// 			log::error!("Failed to fetch usage for node {}: {:?}", node_pub_key, e);
			// 		},
			// 	}
			// }
		}
	}

	impl<T: Config> Pallet<T> {
		fn fetch_node_usage(
			era_id: DdcEra,
			node_params: &StorageNodeParams,
		) -> Result<NodeUsage, http::Error> {
			let scheme = if node_params.ssl { "https" } else { "http" };
			let host = str::from_utf8(&node_params.host).expect("Invalid UTF-8 in host");
			let url = format!(
				"{}://{}:{}/activity/node?eraId={}",
				scheme, host, node_params.http_port, era_id
			);

			let request = http::Request::get(&url);
			let timeout =
				sp_io::offchain::timestamp().add(rt_offchain::Duration::from_millis(3000));
			let pending = request.deadline(timeout).send().map_err(|_| http::Error::IoError)?;

			let response =
				pending.try_wait(timeout).map_err(|_| http::Error::DeadlineReached)??;
			if response.code != 200 {
				return Err(http::Error::Unknown);
			}

			let body = response.body().collect::<Vec<u8>>();
			let node_activity: NodeActivity =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;
			Ok(node_activity.into_node_usage())
		}

		fn fetch_nodes_usage(
			dac_nodes: Vec<(NodePubKey, StorageNodeParams)>,
			era_id: DdcEra,
		) -> Vec<(NodePubKey, Result<NodeUsage, http::Error>)> {
			dac_nodes
				.into_iter()
				.map(|(node_pub_key, storage_node_params)| {
					let usage_result = Self::fetch_node_usage(era_id, &storage_node_params);
					(node_pub_key, usage_result)
				})
				.collect()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_billing_reports())]
		pub fn create_billing_reports(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			merkle_root_hash: MmrRootHash,
			verification_key: Vec<u8>,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			ensure!(
				ActiveBillingReports::<T>::get(cluster_id, era).is_none(),
				Error::<T>::BillingReportAlreadyExist
			);

			let bounded_verification_key: BoundedVec<u8, T::MaxVerificationKeyLimit> =
				verification_key
					.clone()
					.try_into()
					.map_err(|_| Error::<T>::BadVerificationKey)?;

			let receipt_params =
				ReceiptParams { verification_key: bounded_verification_key, merkle_root_hash };

			ActiveBillingReports::<T>::insert(cluster_id, era, receipt_params);

			Self::deposit_event(Event::<T>::BillingReportCreated { cluster_id, era });
			Ok(())
		}
	}
}
