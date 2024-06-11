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
	traits::{ClusterManager, NodeVisitor, ValidatorVisitor},
	ActivityHash, BatchIndex, ClusterId, CustomerUsage, DdcEra, NodeParams, NodePubKey, NodeUsage,
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
use sp_runtime::{offchain as rt_offchain, offchain::http, traits::Hash, Percent};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

pub mod weights;
use itertools::Itertools;

use crate::weights::WeightInfo;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use ddc_primitives::BucketId;
	use frame_election_provider_support::SortedListProvider;
	use frame_support::PalletId;
	use sp_runtime::SaturatedConversion;
	use sp_staking::StakingInterface;

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

		type ActivityHash: Member
			+ Parameter
			+ MaybeSerializeDeserialize
			+ Ord
			+ Into<ActivityHash>
			+ From<ActivityHash>;
		type ActivityHasher: Hash<Output = Self::ActivityHash>;
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ Into<sp_core::sr25519::Public>
			+ From<sp_core::sr25519::Public>;

		type OffchainIdentifierId: AppCrypto<Self::Public, Self::Signature>;
		const MAJORITY: u8;
		const BLOCK_TO_START: u32;
		type Staking: StakingInterface<AccountId = Self::AccountId>;
		type ValidatorList: SortedListProvider<Self::AccountId>;
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
			id: ActivityHash,
			validator: T::AccountId,
		},
		ActivityNotInConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			id: ActivityHash,
			validator: T::AccountId,
		},
	}

	#[derive(Debug, Encode, Decode, Clone, TypeInfo, PartialEq)]
	pub enum ConsensusError {
		NotEnoughNodesForConsensus { cluster_id: ClusterId, era_id: DdcEra, id: ActivityHash },
		ActivityNotInConsensus { cluster_id: ClusterId, era_id: DdcEra, id: ActivityHash },
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
		EraPerNodeRetrievalError,
		FailToFetchIds,
		NoValidatorExist,
		NotController,
		NotValidatorStash,
		DDCValidatorKeyNotRegistered,
	}

	#[pallet::storage]
	#[pallet::getter(fn active_billing_reports)]
	pub type ActiveBillingReports<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		T::AccountId,
		ReceiptParams,
	>;

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
		ActivityHash,
		Vec<T::AccountId>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn cluster_to_validate)]
	pub type ClusterToValidate<T: Config> = StorageValue<_, ClusterId>; // todo! setter out of scope

	#[pallet::storage]
	#[pallet::getter(fn verification_key)]
	pub type VerificationKey<T: Config> =
		StorageValue<_, BoundedVec<u8, T::MaxVerificationKeyLimit>>;

	#[pallet::storage]
	#[pallet::getter(fn validator_set)]
	pub type ValidatorSet<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_stash_for_ddc_validator)]
	pub type ValidatorToStashKey<T: Config> = StorageMap<_, Identity, T::AccountId, T::AccountId>;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct ReceiptParams {
		pub era: DdcEra,
		pub payers_merkle_root_hash: ActivityHash,
		pub payees_merkle_root_hash: ActivityHash,
	}

	#[derive(Serialize, Copy, Deserialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
	pub(crate) struct EraActivity {
		pub id: DdcEra,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode,
	)]
	pub(crate) struct NodeActivity {
		pub(crate) node_id: [u8; 32],
		pub(crate) provider_id: [u8; 32],
		pub(crate) stored_bytes: u64,
		pub(crate) transferred_bytes: u64,
		pub(crate) number_of_puts: u64,
		pub(crate) number_of_gets: u64,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode,
	)]
	pub(crate) struct CustomerActivity {
		pub(crate) customer_id: [u8; 32],
		pub(crate) bucket_id: BucketId,
		pub(crate) stored_bytes: u64,
		pub(crate) transferred_bytes: u64,
		pub(crate) number_of_puts: u64,
		pub(crate) number_of_gets: u64,
	}

	// Define a common trait
	pub trait Activity:
		Clone + Ord + PartialEq + Eq + Serialize + for<'de> Deserialize<'de>
	{
		fn get_consensus_id<T: Config>(&self) -> ActivityHash;
		fn hash<T: Config>(&self) -> ActivityHash;
	}

	impl Activity for NodeActivity {
		fn get_consensus_id<T: Config>(&self) -> ActivityHash {
			T::ActivityHasher::hash(&self.node_id).into()
		}

		fn hash<T: Config>(&self) -> ActivityHash {
			T::ActivityHasher::hash(&self.encode()).into()
		}
	}
	impl Activity for CustomerActivity {
		fn get_consensus_id<T: Config>(&self) -> ActivityHash {
			let mut data = self.customer_id.to_vec();
			data.extend_from_slice(&self.bucket_id.encode());
			T::ActivityHasher::hash(&data).into()
		}

		fn hash<T: Config>(&self) -> ActivityHash {
			T::ActivityHasher::hash(&self.encode()).into()
		}
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
		pub hash: ActivityHash,
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
		fn offchain_worker(block_number: BlockNumberFor<T>) {
			if !sp_io::offchain::is_validator() {
				return
			}
			log::info!("Hello from ocw!!!!!!!!!!");
			if block_number.saturated_into::<u32>() % T::BLOCK_TO_START != 0 {
				return;
			}
			log::info!("Hello from pallet-ddc-verification.");
			let signer = Signer::<T, T::OffchainIdentifierId>::all_accounts();
			if !signer.can_sign() {
				log::error!("No local accounts available");
				return;
			}

			let results =
				signer.send_signed_transaction(|_account| Call::set_validate_payout_batch {
					cluster_id: Default::default(),
					era: DdcEra::default(),
					payout_data: PayoutData { hash: ActivityHash::default() },
				});

			for (acc, res) in &results {
				match res {
					Ok(()) => log::info!("[{:?}] Submitted response", acc.id),
					Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
				}
			}

			let cluster_id = unwrap_or_log_error!(
				Self::get_cluster_to_validate(),
				"Error retrieving cluster to validate"
			);

			let dac_nodes = unwrap_or_log_error!(
				Self::get_dac_nodes(&cluster_id),
				"Error retrieving dac nodes to validate"
			);

			let era_id = unwrap_or_log_error!(
				Self::get_era_to_validate(cluster_id, dac_nodes.clone()),
				"Error retrieving era to validate"
			);

			if era_id.is_none() {
				return;
			}
			let id = era_id.unwrap();
			let nodes_usage = unwrap_or_log_error!(
				Self::fetch_nodes_usage_for_era(&cluster_id, id, &dac_nodes),
				"Error retrieving node activities to validate"
			);

			let customers_usage = unwrap_or_log_error!(
				Self::fetch_customers_usage_for_era(&cluster_id, id, &dac_nodes),
				"Error retrieving customers activities to validate"
			);
			let min_nodes = dac_nodes.len().ilog2() as usize;

			let _customers_activity_mmr_root = match Self::get_consensus_for_activities(
				&cluster_id,
				id,
				&customers_usage,
				min_nodes,
				Percent::from_percent(T::MAJORITY),
			) {
				Ok(customers_activity_in_consensus) => {
					// Process node_activities
					let sorted_activities = customers_activity_in_consensus.clone();

					let leaves: Vec<ActivityHash> =
						sorted_activities.iter().map(|activity| activity.hash::<T>()).collect();

					Some(Self::create_merkle_root(leaves))
				},
				Err(errors) => {
					let results = signer.send_signed_transaction(|_account| {
						Call::emit_consensus_errors { errors: errors.clone() }
					});

					for (acc, res) in results {
						match res {
							Ok(_) => log::info!(
								"Successfully submitted error processing tx: {:?}",
								acc.id
							),
							Err(e) => log::error!(
								"Failed to submit error processing tx: {:?} ({:?})",
								acc.id,
								e
							),
						}
					}
					None
				},
			};

			let _nodes_activity_mmr_root = match Self::get_consensus_for_activities(
				&cluster_id,
				id,
				&nodes_usage,
				min_nodes,
				Percent::from_percent(T::MAJORITY),
			) {
				Ok(nodes_activity_in_consensus) => {
					// Process node_activities
					let sorted_activities = nodes_activity_in_consensus.clone();

					let leaves: Vec<ActivityHash> =
						sorted_activities.iter().map(|activity| activity.hash::<T>()).collect();

					Some(Self::create_merkle_root(leaves))
				},
				Err(errors) => {
					let results = signer.send_signed_transaction(|_account| {
						Call::emit_consensus_errors { errors: errors.clone() }
					});

					for (acc, res) in results {
						match res {
							Ok(_) => log::info!(
								"Successfully submitted error processing tx: {:?}",
								acc.id
							),
							Err(e) => log::error!(
								"Failed to submit error processing tx: {:?} ({:?})",
								acc.id,
								e
							),
						}
					}
					None
				},
			};
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn create_merkle_root(leaves: Vec<ActivityHash>) -> ActivityHash {
			let leaves = leaves.iter().map(|l| l.to_vec()).collect::<Vec<Vec<u8>>>();

			T::ActivityHasher::ordered_trie_root(leaves, Default::default()).into()
		}

		pub(crate) fn get_era_to_validate(
			cluster_id: ClusterId,
			dac_nodes: Vec<(NodePubKey, StorageNodeParams)>,
		) -> Result<Option<DdcEra>, Error<T>> {
			let current_validator = T::NodeVisitor::get_current_validator();
			let last_validated_era = Self::active_billing_reports(cluster_id, current_validator)
				.ok_or(Error::EraToValidateRetrievalError)?;

			let all_ids = Self::fetch_processed_era_for_node(dac_nodes.clone())
				.map_err(|_| Error::<T>::FailToFetchIds)?;

			let ids_greater_than_last_validated_era: Vec<DdcEra> = all_ids
				.iter()
				.flat_map(|eras| {
					eras.iter()
						.cloned()
						.filter(|ids| ids.id > last_validated_era.era)
						.map(|era| era.id)
				})
				.sorted()
				.collect::<Vec<DdcEra>>();

			let mut grouped_data: Vec<(u32, DdcEra)> = Vec::new();
			for (key, chunk) in
				&ids_greater_than_last_validated_era.into_iter().chunk_by(|elt| *elt)
			{
				grouped_data.push((chunk.count() as u32, key));
			}

			let all_node_eras = grouped_data
				.into_iter()
				.filter(|(v, _)| *v == dac_nodes.len() as u32)
				.map(|(_, id)| id)
				.collect::<Vec<DdcEra>>();

			Ok(all_node_eras.iter().cloned().min())
		}

		pub(crate) fn reach_consensus<A: Activity>(
			activities: &[A],
			threshold: usize,
		) -> Option<A> {
			let mut count_map: BTreeMap<A, usize> = BTreeMap::new();

			for activity in activities {
				*count_map.entry(activity.clone()).or_default() += 1;
			}

			count_map
				.into_iter()
				.find(|&(_, count)| count >= threshold)
				.map(|(activity, _)| activity)
		}

		pub(crate) fn get_consensus_for_activities<A: Activity>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			activities: &[(NodePubKey, Vec<A>)],
			min_nodes: usize,
			threshold: Percent,
		) -> Result<Vec<A>, Vec<ConsensusError>> {
			let mut customer_buckets: BTreeMap<ActivityHash, Vec<A>> = BTreeMap::new();

			// Flatten and collect all customer activities
			for (_node_id, activities) in activities.iter() {
				for activity in activities.iter() {
					customer_buckets
						.entry(activity.get_consensus_id::<T>())
						.or_default()
						.push(activity.clone());
				}
			}

			let mut consensus_activities = Vec::new();
			let mut errors = Vec::new();
			let min_threshold = threshold * min_nodes;

			// Check if each customer/bucket appears in at least `min_nodes` nodes
			for (id, activities) in customer_buckets {
				if activities.len() < min_nodes {
					errors.push(ConsensusError::NotEnoughNodesForConsensus {
						cluster_id: (*cluster_id),
						era_id,
						id,
					});
				} else if let Some(activity) = Self::reach_consensus(&activities, min_threshold) {
					consensus_activities.push(activity);
				} else {
					errors.push(ConsensusError::ActivityNotInConsensus {
						cluster_id: (*cluster_id),
						era_id,
						id,
					});
				}
			}

			if errors.is_empty() {
				Ok(consensus_activities)
			} else {
				Err(errors)
			}
		}

		fn get_cluster_to_validate() -> Result<ClusterId, Error<T>> {
			Self::cluster_to_validate().ok_or(Error::ClusterToValidateRetrievalError)
		}

		pub(crate) fn fetch_processed_era(
			node_params: StorageNodeParams,
		) -> Result<Vec<EraActivity>, http::Error> {
			let scheme = if node_params.ssl { "https" } else { "http" };
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let url = format!("{}://{}:{}/activity/era", scheme, host, node_params.http_port);
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

		fn fetch_processed_era_for_node(
			dac_nodes: Vec<(NodePubKey, StorageNodeParams)>,
		) -> Result<Vec<Vec<EraActivity>>, Error<T>> {
			let mut eras = Vec::new();

			for (_, node_params) in dac_nodes {
				let ids = Self::fetch_processed_era(node_params)
					.map_err(|_| Error::<T>::EraPerNodeRetrievalError)?;

				eras.push(ids);
			}
			Ok(eras)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())]
		pub fn create_billing_reports(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			payers_merkle_root_hash: ActivityHash,
			payees_merkle_root_hash: ActivityHash,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				ActiveBillingReports::<T>::get(cluster_id, who.clone()).is_none(),
				Error::<T>::BillingReportAlreadyExist
			);

			let receipt_params =
				ReceiptParams { era, payers_merkle_root_hash, payees_merkle_root_hash };

			ActiveBillingReports::<T>::insert(cluster_id, who, receipt_params);

			Self::deposit_event(Event::<T>::BillingReportCreated { cluster_id, era });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn set_validate_payout_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			payout_data: PayoutData,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let validators = <ValidatorSet<T>>::get();

			let stash_key = ValidatorToStashKey::<T>::get(who.clone())
				.ok_or(Error::<T>::DDCValidatorKeyNotRegistered)?;

			ensure!(validators.contains(&stash_key), Error::<T>::NotAValidator);

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

			let percent = Percent::from_percent(T::MAJORITY);
			let threshold = percent * validators.len();

			let signed_validators = <PayoutValidators<T>>::get((cluster_id, era), payout_data.hash);

			if threshold < signed_validators.len() {
				PayoutBatch::<T>::insert(cluster_id, era, payout_data);
				Self::deposit_event(Event::<T>::PayoutBatchCreated { cluster_id, era });
			}

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn emit_consensus_errors(
			origin: OriginFor<T>,
			errors: Vec<ConsensusError>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let validators = <ValidatorSet<T>>::get();

			let stash_key = ValidatorToStashKey::<T>::get(who.clone())
				.ok_or(Error::<T>::DDCValidatorKeyNotRegistered)?;

			ensure!(validators.contains(&stash_key), Error::<T>::NotAValidator);

			for error in errors {
				match error {
					ConsensusError::NotEnoughNodesForConsensus { cluster_id, era_id, id } => {
						Self::deposit_event(Event::NotEnoughNodesForConsensus {
							cluster_id,
							era_id,
							id,
							validator: who.clone(),
						});
					},
					ConsensusError::ActivityNotInConsensus { cluster_id, era_id, id } => {
						Self::deposit_event(Event::ActivityNotInConsensus {
							cluster_id,
							era_id,
							id,
							validator: who.clone(),
						});
					},
				}
			}

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn set_validator_key(
			origin: OriginFor<T>,
			ddc_validator_pub: T::AccountId,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let stash_account =
				T::Staking::stash_by_ctrl(&controller).map_err(|_| Error::<T>::NotController)?;

			ensure!(T::ValidatorList::contains(&stash_account), Error::<T>::NotValidatorStash);

			ValidatorToStashKey::<T>::insert(ddc_validator_pub, &stash_account);
			Ok(())
		}
	}

	impl<T: Config> ValidatorVisitor<T> for Pallet<T> {
		fn setup_validators(validators: Vec<T::AccountId>) {
			ValidatorSet::<T>::put(validators);
		}
		fn is_ocw_validator(caller: T::AccountId) -> bool {
			if let Some(stash) = ValidatorToStashKey::<T>::get(caller) {
				<ValidatorSet<T>>::get().contains(&stash)
			} else {
				false
			}
		}

		fn is_customers_batch_valid(
			_cluster_id: ClusterId,
			_era: DdcEra,
			_batch_index: BatchIndex,
			_payers: Vec<(T::AccountId, CustomerUsage)>,
		) -> bool {
			true
		}
		fn is_providers_batch_valid(
			_cluster_id: ClusterId,
			_era: DdcEra,
			_batch_index: BatchIndex,
			_payees: Vec<(T::AccountId, NodeUsage)>,
		) -> bool {
			true
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
			let validators = validators
				.map(|(_, k)| T::AccountId::decode(&mut &k.into().encode()[..]).unwrap())
				.collect::<Vec<_>>();

			ValidatorSet::<T>::put(validators);
		}

		fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_authorities: I)
		where
			I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
		{
			let validators = validators
				.map(|(_, k)| T::AccountId::decode(&mut &k.into().encode()[..]).unwrap())
				.collect::<Vec<_>>();
			ValidatorSet::<T>::put(validators);
		}

		fn on_disabled(_i: u32) {}
	}
}
