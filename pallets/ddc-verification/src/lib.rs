//! # DDC Verification Pallet
//!
//! The DDC Verification pallet is used to validate zk-SNARK Proof and Signature
//!
//! - [`Call`]
//! - [`Pallet`]
//!

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use ddc_primitives::{
	ClusterId, DdcEra, MmrRootHash
};
use frame_support::{
	pallet_prelude::*,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sp_std::prelude::*;

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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		BillingReportCreated {
			cluster_id: ClusterId,
			era: DdcEra,
		},
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
		pub merkle_root_hash: MmrRootHash
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
			verification_key: Vec<u8>
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			ensure!(
				ActiveBillingReports::<T>::get(cluster_id, era).is_none(),
				Error::<T>::BillingReportAlreadyExist
			);

			let bounded_verification_key: BoundedVec<u8, T::MaxVerificationKeyLimit> =
				verification_key.clone().try_into().map_err(|_| Error::<T>::BadVerificationKey)?;

			let receipt_params = ReceiptParams{
				verification_key: bounded_verification_key,
				merkle_root_hash
			};

			ActiveBillingReports::<T>::insert(cluster_id, era, receipt_params);

			Self::deposit_event(Event::<T>::BillingReportCreated { cluster_id, era });
			Ok(())
		}
	}


}
