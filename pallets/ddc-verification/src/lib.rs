//! # DDC Verification Pallet
//!
//! The DDC Verification pallet is used to validate zk-SNARK Proof and Signature
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
#![allow(clippy::missing_docs_in_private_items)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use core::str;

use ddc_primitives::{
	traits::{ClusterManager, NodeVisitor, ValidatorVisitor},
	ActivityHash, BatchIndex, ClusterId, CustomerUsage, DdcEra, NodeParams, NodePubKey, NodeUsage,
	PayoutState, StorageNodeMode, StorageNodeParams,
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
use sp_runtime::{
	offchain as rt_offchain,
	offchain::{http, StorageKind},
	traits::Hash,
	Percent,
};
use sp_std::{cmp::min, collections::btree_map::BTreeMap, prelude::*};

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
	/// The module configuration trait.
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The accounts's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Weight info type.
		type WeightInfo: WeightInfo;
		/// DDC clusters nodes manager.
		type ClusterManager: ClusterManager<Self>;
		/// DDC nodes read-only registry.
		type NodeVisitor: NodeVisitor<Self>;
		/// The output of the `ActivityHasher` function.
		type ActivityHash: Member
			+ Parameter
			+ MaybeSerializeDeserialize
			+ Ord
			+ Into<ActivityHash>
			+ From<ActivityHash>;
		/// The hashing system (algorithm)
		type ActivityHasher: Hash<Output = Self::ActivityHash>;
		/// The identifier type for an authority.
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ Into<sp_core::sr25519::Public>
			+ From<sp_core::sr25519::Public>;
		/// The identifier type for an offchain worker.
		type OffchainIdentifierId: AppCrypto<Self::Public, Self::Signature>;
		/// The majority of validators.
		const MAJORITY: u8;
		/// Block to start from.
		const BLOCK_TO_START: u32; // todo! rename to BLOCK_TO_MODULO
		/// The access to staking functionality.
		type Staking: StakingInterface<AccountId = Self::AccountId>;
		/// The access to validator list.
		type ValidatorList: SortedListProvider<Self::AccountId>;
	}

	/// The event type.
	#[pallet::event]
	/// The `generate_deposit` macro generates a function on `Pallet` called `deposit_event` which
	/// will properly convert the error type of your pallet into `RuntimeEvent` (recall `type
	/// RuntimeEvent: From<Event<Self>>`, so it can be converted) and deposit it via
	/// `frame_system::Pallet::deposit_event`.
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new billing report was created from `ClusterId` and `ERA`.
		BillingReportCreated {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		/// A verification key was stored with `VerificationKey`.
		VerificationKeyStored {
			verification_key: Vec<u8>,
		},
		/// A new payout batch was created from `ClusterId` and `ERA`.
		PayoutBatchCreated {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		EraValidationReady {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		/// Not enough nodes for consensus.
		NotEnoughNodesForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			id: ActivityHash,
			validator: T::AccountId,
		},
		/// No activity in consensus.
		ActivityNotInConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			id: ActivityHash,
			validator: T::AccountId,
		},
	}

	/// Consensus Errors
	#[derive(Debug, Encode, Decode, Clone, TypeInfo, PartialEq)]
	pub enum ConsensusError {
		/// Not enough nodes for consensus.
		NotEnoughNodesForConsensus { cluster_id: ClusterId, era_id: DdcEra, id: ActivityHash },
		/// No activity in consensus.
		ActivityNotInConsensus { cluster_id: ClusterId, era_id: DdcEra, id: ActivityHash },
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		/// Billing report already exists.
		BillingReportAlreadyExist,
		/// Bad verification key.
		BadVerificationKey,
		/// Bad requests.
		BadRequest,
		/// Not a validator.
		Unauthorised,
		/// Already signed era.
		AlreadySignedEra,
		/// Already signed payout batch.
		AlreadySignedPayoutBatch,
		/// Node Retrieval Error.
		NodeRetrievalError,
		/// Node Usage Retrieval Error.
		NodeUsageRetrievalError,
		/// Cluster To Validate Retrieval Error.
		ClusterToValidateRetrievalError,
		/// Era To Validate Retrieval Error.
		EraToValidateRetrievalError,
		/// Era Per Node Retrieval Error.
		EraPerNodeRetrievalError,
		/// Fail to fetch Ids.
		FailToFetchIds,
		/// No validator exists.
		NoValidatorExist,
		/// Not a controller.
		NotController,
		/// Not a validator stash.
		NotValidatorStash,
		/// DDC Validator Key Not Registered
		DDCValidatorKeyNotRegistered,
	}

	/// Active billing report of a cluster id and a validator.
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

	/// Payout data for a cluster id and an era.
	#[pallet::storage]
	#[pallet::getter(fn payout_batch)]
	pub type PayoutBatch<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ClusterId, Blake2_128Concat, DdcEra, PayoutData>;

	/// Payout validators.
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

	/// Payout validators.
	#[pallet::storage]
	#[pallet::getter(fn era_validations_in_progress)]
	pub type EraValidationsInProgress<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		(ClusterId, DdcEra),
		Blake2_128Concat,
		(ActivityHash, ActivityHash),
		Vec<T::AccountId>,
		ValueQuery,
	>;

	/// Era validation
	#[pallet::storage]
	#[pallet::getter(fn era_validations)]
	pub type EraValidations<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		DdcEra,
		EraValidation<T>,
	>;

	/// Cluster id storage
	#[pallet::storage]
	#[pallet::getter(fn cluster_to_validate)]
	pub type ClusterToValidate<T: Config> = StorageValue<_, ClusterId>; // todo! setter out of scope

	/// List of validators.
	#[pallet::storage]
	#[pallet::getter(fn validator_set)]
	pub type ValidatorSet<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	/// Validator stash key mapping
	#[pallet::storage]
	#[pallet::getter(fn get_stash_for_ddc_validator)]
	pub type ValidatorToStashKey<T: Config> = StorageMap<_, Identity, T::AccountId, T::AccountId>;

	/// ReceiptParams of an active billing report.
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)] // todo! rename to better naming
	pub struct ReceiptParams {
		/// DDC era.
		pub era: DdcEra,
		/// payers merkle root hash.
		pub payers_merkle_root_hash: ActivityHash,
		/// payees merkle root hash.
		pub payees_merkle_root_hash: ActivityHash,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub enum EraValidationStatus {
		ValidatingData, // todo! put it by the 1st OCW that starts to prepare validation
		ReadyForPayout,
		PayoutInProgress,
		PayoutFailed,
		PayoutSuccess,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct EraValidation<T: Config> {
		pub payers_merkle_root_hash: ActivityHash,
		pub payees_merkle_root_hash: ActivityHash,
		pub validation_status: EraValidationStatus,
		pub payout_state: PayoutState,
		pub validators: Vec<T::AccountId>, // todo! change to signatures
	}

	/// Era activity of a node.
	#[derive(Serialize, Copy, Deserialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
	pub(crate) struct EraActivity {
		/// Era id.
		pub id: DdcEra,
	}

	/// Node activity of a node.
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct NodeActivity {
		/// Node id.
		pub(crate) node_id: [u8; 32],
		/// Provider id.
		pub(crate) provider_id: [u8; 32],
		/// Total amount of stored bytes.
		pub(crate) stored_bytes: u64,
		/// Total amount of transferred bytes.
		pub(crate) transferred_bytes: u64,
		/// Total number of puts.
		pub(crate) number_of_puts: u64,
		/// Total number of gets.
		pub(crate) number_of_gets: u64,
	}

	/// Customer Activity of a bucket.
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct CustomerActivity {
		/// Customer id.
		pub(crate) customer_id: [u8; 32],
		/// Bucket id
		pub(crate) bucket_id: BucketId,
		/// Total amount of stored bytes.
		pub(crate) stored_bytes: u64,
		/// Total amount of transferred bytes.
		pub(crate) transferred_bytes: u64,
		/// Total number of puts.
		pub(crate) number_of_puts: u64,
		/// Total number of gets.
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

	/// Unwrap or send an error log
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
				return;
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

			let cluster_id = unwrap_or_log_error!(
				Self::get_cluster_to_validate(),
				"Error retrieving cluster to validate"
			);

			let dac_nodes = unwrap_or_log_error!(
				Self::get_dac_nodes(&cluster_id),
				"Error retrieving dac nodes to validate"
			);

			let min_nodes = dac_nodes.len().ilog2() as usize;
			if min_nodes < 3 {
				// todo!: factor out min nodes config
				return;
			}

			{
				// process batches for era
				let era_id_result = unwrap_or_log_error!(
					Self::get_era_to_prepare_for_payout(cluster_id, &dac_nodes),
					"Error retrieving era to validate"
				);

				if era_id_result.is_none() {
					return;
				}
				let era_id = era_id_result.unwrap();

				let (
					customers_activity_in_consensus,
					customers_activity_root,
					customers_activity_batch_roots,
					nodes_activity_in_consensus,
					nodes_activity_root,
					nodes_activity_batch_roots,
				) = match Self::fetch_activities::<CustomerActivity, NodeActivity>(&cluster_id, era_id)
				{
					Some(result) => result,
					None => {
						// todo! fetch data (fetch_nodes_usage_for_era,
						// fetch_customers_usage_for_era) and then use
						// get_consensus_for_activities
						// todo! set customers_activity_in_consensus and
						// nodes_activity_in_consensus
						Self::get_activities_in_consensus::<CustomerActivity, NodeActivity>(
							&cluster_id,
							era_id,
						);
						(
							vec![],
							ActivityHash::default(),
							vec![],
							vec![],
							ActivityHash::default(),
							vec![],
						)
					},
				};
			}

			{
				// prepare era for payout
				let era_id_result =
					Self::get_era_for_payout(&cluster_id, EraValidationStatus::ReadyForPayout);
				if era_id_result.is_none() {
					return;
				}
				let era_id = era_id_result.unwrap();

				// todo! factor this into separate method to be used in process batches for era
				// section as well
				let nodes_usage = unwrap_or_log_error!(
					Self::fetch_nodes_usage_for_era(&cluster_id, era_id, &dac_nodes),
					"Error retrieving node activities to validate"
				);

				let customers_usage = unwrap_or_log_error!(
					Self::fetch_customers_usage_for_era(&cluster_id, era_id, &dac_nodes),
					"Error retrieving customers activities to validate"
				);

				let customers_activity_result = match Self::get_consensus_for_activities(
					&cluster_id,
					era_id,
					&customers_usage,
					min_nodes,
					Percent::from_percent(T::MAJORITY),
				) {
					Ok(customers_activity_in_consensus) => {
						let leaves: Vec<ActivityHash> =
							Self::get_batch_hashes(&customers_activity_in_consensus, 100); // todo! factor batch_size into config through runtime

						Some((
							customers_activity_in_consensus,
							Self::create_merkle_root(&leaves),
							leaves,
						))
					},
					Err(errors) => {
						let results = signer.send_signed_transaction(|_account| {
							Call::emit_consensus_errors { errors: errors.clone() }
						});

						for (acc, res) in results {
							match res {
							Ok(_) => log::info!( // todo! expand with cluster, era
								"Successfully submitted emit_consensus_errors tx for customers_activity_root: {:?}",
								acc.id
							),
							Err(e) => log::error!(
								"Failed to submit emit_consensus_errors tx for customers_activity_root: {:?} ({:?})",
								acc.id,
								e
							),
						}
						}
						None
					},
				};

				if customers_activity_result.is_none() {
					return;
				}
				let (
					customers_activity_in_consensus,
					customers_activity_root,
					customers_activity_batch_roots,
				) = customers_activity_result.unwrap();

				let nodes_activity_result = match Self::get_consensus_for_activities(
					&cluster_id,
					era_id,
					&nodes_usage,
					min_nodes,
					Percent::from_percent(T::MAJORITY),
				) {
					Ok(nodes_activity_in_consensus) => {
						let leaves: Vec<ActivityHash> =
							Self::get_batch_hashes(&nodes_activity_in_consensus, 100); // todo! factor batch_size into config through runtime

						Some((
							nodes_activity_in_consensus,
							Self::create_merkle_root(&leaves),
							leaves,
						))
					},
					Err(errors) => {
						let results = signer.send_signed_transaction(|_account| {
							Call::emit_consensus_errors { errors: errors.clone() }
						});

						for (acc, res) in results {
							// todo! create macro for this type of loop
							match res {
							Ok(_) => log::info!( // todo! expand with cluster, era
								"Successfully submitted emit_consensus_errors tx for nodes_activit_root: {:?}",
								acc.id
							),
							Err(e) => log::error!(
								"Failed to submit emit_consensus_errors tx for nodes_activit_root: {:?} ({:?})",
								acc.id,
								e
							),
						}
						}
						None
					},
				};

				if nodes_activity_result.is_none() {
					return;
				}
				let (nodes_activity_in_consensus, nodes_activity_root, nodes_activity_batch_roots) =
					nodes_activity_result.unwrap();

				Self::store_activities(
					&cluster_id,
					era_id,
					&customers_activity_in_consensus,
					customers_activity_root,
					&customers_activity_batch_roots,
					&nodes_activity_in_consensus,
					nodes_activity_root,
					&nodes_activity_batch_roots,
				);

				let results =
					signer.send_signed_transaction(|_account| Call::set_prepare_era_for_payout {
						cluster_id,
						era: era_id,
						payers_merkle_root_hash: customers_activity_root,
						payees_merkle_root_hash: nodes_activity_root,
					});

				for (acc, res) in results {
					match res {
						Ok(_) => log::info!(
							// todo! expand with cluster, era
							"Successfully submitted set_validate_era tx: {:?}",
							acc.id
						),
						Err(e) => {
							log::error!(
								"Failed to submit set_validate_era tx: {:?} ({:?})",
								acc.id,
								e
							)
						},
					}
				}
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn get_activities_in_consensus<A: Activity, B: Activity>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
		) -> Result<(Vec<A>, Vec<B>), Vec<ConsensusError>> {
			unimplemented!()
		}

		pub(crate) fn derive_key(cluster_id: &ClusterId, era_id: DdcEra) -> Vec<u8> {
			format!("offchain::activities::{:?}::{:?}", cluster_id, era_id).into_bytes()
		}

		pub(crate) fn store_activities<A: Encode, B: Encode>(
			// todo! add tests
			cluster_id: &ClusterId,
			era_id: DdcEra,
			customers_activity_in_consensus: &[A],
			customers_activity_root: ActivityHash,
			customers_activity_batch_roots: &[ActivityHash],
			nodes_activity_in_consensus: &[B],
			nodes_activity_root: ActivityHash,
			nodes_activity_batch_roots: &[ActivityHash],
		) {
			let key = Self::derive_key(cluster_id, era_id);
			let encoded_tuple = (
				customers_activity_in_consensus,
				customers_activity_root,
				customers_activity_batch_roots,
				nodes_activity_in_consensus,
				nodes_activity_root,
				nodes_activity_batch_roots,
			)
				.encode();

			// Store the serialized data in local offchain storage
			sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, &key, &encoded_tuple);
		}

		pub(crate) fn fetch_activities<A: Decode, B: Decode>(
			// todo! add tests
			cluster_id: &ClusterId,
			era_id: DdcEra,
		) -> Option<(
			Vec<A>,
			ActivityHash,
			Vec<ActivityHash>,
			Vec<B>,
			ActivityHash,
			Vec<ActivityHash>,
		)> {
			let key = Self::derive_key(cluster_id, era_id);

			// Retrieve encoded tuple from local storage
			let encoded_tuple =
				match sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
					Some(data) => data,
					None => return None,
				};

			// Attempt to decode tuple from bytes
			match Decode::decode(&mut &encoded_tuple[..]) {
				Ok((
					customers_activity_in_consensus,
					customers_activity_root,
					customers_activity_batch_roots,
					nodes_activity_in_consensus,
					nodes_activity_root,
					nodes_activity_batch_roots,
				)) => Some((
					customers_activity_in_consensus,
					customers_activity_root,
					customers_activity_batch_roots,
					nodes_activity_in_consensus,
					nodes_activity_root,
					nodes_activity_batch_roots,
				)),
				Err(err) => {
					// Print error message with details of the decoding error
					log::error!("Decoding error: {:?}", err);
					None
				},
			}
		}

		pub(crate) fn get_batch_hashes<A: Activity>(
			// todo! add tests
			activities: &[A],
			batch_size: usize,
		) -> Vec<ActivityHash> {
			let batch_size = (activities.len() + batch_size - 1) / batch_size; // Equivalent to ceil(activities.len() / k)

			activities
				.iter()
				.sorted() // Sort the iterator - so that each time we'll receive the same roots
				.chunks(batch_size) // Split activities into chunks of batch_size
				.into_iter() // Convert the chunks iterator into an iterator
				.map(|chunk| {
					// For each chunk...
					let hashes: Vec<ActivityHash> =
						chunk.map(|activity| activity.hash::<T>()).collect();
					Self::create_merkle_root(&hashes) // Create Merkle root of each batch
				})
				.collect() // Collect all the Merkle roots into a final vector
		}

		/// Create merkle root of given leaves.
		///
		/// Parameters:
		/// - `leaves`: collection of leaf
		pub(crate) fn create_merkle_root(leaves: &[ActivityHash]) -> ActivityHash {
			// todo! add tests
			let leaves = leaves.iter().map(|l| l.to_vec()).collect::<Vec<Vec<u8>>>();

			T::ActivityHasher::ordered_trie_root(leaves, Default::default()).into()
		}

		pub(crate) fn get_era_for_payout(
			cluster_id: &ClusterId,
			status: EraValidationStatus,
		) -> Option<DdcEra> {
			let mut smallest_era_id: Option<DdcEra> = None;

			for (stored_cluster_id, era_id, validation) in EraValidations::<T>::iter() {
				if stored_cluster_id == *cluster_id && validation.status == status {
					smallest_era_id = match smallest_era_id {
						Some(current_smallest) => Some(min(current_smallest, era_id)),
						None => Some(era_id),
					};
				}
			}

			smallest_era_id
		}

		/// Fetch current era across all DAC nodes to validate.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `dac_nodes`: List of DAC nodes
		pub(crate) fn get_era_to_prepare_for_payout(
			// todo! this needs to be rewriten - too complex and inefficient
			cluster_id: ClusterId,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Option<DdcEra>, Error<T>> {
			let current_validator = T::NodeVisitor::get_current_validator();
			let last_validated_era = Self::active_billing_reports(cluster_id, current_validator)
				.ok_or(Error::EraToValidateRetrievalError)?;

			let all_ids = Self::fetch_processed_era_for_node(&dac_nodes)
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

		/// Reach consensus for ddc activity.
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

		/// Fetch consensus for given activities.
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

		/// Fetch cluster to validate.
		fn get_cluster_to_validate() -> Result<ClusterId, Error<T>> {
			// todo! to implement
			Self::cluster_to_validate().ok_or(Error::ClusterToValidateRetrievalError)
		}

		/// Fetch processed era.
		///
		/// Parameters:
		/// - `node_params`: DAC node parameters
		pub(crate) fn fetch_processed_era(
			node_params: &StorageNodeParams,
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
		/// Fetch customer usage.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
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

		/// Fetch node usage.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
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

		/// Fetch DAC nodes of a cluster.
		/// Parameters:
		/// - `cluster_id`: Cluster id of a cluster.
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

		/// Fetch node usage of an era.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
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

		/// Fetch customer usage for an era.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
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

		/// Fetch processed era for across all nodes.
		///
		/// Parameters:
		/// - `node_params`: DAC node parameters
		fn fetch_processed_era_for_node(
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
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
		/// Create billing reports from a public origin.
		///
		/// The origin must be Signed.
		///
		/// Parameters:
		/// - `cluster_id`: Cluster id of a cluster.
		/// - `era`: Era id.
		/// - `payers_merkle_root_hash`: Merkle root hash of payers
		/// - `payees_merkle_root_hash`: Merkle root hash of payees
		///
		/// Emits `BillingReportCreated` event when successful.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())]
		pub fn set_prepare_era_for_payout(
			// todo! add tests
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			payers_merkle_root_hash: ActivityHash,
			payees_merkle_root_hash: ActivityHash,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorised);

			ensure!(
				!<EraValidationsInProgress<T>>::get(
					(cluster_id, era),
					(payers_merkle_root_hash, payees_merkle_root_hash)
				)
				.contains(&caller.clone()),
				Error::<T>::AlreadySignedEra
			);

			// todo! if there was no any records for (cluster_id, era) in EraValidationsInProgress,
			// then put 'ValidatingData' status in EraValidations for (cluster_id, era)
			<EraValidationsInProgress<T>>::try_mutate(
				(cluster_id, era),
				(payers_merkle_root_hash, payees_merkle_root_hash),
				|validators| -> DispatchResult {
					validators.push(caller);
					Ok(())
				},
			)?;

			let percent = Percent::from_percent(T::MAJORITY);
			let threshold = percent * <ValidatorSet<T>>::get().len();

			let signed_validators = <EraValidationsInProgress<T>>::get(
				(cluster_id, era),
				(payers_merkle_root_hash, payees_merkle_root_hash),
			);

			if threshold < signed_validators.len() {
				EraValidations::<T>::insert(
					cluster_id,
					era,
					EraValidation {
						payers_merkle_root_hash,
						payees_merkle_root_hash,
						status: EraValidationStatus::ReadyForPayout,
						validators: signed_validators,
					},
				);

				// todo! delete from EraValidationsInProgress

				Self::deposit_event(Event::<T>::EraValidationReady { cluster_id, era });
			}

			Ok(())
		}

		/// Set payout batch.
		///
		/// The origin must be a validator.
		///
		/// Parameters:
		/// - `cluster_id`: Cluster id of a cluster
		/// - `era`: Era id
		/// - `payout_data`: Payout Data
		///
		/// Emits `PayoutBatchCreated` event when successful.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn set_validate_payout_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			payout_data: PayoutData,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorised);
			ensure!(
				!<PayoutValidators<T>>::get((cluster_id, era), payout_data.hash)
					.contains(&caller.clone()),
				Error::<T>::AlreadySignedPayoutBatch
			);

			<PayoutValidators<T>>::try_mutate(
				(cluster_id, era),
				payout_data.hash,
				|validators| -> DispatchResult {
					validators.push(caller);
					Ok(())
				},
			)?;

			let percent = Percent::from_percent(T::MAJORITY);
			let threshold = percent * <ValidatorSet<T>>::get().len();

			let signed_validators = <PayoutValidators<T>>::get((cluster_id, era), payout_data.hash);

			if threshold < signed_validators.len() {
				PayoutBatch::<T>::insert(cluster_id, era, payout_data);
				Self::deposit_event(Event::<T>::PayoutBatchCreated { cluster_id, era });
			}

			Ok(())
		}

		/// Emit consensus errors.
		///
		/// The origin must be a validator.
		///
		/// Parameters:
		/// - errors`: List of consensus errors
		///
		/// Emits `NotEnoughNodesForConsensus`  OR `ActivityNotInConsensus` event depend of error
		/// type, when successful.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn emit_consensus_errors(
			origin: OriginFor<T>,
			errors: Vec<ConsensusError>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorised);

			for error in errors {
				match error {
					ConsensusError::NotEnoughNodesForConsensus { cluster_id, era_id, id } => {
						Self::deposit_event(Event::NotEnoughNodesForConsensus {
							cluster_id,
							era_id,
							id,
							validator: caller.clone(),
						});
					},
					ConsensusError::ActivityNotInConsensus { cluster_id, era_id, id } => {
						Self::deposit_event(Event::ActivityNotInConsensus {
							cluster_id,
							era_id,
							id,
							validator: caller.clone(),
						});
					},
				}
			}

			Ok(())
		}

		/// Set validator key.
		///
		/// The origin must be a validator.
		///
		/// Parameters:
		/// - `ddc_validator_pub`: validator Key
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
