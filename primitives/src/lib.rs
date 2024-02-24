#![cfg_attr(not(feature = "std"), no_std)]

use blake2::{Blake2s256, Digest};
use codec::{Decode, Encode};
use frame_support::parameter_types;
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
use frame_system::Config;
use polkadot_ckb_merkle_mountain_range::Merge;
use scale_info::{
	prelude::{collections::BTreeMap, string::String, vec::Vec},
<<<<<<< HEAD
	TypeInfo,
};
=======
use polkadot_ckb_merkle_mountain_range::Merge;
<<<<<<< HEAD
use scale_info::{prelude::vec::Vec, TypeInfo};
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
use scale_info::{
	prelude::{string::String, vec::Vec},
=======
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
	TypeInfo,
};
>>>>>>> 492c0466 (chore: more logging info for tracing)
use serde::{Deserialize, Serialize};
use sp_core::{crypto::KeyTypeId, hash::H160, H256};
use sp_runtime::{AccountId32, Perquintill, RuntimeDebug};
use sp_std::collections::btree_set::BTreeSet;

pub mod traits;

parameter_types! {
	pub MaxHostLen: u8 = 255;
	pub MaxDomainLen: u8 = 255;
}

pub const MAX_PAYOUT_BATCH_COUNT: u16 = 1000;
pub const MAX_PAYOUT_BATCH_SIZE: u16 = 500;
pub const MILLICENTS: u128 = 100_000;
pub const CENTS: u128 = 1_000 * MILLICENTS; // assume this is worth about a cent.
pub const DOLLARS: u128 = 100 * CENTS;
pub type ClusterId = H160;
pub type DdcEra = u32;
pub type BucketId = u64;
pub type ClusterNodesCount = u16;
pub type StorageNodePubKey = AccountId32;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
pub type ActivityHash = [u8; 32];
=======
/// Hash of verified or unverified delta usage of a bucket or a node.
pub type DeltaUsageHash = H256;
/// Hash of usage that a customer is supposed to be charged for, or a provider supposed to be
/// rewarded for. Includes the current usage and verified delta usage.
pub type PayableUsageHash = H256;
/// Selective hash of sensitive information for payouts.
pub type Fingerprint = H256;

>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
pub type BatchIndex = u16;
pub const AVG_SECONDS_MONTH: i64 = 2630016; // 30.44 * 24.0 * 3600.0;
<<<<<<< HEAD
=======
pub type ActivityHash = [u8; 32];
pub type BatchIndex = u16;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)

pub struct MergeMMRHash;
impl Merge for MergeMMRHash {
	type Item = H256;
	fn merge(
		lhs: &Self::Item, // Left side of tree
		rhs: &Self::Item, // Right side of tree
	) -> Result<Self::Item, polkadot_ckb_merkle_mountain_range::Error> {
		let mut hasher = Blake2s256::new();

		hasher.update(lhs.0.as_slice());
		hasher.update(rhs.0.as_slice());
		let hash = hasher.finalize();

		Ok(H256::from_slice(hash.as_slice()))
	}
}
<<<<<<< HEAD
=======
pub type ClusterNodesCount = u16;
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))

// ClusterParams includes Governance non-sensetive parameters only
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterParams<AccountId> {
	pub node_provider_auth_contract: Option<AccountId>,
	pub erasure_coding_required: u32,
	pub erasure_coding_total: u32,
	pub replication_total: u32,
}

#[cfg(feature = "std")]
impl<AccountId> Default for ClusterParams<AccountId> {
	fn default() -> Self {
		ClusterParams {
			node_provider_auth_contract: None,
			erasure_coding_required: 0,
			erasure_coding_total: 0,
			replication_total: 0,
		}
	}
}

// ClusterProtocolParams includes Governance sensitive parameters
#[derive(
	Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Default, Serialize, Deserialize,
)]
#[scale_info(skip_type_params(Balance, BlockNumber, T))]
pub struct ClusterProtocolParams<Balance, BlockNumber> {
	pub treasury_share: Perquintill,
	pub validators_share: Perquintill,
	pub cluster_reserve_share: Perquintill,
	pub storage_bond_size: Balance,
	pub storage_chill_delay: BlockNumber,
	pub storage_unbonding_delay: BlockNumber,
	pub unit_per_mb_stored: u128,
	pub unit_per_mb_streamed: u128,
	pub unit_per_put_request: u128,
	pub unit_per_get_request: u128,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterPricingParams {
	pub unit_per_mb_stored: u128,
	pub unit_per_mb_streamed: u128,
	pub unit_per_put_request: u128,
	pub unit_per_get_request: u128,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterFeesParams {
	pub treasury_share: Perquintill,
	pub validators_share: Perquintill,
	pub cluster_reserve_share: Perquintill,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterBondingParams<BlockNumber> {
	pub storage_bond_size: u128,
	pub storage_chill_delay: BlockNumber,
	pub storage_unbonding_delay: BlockNumber,
}

#[derive(
<<<<<<< HEAD
<<<<<<< HEAD
	Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, PartialEq, Eq, Encode, Decode, TypeInfo,
)]
pub struct AggregatorInfo {
<<<<<<< HEAD
	pub node_pub_key: NodePubKey,
	pub node_params: StorageNodeParams,
}

#[derive(
	Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, PartialEq, Eq, Encode, Decode, TypeInfo,
=======
	Debug,
	Serialize,
	Deserialize,
	Clone,
	Ord,
	PartialOrd,
	PartialEq,
	Eq,
	Encode,
	Decode,
	TypeInfo,
=======
	Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, PartialEq, Eq, Encode, Decode, TypeInfo,
>>>>>>> c54cb6d0 (refactoring)
)]
pub struct NodeData {
=======
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
	pub node_pub_key: NodePubKey,
	pub node_params: StorageNodeParams,
}

#[derive(
<<<<<<< HEAD
	Debug,
	Serialize,
	Deserialize,
	Clone,
	Ord,
	PartialOrd,
	PartialEq,
	Eq,
	Encode,
	Decode,
	TypeInfo,
>>>>>>> 874386e0 (Addressed PR comments)
=======
	Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, PartialEq, Eq, Encode, Decode, TypeInfo,
>>>>>>> c54cb6d0 (refactoring)
)]
pub enum NodePubKey {
	StoragePubKey(StorageNodePubKey),
}

impl NodePubKey {
	pub fn get_hex(&self) -> String {
		match self {
			NodePubKey::StoragePubKey(pub_key_ref) => hex::encode(pub_key_ref),
		}
	}
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodeType {
	Storage = 1,
}

impl From<NodeType> for u8 {
	fn from(node_type: NodeType) -> Self {
		match node_type {
			NodeType::Storage => 1,
		}
	}
}

impl TryFrom<u8> for NodeType {
	type Error = ();
	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			1 => Ok(NodeType::Storage),
			_ => Err(()),
		}
	}
}

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
	Encode,
	Decode,
	TypeInfo,
)]
pub enum StorageNodeMode {
	/// DDC Storage node operates with enabled caching in RAM and stores data in Hard Drive
	Full = 1,
	/// DDC Storage node operates with disabled caching in RAM and stores data in Hard Drive
	Storage = 2,
	/// DDC Storage node operates with enabled caching in RAM and doesn't store data in Hard Drive
	Cache = 3,
	// DAC node
	DAC = 4,
}

<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 874386e0 (Addressed PR comments)
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
	Encode,
	Decode,
	TypeInfo,
)]
<<<<<<< HEAD
=======
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
=======
>>>>>>> 874386e0 (Addressed PR comments)
pub struct StorageNodeParams {
	pub mode: StorageNodeMode,
	pub host: Vec<u8>,
	pub domain: Vec<u8>,
	pub ssl: bool,
	pub http_port: u16,
	pub grpc_port: u16,
	pub p2p_port: u16,
}

#[cfg(feature = "std")]
impl Default for StorageNodeParams {
	fn default() -> Self {
		StorageNodeParams {
			mode: StorageNodeMode::Full,
			host: Default::default(),
			domain: Default::default(),
			ssl: Default::default(),
			http_port: Default::default(),
			grpc_port: Default::default(),
			p2p_port: Default::default(),
		}
	}
}

// Params fields are always coming from extrinsic input
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodeParams {
	StorageParams(StorageNodeParams),
}

/// DDC cluster status
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
pub enum ClusterStatus {
	Unbonded,
	Bonded,
	Activated,
	Unbonding,
}

/// DDC node kind added to DDC cluster
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
pub enum ClusterNodeKind {
	Genesis,
	External,
}

/// DDC node status in to DDC cluster
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
pub enum ClusterNodeStatus {
	AwaitsValidation,
	ValidationSucceeded,
	ValidationFailed,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterNodeState<BlockNumber> {
	pub kind: ClusterNodeKind,
	pub status: ClusterNodeStatus,
	pub added_at: BlockNumber,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Default)]
pub struct ClusterNodesStats {
	pub await_validation: ClusterNodesCount,
	pub validation_succeeded: ClusterNodesCount,
	pub validation_failed: ClusterNodesCount,
}
<<<<<<< HEAD
<<<<<<< HEAD

/// Stores usage of a bucket
#[derive(
	PartialEq,
	Eq,
	Encode,
	Decode,
	Debug,
	TypeInfo,
	Default,
	Clone,
	Serialize,
	Deserialize,
	PartialOrd,
	Ord,
)]
pub struct BucketUsage {
	pub transferred_bytes: u64,
	pub stored_bytes: i64,
=======

/// Stores usage of a bucket
#[derive(
	PartialEq, Eq, Encode, Decode, Debug, TypeInfo, Default, Clone, Serialize, Deserialize,
)]
pub struct BucketUsage {
	pub transferred_bytes: u64,
<<<<<<< HEAD
	pub stored_bytes: u64,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	pub stored_bytes: i64,
>>>>>>> 00eed38c (Changes to accept stored_bytes as signed input (#410))
	pub number_of_puts: u64,
	pub number_of_gets: u64,
}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
/// Stores charge in tokens(units) of customer as per BucketUsage
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct CustomerCharge {
	pub transfer: u128, // charge in tokens for BucketUsage::transferred_bytes
	pub storage: u128,  // charge in tokens for BucketUsage::stored_bytes
	pub puts: u128,     // charge in tokens for BucketUsage::number_of_puts
	pub gets: u128,     // charge in tokens for BucketUsage::number_of_gets
}

/// Stores usage of a node
=======
/// Stores charge in tokens(units) of customer as per CustomerUsage
=======
/// Stores charge in tokens(units) of customer as per BucketUsage
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct CustomerCharge {
	pub transfer: u128, // charge in tokens for BucketUsage::transferred_bytes
	pub storage: u128,  // charge in tokens for BucketUsage::stored_bytes
	pub puts: u128,     // charge in tokens for BucketUsage::number_of_puts
	pub gets: u128,     // charge in tokens for BucketUsage::number_of_gets
}

<<<<<<< HEAD
/// Stores usage of node provider
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
=======
/// Stores usage of a node
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
#[derive(
	PartialEq,
	Eq,
	Encode,
	Decode,
	Debug,
	TypeInfo,
	Default,
	Clone,
	Serialize,
	Deserialize,
	PartialOrd,
	Ord,
)]
pub struct NodeUsage {
	pub transferred_bytes: u64,
	pub stored_bytes: i64,
=======
/// Stores usage of node provider
#[derive(
	PartialEq, Eq, Encode, Decode, Debug, TypeInfo, Default, Clone, Serialize, Deserialize,
)]
pub struct NodeUsage {
	pub transferred_bytes: u64,
<<<<<<< HEAD
	pub stored_bytes: u64,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	pub stored_bytes: i64,
>>>>>>> 00eed38c (Changes to accept stored_bytes as signed input (#410))
	pub number_of_puts: u64,
	pub number_of_gets: u64,
}

<<<<<<< HEAD
<<<<<<< HEAD
/// Stores reward in tokens(units) of node provider as per NodeUsage
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct ProviderReward {
=======
/// Stores reward in tokens(units) of node provider as per NodeUsage
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
<<<<<<< HEAD
pub struct NodeReward {
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
=======
pub struct ProviderReward {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
	pub transfer: u128, // reward in tokens for NodeUsage::transferred_bytes
	pub storage: u128,  // reward in tokens for NodeUsage::stored_bytes
	pub puts: u128,     // reward in tokens for NodeUsage::number_of_puts
	pub gets: u128,     // reward in tokens for NodeUsage::number_of_gets
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Default)]
pub struct MMRProof {
<<<<<<< HEAD
	pub proof: Vec<ActivityHash>,
=======
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Default)]
pub struct MMRProof {
	pub proof: Vec<ActivityHash>,
<<<<<<< HEAD
	pub leaf_with_position: (u64, ActivityHash),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 29b2c1bf (chore: hash and position of the leaf is removed from tx parameters)
=======
	pub proof: Vec<DeltaUsageHash>,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
}

#[derive(Debug, PartialEq)]
pub enum NodeRepositoryError {
	StorageNodeAlreadyExists,
	StorageNodeDoesNotExist,
}

#[derive(Debug, PartialEq)]
<<<<<<< HEAD
<<<<<<< HEAD
=======
pub enum BucketVisitorError {
	NoBucketWithId,
	NotBucketOwner,
	IncorrectClusterId,
}

#[derive(Debug, PartialEq)]
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> c3a18c28 (chore: using generic type for bucket visitor)
pub enum PayoutError {
	BillingReportDoesNotExist,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Default)]
// don't remove or change numbers, if needed add a new state to the end with new number
// DAC uses the state value for integration!
pub enum PayoutState {
	#[default]
	NotInitialized = 1,
	Initialized = 2,
	ChargingCustomers = 3,
	CustomersChargedWithFees = 4,
	RewardingProviders = 5,
	ProvidersRewarded = 6,
	Finalized = 7,
}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct BucketParams {
	pub is_public: bool,
}

pub const DAC_VERIFICATION_KEY_TYPE: KeyTypeId = KeyTypeId(*b"cer!");

pub mod sr25519 {
	mod app_sr25519 {
<<<<<<< HEAD
		use sp_application_crypto::{app_crypto, sr25519};

		use crate::{String, DAC_VERIFICATION_KEY_TYPE};

<<<<<<< HEAD
		app_crypto!(sr25519, DAC_VERIFICATION_KEY_TYPE);
=======
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"cer!");
=======
pub const DAC_VERIFICATION_KEY_TYPE: KeyTypeId = KeyTypeId(*b"cer!");
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)

pub mod sr25519 {
	mod app_sr25519 {
		use scale_info::prelude::string::String;
=======
>>>>>>> 8a36f637 (fix: clippy and formatiing)
		use sp_application_crypto::{app_crypto, sr25519};

<<<<<<< HEAD
		use crate::KEY_TYPE;
		app_crypto!(sr25519, KEY_TYPE);
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		use crate::DAC_VERIFICATION_KEY_TYPE;
=======
>>>>>>> fc5f2ebc (Fix `ddc-primitives` missing import build error)
		app_crypto!(sr25519, DAC_VERIFICATION_KEY_TYPE);
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
	}

	sp_application_crypto::with_pair! {
		pub type AuthorityPair = app_sr25519::Pair;
	}
	pub type AuthoritySignature = app_sr25519::Signature;
	pub type AuthorityId = app_sr25519::Public;
}

pub mod crypto {
<<<<<<< HEAD
<<<<<<< HEAD
=======
	use scale_info::prelude::string::String;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 8a36f637 (fix: clippy and formatiing)
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
	use crate::{String, DAC_VERIFICATION_KEY_TYPE};

	app_crypto!(sr25519, DAC_VERIFICATION_KEY_TYPE);
=======
	use super::KEY_TYPE;
	app_crypto!(sr25519, KEY_TYPE);
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	use super::DAC_VERIFICATION_KEY_TYPE;
=======
	use crate::{String, DAC_VERIFICATION_KEY_TYPE};

>>>>>>> fc5f2ebc (Fix `ddc-primitives` missing import build error)
	app_crypto!(sr25519, DAC_VERIFICATION_KEY_TYPE);
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
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
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)

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
	pub validators: BTreeMap<(DeltaUsageHash, DeltaUsageHash), Vec<T::AccountId>>,
	pub start_era: i64,
	pub end_era: i64,
	pub payers_merkle_root_hash: DeltaUsageHash,
	pub payees_merkle_root_hash: DeltaUsageHash,
	pub status: EraValidationStatus,
}
<<<<<<< HEAD
<<<<<<< HEAD

impl<T: Config> Default for EraValidation<T> {
	fn default() -> Self {
		EraValidation {
			validators: Default::default(),
			start_era: Default::default(),
			end_era: Default::default(),
			payers_merkle_root_hash: Default::default(),
			payees_merkle_root_hash: Default::default(),
			status: EraValidationStatus::PayoutSkipped,
		}
	}
}

#[derive(Default)]
<<<<<<< HEAD
=======

>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
=======
>>>>>>> 27474823 (Default trait for primitives)
pub struct BillingReportParams {
	pub cluster_id: ClusterId,
	pub era: DdcEra,
	pub state: PayoutState,
	pub fingerprint: Fingerprint,
	pub total_customer_charge: CustomerCharge,
	pub total_distributed_reward: u128,
	pub charging_max_batch_index: BatchIndex,
	pub charging_processed_batches: Vec<BatchIndex>,
	pub rewarding_max_batch_index: BatchIndex,
	pub rewarding_processed_batches: Vec<BatchIndex>,
}
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
=======
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
=======

#[derive(Default)]
pub struct BillingFingerprintParams<AccountId> {
	pub cluster_id: ClusterId,
	pub era: DdcEra,
	pub start_era: i64,
	pub end_era: i64,
	pub payers_merkle_root: PayableUsageHash,
	pub payees_merkle_root: PayableUsageHash,
	pub cluster_usage: NodeUsage,
	pub validators: BTreeSet<AccountId>,
}

pub struct BucketStorageUsage<AccountId> {
	pub bucket_id: BucketId,
	pub owner_id: AccountId,
	pub stored_bytes: i64,
}

pub struct NodeStorageUsage<AccountId> {
	pub node_key: NodePubKey,
	pub provider_id: AccountId,
	pub stored_bytes: i64,
}
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
