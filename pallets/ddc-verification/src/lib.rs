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
	traits::{ClusterManager, NodeVisitor, PayoutVisitor, ValidatorVisitor},
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
use polkadot_ckb_merkle_mountain_range::{
	util::{MemMMR, MemStore},
	MerkleProof, MMR,
};
use scale_info::prelude::{format, string::String};
use serde::{Deserialize, Serialize};
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::{
	offchain as rt_offchain,
	offchain::{http, StorageKind},
	traits::Hash,
	Percent,
};
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
	use ddc_primitives::{BucketId, MergeActivityHash, KEY_TYPE};
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
		type PayoutVisitor: PayoutVisitor<Self>;
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
		const BLOCK_TO_START: u16;
		const MIN_DAC_NODES_FOR_CONSENSUS: u16;
		const MAX_PAYOUT_BATCH_COUNT: u16;
		const MAX_PAYOUT_BATCH_SIZE: u16;
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
			era_id: DdcEra,
		},
		/// A verification key was stored with `VerificationKey`.
		VerificationKeyStored {
			verification_key: Vec<u8>,
		},
		/// A new payout batch was created from `ClusterId` and `ERA`.
		PayoutBatchCreated {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		EraValidationReady {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		EraValidationNotReady {
			cluster_id: ClusterId,
			era_id: DdcEra,
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
		/// Node Usage Retrieval Error.
		NodeUsageRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			node_pub_key: NodePubKey,
			validator: T::AccountId,
		},
		/// Customer Usage Retrieval Error.
		CustomerUsageRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			node_pub_key: NodePubKey,
			validator: T::AccountId,
		},
		EraRetrievalError {
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
			validator: T::AccountId,
		},
		PrepareEraTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			payers_merkle_root_hash: ActivityHash,
			payees_merkle_root_hash: ActivityHash,
			validator: T::AccountId,
		},
		BeginBillingReportTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
		BeginChargingCustomersTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
		SendChargingCustomersBatchTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
			validator: T::AccountId,
		},
		EndChargingCustomersTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
		BeginRewardingProvidersTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
		EndRewardingProvidersTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
		EndBillingReportTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
		BillingReportDoesNotExist {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
		EmptyCustomerActivity {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
		BatchIndexConversionFailed {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
		NoAvailableSigner {
			validator: T::AccountId,
		},
		NotEnoughDACNodes {
			num_nodes: u16,
			validator: T::AccountId,
		},
		FailedToCreateMerkleRoot {
			validator: T::AccountId,
		},
		FailedToFetchCurrentValidator {
			validator: T::AccountId,
		},
	}

	/// Consensus Errors
	#[derive(Debug, Encode, Decode, Clone, TypeInfo, PartialEq)]
	pub enum OCWError {
		/// Not enough nodes for consensus.
		NotEnoughNodesForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			id: ActivityHash,
		},
		/// No activity in consensus.
		ActivityNotInConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			id: ActivityHash,
		},
		/// Node Usage Retrieval Error.
		NodeUsageRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			node_pub_key: NodePubKey,
		},
		/// Customer Usage Retrieval Error.
		CustomerUsageRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			node_pub_key: NodePubKey,
		},
		EraRetrievalError {
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
		},
		PrepareEraTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			payers_merkle_root_hash: ActivityHash,
			payees_merkle_root_hash: ActivityHash,
		},
		BeginBillingReportTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		BeginChargingCustomersTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		SendChargingCustomersBatchTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
		},
		EndChargingCustomersTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		BeginRewardingProvidersTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		EndRewardingProvidersTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		EndBillingReportTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		BillingReportDoesNotExist {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		EmptyCustomerActivity {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		BatchIndexConversionFailed {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		NoAvailableSigner,
		NotEnoughDACNodes {
			num_nodes: u16,
		},
		FailedToCreateMerkleRoot,
		FailedToFetchCurrentValidator,
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
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
		TransactionSubmissionError,
		NoAvailableSigner,
		/// Fail to generate proof
		FailToGenerateProof,
		/// Fail to verify merkle proof
		FailToVerifyMerkleProof,
		/// No Era Validation exist
		NoEraValidation,
	}

	/// Era validations
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
	pub type ClusterToValidate<T: Config> = StorageValue<_, ClusterId>;

	/// List of validators.
	#[pallet::storage]
	#[pallet::getter(fn validator_set)]
	pub type ValidatorSet<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	/// Validator stash key mapping
	#[pallet::storage]
	#[pallet::getter(fn get_stash_for_ddc_validator)]
	pub type ValidatorToStashKey<T: Config> = StorageMap<_, Identity, T::AccountId, T::AccountId>;
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub enum EraValidationStatus {
		ValidatingData,
		ReadyForPayout,
		PayoutInProgress,
		PayoutFailed,
		PayoutSuccess,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct EraValidation<T: Config> {
		pub validators: BTreeMap<(ActivityHash, ActivityHash), Vec<T::AccountId>>, /* todo! change to signatures (T::AccountId, Signature) */
		pub start_era: i64,
		pub end_era: i64,
		pub payers_merkle_root_hash: ActivityHash,
		pub payees_merkle_root_hash: ActivityHash,
		pub status: EraValidationStatus,
	}

	/// Era activity of a node.
	#[derive(Serialize, Copy, Deserialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
	pub(crate) struct EraActivity {
		/// Era id.
		pub id: DdcEra,
		pub start: i64,
		pub end: i64,
	}

	/// Node activity of a node.
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct NodeActivity {
		/// Node id.
		pub(crate) node_id: String,
		/// Provider id.
		pub(crate) provider_id: String,
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
		pub(crate) customer_id: String,
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
			T::ActivityHasher::hash(self.node_id.as_bytes()).into()
		}

		fn hash<T: Config>(&self) -> ActivityHash {
			T::ActivityHasher::hash(&self.encode()).into()
		}
	}
	impl Activity for CustomerActivity {
		fn get_consensus_id<T: Config>(&self) -> ActivityHash {
			let mut data = self.customer_id.as_bytes().to_vec();
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

			let signer = Signer::<T, T::OffchainIdentifierId>::any_account();
			if !signer.can_sign() {
				log::error!("🚨No OCW is available.");
				return;
			}

			if Self::fetch_current_validator().is_err() {
				log::info!("🏄‍ Setting current validator...");
				let _ = signer.send_signed_transaction(|_account| Call::set_current_validator {});
			}

			if (block_number.saturated_into::<u32>() % T::BLOCK_TO_START as u32) != 0 {
				return;
			}

			log::info!("👋 Hello from pallet-ddc-verification.");

			let cluster_id = unwrap_or_log_error!(
				Self::get_cluster_to_validate(),
				"🦀🦀🦀🦀 Error retrieving cluster to validate 🦀🦀🦀🦀"
			);

			let dac_nodes = unwrap_or_log_error!(
				Self::get_dac_nodes(&cluster_id),
				"🦀🦀🦀🦀 Error retrieving dac nodes to validate 🦀🦀🦀🦀"
			);

			let min_nodes = T::MIN_DAC_NODES_FOR_CONSENSUS;
			let batch_size = T::MAX_PAYOUT_BATCH_SIZE;
			let mut errors: Vec<OCWError> = Vec::new();

			let processed_dac_data =
				Self::process_dac_data(&cluster_id, None, &dac_nodes, min_nodes, batch_size.into());

			match processed_dac_data {
				Ok(Some((era_id, payers_merkle_root_hash, payees_merkle_root_hash))) => {
					log::info!(
						"🚀 Processing era_id: {:?} for cluster_id: {:?}",
						era_id,
						cluster_id
					);

					let results = signer.send_signed_transaction(|_account| {
						Call::set_prepare_era_for_payout {
							cluster_id,
							era_id,
							payers_merkle_root_hash,
							payees_merkle_root_hash,
						}
					});

					for (_, res) in &results {
						match res {
							Ok(()) => {
								log::info!(
										"⛳️ Merkle roots posted on-chain for cluster_id: {:?}, era_id: {:?}",
										cluster_id,
										era_id
									);
							},
							Err(e) => {
								log::error!(
										"🦀 Error to post merkle roots on-chain for cluster_id: {:?}, era_id: {:?}: {:?}",
										cluster_id,
										era_id,
										e
									);
								// Extrinsic call failed
								errors.push(OCWError::PrepareEraTransactionError {
									cluster_id,
									era_id,
									payers_merkle_root_hash,
									payees_merkle_root_hash,
								});
							},
						}
					}
				},
				Ok(None) => {
					log::info!("ℹ️ No eras for DAC process for cluster_id: {:?}", cluster_id);
				},
				Err(process_errors) => {
					errors.extend(process_errors);
				},
			};

			// todo! factor out as macro as this is repetitive
			match Self::prepare_begin_billing_report(&cluster_id) {
				Ok(Some((era_id, start_era, end_era))) => {
					log::info!(
						"🚀 process_start_payout processed successfully for cluster_id: {:?}, era_id: {:?}",
						cluster_id,
						era_id
					);

					if let Some((_, res)) = signer.send_signed_transaction(|_acc| {
						Call::begin_billing_report { cluster_id, era_id, start_era, end_era }
					}) {
						match res {
							Ok(_) => {
								// Extrinsic call succeeded
								log::info!(
									"Sent begin_billing_report successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"Error to post begin_billing_report for cluster_id: {:?}, era_id: {:?}: {:?}",
										cluster_id,
										era_id,
										e
									);
								// Extrinsic call failed
								errors.push(OCWError::BeginBillingReportTransactionError {
									cluster_id,
									era_id,
								});
							},
						}
					} else {
						log::error!("No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!("No era for payout for cluster_id: {:?}", cluster_id);
				},
				Err(e) => {
					errors.push(e);
				},
			}

			// todo! factor out as macro as this is repetitive
			match Self::prepare_begin_charging_customers(&cluster_id) {
				Ok(Some((era_id, max_batch_index))) => {
					log::info!(
						"prepare_begin_charging_customers processed successfully for cluster_id: {:?}, era_id: {:?}",
						cluster_id,
						era_id
					);

					if let Some((_, res)) = signer.send_signed_transaction(|_acc| {
						Call::begin_charging_customers { cluster_id, era_id, max_batch_index }
					}) {
						match res {
							Ok(_) => {
								// Extrinsic call succeeded
								log::info!(
									"Sent begin_charging_customers successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"Error to post begin_charging_customers for cluster_id: {:?}, era_id: {:?}: {:?}",
										cluster_id,
										era_id,
										e
									);
								// Extrinsic call failed
								errors.push(OCWError::BeginChargingCustomersTransactionError {
									cluster_id,
									era_id,
								});
							},
						}
					} else {
						log::error!("No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!(
						"No era for begin_charging_customers for cluster_id: {:?}",
						cluster_id
					);
				},
				Err(e) => {
					errors.push(e);
				},
			}

			// todo! factor out as macro as this is repetitive
			match Self::prepare_end_charging_customers(&cluster_id) {
				Ok(Some(era_id)) => {
					log::info!(
						"prepare_end_charging_customers processed successfully for cluster_id: {:?}, era_id: {:?}",
						cluster_id,
						era_id
					);

					if let Some((_, res)) = signer.send_signed_transaction(|_acc| {
						Call::end_charging_customers { cluster_id, era_id }
					}) {
						match res {
							Ok(_) => {
								// Extrinsic call succeeded
								log::info!(
									"Sent end_charging_customers successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"Error to post end_charging_customers for cluster_id: {:?}, era_id: {:?}: {:?}",
										cluster_id,
										era_id,
										e
									);
								// Extrinsic call failed
								errors.push(OCWError::EndChargingCustomersTransactionError {
									cluster_id,
									era_id,
								});
							},
						}
					} else {
						log::error!("No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!(
						"No era for end_charging_customers for cluster_id: {:?}",
						cluster_id
					);
				},
				Err(e) => {
					errors.push(e);
				},
			}

			// todo! factor out as macro as this is repetitive
			match Self::prepare_begin_rewarding_providers(&cluster_id) {
				Ok(Some((era_id, max_batch_index, total_node_usage))) => {
					log::info!(
						"prepare_begin_rewarding_providers processed successfully for cluster_id: {:?}, era_id: {:?}",
						cluster_id,
						era_id
					);

					if let Some((_, res)) =
						signer.send_signed_transaction(|_acc| Call::begin_rewarding_providers {
							cluster_id,
							era_id,
							max_batch_index,
							total_node_usage: total_node_usage.clone(),
						}) {
						match res {
							Ok(_) => {
								// Extrinsic call succeeded
								log::info!(
									"Sent begin_rewarding_providers successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"Error to post begin_rewarding_providers for cluster_id: {:?}, era_id: {:?}: {:?}",
										cluster_id,
										era_id,
										e
									);
								// Extrinsic call failed
								errors.push(OCWError::BeginRewardingProvidersTransactionError {
									cluster_id,
									era_id,
								});
							},
						}
					} else {
						log::error!("No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!(
						"No era for begin_rewarding_providers for cluster_id: {:?}",
						cluster_id
					);
				},
				Err(e) => {
					errors.push(e);
				},
			}

			// todo! factor out as macro as this is repetitive
			match Self::prepare_end_rewarding_providers(&cluster_id) {
				Ok(Some(era_id)) => {
					log::info!(
						"prepare_end_rewarding_providers processed successfully for cluster_id: {:?}, era_id: {:?}",
						cluster_id,
						era_id
					);

					if let Some((_, res)) = signer.send_signed_transaction(|_acc| {
						Call::end_rewarding_providers { cluster_id, era_id }
					}) {
						match res {
							Ok(_) => {
								// Extrinsic call succeeded
								log::info!(
									"Sent end_rewarding_providers successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"Error to post end_rewarding_providers for cluster_id: {:?}, era_id: {:?}: {:?}",
										cluster_id,
										era_id,
										e
									);
								// Extrinsic call failed
								errors.push(OCWError::EndRewardingProvidersTransactionError {
									cluster_id,
									era_id,
								});
							},
						}
					} else {
						log::error!("No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!(
						"No era for end_rewarding_providers for cluster_id: {:?}",
						cluster_id
					);
				},
				Err(e) => {
					errors.push(e);
				},
			}

			// todo! factor out as macro as this is repetitive
			match Self::prepare_end_billing_report(&cluster_id) {
				Ok(Some(era_id)) => {
					log::info!(
						"prepare_end_billing_report processed successfully for cluster_id: {:?}, era_id: {:?}",
						cluster_id,
						era_id
					);

					if let Some((_, res)) = signer.send_signed_transaction(|_acc| {
						Call::end_billing_report { cluster_id, era_id }
					}) {
						match res {
							Ok(_) => {
								// Extrinsic call succeeded
								log::info!(
									"Sent end_billing_report successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"Error to post end_billing_report for cluster_id: {:?}, era_id: {:?}: {:?}",
										cluster_id,
										era_id,
										e
									);
								// Extrinsic call failed
								errors.push(OCWError::EndBillingReportTransactionError {
									cluster_id,
									era_id,
								});
							},
						}
					} else {
						log::error!("No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!("No era for end_billing_report for cluster_id: {:?}", cluster_id);
				},
				Err(e) => {
					errors.push(e);
				},
			}

			if !errors.is_empty() {
				let results = signer.send_signed_transaction(|_account| {
					Call::emit_consensus_errors { errors: errors.clone() }
				});

				for (_, res) in &results {
					match res {
						Ok(()) => log::info!("✅ Successfully submitted emit_consensus_errors tx"),
						Err(_) => log::error!("🦀 Failed to submit emit_consensus_errors tx"),
					}
				}
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn process_dac_data(
			cluster_id: &ClusterId,
			era_id_to_process: Option<DdcEra>,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
			min_nodes: u16,
			batch_size: usize,
		) -> Result<Option<(DdcEra, ActivityHash, ActivityHash)>, Vec<OCWError>> {
			log::info!("🚀 Processing dac data for cluster_id: {:?}", cluster_id);
			if dac_nodes.len().ilog2() < min_nodes.into() {
				return Err(vec![OCWError::NotEnoughDACNodes { num_nodes: min_nodes }]);
			}

			let era_id = if let Some(era_id) = era_id_to_process {
				era_id
			} else {
				match Self::get_era_for_validation(cluster_id, dac_nodes) {
					Ok(Some(era_id)) => era_id,
					Ok(None) => return Ok(None),
					Err(err) => return Err(vec![err]),
				}
			};

			let nodes_usage = Self::fetch_nodes_usage_for_era(cluster_id, era_id, dac_nodes)
				.map_err(|err| vec![err])?;
			let customers_usage =
				Self::fetch_customers_usage_for_era(cluster_id, era_id, dac_nodes)
					.map_err(|err| vec![err])?;

			let customers_activity_in_consensus = Self::get_consensus_for_activities(
				cluster_id,
				era_id,
				&customers_usage,
				min_nodes,
				Percent::from_percent(T::MAJORITY),
			)?;

			let customers_activity_batch_roots = Self::convert_to_batch_merkle_roots(
				Self::split_to_batches(&customers_activity_in_consensus, batch_size),
			)
			.map_err(|err| vec![err])?;

			let customers_activity_root = Self::create_merkle_root(&customers_activity_batch_roots)
				.map_err(|err| vec![err])?;

			let nodes_activity_in_consensus = Self::get_consensus_for_activities(
				cluster_id,
				era_id,
				&nodes_usage,
				min_nodes,
				Percent::from_percent(T::MAJORITY),
			)?;

			let nodes_activity_batch_roots = Self::convert_to_batch_merkle_roots(
				Self::split_to_batches(&customers_activity_in_consensus, batch_size),
			)
			.map_err(|err| vec![err])?;

			let nodes_activity_root =
				Self::create_merkle_root(&nodes_activity_batch_roots).map_err(|err| vec![err])?;

			Self::store_validation_activities(
				cluster_id,
				era_id,
				&customers_activity_in_consensus,
				customers_activity_root,
				&customers_activity_batch_roots,
				&nodes_activity_in_consensus,
				nodes_activity_root,
				&nodes_activity_batch_roots,
			);
			log::info!("🙇‍ Dac data processing completed for cluster_id: {:?}", cluster_id);
			Ok(Some((era_id, customers_activity_root, nodes_activity_root)))
		}

		// let batches = split into batches customers_activity_in_consensus
		// for i in batches.len() {
		// let batch  = batches[i];
		// let batch_root1 = customers_activity_batch_roots[i]; // C
		// let batch_root2 = create_merkle_tree(batch)
		// assert!(batch_root1, batch_root2)
		// let adjacent_hashes = get_adjacent_hashes(batch_root1, customers_activity_root,
		// customers_activity_batch_roots) // provide hash(D) and hash(A,B).
		// call payout::send_charging_customers_batch(clusterid, era_id, batch_index, batch,
		// adjacent_hashes) }

		#[allow(dead_code)]
		pub(crate) fn prepare_begin_billing_report(
			cluster_id: &ClusterId,
		) -> Result<Option<(DdcEra, i64, i64)>, OCWError> {
			Ok(Self::get_era_for_payout(cluster_id, EraValidationStatus::ReadyForPayout))
			// todo! get start and end values based on result
		}

		pub(crate) fn prepare_begin_charging_customers(
			cluster_id: &ClusterId,
		) -> Result<Option<(DdcEra, BatchIndex)>, OCWError> {
			if let Some((era_id, _start, _end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::Initialized
				{
					if let Some((_, _, customers_activity_batch_roots, _, _, _)) =
						Self::fetch_validation_activities::<CustomerActivity, NodeActivity>(
							cluster_id, era_id,
						) {
						if let Some(max_batch_index) =
							customers_activity_batch_roots.len().checked_sub(1)
						// todo! remove this -1 to support empty usage
						{
							let max_batch_index: u16 =
								max_batch_index.try_into().map_err(|_| {
									OCWError::BatchIndexConversionFailed {
										cluster_id: *cluster_id,
										era_id,
									}
								})?;
							return Ok(Some((era_id, max_batch_index)));
						} else {
							return Err(OCWError::EmptyCustomerActivity {
								cluster_id: *cluster_id,
								era_id,
							});
						}
					} /*else {
						 // todo! no data - reconstruct the data from DAC
					 }*/
				}
			}
			Ok(None)
		}

		#[allow(dead_code)]
		pub(crate) fn prepare_send_charging_customers_batch(
			cluster_id: &ClusterId,
		) -> Result<Option<(DdcEra, BatchIndex)>, OCWError> {
			if let Some((era_id, _start, _end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::ChargingCustomers
				{
					if let Some((
						_customers_activity_in_consensus,
						_customers_activity_root,
						_customers_activity_batch_roots,
						_,
						_,
						_,
					)) = Self::fetch_validation_activities::<CustomerActivity, NodeActivity>(
						cluster_id, era_id,
					) {
						let _batch_index = T::PayoutVisitor::get_next_customer_batch_for_payment(
							cluster_id, era_id,
						)
						.map_err(|_| OCWError::BillingReportDoesNotExist {
							cluster_id: *cluster_id,
							era_id,
						})?;
						// todo! get root for a batch_index
						// todo! get batch by batch_index
						// todo! create proof
					} /*else {
						 // todo! no data - reconstruct the data from DAC
					 }*/
				}
			}
			Ok(None)
		}

		pub(crate) fn prepare_end_charging_customers(
			cluster_id: &ClusterId,
		) -> Result<Option<DdcEra>, OCWError> {
			if let Some((era_id, _start, _end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::ChargingCustomers &&
					T::PayoutVisitor::all_customer_batches_processed(cluster_id, era_id)
				{
					return Ok(Some(era_id));
				}
			}
			Ok(None)
		}

		pub(crate) fn prepare_begin_rewarding_providers(
			cluster_id: &ClusterId,
		) -> Result<Option<(DdcEra, BatchIndex, NodeUsage)>, OCWError> {
			if let Some((era_id, _start, _end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::CustomersChargedWithFees
				{
					if let Some((
						_,
						_,
						_,
						nodes_activity_in_consensus,
						_,
						nodes_activity_batch_roots,
					)) = Self::fetch_validation_activities::<CustomerActivity, NodeActivity>(
						cluster_id, era_id,
					) {
						if let Some(max_batch_index) =
							nodes_activity_batch_roots.len().checked_sub(1)
						{
							// todo! remove this -1 to support empty usage
							let max_batch_index: u16 =
								max_batch_index.try_into().map_err(|_| {
									OCWError::BatchIndexConversionFailed {
										cluster_id: *cluster_id,
										era_id,
									}
								})?;

							let total_node_usage = nodes_activity_in_consensus.into_iter().fold(
								NodeUsage {
									transferred_bytes: 0,
									stored_bytes: 0,
									number_of_puts: 0,
									number_of_gets: 0,
								},
								|mut acc, activity| {
									acc.transferred_bytes += activity.transferred_bytes;
									acc.stored_bytes += activity.stored_bytes;
									acc.number_of_puts += activity.number_of_puts;
									acc.number_of_gets += activity.number_of_gets;
									acc
								},
							);

							return Ok(Some((era_id, max_batch_index, total_node_usage)));
						} else {
							return Err(OCWError::EmptyCustomerActivity {
								cluster_id: *cluster_id,
								era_id,
							});
						}
					} /*else {
						 // todo! no data - reconstruct the data from DAC
					 }*/
				}
			}
			Ok(None)
		}

		pub(crate) fn prepare_end_rewarding_providers(
			cluster_id: &ClusterId,
		) -> Result<Option<DdcEra>, OCWError> {
			if let Some((era_id, _start, _end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::RewardingProviders &&
					T::PayoutVisitor::all_provider_batches_processed(cluster_id, era_id)
				{
					return Ok(Some(era_id));
				}
			}
			Ok(None)
		}

		pub(crate) fn prepare_end_billing_report(
			cluster_id: &ClusterId,
		) -> Result<Option<DdcEra>, OCWError> {
			if let Some((era_id, _start, _end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::ProvidersRewarded
				{
					return Ok(Some(era_id));
				}
			}
			Ok(None)
		}

		pub(crate) fn derive_key(cluster_id: &ClusterId, era_id: DdcEra) -> Vec<u8> {
			format!("offchain::activities::{:?}::{:?}", cluster_id, era_id).into_bytes()
		}

		pub(crate) fn store_current_validator(validator: Vec<u8>) {
			let key = format!("offchain::validator::{:?}", KEY_TYPE).into_bytes();
			sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, &key, &validator);
		}

		pub(crate) fn fetch_current_validator() -> Result<Vec<u8>, OCWError> {
			let key = format!("offchain::validator::{:?}", KEY_TYPE).into_bytes();

			match sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
				Some(data) => Ok(data),
				None => Err(OCWError::FailedToFetchCurrentValidator),
			}
		}

		#[allow(clippy::too_many_arguments)] // todo! (2) refactor into 2 different methods (for customers and nodes) + use type info for
									 // derive_key
									 // todo! introduce new struct for input and remove clippy::type_complexity
		pub(crate) fn store_validation_activities<A: Encode, B: Encode>(
			// todo! (3) add tests
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

		#[allow(clippy::type_complexity)]
		pub(crate) fn fetch_validation_activities<A: Decode, B: Decode>(
			// todo! (4) add tests
			// todo! introduce new struct for results and remove clippy::type_complexity
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
					log::error!("🦀Decoding error: {:?}", err);
					None
				},
			}
		}

		/// Converts a vector of activity batches into their corresponding Merkle root hashes.
		///
		/// This function takes a vector of activity batches, where each batch is a vector of
		/// activities. It computes the Merkle root for each batch by first hashing each activity
		/// and then combining these hashes into a single Merkle root.
		///
		/// # Input Parameters
		/// - `activities: Vec<Vec<A>>`: A vector of vectors, where each inner vector represents a
		///   batch of activities.
		///
		/// # Output
		/// - `Vec<ActivityHash>`: A vector of Merkle root hashes, one for each batch of activities.
		pub(crate) fn convert_to_batch_merkle_roots<A: Activity>(
			activities: Vec<Vec<A>>,
		) -> Result<Vec<ActivityHash>, OCWError> {
			activities
				.into_iter()
				.map(|inner_vec| {
					let activity_hashes: Vec<ActivityHash> =
						inner_vec.into_iter().map(|a| a.hash::<T>()).collect();
					Self::create_merkle_root(&activity_hashes)
						.map_err(|_| OCWError::FailedToCreateMerkleRoot)
				})
				.collect::<Result<Vec<ActivityHash>, OCWError>>()
		}

		/// Splits a slice of activities into batches of a specified size.
		///
		/// This function sorts the given activities and splits them into batches of the specified
		/// size. Each batch is returned as a separate vector.
		///
		/// # Input Parameters
		/// - `activities: &[A]`: A slice of activities to be split into batches.
		/// - `batch_size: usize`: The size of each batch.
		///
		/// # Output
		/// - `Vec<Vec<A>>`: A vector of vectors, where each inner vector is a batch of activities.
		pub(crate) fn split_to_batches<A: Activity>(
			activities: &[A],
			batch_size: usize,
		) -> Vec<Vec<A>> {
			if activities.is_empty() {
				return vec![];
			}
			// Sort the activities first
			let mut sorted_activities = activities.to_vec();
			sorted_activities.sort(); // Sort using the derived Ord trait

			// Split the sorted activities into chunks and collect them into vectors
			sorted_activities.chunks(batch_size).map(|chunk| chunk.to_vec()).collect()
		}

		/// Create merkle root of given leaves.
		///
		/// Parameters:
		/// - `leaves`: collection of leaf
		pub(crate) fn create_merkle_root(
			leaves: &[ActivityHash],
		) -> Result<ActivityHash, OCWError> {
			let store = MemStore::default();
			let mut mmr: MMR<ActivityHash, MergeActivityHash, &MemStore<ActivityHash>> =
				MemMMR::<_, MergeActivityHash>::new(0, &store);
			let _ = leaves.iter().map(|a| mmr.push(*a)).collect::<Vec<_>>();
			mmr.get_root().map_err(|_| OCWError::FailedToCreateMerkleRoot)
		}

		/// Verify whether leaf is part of tree
		///
		/// Parameters:
		/// - `root`: merkle root
		/// - `leaf`: Leaf of the tree
		pub(crate) fn proof_merkle_leaf(
			root: ActivityHash,
			proof: MerkleProof<ActivityHash, MergeActivityHash>,
			leaf_with_position: Vec<(u64, ActivityHash)>,
		) -> Result<bool, Error<T>> {
			proof
				.verify(root, leaf_with_position)
				.map_err(|_| Error::<T>::FailToVerifyMerkleProof)
		}

		// todo! simplify method by removing start/end from the result
		pub(crate) fn get_era_for_payout(
			cluster_id: &ClusterId,
			status: EraValidationStatus,
		) -> Option<(DdcEra, i64, i64)> {
			let mut smallest_era_id: Option<DdcEra> = None;
			let mut start_era: i64 = Default::default();
			let mut end_era: i64 = Default::default();

			for (stored_cluster_id, era_id, validation) in EraValidations::<T>::iter() {
				if stored_cluster_id == *cluster_id &&
					validation.status == status &&
					(smallest_era_id.is_none() || era_id < smallest_era_id.unwrap())
				{
					smallest_era_id = Some(era_id);
					start_era = validation.start_era;
					end_era = validation.end_era;
				}
			}

			smallest_era_id.map(|era_id| (era_id, start_era, end_era))
		}

		/// Retrieves the last era in which the specified validator participated for a given
		/// cluster.
		///
		/// This function iterates through all eras in `EraValidations` for the given `cluster_id`,
		/// filtering for eras where the specified `validator` is present in the validators list.
		/// It returns the maximum era found where the validator participated.
		///
		/// # Input Parameters
		/// - `cluster_id: &ClusterId`: The ID of the cluster to check for the validator's
		///   participation.
		/// - `validator: T::AccountId`: The account ID of the validator whose participation is
		///   being checked.
		///
		/// # Output
		/// - `Result<Option<DdcEra>, OCWError>`:
		///   - `Ok(Some(DdcEra))`: The maximum era in which the validator participated.
		///   - `Ok(None)`: The validator did not participate in any era for the given cluster.
		///   - `Err(OCWError)`: An error occurred while retrieving the data.
		// todo! add tests for start and end era
		pub(crate) fn get_last_validated_era(
			cluster_id: &ClusterId,
			validator: T::AccountId,
		) -> Result<Option<DdcEra>, OCWError> {
			let mut max_era: Option<DdcEra> = None;

			// Iterate through all eras in EraValidations for the given cluster_id
			<EraValidations<T>>::iter_prefix(cluster_id)
				.filter_map(|(era, validation)| {
					// Filter for validators that contain the given validator
					if validation
						.validators
						.values()
						.any(|validators| validators.contains(&validator))
					{
						Some(era)
					} else {
						None
					}
				})
				.for_each(|era| {
					// Update max_era to the maximum era found
					if let Some(current_max) = max_era {
						if era > current_max {
							max_era = Some(era);
						}
					} else {
						max_era = Some(era);
					}
				});

			Ok(max_era)
		}

		/// Fetch current era across all DAC nodes to validate.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `dac_nodes`: List of DAC nodes
		pub(crate) fn get_era_for_validation(
			// todo! this needs to be rewriten - too complex and inefficient
			cluster_id: &ClusterId,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Option<DdcEra>, OCWError> {
			let current_validator_data = Self::fetch_current_validator()?;

			let current_validator = T::AccountId::decode(&mut &current_validator_data[..]).unwrap();

			let last_validated_era = Self::get_last_validated_era(cluster_id, current_validator)?
				.unwrap_or_else(DdcEra::default);

			let all_ids = Self::fetch_processed_era_for_node(cluster_id, dac_nodes)?;

			let ids_greater_than_last_validated_era: Vec<DdcEra> = all_ids
				.iter()
				.flat_map(|eras| {
					eras.iter().cloned().filter(|ids| ids.id > last_validated_era).map(|era| era.id)
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

		/// Determines if a consensus is reached for a set of activities based on a specified
		/// threshold.
		///
		/// This function counts the occurrences of each activity in the provided list and checks if
		/// any activity's count meets or exceeds the given threshold. If such an activity is found,
		/// it is returned.
		///
		/// # Input Parameters
		/// - `activities: &[A]`: A slice of activities to be analyzed for consensus.
		/// - `threshold: usize`: The minimum number of occurrences required for an activity to be
		///   considered in consensus.
		///
		/// # Output
		/// - `Option<A>`:
		///   - `Some(A)`: An activity that has met or exceeded the threshold.
		///   - `None`: No activity met the threshold.
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

		/// Computes the consensus for a set of activities across multiple nodes within a given
		/// cluster and era.
		///
		/// This function collects activities from various nodes, groups them by their consensus ID,
		/// and then determines if a consensus is reached for each group based on the minimum number
		/// of nodes and a given threshold. If the consensus is reached, the activity is included
		/// in the result. Otherwise, appropriate errors are returned.
		///
		/// # Input Parameters
		/// - `cluster_id: &ClusterId`: The ID of the cluster for which consensus is being computed.
		/// - `era_id: DdcEra`: The era ID within the cluster.
		/// - `activities: &[(NodePubKey, Vec<A>)]`: A list of tuples, where each tuple contains a
		///   node's public key and a vector of activities reported by that node.
		/// - `min_nodes: u16`: The minimum number of nodes that must report an activity for it to
		///   be considered for consensus.
		/// - `threshold: Percent`: The threshold percentage that determines if an activity is in
		///   consensus.
		///
		/// # Output
		/// - `Result<Vec<A>, Vec<OCWError>>`:
		///   - `Ok(Vec<A>)`: A vector of activities that have reached consensus.
		///   - `Err(Vec<OCWError>)`: A vector of errors indicating why consensus was not reached
		///     for some activities.
		pub(crate) fn get_consensus_for_activities<A: Activity>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			activities: &[(NodePubKey, Vec<A>)],
			min_nodes: u16,
			threshold: Percent,
		) -> Result<Vec<A>, Vec<OCWError>> {
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
				if activities.len() < min_nodes.into() {
					errors.push(OCWError::NotEnoughNodesForConsensus {
						cluster_id: (*cluster_id),
						era_id,
						id,
					});
				} else if let Some(activity) =
					Self::reach_consensus(&activities, min_threshold.into())
				{
					consensus_activities.push(activity);
				} else {
					errors.push(OCWError::ActivityNotInConsensus {
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
		#[allow(dead_code)]
		pub(crate) fn fetch_processed_era(
			node_params: &StorageNodeParams,
		) -> Result<Vec<EraActivity>, http::Error> {
			let scheme = if node_params.ssl { "https" } else { "http" };
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let url = format!("{}://{}:{}/activity/eras", scheme, host, node_params.http_port);
			let request = http::Request::get(&url);
			let timeout = sp_io::offchain::timestamp()
				.add(sp_runtime::offchain::Duration::from_millis(10000));
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
			let timeout = sp_io::offchain::timestamp()
				.add(sp_runtime::offchain::Duration::from_millis(10000));
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
				"{}://{}:{}/activity/nodes?eraId={}",
				scheme, host, node_params.http_port, era_id
			);

			let request = http::Request::get(&url);
			let timeout =
				sp_io::offchain::timestamp().add(rt_offchain::Duration::from_millis(10000));
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
		) -> Result<Vec<(NodePubKey, Vec<NodeActivity>)>, OCWError> {
			let mut node_usages = Vec::new();

			for (node_pub_key, node_params) in dac_nodes {
				// todo! probably shouldn't stop when some DAC is not responding as we can still
				// work with others
				let usage =
					Self::fetch_node_usage(cluster_id, era_id, node_params).map_err(|_| {
						OCWError::NodeUsageRetrievalError {
							cluster_id: *cluster_id,
							era_id,
							node_pub_key: node_pub_key.clone(),
						}
					})?;

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
		) -> Result<Vec<(NodePubKey, Vec<CustomerActivity>)>, OCWError> {
			let mut customers_usages = Vec::new();

			for (node_pub_key, node_params) in dac_nodes {
				// todo! probably shouldn't stop when some DAC is not responding as we can still
				// work with others
				let usage =
					Self::fetch_customers_usage(cluster_id, era_id, node_params).map_err(|_| {
						OCWError::CustomerUsageRetrievalError {
							cluster_id: *cluster_id,
							era_id,
							node_pub_key: node_pub_key.clone(),
						}
					})?;

				customers_usages.push((node_pub_key.clone(), usage));
			}

			Ok(customers_usages)
		}

		/// Fetch processed era for across all nodes.
		///
		/// Parameters:
		/// - `cluster_id`: Cluster id
		/// - `node_params`: DAC node parameters
		fn fetch_processed_era_for_node(
			cluster_id: &ClusterId,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<Vec<EraActivity>>, OCWError> {
			let mut eras = Vec::new();

			for (node_pub_key, node_params) in dac_nodes {
				// todo! probably shouldn't stop when some DAC is not responding as we can still
				// work with others

				let ids = Self::fetch_processed_era(node_params).map_err(|_| {
					OCWError::EraRetrievalError {
						cluster_id: *cluster_id,
						node_pub_key: node_pub_key.clone(),
					}
				})?;

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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn set_prepare_era_for_payout(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			payers_merkle_root_hash: ActivityHash,
			payees_merkle_root_hash: ActivityHash,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			//ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorised); // todo!
			// need to refactor this Retrieve or initialize the EraValidation
			let mut era_validation = {
				let era_validations = <EraValidations<T>>::get(cluster_id, era_id);

				if era_validations.is_none() {
					EraValidation {
						payers_merkle_root_hash: ActivityHash::default(),
						payees_merkle_root_hash: ActivityHash::default(),
						start_era: Default::default(),
						end_era: Default::default(),
						validators: Default::default(),
						status: EraValidationStatus::ValidatingData,
					}
				} else {
					era_validations.unwrap()
				}
			};

			// Ensure the validators entry exists for the specified (payers_merkle_root_hash,
			// payees_merkle_root_hash)
			let signed_validators = era_validation
				.validators
				.entry((payers_merkle_root_hash, payees_merkle_root_hash))
				.or_insert_with(Vec::new);

			ensure!(!signed_validators.contains(&caller.clone()), Error::<T>::AlreadySignedEra);
			signed_validators.push(caller.clone());

			let percent = Percent::from_percent(T::MAJORITY);
			let threshold = percent * <ValidatorSet<T>>::get().len();

			let mut should_deposit_ready_event = false;
			if threshold < signed_validators.len() {
				// Update payers_merkle_root_hash and payees_merkle_root_hash as ones passed the
				// threshold
				era_validation.payers_merkle_root_hash = payers_merkle_root_hash;
				era_validation.payees_merkle_root_hash = payees_merkle_root_hash;
				era_validation.status = EraValidationStatus::ReadyForPayout;

				should_deposit_ready_event = true;
			}

			// Update the EraValidations storage
			<EraValidations<T>>::insert(cluster_id, era_id, era_validation);
			if should_deposit_ready_event {
				Self::deposit_event(Event::<T>::EraValidationReady { cluster_id, era_id });
			} else {
				Self::deposit_event(Event::<T>::EraValidationNotReady { cluster_id, era_id });
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
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn emit_consensus_errors(
			origin: OriginFor<T>,
			errors: Vec<OCWError>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			//ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorised); // todo!
			// need to refactor this

			for error in errors {
				match error {
					OCWError::NotEnoughNodesForConsensus { cluster_id, era_id, id } => {
						Self::deposit_event(Event::NotEnoughNodesForConsensus {
							cluster_id,
							era_id,
							id,
							validator: caller.clone(),
						});
					},
					OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
						Self::deposit_event(Event::ActivityNotInConsensus {
							cluster_id,
							era_id,
							id,
							validator: caller.clone(),
						});
					},
					OCWError::NodeUsageRetrievalError { cluster_id, era_id, node_pub_key } => {
						Self::deposit_event(Event::NodeUsageRetrievalError {
							cluster_id,
							era_id,
							node_pub_key,
							validator: caller.clone(),
						});
					},
					OCWError::CustomerUsageRetrievalError { cluster_id, era_id, node_pub_key } => {
						Self::deposit_event(Event::CustomerUsageRetrievalError {
							cluster_id,
							era_id,
							node_pub_key,
							validator: caller.clone(),
						});
					},
					OCWError::EraRetrievalError { cluster_id, node_pub_key } => {
						Self::deposit_event(Event::EraRetrievalError {
							cluster_id,
							node_pub_key,
							validator: caller.clone(),
						});
					},
					OCWError::PrepareEraTransactionError {
						cluster_id,
						era_id,
						payers_merkle_root_hash,
						payees_merkle_root_hash,
					} => {
						Self::deposit_event(Event::PrepareEraTransactionError {
							cluster_id,
							era_id,
							payers_merkle_root_hash,
							payees_merkle_root_hash,
							validator: caller.clone(),
						});
					},
					OCWError::BeginBillingReportTransactionError { cluster_id, era_id } => {
						Self::deposit_event(Event::BeginBillingReportTransactionError {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::BeginChargingCustomersTransactionError { cluster_id, era_id } => {
						Self::deposit_event(Event::BeginChargingCustomersTransactionError {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::SendChargingCustomersBatchTransactionError {
						cluster_id,
						era_id,
						batch_index,
					} => {
						Self::deposit_event(Event::SendChargingCustomersBatchTransactionError {
							cluster_id,
							era_id,
							batch_index,
							validator: caller.clone(),
						});
					},
					OCWError::EndChargingCustomersTransactionError { cluster_id, era_id } => {
						Self::deposit_event(Event::EndChargingCustomersTransactionError {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::BeginRewardingProvidersTransactionError { cluster_id, era_id } => {
						Self::deposit_event(Event::BeginRewardingProvidersTransactionError {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::EndRewardingProvidersTransactionError { cluster_id, era_id } => {
						Self::deposit_event(Event::EndRewardingProvidersTransactionError {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::EndBillingReportTransactionError { cluster_id, era_id } => {
						Self::deposit_event(Event::EndBillingReportTransactionError {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::BillingReportDoesNotExist { cluster_id, era_id } => {
						Self::deposit_event(Event::BillingReportDoesNotExist {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::EmptyCustomerActivity { cluster_id, era_id } => {
						Self::deposit_event(Event::EmptyCustomerActivity {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::BatchIndexConversionFailed { cluster_id, era_id } => {
						Self::deposit_event(Event::BatchIndexConversionFailed {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::NoAvailableSigner => {
						Self::deposit_event(Event::NoAvailableSigner { validator: caller.clone() });
					},
					OCWError::NotEnoughDACNodes { num_nodes } => {
						Self::deposit_event(Event::NotEnoughDACNodes {
							num_nodes,
							validator: caller.clone(),
						});
					},
					OCWError::FailedToCreateMerkleRoot => {
						Self::deposit_event(Event::FailedToCreateMerkleRoot {
							validator: caller.clone(),
						});
					},
					OCWError::FailedToFetchCurrentValidator => {
						Self::deposit_event(Event::FailedToFetchCurrentValidator {
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
		#[pallet::call_index(2)]
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

		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn begin_billing_report(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			start_era: i64,
			end_era: i64,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutVisitor::begin_billing_report(sender, cluster_id, era_id, start_era, end_era)?;

			let mut era_validation = <EraValidations<T>>::get(cluster_id, era_id).unwrap(); // should exist
			era_validation.status = EraValidationStatus::PayoutInProgress;
			<EraValidations<T>>::insert(cluster_id, era_id, era_validation);

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn begin_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutVisitor::begin_charging_customers(sender, cluster_id, era_id, max_batch_index)
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		// todo! remove clippy::too_many_arguments
		#[allow(clippy::too_many_arguments)]
		pub fn send_charging_customers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
			payers: Vec<(T::AccountId, BucketId, CustomerUsage)>,
			mmr_size: u64,
			proof: Vec<ActivityHash>,
			leaf_with_position: (u64, ActivityHash),
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutVisitor::send_charging_customers_batch(
				sender,
				cluster_id,
				era_id,
				batch_index,
				payers,
				mmr_size,
				proof,
				leaf_with_position,
			)
		}

		#[pallet::call_index(6)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn end_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutVisitor::end_charging_customers(sender, cluster_id, era_id)
		}

		#[pallet::call_index(7)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn begin_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			max_batch_index: BatchIndex,
			total_node_usage: NodeUsage,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutVisitor::begin_rewarding_providers(
				sender,
				cluster_id,
				era_id,
				max_batch_index,
				total_node_usage,
			)
		}

		#[pallet::call_index(8)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		// todo! remove clippy::too_many_arguments
		#[allow(clippy::too_many_arguments)]
		pub fn send_rewarding_providers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
			payees: Vec<(T::AccountId, BucketId, NodeUsage)>,
			mmr_size: u64,
			proof: Vec<ActivityHash>,
			leaf_with_position: (u64, ActivityHash),
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutVisitor::send_rewarding_providers_batch(
				sender,
				cluster_id,
				era_id,
				batch_index,
				payees,
				mmr_size,
				proof,
				leaf_with_position,
			)
		}

		#[pallet::call_index(9)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn end_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutVisitor::end_rewarding_providers(sender, cluster_id, era_id)
		}

		#[pallet::call_index(10)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn end_billing_report(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutVisitor::end_billing_report(sender, cluster_id, era_id)?;

			let mut era_validation = <EraValidations<T>>::get(cluster_id, era_id).unwrap(); // should exist
			era_validation.status = EraValidationStatus::PayoutSuccess;
			<EraValidations<T>>::insert(cluster_id, era_id, era_validation);

			Ok(())
		}

		#[pallet::call_index(11)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn set_current_validator(origin: OriginFor<T>) -> DispatchResult {
			let validator: T::AccountId = ensure_signed(origin)?;
			Self::store_current_validator(validator.encode());
			Ok(())
		}

		#[pallet::call_index(12)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn set_cluster_to_validate(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
		) -> DispatchResult {
			ensure_root(origin)?;
			ClusterToValidate::<T>::put(cluster_id);

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

		// todo! use batch_index and payers as part of the validation
		fn is_customers_batch_valid(
			cluster_id: ClusterId,
			era_id: DdcEra,
			_batch_index: BatchIndex,
			_payers: &[(T::AccountId, BucketId, CustomerUsage)],
			proof: MerkleProof<ActivityHash, MergeActivityHash>,
			leaf_with_position: (u64, ActivityHash),
		) -> bool {
			let validation_era = EraValidations::<T>::get(cluster_id, era_id);

			match validation_era {
				Some(valid_era) => {
					let root = valid_era.payers_merkle_root_hash;
					Self::proof_merkle_leaf(root, proof, vec![leaf_with_position]).unwrap_or(false)
				},
				None => false,
			}
		}

		// todo! use batch_index and payees as part of the validation
		fn is_providers_batch_valid(
			cluster_id: ClusterId,
			era_id: DdcEra,
			_batch_index: BatchIndex,
			_payees: &[(T::AccountId, BucketId, NodeUsage)],
			proof: MerkleProof<ActivityHash, MergeActivityHash>,
			leaf_with_position: (u64, ActivityHash),
		) -> bool {
			let validation_era = EraValidations::<T>::get(cluster_id, era_id);

			match validation_era {
				Some(valid_era) => {
					let root = valid_era.payees_merkle_root_hash;
					Self::proof_merkle_leaf(root, proof, vec![leaf_with_position]).unwrap_or(false)
				},
				None => false,
			}
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
			log::info!("🙌Adding Validator from genesis session.");
			let validators = validators
				.map(|(_, k)| T::AccountId::decode(&mut &k.into().encode()[..]).unwrap())
				.collect::<Vec<_>>();

			ValidatorSet::<T>::put(validators);
		}

		fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_authorities: I)
		where
			I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
		{
			log::info!("🙌Adding Validator from new session.");
			let validators = validators
				.map(|(_, k)| T::AccountId::decode(&mut &k.into().encode()[..]).unwrap())
				.collect::<Vec<_>>();
			ValidatorSet::<T>::put(validators);
		}

		fn on_disabled(_i: u32) {}
	}
}
