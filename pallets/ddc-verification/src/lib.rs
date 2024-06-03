//! # DDC Verification Pallet
//!
//! The DDC Verification pallet is used to validate zk-SNARK Proof and Signature
//!
//! - [`Call`]
//! - [`Pallet`]

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use core::str;

use ddc_primitives::{
	traits::{ClusterManager, NodeVisitor},
	ClusterId, CustomerUsage, DdcEra, MmrRootHash, NodeParams, NodePubKey, NodeUsage,
	StorageNodeMode, StorageNodeParams,
};
use frame_support::{
	pallet_prelude::*,
	traits::{Get, OneSessionHandler},
};
use frame_system::{
	offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer},
	pallet_prelude::*,
};
pub use pallet::*;
use scale_info::prelude::format;
use serde::{Deserialize, Serialize};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{offchain as rt_offchain, offchain::http, Percent};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

pub mod weights;
use crate::weights::WeightInfo;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

const KEY_TYPE: KeyTypeId = KeyTypeId(*b"cer!");
pub mod sr25519 {
	mod app_sr25519 {
		use sp_application_crypto::{app_crypto, sr25519};

		use crate::KEY_TYPE;
		app_crypto!(sr25519, KEY_TYPE);
	}

	sp_application_crypto::with_pair! {
		pub type AuthorityPair = app_sr25519::Pair;
	}
	pub type AuthoritySignature = app_sr25519::Signature;
	pub type AuthorityId = app_sr25519::Public;
}
pub mod crypto {
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};

	use super::KEY_TYPE;
	app_crypto!(sr25519, KEY_TYPE);

	pub struct OffchainIdentifierId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for OffchainIdentifierId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for OffchainIdentifierId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[frame_support::pallet]
pub mod pallet {
	use ddc_primitives::BucketId;
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
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type MaxVerificationKeyLimit: Get<u32>;
		type WeightInfo: WeightInfo;
		type ClusterManager: ClusterManager<Self>;
		type NodeVisitor: NodeVisitor<Self>;
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ Into<sp_core::sr25519::Public>
			+ From<sp_core::sr25519::Public>;
		type AuthorityIdParameter: Parameter
			+ From<sp_core::sr25519::Public>
			+ Into<Self::AuthorityId>
			+ MaxEncodedLen;
		type OffchainIdentifierId: AppCrypto<Self::Public, Self::Signature>;
		const MAJORITY: u8;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		BillingReportCreated {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		VerificationKeyStored {
			verification_key: Vec<u8>,
		},
		PayoutBatchCreated {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		NotEnoughNodesForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			customer_id: T::AccountId,
			bucket_id: BucketId,
		},
		CustomerActivityNotInConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			customer_id: T::AccountId,
			bucket_id: BucketId,
		},
	}

	#[derive(Debug, Encode, Decode)]
	pub enum ConsensusError<T: Config> {
		NotEnoughNodesForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			customer_id: T::AccountId,
			bucket_id: BucketId,
		},
		CustomerActivityNotInConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			customer_id: T::AccountId,
			bucket_id: BucketId,
		},
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		BillingReportAlreadyExist,
		BadVerificationKey,
		BadRequest,
		NotAValidator,
		AlreadySigned,
		NodeRetrievalError,
		NodeUsageRetrievalError,
		ClusterToValidateRetrievalError,
		EraToValidateRetrievalError,
	}

	#[pallet::storage]
	#[pallet::getter(fn active_billing_reports)]
	pub type ActiveBillingReports<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ClusterId, Blake2_128Concat, DdcEra, ReceiptParams>;

	#[pallet::storage]
	#[pallet::getter(fn payout_batch)]
	pub type PayoutBatch<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ClusterId, Blake2_128Concat, DdcEra, PayoutData>;

	#[pallet::storage]
	#[pallet::getter(fn payout_validators)]
	pub type PayoutValidators<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		(ClusterId, DdcEra),
		Blake2_128Concat,
		MmrRootHash,
		Vec<T::AuthorityId>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn cluster_to_validate)]
	pub type ClusterToValidate<T: Config> = StorageValue<_, ClusterId>; // todo! setter out of scope

	#[pallet::storage]
	#[pallet::getter(fn era_to_validate)] // last_validated_era
	pub type EraToValidate<T: Config> = StorageValue<_, DdcEra>;

	#[pallet::storage]
	#[pallet::getter(fn verification_key)]
	pub type VerificationKey<T: Config> =
		StorageValue<_, BoundedVec<u8, T::MaxVerificationKeyLimit>>;

	#[pallet::storage]
	#[pallet::getter(fn validator_set)]
	pub type ValidatorSet<T: Config> = StorageValue<_, Vec<T::AuthorityId>>;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct ReceiptParams {
		pub merkle_root_hash: MmrRootHash,
	}

	#[derive(Serialize, Deserialize, Debug, Clone)]
	pub(crate) struct NodeActivity {
		pub(crate) node_id: [u8; 32],
		pub(crate) provider_id: [u8; 32],
		pub(crate) stored_bytes: u64,
		pub(crate) transferred_bytes: u64,
		pub(crate) number_of_puts: u64,
		pub(crate) number_of_gets: u64,
	}

	#[derive(Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq)]
	pub(crate) struct CustomerActivity {
		pub(crate) customer_id: [u8; 32],
		pub(crate) bucket_id: BucketId,
		pub(crate) stored_bytes: u64,
		pub(crate) transferred_bytes: u64,
		pub(crate) number_of_puts: u64,
		pub(crate) number_of_gets: u64,
	}

	impl From<CustomerActivity> for CustomerUsage {
		fn from(activity: CustomerActivity) -> Self {
			CustomerUsage {
				transferred_bytes: activity.transferred_bytes,
				stored_bytes: activity.stored_bytes,
				number_of_puts: activity.number_of_puts,
				number_of_gets: activity.number_of_gets,
			}
		}
	}

	impl From<NodeActivity> for NodeUsage {
		fn from(activity: NodeActivity) -> Self {
			NodeUsage {
				transferred_bytes: activity.transferred_bytes,
				stored_bytes: activity.stored_bytes,
				number_of_puts: activity.number_of_puts,
				number_of_gets: activity.number_of_gets,
			}
		}
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(Hash))]
	pub struct PayoutData {
		pub hash: MmrRootHash,
	}

	macro_rules! unwrap_or_log_error {
		($result:expr, $error_msg:expr) => {
			match $result {
				Ok(val) => val,
				Err(err) => {
					log::error!("{}: {:?}", $error_msg, err);
					return;
				},
			}
		};
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(_block_number: BlockNumberFor<T>) {
			log::info!("Hello from pallet-ocw.");

			let signer = Signer::<T, T::OffchainIdentifierId>::all_accounts();
			if !signer.can_sign() {
				log::error!("No local accounts available");
				return;
			}

			let results =
				signer.send_signed_transaction(|_account| Call::set_validate_payout_batch {
					cluster_id: Default::default(),
					era: DdcEra::default(),
					payout_data: PayoutData { hash: MmrRootHash::default() },
				});

			for (acc, res) in &results {
				match res {
					Ok(()) => log::info!("[{:?}] Submitted response", acc.id),
					Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
				}
			}

			let era_id = unwrap_or_log_error!(
				Self::get_era_to_validate(),
				"Error retrieving era to validate"
			);
			let cluster_id = unwrap_or_log_error!(
				Self::get_cluster_to_validate(),
				"Error retrieving cluster to validate"
			);
			let dac_nodes = unwrap_or_log_error!(
				Self::get_dac_nodes(&cluster_id),
				"Error retrieving dac nodes to validate"
			);
			let _nodes_usage = unwrap_or_log_error!(
				Self::fetch_nodes_usage_for_era(&cluster_id, era_id, &dac_nodes),
				"Error retrieving node activities to validate"
			);

			let customers_usage = unwrap_or_log_error!(
				Self::fetch_customers_usage_for_era(&cluster_id, era_id, &dac_nodes),
				"Error retrieving customers activities to validate"
			);

			let min_nodes = dac_nodes.len().ilog2() as usize;
			let _ = Self::get_consensus_customers_activity(
				&cluster_id,
				era_id,
				customers_usage,
				min_nodes,
				Percent::from_percent(T::MAJORITY),
			);
		}
	}

	impl<T: Config> Pallet<T> {
		// Function to check if there is a consensus for a particular customer/bucket
		pub(crate) fn reach_consensus(
			activities: &[CustomerActivity],
			threshold: usize,
		) -> Option<CustomerActivity> {
			let mut count_map: BTreeMap<CustomerActivity, usize> = BTreeMap::new();

			for activity in activities {
				*count_map.entry(activity.clone()).or_default() += 1;
			}

			count_map
				.into_iter()
				.find(|&(_, count)| count >= threshold)
				.map(|(activity, _)| activity)
		}

		pub(crate) fn get_consensus_customers_activity(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			customers_activity: Vec<(NodePubKey, Vec<CustomerActivity>)>,
			min_nodes: usize,
			threshold: Percent,
		) -> Result<Vec<CustomerActivity>, Vec<ConsensusError<T>>> {
			let mut customer_buckets: BTreeMap<([u8; 32], BucketId), Vec<CustomerActivity>> =
				BTreeMap::new();

			// Flatten and collect all customer activities
			for (_node_id, activities) in customers_activity.iter() {
				for activity in activities.iter() {
					customer_buckets
						.entry((activity.customer_id, activity.bucket_id))
						.or_default()
						.push(activity.clone());
				}
			}

			let mut consensus_activities = Vec::new();
			let mut errors = Vec::new();
			let min_threshold = threshold * min_nodes;

			// Check if each customer/bucket appears in at least `min_nodes` nodes
			for ((customer_id, bucket_id), activities) in customer_buckets {
				if activities.len() < min_nodes {
					if let Ok(account_id) = T::AccountId::decode(&mut &customer_id[..]) {
						errors.push(ConsensusError::<T>::NotEnoughNodesForConsensus {
							cluster_id: (*cluster_id),
							era_id,
							customer_id: account_id,
							bucket_id,
						});
					} else {
						unreachable!();
					}
				} else if let Some(activity) = Self::reach_consensus(&activities, min_threshold) {
					consensus_activities.push(activity);
				} else if let Ok(account_id) = T::AccountId::decode(&mut &customer_id[..]) {
					errors.push(ConsensusError::<T>::CustomerActivityNotInConsensus {
						cluster_id: (*cluster_id),
						era_id,
						customer_id: account_id,
						bucket_id,
					});
				} else {
					unreachable!();
				}
			}

			if errors.is_empty() {
				Ok(consensus_activities)
			} else {
				Err(errors)
			}
		}

		fn get_era_to_validate() -> Result<DdcEra, Error<T>> {
			Self::era_to_validate().ok_or(Error::EraToValidateRetrievalError)

			/*
   			Add scheduleer to repeat cycle after 100 blocks
			get LAST_VALIDATED_ERA from verification pallet
			for each dac_node fetch processed eras by calling - https://storage-1.devnet.cere.network/activity/era (similar to fetch_customers_usage_for_era)
			find ids that all nodes have it and is larger than LAST_VALIDATED_ERA (cause all nodes need to be called for the same era)
			(if some node has 18 and some have 17, the result is 17)
			(if all nodes have 17 and 18 and LAST_VALIDATED_ERA is 16, then we return 17 and 18)

			if no new eras then do nothing   
			if there are several new eras then take the smallest
				then call fetch_nodes_usage_for_era and fetch_customers_usage_for_era for the smallest era
				payer_list and payee_list - aggregate it (victor todo)
				create merkle tree for both lists
				send to validation pallet merkle roots for payer_list and payee_list for the cluster id and era id
				reach consensus on the merkle roots for payer_list and payee_list for the cluster id and era id in the pallet on-chain
			*/
		}

		fn get_cluster_to_validate() -> Result<ClusterId, Error<T>> {
			Self::cluster_to_validate().ok_or(Error::ClusterToValidateRetrievalError)
		}

		pub(crate) fn fetch_customers_usage(
			_cluster_id: &ClusterId,
			era_id: DdcEra,
			node_params: &StorageNodeParams,
		) -> Result<Vec<CustomerActivity>, http::Error> {
			let scheme = if node_params.ssl { "https" } else { "http" };
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let url = format!(
				"{}://{}:{}/activity/buckets?eraId={}",
				scheme, host, node_params.http_port, era_id
			);

			let request = http::Request::get(&url);
			let timeout =
				sp_io::offchain::timestamp().add(sp_runtime::offchain::Duration::from_millis(3000));
			let pending = request.deadline(timeout).send().map_err(|_| http::Error::IoError)?;

			let response =
				pending.try_wait(timeout).map_err(|_| http::Error::DeadlineReached)??;
			if response.code != 200 {
				return Err(http::Error::Unknown);
			}

			let body = response.body().collect::<Vec<u8>>();
			serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)
		}

		pub(crate) fn fetch_node_usage(
			_cluster_id: &ClusterId,
			era_id: DdcEra,
			node_params: &StorageNodeParams,
		) -> Result<Vec<NodeActivity>, http::Error> {
			let scheme = if node_params.ssl { "https" } else { "http" };
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
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
			serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)
		}

		fn get_dac_nodes(
			cluster_id: &ClusterId,
		) -> Result<Vec<(NodePubKey, StorageNodeParams)>, Error<T>> {
			let mut dac_nodes = Vec::new();

			let nodes = T::ClusterManager::get_nodes(cluster_id)
				.map_err(|_| Error::<T>::NodeRetrievalError)?;

			// Iterate over each node
			for node_pub_key in nodes {
				// Get the node parameters
				if let Ok(NodeParams::StorageParams(storage_params)) =
					T::NodeVisitor::get_node_params(&node_pub_key)
				{
					// Check if the mode is StorageNodeMode::DAC
					if storage_params.mode == StorageNodeMode::DAC {
						// Add to the results if the mode matches
						dac_nodes.push((node_pub_key, storage_params));
					}
				}
			}

			Ok(dac_nodes)
		}

		fn fetch_nodes_usage_for_era(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<(NodePubKey, Vec<NodeActivity>)>, Error<T>> {
			let mut node_usages = Vec::new();

			for (node_pub_key, node_params) in dac_nodes {
				let usage = Self::fetch_node_usage(cluster_id, era_id, node_params)
					.map_err(|_| Error::<T>::NodeUsageRetrievalError)?;

				node_usages.push((node_pub_key.clone(), usage));
			}

			Ok(node_usages)
		}

		fn fetch_customers_usage_for_era(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<(NodePubKey, Vec<CustomerActivity>)>, Error<T>> {
			let mut customers_usages = Vec::new();

			for (node_pub_key, node_params) in dac_nodes {
				let usage = Self::fetch_customers_usage(cluster_id, era_id, node_params)
					.map_err(|_| Error::<T>::NodeUsageRetrievalError)?;

				customers_usages.push((node_pub_key.clone(), usage));
			}

			Ok(customers_usages)
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
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			ensure!(
				ActiveBillingReports::<T>::get(cluster_id, era).is_none(),
				Error::<T>::BillingReportAlreadyExist
			);

			let receipt_params = ReceiptParams { merkle_root_hash };

			ActiveBillingReports::<T>::insert(cluster_id, era, receipt_params);

			Self::deposit_event(Event::<T>::BillingReportCreated { cluster_id, era });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::create_billing_reports())]
		pub fn set_verification_key(
			origin: OriginFor<T>,
			verification_key: Vec<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let bounded_verification_key: BoundedVec<u8, T::MaxVerificationKeyLimit> =
				verification_key
					.clone()
					.try_into()
					.map_err(|_| Error::<T>::BadVerificationKey)?;

			VerificationKey::<T>::put(bounded_verification_key);
			Self::deposit_event(Event::<T>::VerificationKeyStored { verification_key });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::create_billing_reports())]
		pub fn set_validate_payout_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			payout_data: PayoutData,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let account_bytes: [u8; 32] = Self::account_to_bytes(&who)?;
			let who: T::AuthorityIdParameter =
				sp_application_crypto::sr25519::Public::from_raw(account_bytes).into();
			let who: T::AuthorityId = who.into();
			let authorities = <ValidatorSet<T>>::get().unwrap();

			ensure!(authorities.contains(&who.clone()), Error::<T>::NotAValidator);

			ensure!(
				!<PayoutValidators<T>>::get((cluster_id, era), payout_data.hash)
					.contains(&who.clone()),
				Error::<T>::AlreadySigned
			);

			<PayoutValidators<T>>::try_mutate(
				(cluster_id, era),
				payout_data.hash,
				|validators| -> DispatchResult {
					validators.push(who);
					Ok(())
				},
			)?;

			let p = Percent::from_percent(T::MAJORITY);
			let threshold = p * authorities.len();

			let signed_validators = <PayoutValidators<T>>::get((cluster_id, era), payout_data.hash);

			if threshold < signed_validators.len() {
				PayoutBatch::<T>::insert(cluster_id, era, payout_data);
				Self::deposit_event(Event::<T>::PayoutBatchCreated { cluster_id, era });
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// This function converts a 32 byte AccountId to its byte-array equivalent form.
		fn account_to_bytes<AccountId>(account: &AccountId) -> Result<[u8; 32], DispatchError>
		where
			AccountId: Encode,
		{
			let account_vec = account.encode();
			ensure!(account_vec.len() == 32, "AccountId must be 32 bytes.");
			let mut bytes = [0u8; 32];
			bytes.copy_from_slice(&account_vec);
			Ok(bytes)
		}
	}
	impl<T: Config> sp_application_crypto::BoundToRuntimeAppPublic for Pallet<T> {
		type Public = T::AuthorityId;
	}

	impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
		type Key = T::AuthorityId;

		fn on_genesis_session<'a, I: 'a>(validators: I)
		where
			I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
		{
			let validators = validators.map(|(_, k)| k).collect::<Vec<_>>();

			ValidatorSet::<T>::put(validators);
		}

		fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_authorities: I)
		where
			I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
		{
			let validators = validators.map(|(_, k)| k).collect::<Vec<_>>();
			ValidatorSet::<T>::put(validators);
		}

		fn on_disabled(_i: u32) {}
	}
}
