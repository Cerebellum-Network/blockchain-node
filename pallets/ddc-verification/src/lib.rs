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

use base64ct::{Base64, Encoding};
use ddc_primitives::{
	traits::{
		ClusterManager, ClusterValidator, CustomerVisitor, NodeVisitor, PayoutVisitor,
		ValidatorVisitor,
	},
	ActivityHash, BatchIndex, ClusterId, CustomerUsage, DdcEra, MMRProof, NodeParams, NodePubKey,
	NodeUsage, PayoutState, StorageNodeParams,
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
use rand::{prelude::*, rngs::SmallRng, SeedableRng};
use sp_staking::StakingInterface;

use crate::weights::WeightInfo;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use ddc_primitives::{BucketId, MergeActivityHash, KEY_TYPE};
	use frame_support::PalletId;
	use sp_core::crypto::AccountId32;
	use sp_runtime::SaturatedConversion;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
		frame_support::traits::StorageVersion::new(0);

	const SUCCESS_CODE: u16 = 200;
	const BUF_SIZE: usize = 128;
	const RESPONSE_TIMEOUT: u64 = 20000;

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
		type ClusterValidator: ClusterValidator<Self>;
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
		const MAX_MERKLE_NODE_IDENTIFIER: u16;
		/// The access to staking functionality.
		type StakingVisitor: StakingInterface<AccountId = Self::AccountId>;
		type AccountIdConverter: From<Self::AccountId> + Into<AccountId32>;
		type CustomerVisitor: CustomerVisitor<Self>;
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
			node_id: String,
			validator: T::AccountId,
		},
		/// Not enough buckets for consensus.
		NotEnoughBucketsForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			bucket_id: BucketId,
			validator: T::AccountId,
		},
		/// Not enough records for consensus.
		NotEnoughRecordsForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			record_id: String,
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
		SendRewardingProvidersBatchTransactionError {
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
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
		FailedToCreateMerkleProof {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
		FailedToFetchCurrentValidator {
			validator: T::AccountId,
		},
		FailedToFetchNodeProvider {
			validator: T::AccountId,
		},
		ValidatorKeySet {
			validator: T::AccountId,
		},
		FailedToFetchNodeTotalUsage {
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
			validator: T::AccountId,
		},
		EraValidationRootsPosted {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
			payers_merkle_root_hash: ActivityHash,
			payees_merkle_root_hash: ActivityHash,
			payers_batch_merkle_root_hashes: Vec<ActivityHash>,
			payees_batch_merkle_root_hashes: Vec<ActivityHash>,
		},
		BucketAggregatesRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			bucket_id: BucketId,
			node_pub_key: NodePubKey,
			validator: T::AccountId,
		},
		NotEnoughNodeAggregatesForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			bucket_id: BucketId,
			node_id: String,
			validator: T::AccountId,
		},
		ChallengeResponseRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			node_id: String,
			bucket_id: Option<BucketId>,
			node_pub_key: NodePubKey,
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
			node_id: String,
		},
		NotEnoughBucketsForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			bucket_id: BucketId,
		},
		NotEnoughRecordsForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			record_id: String,
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
		/// Bucket aggregates Retrieval Error.
		BucketAggregatesRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			bucket_id: BucketId,
			node_pub_key: NodePubKey,
		},
		/// Challenge Response Retrieval Error.
		ChallengeResponseRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			node_id: String,
			bucket_id: Option<BucketId>,
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
		SendRewardingProvidersBatchTransactionError {
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
		FailedToCreateMerkleRoot {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		FailedToCreateMerkleProof {
			cluster_id: ClusterId,
			era_id: DdcEra,
		},
		FailedToFetchCurrentValidator,
		FailedToFetchNodeProvider,
		FailedToFetchNodeTotalUsage {
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
		},
		/// Not enough subaggregates for consensus.
		NotEnoughNodeAggregatesForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			bucket_id: BucketId,
			node_id: String,
		},
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
		NotExpectedState,
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
		PayoutSkipped,
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
	#[derive(
		Debug,
		Serialize,
		Deserialize,
		Clone,
		Hash,
		Ord,
		PartialOrd,
		PartialEq,
		Eq,
		TypeInfo,
		Encode,
		Decode,
	)]
	pub struct EraActivity {
		/// Era id.
		pub id: DdcEra,
		pub start: i64,
		pub end: i64,
	}

	pub struct CustomerBatch<T: Config> {
		pub(crate) batch_index: BatchIndex,
		pub(crate) payers: Vec<(T::AccountId, BucketId, CustomerUsage)>,
		pub(crate) batch_proof: MMRProof,
	}

	pub struct ProviderBatch<T: Config> {
		pub(crate) batch_index: BatchIndex,
		pub(crate) payees: Vec<(T::AccountId, NodeUsage)>,
		pub(crate) batch_proof: MMRProof,
	}

	/// Node activity of a node.
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct NodeActivity {
		/// Node id.
		pub(crate) node_id: String,
		/// Total amount of stored bytes.
		pub(crate) stored_bytes: i64,
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
		/// Bucket id
		pub(crate) bucket_id: BucketId,
		/// SubAggregates.
		pub(crate) sub_aggregates: Vec<BucketSubAggregate>,
	}

	/// Sub Aggregates of a bucket.
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	#[allow(non_snake_case)]
	pub(crate) struct BucketSubAggregate {
		/// Node id.
		pub(crate) NodeID: String,
		/// Total amount of stored bytes.
		pub(crate) stored_bytes: i64,
		/// Total amount of transferred bytes.
		pub(crate) transferred_bytes: u64,
		/// Total number of puts.
		pub(crate) number_of_puts: u64,
		/// Total number of gets.
		pub(crate) number_of_gets: u64,
	}

	/// Bucket aggregate by bucket id.
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct BucketNodeAggregatesActivity {
		/// Bucket id
		pub(crate) bucket_id: BucketId,
		/// Node id.
		pub(crate) node_id: String,
		/// Total amount of stored bytes.
		pub(crate) stored_bytes: i64,
		/// Total amount of transferred bytes.
		pub(crate) transferred_bytes: u64,
		/// Total number of puts.
		pub(crate) number_of_puts: u64,
		/// Total number of gets.
		pub(crate) number_of_gets: u64,
	}

	/// Challenge Response
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct ChallengeAggregateResponse {
		/// proofs
		pub proofs: Vec<Proof>, //todo! add optional fields
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct Proof {
		pub merkle_tree_node_id: u64,
		pub usage: Usage,
		pub path: Vec<String>, //todo! add base64 deserialization
		pub leafs: Vec<Leaf>,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct Usage {
		/// Total amount of stored bytes.
		pub stored_bytes: i64,
		/// Total amount of transferred bytes.
		pub transferred_bytes: u64,
		/// Total number of puts.
		pub number_of_puts: u64,
		/// Total number of gets.
		pub number_of_gets: u64,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct Leaf {
		pub record: Record,
		pub transferred_bytes: u64,
		pub stored_bytes: i64,
		// todo! add links if there is no record
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	#[allow(non_snake_case)]
	pub(crate) struct Record {
		pub id: String,
		pub upstream: Upstream,
		pub downstream: Vec<Downstream>,
		pub timestamp: String,
		pub signature: Signature,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct Upstream {
		pub request: Request,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct Downstream {
		pub request: Request,
	}
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	#[allow(non_snake_case)]
	pub(crate) struct Request {
		pub requestId: String,
		pub requestType: String,
		pub contentType: String,
		pub bucketId: String,
		pub pieceCid: String,
		pub offset: String,
		pub size: String,
		pub timestamp: String,
		pub signature: Signature,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub(crate) struct Signature {
		pub algorithm: String,
		pub signer: String,
		pub value: String,
	}

	// Define a common trait
	pub trait Activity:
		Clone + Ord + PartialEq + Eq + Serialize + for<'de> Deserialize<'de>
	{
		fn get_consensus_id<T: Config>(&self) -> ActivityHash;
		fn hash<T: Config>(&self) -> ActivityHash;

		fn get_consensus_error(&self, cluster_id: ClusterId, era_id: DdcEra) -> OCWError;
	}

	impl Activity for NodeActivity {
		fn get_consensus_id<T: Config>(&self) -> ActivityHash {
			T::ActivityHasher::hash(self.node_id.as_bytes()).into()
		}

		fn hash<T: Config>(&self) -> ActivityHash {
			T::ActivityHasher::hash(&self.encode()).into()
		}

		fn get_consensus_error(&self, cluster_id: ClusterId, era_id: DdcEra) -> OCWError {
			let node_id = &self.node_id;

			OCWError::NotEnoughNodesForConsensus { cluster_id, era_id, node_id: node_id.clone() }
		}
	}
	impl Activity for CustomerActivity {
		fn get_consensus_id<T: Config>(&self) -> ActivityHash {
			T::ActivityHasher::hash(&self.bucket_id.encode()).into()
		}

		fn hash<T: Config>(&self) -> ActivityHash {
			T::ActivityHasher::hash(&self.encode()).into()
		}

		fn get_consensus_error(&self, cluster_id: ClusterId, era_id: DdcEra) -> OCWError {
			let bucket_id = &self.bucket_id;

			OCWError::NotEnoughBucketsForConsensus { cluster_id, era_id, bucket_id: *bucket_id }
		}
	}

	impl Activity for BucketNodeAggregatesActivity {
		fn get_consensus_id<T: Config>(&self) -> ActivityHash {
			let mut data = self.bucket_id.encode();
			data.extend_from_slice(&self.node_id.encode());
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			data.extend_from_slice(&self.number_of_puts.encode());
			data.extend_from_slice(&self.number_of_gets.encode());
			T::ActivityHasher::hash(&data).into()
		}

		fn hash<T: Config>(&self) -> ActivityHash {
			T::ActivityHasher::hash(&self.encode()).into()
		}

		fn get_consensus_error(&self, cluster_id: ClusterId, era_id: DdcEra) -> OCWError {
			let node_id = &self.node_id;
			let bucket_id = &self.bucket_id;

			OCWError::NotEnoughNodeAggregatesForConsensus {
				cluster_id,
				era_id,
				bucket_id: *bucket_id,
				node_id: node_id.clone(),
			}
		}
	}

	impl Activity for Leaf {
		fn get_consensus_id<T: Config>(&self) -> ActivityHash {
			T::ActivityHasher::hash(&self.record.id.encode()).into()
		}

		fn hash<T: Config>(&self) -> ActivityHash {
			let mut data = self.record.id.encode();
			data.extend_from_slice(&self.record.upstream.request.requestType.encode());
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			T::ActivityHasher::hash(&data).into()
		}

		fn get_consensus_error(&self, cluster_id: ClusterId, era_id: DdcEra) -> OCWError {
			let record_id = &self.record.id;

			OCWError::NotEnoughRecordsForConsensus {
				cluster_id,
				era_id,
				record_id: record_id.clone(),
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

			// todo! Need to uncomment this code
			// if Self::fetch_current_validator().is_err() {
			// 	let _ = signer.send_signed_transaction(|account| {
			// 		Self::store_current_validator(account.id.encode());
			//
			// 		Call::set_current_validator {}
			// 	});
			// }
			// todo! need to remove below code
			if (block_number.saturated_into::<u32>() % 70) == 0 {
				let _ = signer.send_signed_transaction(|account| {
					Self::store_current_validator(account.id.encode());
					log::info!("🏭📋‍ Setting current validator...  {:?}", account.id);
					Call::set_current_validator {}
				});
			}

			if (block_number.saturated_into::<u32>() % T::BLOCK_TO_START as u32) != 0 {
				return;
			}

			log::info!("👋 Hello from pallet-ddc-verification.");

			// todo! fetch clusters from ddc-clusters and loop the whole process for each cluster
			let cluster_id = unwrap_or_log_error!(
				Self::get_cluster_to_validate(),
				"🏭❌ Error retrieving cluster to validate"
			);

			let dac_nodes = unwrap_or_log_error!(
				Self::get_dac_nodes(&cluster_id),
				"🏭❌ Error retrieving dac nodes to validate"
			);

			let min_nodes = T::MIN_DAC_NODES_FOR_CONSENSUS;
			let batch_size = T::MAX_PAYOUT_BATCH_SIZE;
			let mut errors: Vec<OCWError> = Vec::new();

			let processed_dac_data =
				Self::process_dac_data(&cluster_id, None, &dac_nodes, min_nodes, batch_size.into());

			match processed_dac_data {
				Ok(Some((
					era_activity,
					payers_merkle_root_hash,
					payees_merkle_root_hash,
					payers_batch_merkle_root_hashes,
					payees_batch_merkle_root_hashes,
				))) => {
					log::info!(
						"🏭🚀 Processing era_id: {:?} for cluster_id: {:?}",
						era_activity.clone(),
						cluster_id
					);

					let results = signer.send_signed_transaction(|_account| {
						Call::set_prepare_era_for_payout {
							cluster_id,
							era_activity: era_activity.clone(),
							payers_merkle_root_hash,
							payees_merkle_root_hash,
							payers_batch_merkle_root_hashes: payers_batch_merkle_root_hashes
								.clone(),
							payees_batch_merkle_root_hashes: payees_batch_merkle_root_hashes
								.clone(),
						}
					});

					for (_, res) in &results {
						match res {
							Ok(()) => {
								log::info!(
										"🏭⛳️ Merkle roots posted on-chain for cluster_id: {:?}, era: {:?}",
										cluster_id,
										era_activity.clone()
									);
							},
							Err(e) => {
								log::error!(
										"🏭❌ Error to post merkle roots on-chain for cluster_id: {:?}, era: {:?}: {:?}",
										cluster_id,
										era_activity.clone(),
										e
									);
								// Extrinsic call failed
								errors.push(OCWError::PrepareEraTransactionError {
									cluster_id,
									era_id: era_activity.id,
									payers_merkle_root_hash,
									payees_merkle_root_hash,
								});
							},
						}
					}
				},
				Ok(None) => {
					log::info!("🏭ℹ️ No eras for DAC process for cluster_id: {:?}", cluster_id);
				},
				Err(process_errors) => {
					errors.extend(process_errors);
				},
			};

			// todo! factor out as macro as this is repetitive
			match Self::prepare_begin_billing_report(&cluster_id) {
				Ok(Some((era_id, start_era, end_era))) => {
					log::info!(
						"🏭🚀 process_start_payout processed successfully for cluster_id: {:?}, era_id: {:?},  start_era: {:?},  end_era: {:?} ",
						cluster_id,
						era_id,
						start_era,
						end_era
					);
					let results = signer.send_signed_transaction(|_account| {
						Call::begin_billing_report { cluster_id, era_id, start_era, end_era }
					});

					for (_, res) in &results {
						match res {
							Ok(()) => {
								log::info!(
									"🏭🏄‍ Sent begin_billing_report successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"🏭❌ Error to post begin_billing_report for cluster_id: {:?}, era_id: {:?}: {:?}",
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
					}
				},
				Ok(None) => {
					log::info!("🏭❌ No era for payout for cluster_id: {:?}", cluster_id);
				},
				Err(e) => {
					errors.push(e);
				},
			}

			// todo! factor out as macro as this is repetitive
			match Self::prepare_begin_charging_customers(
				&cluster_id,
				&dac_nodes,
				min_nodes,
				batch_size.into(),
			) {
				Ok(Some((era_id, max_batch_index))) => {
					log::info!(
						"🏭🎁 prepare_begin_charging_customers processed successfully for cluster_id: {:?}, era_id: {:?}",
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
									"🏭🚀 Sent begin_charging_customers successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"🏭❌ Error to post begin_charging_customers for cluster_id: {:?}, era_id: {:?}: {:?}",
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
						log::error!("🏭❌ No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::error!(
						"🏭🦀 No era for begin_charging_customers for cluster_id: {:?}",
						cluster_id
					);
				},
				Err(e) => errors.extend(e),
			}

			// todo! factor out as macro as this is repetitive
			match Self::prepare_send_charging_customers_batch(
				&cluster_id,
				batch_size.into(),
				&dac_nodes,
				min_nodes,
			) {
				Ok(Some((era_id, batch_payout))) => {
					let payers_log: Vec<(String, BucketId, CustomerUsage)> = batch_payout
						.payers
						.clone()
						.into_iter()
						.map(|(acc_id, bucket_id, customer_usage)| {
							let account_id: T::AccountIdConverter = acc_id.into();
							let account_id_32: AccountId32 = account_id.into();
							let account_ref: &[u8; 32] = account_id_32.as_ref();
							(hex::encode(account_ref), bucket_id, customer_usage)
						})
						.collect();
					log::info!(
						"🏭🎁 prepare_send_charging_customers_batch processed successfully for cluster_id: {:?}, era_id: {:?} , batch_payout: {:?}",
						cluster_id,
						era_id,
						payers_log
					);

					if let Some((_, res)) =
						signer.send_signed_transaction(|_acc| Call::send_charging_customers_batch {
							cluster_id,
							era_id,
							batch_index: batch_payout.batch_index,
							payers: batch_payout.payers.clone(),
							batch_proof: batch_payout.batch_proof.clone(),
						}) {
						match res {
							Ok(_) => {
								// Extrinsic call succeeded
								log::info!(
									"🏭🚀 Sent send_charging_customers_batch successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"🏭❌ Error to post send_charging_customers_batch for cluster_id: {:?}, era_id: {:?}: {:?}",
										cluster_id,
										era_id,
										e
									);
								// Extrinsic call failed
								errors.push(OCWError::SendChargingCustomersBatchTransactionError {
									cluster_id,
									era_id,
									batch_index: batch_payout.batch_index,
								});
							},
						}
					} else {
						log::error!("🏭❌ No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!(
						"🏭🦀 No era for send_charging_customers_batch for cluster_id: {:?}",
						cluster_id
					);
				},
				Err(e) => {
					errors.extend(e);
				},
			}

			// todo! factor out as macro as this is repetitive
			match Self::prepare_end_charging_customers(&cluster_id) {
				Ok(Some(era_id)) => {
					log::info!(
						"🏭📝prepare_end_charging_customers processed successfully for cluster_id: {:?}, era_id: {:?}",
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
									"🏭📝Sent end_charging_customers successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"🏭❌Error to post end_charging_customers for cluster_id: {:?}, era_id: {:?}: {:?}",
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
						log::error!("🏭❌No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!(
						"🏭📝No era for end_charging_customers for cluster_id: {:?}",
						cluster_id
					);
				},
				Err(e) => {
					errors.push(e);
				},
			}

			// todo! factor out as macro as this is repetitive
			match Self::prepare_begin_rewarding_providers(
				&cluster_id,
				&dac_nodes,
				min_nodes,
				batch_size.into(),
			) {
				Ok(Some((era_id, max_batch_index, total_node_usage))) => {
					log::info!(
						"🏭📝prepare_begin_rewarding_providers processed successfully for cluster_id: {:?}, era_id: {:?}",
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
									"🏭📝Sent begin_rewarding_providers successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"🏭❌Error to post begin_rewarding_providers for cluster_id: {:?}, era_id: {:?}: {:?}",
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
						log::error!("🏭❌No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!(
						"🏭📝No era for begin_rewarding_providers for cluster_id: {:?}",
						cluster_id
					);
				},
				Err(e) => {
					errors.extend(e);
				},
			}

			// todo! factor out as macro as this is repetitive
			match Self::prepare_send_rewarding_providers_batch(
				&cluster_id,
				batch_size.into(),
				&dac_nodes,
				min_nodes,
			) {
				Ok(Some((era_id, batch_payout))) => {
					log::info!(
						"🎁 prepare_send_rewarding_providers_batch processed successfully for cluster_id: {:?}, era_id: {:?}",
						cluster_id,
						era_id
					);

					if let Some((_, res)) = signer.send_signed_transaction(|_acc| {
						Call::send_rewarding_providers_batch {
							cluster_id,
							era_id,
							batch_index: batch_payout.batch_index,
							payees: batch_payout.payees.clone(),
							batch_proof: batch_payout.batch_proof.clone(),
						}
					}) {
						match res {
							Ok(_) => {
								// Extrinsic call succeeded
								log::info!(
									"🚀 Sent send_rewarding_providers_batch successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"🦀 Error to post send_rewarding_providers_batch for cluster_id: {:?}, era_id: {:?}: {:?}",
										cluster_id,
										era_id,
										e
									);
								// Extrinsic call failed
								errors.push(
									OCWError::SendRewardingProvidersBatchTransactionError {
										cluster_id,
										era_id,
										batch_index: batch_payout.batch_index,
									},
								);
							},
						}
					} else {
						log::error!("🦀 No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!(
						"🦀 No era for send_rewarding_providers_batch for cluster_id: {:?}",
						cluster_id
					);
				},
				Err(e) => {
					errors.extend(e);
				},
			}

			// todo! factor out as macro as this is repetitive
			match Self::prepare_end_rewarding_providers(&cluster_id) {
				Ok(Some(era_id)) => {
					log::info!(
						"🏭📝prepare_end_rewarding_providers processed successfully for cluster_id: {:?}, era_id: {:?}",
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
									"🏭📝Sent end_rewarding_providers successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"🏭❌Error to post end_rewarding_providers for cluster_id: {:?}, era_id: {:?}: {:?}",
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
						log::error!("🏭❌No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!(
						"🏭📝No era for end_rewarding_providers for cluster_id: {:?}",
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
						"🏭📝prepare_end_billing_report processed successfully for cluster_id: {:?}, era_id: {:?}",
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
									"🏭📝Sent end_billing_report successfully for cluster_id: {:?}, era_id: {:?}",
									cluster_id,
									era_id
								);
							},
							Err(e) => {
								log::error!(
										"🏭❌Error to post end_billing_report for cluster_id: {:?}, era_id: {:?}: {:?}",
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
						log::error!("🏭❌No account available to sign the transaction");
						errors.push(OCWError::NoAvailableSigner);
					}
				},
				Ok(None) => {
					log::info!(
						"🏭📝No era for end_billing_report for cluster_id: {:?}",
						cluster_id
					);
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
						Err(_) => log::error!("🏭❌ Failed to submit emit_consensus_errors tx"),
					}
				}
			}
		}
	}

	impl<T: Config> Pallet<T> {
		#[allow(clippy::type_complexity)]
		pub(crate) fn process_dac_data(
			cluster_id: &ClusterId,
			era_id_to_process: Option<EraActivity>,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
			min_nodes: u16,
			batch_size: usize,
		) -> Result<
			Option<(EraActivity, ActivityHash, ActivityHash, Vec<ActivityHash>, Vec<ActivityHash>)>,
			Vec<OCWError>,
		> {
			log::info!("🚀 Processing dac data for cluster_id: {:?}", cluster_id);
			// todo! Need to debug follwing condition. Why it is not working on Devnet

			// if dac_nodes.len().ilog2() < min_nodes.into() {
			// 	return Err(vec![OCWError::NotEnoughDACNodes { num_nodes: min_nodes }]);
			// }

			let era_activity = if let Some(era_activity) = era_id_to_process {
				EraActivity {
					id: era_activity.id,
					start: era_activity.start,
					end: era_activity.end,
				}
			} else {
				match Self::get_era_for_validation(cluster_id, dac_nodes) {
					Ok(Some(era_activity)) => era_activity,
					Ok(None) => return Ok(None),
					Err(err) => return Err(vec![err]),
				}
			};

			let nodes_usage =
				Self::fetch_nodes_usage_for_era(cluster_id, era_activity.id, dac_nodes)
					.map_err(|err| vec![err])?;
			let customers_usage =
				Self::fetch_customers_usage_for_era(cluster_id, era_activity.id, dac_nodes)
					.map_err(|err| vec![err])?;

			let (bucket_node_aggregates_in_consensus, bucket_node_aggregates_not_in_consensus) =
				Self::fetch_sub_trees(cluster_id, era_activity.id, customers_usage, min_nodes);

			let mut bucket_aggregates_passed_challenges: Vec<BucketNodeAggregatesActivity> = vec![];

			if !bucket_node_aggregates_not_in_consensus.is_empty() {
				bucket_aggregates_passed_challenges =
					Self::challenge_and_find_valid_bucket_sub_aggregates_not_in_consensus(
						cluster_id,
						era_activity.id,
						dac_nodes,
						bucket_node_aggregates_not_in_consensus,
					)?;
			}

			let mut total_bucket_aggregates = bucket_node_aggregates_in_consensus.clone();
			total_bucket_aggregates.extend(bucket_aggregates_passed_challenges);

			let customer_activity_hashes: Vec<ActivityHash> =
				total_bucket_aggregates.clone().into_iter().map(|c| c.hash::<T>()).collect();

			let customer_activity_hashes_string: Vec<String> =
				customer_activity_hashes.clone().into_iter().map(hex::encode).collect();

			log::info!(
				"🧗‍ Customer Activity hashes for ClusterId: {:?} EraId: {:?}  is: {:?}",
				cluster_id,
				era_activity.id,
				customer_activity_hashes_string
			);
			let customers_activity_batch_roots = Self::convert_to_batch_merkle_roots(
				cluster_id,
				era_activity.id,
				Self::split_to_batches(&total_bucket_aggregates, batch_size),
			)
			.map_err(|err| vec![err])?;

			let customer_batch_roots_string: Vec<String> =
				customers_activity_batch_roots.clone().into_iter().map(hex::encode).collect();

			for (pos, batch_root) in customer_batch_roots_string.iter().enumerate() {
				log::info!(
				"🧗‍  Customer Activity batches for ClusterId: {:?} EraId: {:?}  is: batch {:?} with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
					pos + 1,
					batch_root,
					customer_activity_hashes_string
				);
			}

			let customers_activity_root = Self::create_merkle_root(
				cluster_id,
				era_activity.id,
				&customers_activity_batch_roots,
			)
			.map_err(|err| vec![err])?;

			log::info!(
				"🧗‍  Customer Activity batches tree for ClusterId: {:?} EraId: {:?}  is: batch with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
				hex::encode(customers_activity_root),
					customer_batch_roots_string,
			);

			let (nodes_activity_in_consensus, nodes_activity_not_in_consensus) =
				Self::get_consensus_for_activities(
					cluster_id,
					era_activity.id,
					&nodes_usage,
					min_nodes,
					Percent::from_percent(T::MAJORITY),
				);

			let mut node_aggregates_passed_challenges: Vec<NodeActivity> = vec![];

			if !node_aggregates_passed_challenges.is_empty() {
				node_aggregates_passed_challenges =
					Self::challenge_and_find_valid_node_sub_aggregates_not_in_consensus(
						cluster_id,
						era_activity.id,
						dac_nodes,
						nodes_activity_not_in_consensus,
					)?;
			}

			let mut total_node_aggregates = nodes_activity_in_consensus.clone();
			total_node_aggregates.extend(node_aggregates_passed_challenges);

			let node_activity_hashes: Vec<ActivityHash> =
				total_node_aggregates.clone().into_iter().map(|c| c.hash::<T>()).collect();

			let node_activity_hashes_string: Vec<String> =
				node_activity_hashes.clone().into_iter().map(hex::encode).collect();

			log::info!(
				"🧗‍ Node Activity hashes for ClusterId: {:?} EraId: {:?}  is: {:?}",
				cluster_id,
				era_activity.id,
				node_activity_hashes_string
			);

			let nodes_activity_batch_roots = Self::convert_to_batch_merkle_roots(
				cluster_id,
				era_activity.id,
				Self::split_to_batches(&total_node_aggregates, batch_size),
			)
			.map_err(|err| vec![err])?;

			let nodes_activity_batch_roots_string: Vec<String> =
				nodes_activity_batch_roots.clone().into_iter().map(hex::encode).collect();

			for (pos, batch_root) in nodes_activity_batch_roots_string.iter().enumerate() {
				log::info!(
				"🧗‍  Node Activity batches for ClusterId: {:?} EraId: {:?}  is: batch {:?} with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
					pos + 1,
					batch_root,
					node_activity_hashes_string
				);
			}

			let nodes_activity_root =
				Self::create_merkle_root(cluster_id, era_activity.id, &nodes_activity_batch_roots)
					.map_err(|err| vec![err])?;

			log::info!(
				"🧗‍  Node Activity batches tree for ClusterId: {:?} EraId: {:?}  is: batch with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
				hex::encode(nodes_activity_root),
					nodes_activity_batch_roots_string,
			);

			Self::store_validation_activities(
				cluster_id,
				era_activity.id,
				&total_bucket_aggregates,
				customers_activity_root,
				&customers_activity_batch_roots,
				&total_node_aggregates,
				nodes_activity_root,
				&nodes_activity_batch_roots,
			);
			log::info!("🙇‍ Dac data processing completed for cluster_id: {:?}", cluster_id);
			Ok(Some((
				era_activity,
				customers_activity_root,
				nodes_activity_root,
				customers_activity_batch_roots,
				nodes_activity_batch_roots,
			)))
		}

		pub(crate) fn challenge_and_find_valid_bucket_sub_aggregates_not_in_consensus(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
			bucket_node_aggregates_not_in_consensus: Vec<BucketNodeAggregatesActivity>,
		) -> Result<Vec<BucketNodeAggregatesActivity>, Vec<OCWError>> {
			let mut bucket_aggregates_passed_challenges: Vec<BucketNodeAggregatesActivity> = vec![];
			let mut bucket_aggregates_not_passed_challenges: Vec<BucketNodeAggregatesActivity> =
				vec![];
			let number_of_identifiers = T::MAX_MERKLE_NODE_IDENTIFIER;

			log::info!(
				"🚀 Challenge process starts when bucket sub aggregates are not in consensus!"
			);

			for bucket_node_aggregate_activity in bucket_node_aggregates_not_in_consensus {
				let merkle_node_ids = Self::find_random_merkle_node_ids(
					number_of_identifiers.into(),
					bucket_node_aggregate_activity.number_of_gets +
						bucket_node_aggregate_activity.number_of_puts,
					bucket_node_aggregate_activity.clone().node_id,
				);
				let bucket_id = bucket_node_aggregate_activity.clone().bucket_id;

				log::info!("🚀 Merkle Node Identifiers for bucket_node_aggregate_activity: node id: {:?} bucket_id:{:?}  identifiers: {:?}",
						bucket_node_aggregate_activity.clone().node_id, bucket_id.clone(), merkle_node_ids);

				let challenge_responses = Self::fetch_challenge_responses(
					cluster_id,
					bucket_node_aggregate_activity.clone().node_id,
					era_id,
					Some(bucket_id),
					merkle_node_ids,
					dac_nodes,
				)
				.map_err(|err| vec![err])?;

				log::info!("🚀 Fetched challenge response node id: {:?} bucket_id:{:?}  challenge_response: {:?}",
						bucket_node_aggregate_activity.clone().node_id, bucket_id.clone(), challenge_responses);

				let resulting_hash_from_leafs_and_paths =
					Self::find_resulting_hash_from_leafs_and_paths(
						challenge_responses,
						cluster_id,
						era_id,
						Some(bucket_id),
						bucket_node_aggregate_activity.clone().node_id,
					)?;

				let root_challenge_responses = Self::fetch_challenge_responses(
					cluster_id,
					bucket_node_aggregate_activity.clone().node_id,
					era_id,
					Some(bucket_id),
					vec![1],
					dac_nodes,
				)
				.map_err(|err| vec![err])?;

				log::info!("🚀 Fetched Root challenge response node id: {:?} bucket_id:{:?}  challenge_response: {:?}",
						bucket_node_aggregate_activity.clone().node_id, bucket_id.clone(), root_challenge_responses);

				let merkle_root_hash = Self::find_resulting_hash_from_leafs_and_paths(
					root_challenge_responses,
					cluster_id,
					era_id,
					Some(bucket_id),
					bucket_node_aggregate_activity.clone().node_id,
				)?;

				if resulting_hash_from_leafs_and_paths == merkle_root_hash {
					log::info!(
						"🚀👍 The  node id: {:?} with bucket_id:{:?} has passed the challenge. The activity detail is {:?}",
						bucket_node_aggregate_activity.clone().node_id,
						bucket_id,
						bucket_node_aggregate_activity
					);

					bucket_aggregates_passed_challenges.push(bucket_node_aggregate_activity);
				} else {
					log::info!(
						"🚀👎 The  node id: {:?} with bucket_id:{:?} has not passed the challenge. The activity detail is {:?}",
						bucket_node_aggregate_activity.clone().node_id,
						bucket_id,
						bucket_node_aggregate_activity
					);

					bucket_aggregates_not_passed_challenges.push(bucket_node_aggregate_activity);
				}
			}

			let mut data_grouped = Vec::new();
			for (key, chunk) in
				&bucket_aggregates_passed_challenges.into_iter().chunk_by(|elt| elt.bucket_id)
			{
				data_grouped.push((key, chunk.collect()));
			}

			Ok(Self::fetch_valid_bucket_aggregates_passed_challenges(data_grouped))
		}

		pub(crate) fn challenge_and_find_valid_node_sub_aggregates_not_in_consensus(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
			node_aggregates_not_in_consensus: Vec<NodeActivity>,
		) -> Result<Vec<NodeActivity>, Vec<OCWError>> {
			let mut node_aggregates_passed_challenges: Vec<NodeActivity> = vec![];
			let mut node_aggregates_not_passed_challenges: Vec<NodeActivity> = vec![];
			let number_of_identifiers = T::MAX_MERKLE_NODE_IDENTIFIER;

			log::info!(
				"🚀 Challenge process starts when Node sub aggregates are not in consensus!"
			);

			for node_aggregate_activity in node_aggregates_not_in_consensus {
				let merkle_node_ids = Self::find_random_merkle_node_ids(
					number_of_identifiers.into(),
					node_aggregate_activity.number_of_gets + node_aggregate_activity.number_of_puts,
					node_aggregate_activity.clone().node_id,
				);

				log::info!("🚀 Merkle Node Identifiers for node_aggregate_activity: node id: {:?}  identifiers: {:?}",
						node_aggregate_activity.clone().node_id, merkle_node_ids);

				let challenge_responses = Self::fetch_challenge_responses(
					cluster_id,
					node_aggregate_activity.clone().node_id,
					era_id,
					None,
					merkle_node_ids,
					dac_nodes,
				)
				.map_err(|err| vec![err])?;

				log::info!(
					"🚀 Fetched challenge response node id: {:?}   challenge_response: {:?}",
					node_aggregate_activity.clone().node_id,
					challenge_responses
				);

				let resulting_hash_from_leafs_and_paths =
					Self::find_resulting_hash_from_leafs_and_paths(
						challenge_responses,
						cluster_id,
						era_id,
						None,
						node_aggregate_activity.clone().node_id,
					)?;

				let root_challenge_responses = Self::fetch_challenge_responses(
					cluster_id,
					node_aggregate_activity.clone().node_id,
					era_id,
					None,
					vec![1],
					dac_nodes,
				)
				.map_err(|err| vec![err])?;

				log::info!(
					"🚀 Fetched Root challenge response node id: {:?}   challenge_response: {:?}",
					node_aggregate_activity.clone().node_id,
					root_challenge_responses
				);

				let merkle_root_hash = Self::find_resulting_hash_from_leafs_and_paths(
					root_challenge_responses,
					cluster_id,
					era_id,
					None,
					node_aggregate_activity.clone().node_id,
				)?;

				if resulting_hash_from_leafs_and_paths == merkle_root_hash {
					log::info!(
						"🚀👍 The  node id: {:?}  has passed the challenge. The activity detail is {:?}",
						node_aggregate_activity.clone().node_id,
						node_aggregate_activity
					);

					node_aggregates_passed_challenges.push(node_aggregate_activity);
				} else {
					log::info!(
						"🚀👎 The  node id: {:?}  has not passed the challenge. The activity detail is {:?}",
						node_aggregate_activity.clone().node_id,
						node_aggregate_activity
					);

					node_aggregates_not_passed_challenges.push(node_aggregate_activity);
				}
			}

			let mut data_grouped = Vec::new();
			for (key, chunk) in &node_aggregates_passed_challenges
				.into_iter()
				.chunk_by(|elt| elt.node_id.clone())
			{
				data_grouped.push((key, chunk.collect()));
			}

			Ok(Self::fetch_valid_node_aggregates_passed_challenges(data_grouped))
		}

		pub(crate) fn fetch_valid_bucket_aggregates_passed_challenges(
			bucket_aggregates_passed_challenges: Vec<(BucketId, Vec<BucketNodeAggregatesActivity>)>,
		) -> Vec<BucketNodeAggregatesActivity> {
			let mut valid_aggregates_passed_challenges: Vec<BucketNodeAggregatesActivity> = vec![];

			for (bucket_id, bucket_aggregates_passed_challenge_activities) in
				bucket_aggregates_passed_challenges
			{
				let valid_activities =
					bucket_aggregates_passed_challenge_activities.iter().cloned().max_by_key(
						|activity| activity.transferred_bytes as i64 + activity.stored_bytes,
					);

				if let Some(activity) = valid_activities {
					log::info!(
						"🚀⛳️ The activity bucket_id:{:?} with maximum usage, which has passed the challenge. The activity detail is {:?}",
						bucket_id,
						activity
					);
					valid_aggregates_passed_challenges.push(activity);
				}
			}
			valid_aggregates_passed_challenges
		}

		pub(crate) fn fetch_valid_node_aggregates_passed_challenges(
			node_aggregates_passed_challenges: Vec<(String, Vec<NodeActivity>)>,
		) -> Vec<NodeActivity> {
			let mut valid_aggregates_passed_challenges: Vec<NodeActivity> = vec![];

			for (node_id, node_aggregates_passed_challenge_activities) in
				node_aggregates_passed_challenges
			{
				let valid_activities =
					node_aggregates_passed_challenge_activities.iter().cloned().max_by_key(
						|activity| activity.transferred_bytes as i64 + activity.stored_bytes,
					);

				if let Some(activity) = valid_activities {
					log::info!(
						"🚀⛳️ The activity node_id:{:?} with maximum usage, which has passed the challenge. The activity detail is {:?}",
						node_id,
						activity
					);
					valid_aggregates_passed_challenges.push(activity);
				}
			}
			valid_aggregates_passed_challenges
		}

		pub(crate) fn find_resulting_hash_from_leafs_and_paths(
			challenge_responses: Vec<ChallengeAggregateResponse>,
			cluster_id: &ClusterId,
			era_id: DdcEra,
			bucket_id: Option<BucketId>,
			node_id: String,
		) -> Result<ActivityHash, Vec<OCWError>> {
			let mut resulting_hash_from_leafs_and_paths = ActivityHash::default();

			if let Some(b_id) = bucket_id {
				log::info!("🚀find_resulting_hash_from_leafs_and_paths for bucket_id:{:?}", b_id);
			} else {
				log::info!("🚀find_resulting_hash_from_leafs_and_paths for node_id:{:?}", node_id);
			}

			for challenge_response in challenge_responses {
				for proof in challenge_response.proofs {
					let leaf_record_hashes: Vec<ActivityHash> =
						proof.leafs.into_iter().map(|p| p.hash::<T>()).collect();

					let leaf_record_hashes_string: Vec<String> =
						leaf_record_hashes.clone().into_iter().map(hex::encode).collect();

					log::info!(
						"🚀 Fetched leaf record hashes node id: {:?}  leaf_record_hashes: {:?}",
						node_id,
						leaf_record_hashes_string
					);

					let leaf_node_root =
						Self::create_merkle_root(cluster_id, era_id, &leaf_record_hashes)
							.map_err(|err| vec![err])?;

					log::info!(
						"🚀 Fetched leaf record root node id: {:?}   leaf_record_root_hash: {:?}",
						node_id,
						hex::encode(leaf_node_root)
					);

					let paths = proof.path.iter().rev();

					resulting_hash_from_leafs_and_paths = leaf_node_root;
					for path in paths {
						let mut dec_buf = [0u8; BUF_SIZE];
						let bytes = Base64::decode(path, &mut dec_buf).unwrap(); // todo! remove unwrap
						let path_hash: ActivityHash =
							ActivityHash::from(sp_core::H256::from_slice(bytes));

						let node_root = Self::create_merkle_root(
							cluster_id,
							era_id,
							&[resulting_hash_from_leafs_and_paths, path_hash],
						)
						.map_err(|err| vec![err])?;

						log::info!("🚀 Fetched leaf node root node id: {:?}  for path:{:?} leaf_node_hash: {:?}",
						node_id, path, hex::encode(node_root));

						resulting_hash_from_leafs_and_paths = node_root;
					}
				}
			}

			Ok(resulting_hash_from_leafs_and_paths)
		}
		pub(crate) fn find_random_merkle_node_ids(
			number_of_identifiers: usize,
			total_activity: u64,
			node_id: String,
		) -> Vec<u64> {
			let total_levels = total_activity.ilog2() + 1;

			let int_list: Vec<u64> = (0..total_levels as u64).collect();

			let nonce = Self::store_and_fetch_nonce(node_id);

			let mut small_rng = SmallRng::seed_from_u64(nonce);
			let ids: Vec<u64> = int_list
				.choose_multiple(&mut small_rng, number_of_identifiers)
				.cloned()
				.collect::<Vec<u64>>();

			ids
		}
		pub(crate) fn fetch_sub_trees(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			customer_activities: Vec<CustomerActivity>,
			quorum: u16,
		) -> (Vec<BucketNodeAggregatesActivity>, Vec<BucketNodeAggregatesActivity>) {
			let mut bucket_node_aggregates_activities: Vec<BucketNodeAggregatesActivity> =
				Vec::new();

			log::info!(
				"🏠⏳ Starting fetching bucket node aggregates for cluster_id: {:?} for era_id: {:?}",
				cluster_id,
				era_id
			);
			for customer_activity in customer_activities.clone() {
				for bucket_sub_aggregate in customer_activity.sub_aggregates.clone() {
					let bucket_node_aggregates_activity = BucketNodeAggregatesActivity {
						bucket_id: customer_activity.bucket_id,
						node_id: bucket_sub_aggregate.NodeID,
						stored_bytes: bucket_sub_aggregate.stored_bytes,
						transferred_bytes: bucket_sub_aggregate.transferred_bytes,
						number_of_puts: bucket_sub_aggregate.number_of_puts,
						number_of_gets: bucket_sub_aggregate.number_of_gets,
					};

					bucket_node_aggregates_activities.push(bucket_node_aggregates_activity);
				}
				log::info!("🏠🚀 Fetched Bucket node-aggregates for cluster_id: {:?} for era_id: {:?} for bucket_id {:?}::: Bucket Sub-Aggregates are {:?}", cluster_id, era_id, customer_activity.bucket_id, customer_activity.sub_aggregates);
			}

			Self::get_consensus_for_bucket_node_aggregates(
				cluster_id,
				era_id,
				bucket_node_aggregates_activities,
				quorum,
				Percent::from_percent(T::MAJORITY),
			)
		}
		#[allow(dead_code)]
		pub(crate) fn prepare_begin_billing_report(
			cluster_id: &ClusterId,
		) -> Result<Option<(DdcEra, i64, i64)>, OCWError> {
			Ok(Self::get_era_for_payout(cluster_id, EraValidationStatus::ReadyForPayout))
			// todo! get start and end values based on result
		}

		pub(crate) fn prepare_begin_charging_customers(
			cluster_id: &ClusterId,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
			min_nodes: u16,
			batch_size: usize,
		) -> Result<Option<(DdcEra, BatchIndex)>, Vec<OCWError>> {
			if let Some((era_id, start, end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::Initialized
				{
					if let Some((_, _, customers_activity_batch_roots, _, _, _)) =
						Self::fetch_validation_activities::<
							BucketNodeAggregatesActivity,
							NodeActivity,
						>(cluster_id, era_id)
					{
						Self::fetch_customer_activity(
							cluster_id,
							era_id,
							customers_activity_batch_roots,
						)
					} else {
						let era_activity = EraActivity { id: era_id, start, end };

						let _ = Self::process_dac_data(
							cluster_id,
							Some(era_activity),
							dac_nodes,
							min_nodes,
							batch_size,
						)?;

						if let Some((_, _, customers_activity_batch_roots, _, _, _)) =
							Self::fetch_validation_activities::<
								BucketNodeAggregatesActivity,
								NodeActivity,
							>(cluster_id, era_id)
						{
							Self::fetch_customer_activity(
								cluster_id,
								era_id,
								customers_activity_batch_roots,
							)
						} else {
							Ok(None)
						}
					}
				} else {
					Ok(None)
				}
			} else {
				Ok(None)
			}
		}

		pub(crate) fn fetch_customer_activity(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			customers_activity_batch_roots: Vec<ActivityHash>,
		) -> Result<Option<(DdcEra, BatchIndex)>, Vec<OCWError>> {
			if let Some(max_batch_index) = customers_activity_batch_roots.len().checked_sub(1)
			// -1 cause payout expects max_index, not length
			{
				let max_batch_index: u16 = max_batch_index.try_into().map_err(|_| {
					vec![OCWError::BatchIndexConversionFailed { cluster_id: *cluster_id, era_id }]
				})?;
				Ok(Some((era_id, max_batch_index)))
			} else {
				Err(vec![OCWError::EmptyCustomerActivity { cluster_id: *cluster_id, era_id }])
			}
		}

		pub(crate) fn prepare_send_charging_customers_batch(
			cluster_id: &ClusterId,
			batch_size: usize,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
			min_nodes: u16,
		) -> Result<Option<(DdcEra, CustomerBatch<T>)>, Vec<OCWError>> {
			if let Some((era_id, start, end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::ChargingCustomers
				{
					if let Some((
						bucket_nodes_activity_in_consensus,
						_,
						customers_activity_batch_roots,
						_,
						_,
						_,
					)) = Self::fetch_validation_activities::<
						BucketNodeAggregatesActivity,
						NodeActivity,
					>(cluster_id, era_id)
					{
						Self::fetch_charging_activities(
							cluster_id,
							batch_size,
							era_id,
							bucket_nodes_activity_in_consensus,
							customers_activity_batch_roots,
						)
					} else {
						let era_activity = EraActivity { id: era_id, start, end };

						let _ = Self::process_dac_data(
							cluster_id,
							Some(era_activity),
							dac_nodes,
							min_nodes,
							batch_size,
						)?;

						if let Some((
							bucket_nodes_activity_in_consensus,
							_,
							customers_activity_batch_roots,
							_,
							_,
							_,
						)) = Self::fetch_validation_activities::<
							BucketNodeAggregatesActivity,
							NodeActivity,
						>(cluster_id, era_id)
						{
							Self::fetch_charging_activities(
								cluster_id,
								batch_size,
								era_id,
								bucket_nodes_activity_in_consensus,
								customers_activity_batch_roots,
							)
						} else {
							Ok(None)
						}
					}
				} else {
					Ok(None)
				}
			} else {
				Ok(None)
			}
		}

		fn fetch_charging_activities(
			cluster_id: &ClusterId,
			batch_size: usize,
			era_id: DdcEra,
			bucket_nodes_activity_in_consensus: Vec<BucketNodeAggregatesActivity>,
			customers_activity_batch_roots: Vec<ActivityHash>,
		) -> Result<Option<(DdcEra, CustomerBatch<T>)>, Vec<OCWError>> {
			let batch_index = T::PayoutVisitor::get_next_customer_batch_for_payment(
				cluster_id, era_id,
			)
			.map_err(|_| {
				vec![OCWError::BillingReportDoesNotExist { cluster_id: *cluster_id, era_id }]
			})?;

			if let Some(index) = batch_index {
				let i: usize = index.into();
				// todo! store batched activity to avoid splitting it again each time
				let customers_activity_batched =
					Self::split_to_batches(&bucket_nodes_activity_in_consensus, batch_size);

				let batch_root = customers_activity_batch_roots[i];
				let store = MemStore::default();
				let mut mmr: MMR<ActivityHash, MergeActivityHash, &MemStore<ActivityHash>> =
					MemMMR::<_, MergeActivityHash>::new(0, &store);

				let leaf_position_map: Vec<(ActivityHash, u64)> = customers_activity_batch_roots
					.iter()
					.map(|a| (*a, mmr.push(*a).unwrap()))
					.collect();

				let leaf_position: Vec<(u64, ActivityHash)> = leaf_position_map
					.iter()
					.filter(|&(l, _)| l == &batch_root)
					.map(|&(ref l, p)| (p, *l))
					.collect();
				let position: Vec<u64> =
					leaf_position.clone().into_iter().map(|(p, _)| p).collect();

				let proof = mmr
					.gen_proof(position)
					.map_err(|_| OCWError::FailedToCreateMerkleProof {
						cluster_id: *cluster_id,
						era_id,
					})
					.map_err(|e| vec![e])?
					.proof_items()
					.to_vec();

				let batch_proof = MMRProof {
					mmr_size: mmr.mmr_size(),
					proof,
					leaf_with_position: leaf_position[0],
				};
				Ok(Some((
					era_id,
					CustomerBatch {
						batch_index: index,
						payers: customers_activity_batched[i]
							.iter()
							.map(|activity| {
								let account_id =
									T::CustomerVisitor::get_bucket_owner(&activity.bucket_id)
										.unwrap();
								let customer_usage = CustomerUsage {
									transferred_bytes: activity.transferred_bytes,
									stored_bytes: activity.stored_bytes,
									number_of_puts: activity.number_of_puts,
									number_of_gets: activity.number_of_gets,
								};
								(account_id, activity.bucket_id, customer_usage)
							})
							.collect(),
						batch_proof,
					},
				)))
			} else {
				Ok(None)
			}
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
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
			min_nodes: u16,
			batch_size: usize,
		) -> Result<Option<(DdcEra, BatchIndex, NodeUsage)>, Vec<OCWError>> {
			if let Some((era_id, start, end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				let nodes_total_usages = Self::get_nodes_total_usage(cluster_id, dac_nodes)?;

				let nodes_total_usage: i64 = nodes_total_usages
					.iter()
					.filter_map(|usage| usage.as_ref().map(|u| u.stored_bytes))
					.sum();

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
					)) = Self::fetch_validation_activities::<
						BucketNodeAggregatesActivity,
						NodeActivity,
					>(cluster_id, era_id)
					{
						Self::fetch_reward_activities(
							cluster_id,
							era_id,
							nodes_activity_in_consensus,
							nodes_activity_batch_roots,
							nodes_total_usage,
						)
					} else {
						let era_activity = EraActivity { id: era_id, start, end };

						let _ = Self::process_dac_data(
							cluster_id,
							Some(era_activity),
							dac_nodes,
							min_nodes,
							batch_size,
						)?;

						if let Some((
							_,
							_,
							_,
							nodes_activity_in_consensus,
							_,
							nodes_activity_batch_roots,
						)) = Self::fetch_validation_activities::<
							BucketNodeAggregatesActivity,
							NodeActivity,
						>(cluster_id, era_id)
						{
							Self::fetch_reward_activities(
								cluster_id,
								era_id,
								nodes_activity_in_consensus,
								nodes_activity_batch_roots,
								nodes_total_usage,
							)
						} else {
							Ok(None)
						}
					}
				} else {
					Ok(None)
				}
			} else {
				Ok(None)
			}
		}

		pub(crate) fn fetch_reward_activities(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			nodes_activity_in_consensus: Vec<NodeActivity>,
			nodes_activity_batch_roots: Vec<ActivityHash>,
			current_nodes_total_usage: i64,
		) -> Result<Option<(DdcEra, BatchIndex, NodeUsage)>, Vec<OCWError>> {
			if let Some(max_batch_index) = nodes_activity_batch_roots.len().checked_sub(1)
			// -1 cause payout expects max_index, not length
			{
				let max_batch_index: u16 = max_batch_index.try_into().map_err(|_| {
					vec![OCWError::BatchIndexConversionFailed { cluster_id: *cluster_id, era_id }]
				})?;

				let mut total_node_usage = NodeUsage {
					transferred_bytes: 0,
					stored_bytes: current_nodes_total_usage,
					number_of_puts: 0,
					number_of_gets: 0,
				};

				for activity in nodes_activity_in_consensus {
					total_node_usage.transferred_bytes += activity.transferred_bytes;
					total_node_usage.stored_bytes += activity.stored_bytes;
					total_node_usage.number_of_puts += activity.number_of_puts;
					total_node_usage.number_of_gets += activity.number_of_gets;
				}

				Ok(Some((era_id, max_batch_index, total_node_usage)))
			} else {
				Err(vec![OCWError::EmptyCustomerActivity { cluster_id: *cluster_id, era_id }])
			}
		}

		pub(crate) fn prepare_send_rewarding_providers_batch(
			cluster_id: &ClusterId,
			batch_size: usize,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
			min_nodes: u16,
		) -> Result<Option<(DdcEra, ProviderBatch<T>)>, Vec<OCWError>> {
			if let Some((era_id, start, end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::RewardingProviders
				{
					if let Some((
						_,
						_,
						_,
						nodes_activity_in_consensus,
						_,
						nodes_activity_batch_roots,
					)) = Self::fetch_validation_activities::<
						BucketNodeAggregatesActivity,
						NodeActivity,
					>(cluster_id, era_id)
					{
						Self::fetch_reward_provider_batch(
							cluster_id,
							batch_size,
							era_id,
							nodes_activity_in_consensus,
							nodes_activity_batch_roots,
						)
					} else {
						let era_activity = EraActivity { id: era_id, start, end };

						let _ = Self::process_dac_data(
							cluster_id,
							Some(era_activity),
							dac_nodes,
							min_nodes,
							batch_size,
						)?;

						if let Some((
							_,
							_,
							_,
							nodes_activity_in_consensus,
							_,
							nodes_activity_batch_roots,
						)) = Self::fetch_validation_activities::<
							BucketNodeAggregatesActivity,
							NodeActivity,
						>(cluster_id, era_id)
						{
							Self::fetch_reward_provider_batch(
								cluster_id,
								batch_size,
								era_id,
								nodes_activity_in_consensus,
								nodes_activity_batch_roots,
							)
						} else {
							Ok(None)
						}
					}
				} else {
					Ok(None)
				}
			} else {
				Ok(None)
			}
		}

		fn fetch_reward_provider_batch(
			cluster_id: &ClusterId,
			batch_size: usize,
			era_id: DdcEra,
			nodes_activity_in_consensus: Vec<NodeActivity>,
			nodes_activity_batch_roots: Vec<ActivityHash>,
		) -> Result<Option<(DdcEra, ProviderBatch<T>)>, Vec<OCWError>> {
			let batch_index = T::PayoutVisitor::get_next_provider_batch_for_payment(
				cluster_id, era_id,
			)
			.map_err(|_| {
				vec![OCWError::BillingReportDoesNotExist { cluster_id: *cluster_id, era_id }]
			})?;

			if let Some(index) = batch_index {
				let i: usize = index.into();
				// todo! store batched activity to avoid splitting it again each time
				let nodes_activity_batched =
					Self::split_to_batches(&nodes_activity_in_consensus, batch_size);

				let batch_root = nodes_activity_batch_roots[i];
				let store = MemStore::default();
				let mut mmr: MMR<ActivityHash, MergeActivityHash, &MemStore<ActivityHash>> =
					MemMMR::<_, MergeActivityHash>::new(0, &store);

				let leaf_position_map: Vec<(ActivityHash, u64)> = nodes_activity_batch_roots
					.iter()
					.map(|a| (*a, mmr.push(*a).unwrap()))
					.collect();

				let leaf_position: Vec<(u64, ActivityHash)> = leaf_position_map
					.iter()
					.filter(|&(l, _)| l == &batch_root)
					.map(|&(ref l, p)| (p, *l))
					.collect();
				let position: Vec<u64> =
					leaf_position.clone().into_iter().map(|(p, _)| p).collect();

				let proof = mmr
					.gen_proof(position)
					.map_err(|_| {
						vec![OCWError::FailedToCreateMerkleProof {
							cluster_id: *cluster_id,
							era_id,
						}]
					})?
					.proof_items()
					.to_vec();

				// todo! attend [i] through get(i).ok_or()
				// todo! attend accountid conversion
				let batch_proof = MMRProof {
					mmr_size: mmr.mmr_size(),
					proof,
					leaf_with_position: leaf_position[0],
				};
				Ok(Some((
					era_id,
					ProviderBatch {
						batch_index: index,
						payees: nodes_activity_batched[i]
							.iter()
							.map(|activity| {
								let node_id = activity.clone().node_id;
								let provider_id = Self::fetch_provider_id(node_id).unwrap(); // todo! remove unwrap
								let node_usage = NodeUsage {
									transferred_bytes: activity.transferred_bytes,
									stored_bytes: activity.stored_bytes,
									number_of_puts: activity.number_of_puts,
									number_of_gets: activity.number_of_gets,
								};
								(provider_id, node_usage)
							})
							.collect(),
						batch_proof,
					},
				)))
			} else {
				Ok(None)
			}
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
			bucket_nodes_activity_in_consensus: &[A],
			customers_activity_root: ActivityHash,
			customers_activity_batch_roots: &[ActivityHash],
			nodes_activity_in_consensus: &[B],
			nodes_activity_root: ActivityHash,
			nodes_activity_batch_roots: &[ActivityHash],
		) {
			let key = Self::derive_key(cluster_id, era_id);
			let encoded_tuple = (
				bucket_nodes_activity_in_consensus,
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

		pub(crate) fn get_nodes_total_usage(
			cluster_id: &ClusterId,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<Option<NodeUsage>>, Vec<OCWError>> {
			let mut results: Vec<Option<NodeUsage>> = Vec::new();
			let mut errors: Vec<OCWError> = Vec::new();

			for (node_pub_key, _params) in dac_nodes.iter() {
				match T::NodeVisitor::get_total_usage(node_pub_key) {
					Ok(usage) => results.push(usage),
					Err(_err) => {
						errors.push(OCWError::FailedToFetchNodeTotalUsage {
							cluster_id: *cluster_id,
							node_pub_key: node_pub_key.clone(),
						});
					},
				}
			}

			if !errors.is_empty() {
				return Err(errors);
			}

			Ok(results)
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
			log::info!(
				"🏠 Off-chain validation_activities cache hit for ClusterId: {:?} EraId: {:?}",
				cluster_id,
				era_id
			);
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
					bucket_nodes_activity_in_consensus,
					customers_activity_root,
					customers_activity_batch_roots,
					nodes_activity_in_consensus,
					nodes_activity_root,
					nodes_activity_batch_roots,
				)) => Some((
					bucket_nodes_activity_in_consensus,
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

		pub(crate) fn store_and_fetch_nonce(node_id: String) -> u64 {
			let key = format!("offchain::activities::nonce::{:?}", node_id).into_bytes();
			let encoded_nonce = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key)
				.unwrap_or_else(|| 0.encode());

			let nonce_data = match Decode::decode(&mut &encoded_nonce[..]) {
				Ok(nonce) => nonce,
				Err(err) => {
					// Print error message with details of the decoding error
					log::error!("🦀Decoding error while fetching nonce: {:?}", err);
					0
				},
			};

			let new_nonce = nonce_data + 1;

			sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, &key, &new_nonce.encode());
			nonce_data
		}
		pub(crate) fn store_provider_id<A: Encode>(
			// todo! (3) add tests
			node_id: String,
			provider_id: A,
		) {
			let key = format!("offchain::activities::provider_id::{:?}", node_id).into_bytes();
			let encoded_tuple = provider_id.encode();

			// Store the serialized data in local offchain storage
			sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, &key, &encoded_tuple);
		}

		pub(crate) fn fetch_provider_id<A: Decode>(node_id: String) -> Option<A> {
			let key = format!("offchain::activities::provider_id::{:?}", node_id).into_bytes();
			// Retrieve encoded tuple from local storage
			let encoded_tuple =
				match sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
					Some(data) => data,
					None => return None,
				};

			match Decode::decode(&mut &encoded_tuple[..]) {
				Ok(provider_id) => Some(provider_id),
				Err(err) => {
					// Print error message with details of the decoding error
					log::error!("🦀Decoding error while fetching provider id: {:?}", err);
					None
				},
			}
		}
		/// Converts a vector of activity batches into their corresponding Merkle roots.
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
		/// - `Vec<ActivityHash>`: A vector of Merkle roots, one for each batch of activities.
		pub(crate) fn convert_to_batch_merkle_roots<A: Activity>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			activities: Vec<Vec<A>>,
		) -> Result<Vec<ActivityHash>, OCWError> {
			activities
				.into_iter()
				.map(|inner_vec| {
					let activity_hashes: Vec<ActivityHash> =
						inner_vec.into_iter().map(|a| a.hash::<T>()).collect();
					Self::create_merkle_root(cluster_id, era_id, &activity_hashes).map_err(|_| {
						OCWError::FailedToCreateMerkleRoot { cluster_id: *cluster_id, era_id }
					})
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

		/// Creates a Merkle root from a list of activity hashes.
		///
		/// This function takes a slice of `ActivityHash` and constructs a Merkle tree
		/// using an in-memory store. It returns a tuple containing the Merkle root hash,
		/// the size of the Merkle tree, and a vector mapping each input leaf to its position
		/// in the Merkle tree.
		///
		/// # Input Parameters
		///
		/// * `leaves` - A slice of `ActivityHash` representing the leaves of the Merkle tree.
		///
		/// # Output
		///
		/// A `Result` containing:
		/// * A tuple with the Merkle root `ActivityHash`, the size of the Merkle tree, and a vector
		///   mapping each input leaf to its position in the Merkle tree.
		/// * `OCWError::FailedToCreateMerkleRoot` if there is an error creating the Merkle root.
		pub(crate) fn create_merkle_root(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			leaves: &[ActivityHash],
		) -> Result<ActivityHash, OCWError> {
			if leaves.is_empty() {
				return Ok(ActivityHash::default());
			}

			let store = MemStore::default();
			let mut mmr: MMR<ActivityHash, MergeActivityHash, &MemStore<ActivityHash>> =
				MemMMR::<_, MergeActivityHash>::new(0, &store);

			let mut leaves_with_position: Vec<(u64, ActivityHash)> =
				Vec::with_capacity(leaves.len());

			for &leaf in leaves {
				match mmr.push(leaf) {
					Ok(pos) => leaves_with_position.push((pos, leaf)),
					Err(_) =>
						return Err(OCWError::FailedToCreateMerkleRoot {
							cluster_id: *cluster_id,
							era_id,
						}),
				}
			}

			mmr.get_root()
				.map_err(|_| OCWError::FailedToCreateMerkleRoot { cluster_id: *cluster_id, era_id })
		}

		/// Verify whether leaf is part of tree
		///
		/// Parameters:
		/// - `root`: merkle root
		/// - `leaf`: Leaf of the tree
		pub(crate) fn proof_merkle_leaf(
			root: ActivityHash,
			batch_proof: &MMRProof,
		) -> Result<bool, Error<T>> {
			let proof: MerkleProof<ActivityHash, MergeActivityHash> =
				MerkleProof::new(batch_proof.mmr_size, batch_proof.proof.clone());
			proof
				.verify(root, vec![batch_proof.leaf_with_position])
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
		) -> Result<Option<EraActivity>, OCWError> {
			let current_validator_data = Self::fetch_current_validator()?;

			let current_validator = T::AccountId::decode(&mut &current_validator_data[..]).unwrap();

			let last_validated_era = Self::get_last_validated_era(cluster_id, current_validator)?
				.unwrap_or_else(DdcEra::default);

			log::info!("🚀 last_validated_era for cluster_id: {:?}", last_validated_era);
			let all_ids = Self::fetch_processed_era_for_node(cluster_id, dac_nodes)?;

			let ids_greater_than_last_validated_era: Vec<EraActivity> = all_ids
				.iter()
				.flat_map(|eras| eras.iter().filter(|&ids| ids.id > last_validated_era).cloned())
				.sorted()
				.collect::<Vec<EraActivity>>();

			let mut grouped_data: Vec<(u32, EraActivity)> = Vec::new();
			for (key, chunk) in
				&ids_greater_than_last_validated_era.into_iter().chunk_by(|elt| elt.clone())
			{
				grouped_data.push((chunk.count() as u32, key));
			}

			let all_node_eras = grouped_data
				.into_iter()
				.filter(|(v, _)| *v == dac_nodes.len() as u32)
				.map(|(_, id)| id)
				.collect::<Vec<EraActivity>>();

			Ok(all_node_eras.iter().cloned().min_by_key(|n| n.id))
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
		pub(crate) fn get_consensus_for_activities(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			activities: &[(NodePubKey, Vec<NodeActivity>)],
			min_nodes: u16,
			threshold: Percent,
		) -> (Vec<NodeActivity>, Vec<NodeActivity>) {
			let mut customer_buckets: BTreeMap<ActivityHash, Vec<NodeActivity>> = BTreeMap::new();

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
			let mut not_consensus_activities = Vec::new();
			let min_threshold = threshold * min_nodes;

			// Check if each customer/bucket appears in at least `min_nodes` nodes
			for (_id, activities) in customer_buckets {
				if activities.len() < min_nodes.into() {
					not_consensus_activities.extend(activities);
				} else if let Some(activity) =
					Self::reach_consensus(&activities, min_threshold.into())
				{
					consensus_activities.push(activity);
				} else {
					not_consensus_activities.extend(activities);
				}
			}

			log::info!("🏠👍 Node Sub-Trees, which are in consensus for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, consensus_activities);
			log::info!("🏠👎 Node Sub-Trees, which are not in consensus for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, not_consensus_activities);

			(consensus_activities, not_consensus_activities)
		}

		pub(crate) fn get_consensus_for_bucket_node_aggregates(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			activities: Vec<BucketNodeAggregatesActivity>,
			min_nodes: u16,
			threshold: Percent,
		) -> (Vec<BucketNodeAggregatesActivity>, Vec<BucketNodeAggregatesActivity>) {
			let mut bucket_node_aggregates: BTreeMap<
				ActivityHash,
				Vec<BucketNodeAggregatesActivity>,
			> = BTreeMap::new();

			// Flatten and collect all customer activities
			for activity in activities.iter() {
				bucket_node_aggregates
					.entry(activity.get_consensus_id::<T>())
					.or_default()
					.push(activity.clone());
			}

			let mut consensus_activities = Vec::new();
			let mut not_consensus_activities = Vec::new();
			let min_threshold = threshold * min_nodes;

			// Check if each customer/bucket appears in at least `min_nodes` nodes
			for (_id, activities) in bucket_node_aggregates {
				if activities.len() < min_nodes.into() {
					not_consensus_activities.extend(activities);
				} else if let Some(activity) =
					Self::reach_consensus(&activities, min_threshold.into())
				{
					consensus_activities.push(activity);
				} else {
					not_consensus_activities.extend(activities);
				}
			}

			// todo! Reduce log size and put small message
			log::info!("🏠👍 Bucket Sub-Trees, which are in consensus for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, consensus_activities);
			log::info!("🏠👎 Bucket Sub-Trees, which are not in consensus for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, not_consensus_activities);
			(consensus_activities, not_consensus_activities)
		}

		/// Fetch cluster to validate.
		fn get_cluster_to_validate() -> Result<ClusterId, Error<T>> {
			// todo! to implement
			Self::cluster_to_validate().ok_or(Error::ClusterToValidateRetrievalError)
		}

		/// Fetch Challenge node aggregate or bucket sub-aggregate.

		pub(crate) fn fetch_challenge_responses(
			cluster_id: &ClusterId,
			node_id: String,
			era_id: DdcEra,
			bucket_id: Option<BucketId>,
			merkle_node_identifiers: Vec<u64>,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<ChallengeAggregateResponse>, OCWError> {
			let mut challenge_responses = Vec::new();

			for (node_pub_key, node_params) in dac_nodes {
				// todo! probably shouldn't stop when some DAC is not responding as we can still
				// work with others
				let response = Self::fetch_challenge_response(
					node_id.clone(),
					era_id,
					bucket_id,
					merkle_node_identifiers.clone(),
					node_params,
				)
				.map_err(|_| OCWError::ChallengeResponseRetrievalError {
					cluster_id: *cluster_id,
					era_id,
					node_id: node_id.clone(),
					bucket_id,
					node_pub_key: node_pub_key.clone(),
				})?;

				challenge_responses.push(response);
			}

			Ok(challenge_responses)
		}
		/// Fetch challenge response.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
		pub(crate) fn fetch_challenge_response(
			node_id: String,
			era_id: DdcEra,
			bucket_id: Option<BucketId>,
			merkle_node_identifiers: Vec<u64>,
			node_params: &StorageNodeParams,
		) -> Result<ChallengeAggregateResponse, http::Error> {
			let scheme = "http";
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;

			let result = merkle_node_identifiers
				.iter()
				.map(|x| format!("{}", x.clone()))
				.collect::<Vec<_>>()
				.join(",");

			let url = if let Some(b_id) = bucket_id {
				format!(
					"{}://{}:{}/activity/buckets/{}/challenge?eraId={}&nodeId={}&merkleTreeNodeId={}",
					scheme, host, node_params.http_port, b_id, era_id, node_id, result
				)
			} else {
				format!(
					"{}://{}:{}/activity/nodes/{}/challenge?eraId={}&merkleTreeNodeId={}",
					scheme, host, node_params.http_port, node_id, era_id, result
				)
			};

			let request = http::Request::get(&url);
			let timeout = sp_io::offchain::timestamp()
				.add(sp_runtime::offchain::Duration::from_millis(RESPONSE_TIMEOUT));
			let pending = request.deadline(timeout).send().map_err(|_| http::Error::IoError)?;

			let response =
				pending.try_wait(timeout).map_err(|_| http::Error::DeadlineReached)??;
			if response.code != SUCCESS_CODE {
				return Err(http::Error::Unknown);
			}

			let body = response.body().collect::<Vec<u8>>();
			serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)
		}
		/// Fetch processed era.
		///
		/// Parameters:
		/// - `node_params`: DAC node parameters
		#[allow(dead_code)]
		pub(crate) fn fetch_processed_era(
			node_params: &StorageNodeParams,
		) -> Result<Vec<EraActivity>, http::Error> {
			let scheme = "http";
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let url = format!("{}://{}:{}/activity/eras", scheme, host, node_params.http_port);
			let request = http::Request::get(&url);
			let timeout = sp_io::offchain::timestamp()
				.add(sp_runtime::offchain::Duration::from_millis(RESPONSE_TIMEOUT));
			let pending = request.deadline(timeout).send().map_err(|_| http::Error::IoError)?;

			// todo! filter by status == PROCESSED

			let response =
				pending.try_wait(timeout).map_err(|_| http::Error::DeadlineReached)??;
			if response.code != SUCCESS_CODE {
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
			let scheme = "http";
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let url = format!(
				"{}://{}:{}/activity/buckets?eraId={}",
				scheme, host, node_params.http_port, era_id
			);

			let request = http::Request::get(&url);
			let timeout = sp_io::offchain::timestamp()
				.add(sp_runtime::offchain::Duration::from_millis(RESPONSE_TIMEOUT));
			let pending = request.deadline(timeout).send().map_err(|_| http::Error::IoError)?;

			let response =
				pending.try_wait(timeout).map_err(|_| http::Error::DeadlineReached)??;
			if response.code != SUCCESS_CODE {
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
			let scheme = "http";
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let url = format!(
				"{}://{}:{}/activity/nodes?eraId={}",
				scheme, host, node_params.http_port, era_id
			);

			let request = http::Request::get(&url);
			let timeout = sp_io::offchain::timestamp()
				.add(rt_offchain::Duration::from_millis(RESPONSE_TIMEOUT));
			let pending = request.deadline(timeout).send().map_err(|_| http::Error::IoError)?;

			let response =
				pending.try_wait(timeout).map_err(|_| http::Error::DeadlineReached)??;
			if response.code != SUCCESS_CODE {
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
					let NodePubKey::StoragePubKey(key) = node_pub_key.clone();
					let node_pub_key_ref: &[u8; 32] = key.as_ref();
					let node_pub_key_string = hex::encode(node_pub_key_ref);
					log::info!(
						"🏭📝Get DAC Node for cluster_id: {:?} and node_pub_key: {:?}",
						cluster_id,
						node_pub_key_string
					);

					// Add to the results if the mode matches
					dac_nodes.push((node_pub_key, storage_params));
				}
			}

			Ok(dac_nodes)
		}

		fn get_node_provider_id(node_pub_key: &NodePubKey) -> Result<T::AccountId, OCWError> {
			T::NodeVisitor::get_node_provider_id(node_pub_key)
				.map_err(|_| OCWError::FailedToFetchNodeProvider)
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
				for node_activity in usage.clone() {
					let provider_id = Self::get_node_provider_id(node_pub_key).unwrap();
					Self::store_provider_id(node_activity.node_id, provider_id); // todo! this is not good - needs to be
					                                         // moved payout pallet
				}

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
		pub(crate) fn fetch_customers_usage_for_era(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<CustomerActivity>, OCWError> {
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

				customers_usages.extend(usage);
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
			era_activity: EraActivity,
			payers_merkle_root_hash: ActivityHash,
			payees_merkle_root_hash: ActivityHash,
			payers_batch_merkle_root_hashes: Vec<ActivityHash>,
			payees_batch_merkle_root_hashes: Vec<ActivityHash>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorised);
			let mut era_validation = {
				let era_validations = <EraValidations<T>>::get(cluster_id, era_activity.id);

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

			// disallow signatures after era status change
			ensure!(
				era_validation.status == EraValidationStatus::ValidatingData,
				Error::<T>::NotExpectedState
			);

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
			if threshold <= signed_validators.len() {
				// Update payers_merkle_root_hash and payees_merkle_root_hash as ones passed the
				// threshold
				era_validation.payers_merkle_root_hash = payers_merkle_root_hash;
				era_validation.payees_merkle_root_hash = payees_merkle_root_hash;
				era_validation.start_era = era_activity.start; // todo! start/end is set by the last validator and is not in consensus
				era_validation.end_era = era_activity.end;

				if payers_merkle_root_hash == ActivityHash::default() &&
					payees_merkle_root_hash == payers_merkle_root_hash
				{
					era_validation.status = EraValidationStatus::PayoutSuccess;
				} else {
					era_validation.status = EraValidationStatus::ReadyForPayout;
				}

				should_deposit_ready_event = true;
			}

			// Update the EraValidations storage
			<EraValidations<T>>::insert(cluster_id, era_activity.id, era_validation);
			Self::deposit_event(Event::<T>::EraValidationRootsPosted {
				cluster_id,
				era_id: era_activity.id,
				validator: caller,
				payers_merkle_root_hash,
				payees_merkle_root_hash,
				payers_batch_merkle_root_hashes,
				payees_batch_merkle_root_hashes,
			});
			if should_deposit_ready_event {
				Self::deposit_event(Event::<T>::EraValidationReady {
					cluster_id,
					era_id: era_activity.id,
				});
			} else {
				Self::deposit_event(Event::<T>::EraValidationNotReady {
					cluster_id,
					era_id: era_activity.id,
				});
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
			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorised);

			for error in errors {
				match error {
					OCWError::NotEnoughNodesForConsensus { cluster_id, era_id, node_id } => {
						Self::deposit_event(Event::NotEnoughNodesForConsensus {
							cluster_id,
							era_id,
							node_id,
							validator: caller.clone(),
						});
					},
					OCWError::NotEnoughBucketsForConsensus { cluster_id, era_id, bucket_id } => {
						Self::deposit_event(Event::NotEnoughBucketsForConsensus {
							cluster_id,
							era_id,
							bucket_id,
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
					OCWError::SendRewardingProvidersBatchTransactionError {
						cluster_id,
						era_id,
						batch_index,
					} => {
						Self::deposit_event(Event::SendRewardingProvidersBatchTransactionError {
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
					OCWError::FailedToCreateMerkleRoot { cluster_id, era_id } => {
						Self::deposit_event(Event::FailedToCreateMerkleRoot {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::FailedToCreateMerkleProof { cluster_id, era_id } => {
						Self::deposit_event(Event::FailedToCreateMerkleProof {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::FailedToFetchCurrentValidator => {
						Self::deposit_event(Event::FailedToFetchCurrentValidator {
							validator: caller.clone(),
						});
					},
					OCWError::FailedToFetchNodeProvider => {
						Self::deposit_event(Event::FailedToFetchNodeProvider {
							validator: caller.clone(),
						});
					},
					OCWError::FailedToFetchNodeTotalUsage { cluster_id, node_pub_key } => {
						Self::deposit_event(Event::FailedToFetchNodeTotalUsage {
							cluster_id,
							node_pub_key,
							validator: caller.clone(),
						});
					},
					OCWError::BucketAggregatesRetrievalError {
						cluster_id,
						era_id,
						bucket_id,
						node_pub_key,
					} => {
						Self::deposit_event(Event::BucketAggregatesRetrievalError {
							cluster_id,
							era_id,
							bucket_id,
							node_pub_key,
							validator: caller.clone(),
						});
					},
					OCWError::NotEnoughNodeAggregatesForConsensus {
						cluster_id,
						era_id,
						bucket_id,
						node_id,
					} => {
						Self::deposit_event(Event::NotEnoughNodeAggregatesForConsensus {
							cluster_id,
							era_id,
							bucket_id,
							node_id,
							validator: caller.clone(),
						});
					},
					OCWError::ChallengeResponseRetrievalError {
						cluster_id,
						era_id,
						node_id,
						bucket_id,
						node_pub_key,
					} => {
						Self::deposit_event(Event::ChallengeResponseRetrievalError {
							cluster_id,
							era_id,
							node_id,
							bucket_id,
							node_pub_key,
							validator: caller.clone(),
						});
					},
					OCWError::NotEnoughRecordsForConsensus { cluster_id, era_id, record_id } => {
						Self::deposit_event(Event::NotEnoughRecordsForConsensus {
							cluster_id,
							era_id,
							record_id,
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

			let stash = T::StakingVisitor::stash_by_ctrl(&controller)
				.map_err(|_| Error::<T>::NotController)?;

			ensure!(
				<ValidatorSet<T>>::get().contains(&ddc_validator_pub),
				Error::<T>::NotValidatorStash
			);

			ValidatorToStashKey::<T>::insert(&ddc_validator_pub, &stash);
			Self::deposit_event(Event::<T>::ValidatorKeySet { validator: ddc_validator_pub });
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

			EraValidations::<T>::try_mutate(
				cluster_id,
				era_id,
				|maybe_era_validations| -> DispatchResult {
					maybe_era_validations.as_mut().ok_or(Error::<T>::NoEraValidation)?.status =
						EraValidationStatus::PayoutInProgress;
					Ok(())
				},
			)?;

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
		pub fn send_charging_customers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
			payers: Vec<(T::AccountId, BucketId, CustomerUsage)>,
			batch_proof: MMRProof,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutVisitor::send_charging_customers_batch(
				sender,
				cluster_id,
				era_id,
				batch_index,
				&payers,
				batch_proof,
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
		pub fn send_rewarding_providers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
			payees: Vec<(T::AccountId, NodeUsage)>,
			batch_proof: MMRProof,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutVisitor::send_rewarding_providers_batch(
				sender,
				cluster_id,
				era_id,
				batch_index,
				&payees,
				batch_proof,
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

			T::ClusterValidator::set_last_validated_era(&cluster_id, era_id)
		}

		#[pallet::call_index(11)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn set_cluster_to_validate(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
		) -> DispatchResult {
			ensure_root(origin)?;
			ClusterToValidate::<T>::put(cluster_id);

			Ok(())
		}

		// todo! Need to remove this
		#[pallet::call_index(12)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn set_current_validator(origin: OriginFor<T>) -> DispatchResult {
			let validator = ensure_signed(origin)?;

			if !<ValidatorSet<T>>::get().contains(&validator) {
				ValidatorSet::<T>::append(validator);
			}

			Ok(())
		}

		// todo! remove this after devnet testing
		#[pallet::call_index(13)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn set_era_validations(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			ensure_root(origin)?;
			let era_validations = <EraValidations<T>>::get(cluster_id, era_id);

			if era_validations.is_none() {
				let mut era_validation = EraValidation {
					payers_merkle_root_hash: ActivityHash::default(),
					payees_merkle_root_hash: ActivityHash::default(),
					start_era: Default::default(),
					end_era: Default::default(),
					validators: Default::default(),
					status: EraValidationStatus::PayoutSkipped,
				};

				let signed_validators = era_validation
					.validators
					.entry((ActivityHash::default(), ActivityHash::default()))
					.or_insert_with(Vec::new);

				let validators = <ValidatorSet<T>>::get();

				signed_validators.extend(validators);

				<EraValidations<T>>::insert(cluster_id, era_id, era_validation);
			}

			Self::deposit_event(Event::<T>::EraValidationReady { cluster_id, era_id });

			Ok(())
		}
	}

	impl<T: Config> ValidatorVisitor<T> for Pallet<T> {
		fn setup_validators(validators: Vec<T::AccountId>) {
			ValidatorSet::<T>::put(validators);
		}
		fn is_ocw_validator(caller: T::AccountId) -> bool {
			if ValidatorToStashKey::<T>::contains_key(caller.clone()) {
				<ValidatorSet<T>>::get().contains(&caller)
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
			batch_proof: &MMRProof,
		) -> bool {
			let validation_era = EraValidations::<T>::get(cluster_id, era_id);

			match validation_era {
				Some(valid_era) => {
					//Self::create_merkle_root(leaves)

					let root = valid_era.payers_merkle_root_hash;
					Self::proof_merkle_leaf(root, batch_proof).unwrap_or(false)
				},
				None => false,
			}
		}

		// todo! use batch_index and payees as part of the validation
		fn is_providers_batch_valid(
			cluster_id: ClusterId,
			era_id: DdcEra,
			_batch_index: BatchIndex,
			_payees: &[(T::AccountId, NodeUsage)],
			batch_proof: &MMRProof,
		) -> bool {
			let validation_era = EraValidations::<T>::get(cluster_id, era_id);

			match validation_era {
				Some(valid_era) => {
					let root = valid_era.payees_merkle_root_hash;
					Self::proof_merkle_leaf(root, batch_proof).unwrap_or(false)
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

			ValidatorSet::<T>::put(validators); // only active validators in session - this is NOT all the
			                        // validators
		}

		fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_authorities: I)
		where
			I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
		{
			log::info!("🙌Adding Validator from new session.");
			let validators = validators
				.map(|(_, k)| T::AccountId::decode(&mut &k.into().encode()[..]).unwrap())
				.collect::<Vec<_>>();
			log::info!("🙌Total validator from new session. {:?}", validators.len());
			ValidatorSet::<T>::put(validators);
		}

		fn on_disabled(_i: u32) {}
	}
}
