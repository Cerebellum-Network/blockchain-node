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

<<<<<<< HEAD
<<<<<<< HEAD
use base64ct::{Base64, Encoding};
#[cfg(feature = "runtime-benchmarks")]
use ddc_primitives::traits::{BucketManager, ClusterCreator, CustomerDepositor};
<<<<<<< HEAD
use ddc_primitives::{
	traits::{
		ClusterManager, ClusterValidator, CustomerVisitor, NodeManager, PayoutProcessor,
		StorageUsageProvider, ValidatorVisitor,
	},
	BatchIndex, BillingFingerprintParams, BillingReportParams, BucketStorageUsage, BucketUsage,
	ClusterId, ClusterStatus, DdcEra, EraValidation, EraValidationStatus, MMRProof, NodeParams,
	NodePubKey, NodeStorageUsage, NodeUsage, PayableUsageHash, PayoutState, StorageNodeParams,
	StorageNodePubKey,
};
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, Get, OneSessionHandler},
=======
=======
use base64ct::{Base64, Encoding};
>>>>>>> 18369b0b (Challenge sub trees and make them ready for payout (#434))
=======
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
use ddc_primitives::{
	traits::{
		ClusterManager, ClusterValidator, CustomerVisitor, NodeManager, PayoutProcessor,
		ValidatorVisitor,
	},
	ActivityHash, BatchIndex, BillingReportParams, BucketUsage, ClusterId, ClusterStatus, DdcEra,
	EraValidation, EraValidationStatus, MMRProof, NodeParams, NodePubKey, NodeUsage, PayoutState,
	StorageNodeParams,
};
use frame_support::{
	pallet_prelude::*,
<<<<<<< HEAD
	traits::{Get, OneSessionHandler},
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	traits::{Currency, Get, OneSessionHandler},
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
};
use frame_system::{
	offchain::{Account, AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer},
	pallet_prelude::*,
};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
use itertools::Itertools;
pub use pallet::*;
use polkadot_ckb_merkle_mountain_range::{
	helper::{leaf_index_to_mmr_size, leaf_index_to_pos},
	util::{MemMMR, MemStore},
	MerkleProof, MMR,
};
use rand::{prelude::*, rngs::SmallRng, SeedableRng};
use scale_info::prelude::{format, string::String};
use serde::{Deserialize, Serialize};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::{crypto::UncheckedFrom, H256};
pub use sp_io::{
	crypto::sr25519_public_keys,
<<<<<<< HEAD
<<<<<<< HEAD
	offchain::{
		local_storage_clear, local_storage_compare_and_set, local_storage_get, local_storage_set,
	},
=======
	offchain::{local_storage_get, local_storage_set},
>>>>>>> a58b0e83 (Import offchain storage fns)
=======
	offchain::{
		local_storage_clear, local_storage_compare_and_set, local_storage_get, local_storage_set,
	},
>>>>>>> 6b1cea0d (Mutex for `pallet-ddc-verification` OCW)
};
use sp_runtime::{
	offchain::{http, Duration, StorageKind},
	traits::{Hash, IdentifyAccount},
	Percent,
};
use sp_staking::StakingInterface;
use sp_std::{
	collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	fmt::Debug,
	prelude::*,
};
pub mod weights;
use sp_io::hashing::blake2_256;

use crate::weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
=======
=======
use hex::FromHex;
>>>>>>> ddabd84d (Addressed PR comments)
=======
>>>>>>> 874386e0 (Addressed PR comments)
=======
use itertools::Itertools;
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
pub use pallet::*;
use polkadot_ckb_merkle_mountain_range::{
	helper::{leaf_index_to_mmr_size, leaf_index_to_pos},
	util::{MemMMR, MemStore},
	MerkleProof, MMR,
};
use rand::{prelude::*, rngs::SmallRng, SeedableRng};
use scale_info::prelude::{format, string::String};
use serde::{Deserialize, Serialize};
use sp_application_crypto::RuntimeAppPublic;
<<<<<<< HEAD
use sp_runtime::{
	offchain::{http, Duration, StorageKind},
	traits::Hash,
	Percent,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*, str::from_utf8};
pub mod weights;
use frame_support::traits::Currency;
use itertools::Itertools;
use rand::{prelude::*, rngs::SmallRng, SeedableRng};
use sp_core::crypto::UncheckedFrom;
pub use sp_io::{
	crypto::sr25519_public_keys,
	offchain::{local_storage_clear, local_storage_get, local_storage_set},
};
use sp_runtime::traits::IdentifyAccount;
=======
use sp_core::crypto::UncheckedFrom;
pub use sp_io::crypto::sr25519_public_keys;
use sp_runtime::{
	offchain::{http, Duration, StorageKind},
	traits::{Hash, IdentifyAccount},
	Percent,
};
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
use sp_staking::StakingInterface;
use sp_std::{collections::btree_map::BTreeMap, fmt::Debug, prelude::*};

pub mod weights;
use crate::weights::WeightInfo;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

<<<<<<< HEAD
<<<<<<< HEAD
pub mod migrations;

mod aggregator_client;

pub mod proto {
<<<<<<< HEAD
<<<<<<< HEAD
	include!(concat!(env!("OUT_DIR"), "/activity.rs"));
}

mod signature;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
#[cfg(feature = "runtime-benchmarks")]
>>>>>>> 67d2e038 (feat: benchmarking for ddc-verification pallet calls part 1)
=======
>>>>>>> 9e865129 (feat: benchmarking for ddc-verification pallet calls part 2)
pub(crate) type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {

	use ddc_primitives::{
		AggregatorInfo, BucketId, DeltaUsageHash, Fingerprint, MergeMMRHash,
		DAC_VERIFICATION_KEY_TYPE,
	};
	use frame_support::PalletId;
	use sp_core::crypto::AccountId32;
=======
=======
pub mod migrations;

>>>>>>> 732d4a4c (chore: migration for deprecated storage item)
=======
    include!(concat!(env!("OUT_DIR"),"/activity.rs"));
=======
	include!(concat!(env!("OUT_DIR"), "/activity.rs"));
>>>>>>> e157006b (cargo fmt)
}

>>>>>>> 004a4f79 (Add activity aggregator challenge client)
=======
>>>>>>> 05b8cb78 (Add activity signature verification module)
#[frame_support::pallet]
pub mod pallet {

	use ddc_primitives::{AggregatorInfo, BucketId, MergeActivityHash, DAC_VERIFICATION_KEY_TYPE};
	use frame_support::PalletId;
<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	use sp_core::crypto::AccountId32;
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
	use sp_runtime::SaturatedConversion;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
<<<<<<< HEAD
<<<<<<< HEAD
		frame_support::traits::StorageVersion::new(1);

	const _SUCCESS_CODE: u16 = 200;
<<<<<<< HEAD
	const _BUF_SIZE: usize = 128;
	const RESPONSE_TIMEOUT: u64 = 20000;
	pub const BUCKETS_AGGREGATES_FETCH_BATCH_SIZE: usize = 100;
	pub const NODES_AGGREGATES_FETCH_BATCH_SIZE: usize = 10;
	pub const IS_RUNNING_KEY: &[u8] = b"offchain::validator::is_running";
	pub const IS_RUNNING_VALUE: &[u8] = &[1];
<<<<<<< HEAD
=======
		frame_support::traits::StorageVersion::new(0);
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		frame_support::traits::StorageVersion::new(1);
>>>>>>> 732d4a4c (chore: migration for deprecated storage item)

	const SUCCESS_CODE: u16 = 200;
=======
>>>>>>> ed10b1aa (cargo clippy --fix)
	const _BUF_SIZE: usize = 128;
	const RESPONSE_TIMEOUT: u64 = 20000;
	pub const BUCKETS_AGGREGATES_FETCH_BATCH_SIZE: usize = 100;
	pub const NODES_AGGREGATES_FETCH_BATCH_SIZE: usize = 10;
<<<<<<< HEAD
	pub const VALIDATION_ACTIVITIES_KEY: &[u8] = b"offchain::activities::keys";
	pub const VALIDATION_ACTIVITIES_KEYS_SEP: u8 = b',';
=======
>>>>>>> 6b1cea0d (Mutex for `pallet-ddc-verification` OCW)
=======
	pub const IS_RUNNING_KEY: &[u8] = b"offchain::validator::is_running";
	pub const IS_RUNNING_VALUE: &[u8] = &[1];
>>>>>>> 7c239abb (Clear OCW cache for a specific cluster and era)

	/// Delta usage of a bucket includes only the delta usage for the processing era reported by
	/// collectors. This usage can be verified of unverified by inspectors.
	pub(crate) type BucketDeltaUsage = aggregator_client::json::BucketSubAggregate;

	/// Delta usage of a node includes only the delta usage for the processing era reported by
	/// collectors. This usage can be verified of unverified by inspectors.
	pub(crate) type NodeDeltaUsage = aggregator_client::json::NodeAggregate;

	/// Payable usage of a bucket includes the current storage usage this bucket consumes and the
	/// delta usage verified by inspectors. This is overall amount of bytes that the bucket owner
	/// will be charged for.
	#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Encode, Decode)]
	pub(crate) struct BucketPayableUsage(BucketId, BucketUsage);

	/// Payable usage of a node includes the current storage usage this node provides and the delta
	/// usage verified by inspectors. This is overall amount of bytes that the node owner will be
	/// rewarded for.
	#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Encode, Decode)]
	pub(crate) struct NodePayableUsage(NodePubKey, NodeUsage);

	/// Payable usage of an Era includes all the batches of customers and providers that will be
	/// processed during the payout process along with merkle root hashes and proofs. To calculate
	/// the same billing fingerprint and let the payouts to start the required quorum of validators
	/// need to agree on the same values for the Era usage and commit the same billing fingerprint.
	#[derive(Clone, Encode, Decode)]
	pub(crate) struct PayableEraUsage {
		cluster_id: ClusterId,
		era: EraActivity,
		payers_usage: Vec<BucketPayableUsage>,
		payers_root: PayableUsageHash,
		payers_batch_roots: Vec<PayableUsageHash>,
		payees_usage: Vec<NodePayableUsage>,
		payees_root: PayableUsageHash,
		payees_batch_roots: Vec<PayableUsageHash>,
		cluster_usage: NodeUsage,
	}

	impl PayableEraUsage {
		fn fingerprint(&self) -> Fingerprint {
			let mut data = self.cluster_id.encode();
			data.extend_from_slice(&self.era.id.encode());
			data.extend_from_slice(&self.era.start.encode());
			data.extend_from_slice(&self.era.end.encode());
			data.extend_from_slice(&self.payers_root.encode());
			data.extend_from_slice(&self.payees_root.encode());
			data.extend_from_slice(&self.cluster_usage.encode());
			blake2_256(&data).into()
		}
	}

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
<<<<<<< HEAD
<<<<<<< HEAD
		type ClusterValidator: ClusterValidator<Self>;
		type ClusterManager: ClusterManager<Self>;
		type PayoutProcessor: PayoutProcessor<Self>;
<<<<<<< HEAD
		/// DDC nodes read-only registry.
		type NodeManager: NodeManager<Self>;
<<<<<<< HEAD
=======
=======
		type ClusterValidator: ClusterValidator<Self>;
>>>>>>> 99095ecd (verified copy of PR#393 (#402))
		type ClusterManager: ClusterManager<Self>;
		type PayoutVisitor: PayoutVisitor<Self>;
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
		/// DDC nodes read-only registry.
<<<<<<< HEAD
		type NodeVisitor: NodeVisitor<Self>;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		type NodeManager: NodeManager<Self>;
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
		/// The output of the `ActivityHasher` function.
		type ActivityHash: Member
			+ Parameter
			+ MaybeSerializeDeserialize
			+ Ord
			+ Into<ActivityHash>
			+ From<ActivityHash>;
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		/// The hashing system (algorithm)
		type Hasher: Hash<Output = DeltaUsageHash>;
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
		/// Block to start from.
		const BLOCK_TO_START: u16;
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
		const DAC_REDUNDANCY_FACTOR: u16;

		#[pallet::constant]
		type AggregatorsQuorum: Get<Percent>;
		#[pallet::constant]
		type ValidatorsQuorum: Get<Percent>;

<<<<<<< HEAD
		const MAX_PAYOUT_BATCH_COUNT: u16;
		const MAX_PAYOUT_BATCH_SIZE: u16;
		const MAX_MERKLE_NODE_IDENTIFIER: u16;
		/// The access to staking functionality.
		type ValidatorStaking: StakingInterface<
			AccountId = Self::AccountId,
			Balance = BalanceOf<Self>,
		>;
		type AccountIdConverter: From<Self::AccountId> + Into<AccountId32>;
		type CustomerVisitor: CustomerVisitor<Self>;
		type BucketsStorageUsageProvider: StorageUsageProvider<
			BucketId,
			BucketStorageUsage<Self::AccountId>,
		>;
		type NodesStorageUsageProvider: StorageUsageProvider<
			StorageNodePubKey,
			NodeStorageUsage<Self::AccountId>,
		>;
		type Currency: Currency<Self::AccountId>;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
=======
		const VERIFY_AGGREGATOR_RESPONSE_SIGNATURE: bool;
>>>>>>> be1d48ff (Pallet parameter to enable aggregator resp sig)
		#[cfg(feature = "runtime-benchmarks")]
		type CustomerDepositor: CustomerDepositor<Self>;
		#[cfg(feature = "runtime-benchmarks")]
		type ClusterCreator: ClusterCreator<Self, BalanceOf<Self>>;
		#[cfg(feature = "runtime-benchmarks")]
		type BucketManager: BucketManager<Self>;
<<<<<<< HEAD
=======
		const MIN_DAC_NODES_FOR_CONSENSUS: u16;
=======
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
		const MAX_PAYOUT_BATCH_COUNT: u16;
		const MAX_PAYOUT_BATCH_SIZE: u16;
		const MAX_MERKLE_NODE_IDENTIFIER: u16;
		/// The access to staking functionality.
<<<<<<< HEAD
		type StakingVisitor: StakingInterface<AccountId = Self::AccountId>;
<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
=======
		type ValidatorStaking: StakingInterface<AccountId = Self::AccountId>;
>>>>>>> 67d2e038 (feat: benchmarking for ddc-verification pallet calls part 1)
		type AccountIdConverter: From<Self::AccountId> + Into<AccountId32>;
<<<<<<< HEAD
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		type CustomerVisitor: CustomerVisitor<Self>;
<<<<<<< HEAD
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
		#[cfg(feature = "runtime-benchmarks")]
		type Currency: LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>;
>>>>>>> 67d2e038 (feat: benchmarking for ddc-verification pallet calls part 1)
=======
>>>>>>> 9e865129 (feat: benchmarking for ddc-verification pallet calls part 2)
=======
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
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
<<<<<<< HEAD
<<<<<<< HEAD
=======
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
<<<<<<< HEAD
		/// No activity in consensus.
		ActivityNotInConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			id: ActivityHash,
			validator: T::AccountId,
		},
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 8149e899 (Changes for challenging node sub aggregates)
=======
>>>>>>> f34e347c (feat: grouping aggregates by consistent categories)
		/// Node Usage Retrieval Error.
		NodeUsageRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			node_pub_key: NodePubKey,
			validator: T::AccountId,
		},
<<<<<<< HEAD
<<<<<<< HEAD
		/// Bucket aggregates Retrieval Error.
		BucketAggregatesRetrievalError {
=======
		/// Customer Usage Retrieval Error.
		CustomerUsageRetrievalError {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		/// Bucket aggregates Retrieval Error.
		BucketAggregatesRetrievalError {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
			cluster_id: ClusterId,
			era_id: DdcEra,
			node_pub_key: NodePubKey,
			validator: T::AccountId,
		},
		EraRetrievalError {
			cluster_id: ClusterId,
<<<<<<< HEAD
<<<<<<< HEAD
			node_pub_key: Option<NodePubKey>,
=======
			node_pub_key: NodePubKey,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			node_pub_key: Option<NodePubKey>,
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
			validator: T::AccountId,
		},
		PrepareEraTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			payers_merkle_root_hash: DeltaUsageHash,
			payees_merkle_root_hash: DeltaUsageHash,
			validator: T::AccountId,
		},
		CommitBillingFingerprintTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			payers_root: PayableUsageHash,
			payees_root: PayableUsageHash,
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
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)
		FailedToCollectVerificationKey {
			validator: T::AccountId,
		},
		FailedToFetchVerificationKey {
<<<<<<< HEAD
=======
		FailedToFetchCurrentValidator {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		FailedToRetrieveVerificationKey {
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
=======
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)
			validator: T::AccountId,
		},
		FailedToFetchNodeProvider {
			validator: T::AccountId,
		},
<<<<<<< HEAD
<<<<<<< HEAD
		ValidatorKeySet {
			validator: T::AccountId,
		},
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
		FailedToFetchClusterNodes {
			validator: T::AccountId,
		},
		FailedToFetchDacNodes {
			validator: T::AccountId,
		},
<<<<<<< HEAD
=======
>>>>>>> e0ce0e5b (node integer delta usage (#412))
=======
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
		FailedToFetchNodeTotalUsage {
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
			validator: T::AccountId,
		},
		EraValidationRootsPosted {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
			payers_merkle_root_hash: DeltaUsageHash,
			payees_merkle_root_hash: DeltaUsageHash,
			payers_batch_merkle_root_hashes: Vec<DeltaUsageHash>,
			payees_batch_merkle_root_hashes: Vec<DeltaUsageHash>,
		},
<<<<<<< HEAD
<<<<<<< HEAD
		BucketAggregateRetrievalError {
=======
		BucketAggregatesRetrievalError {
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
		BucketAggregateRetrievalError {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
			cluster_id: ClusterId,
			era_id: DdcEra,
			bucket_id: BucketId,
			node_pub_key: NodePubKey,
			validator: T::AccountId,
		},
<<<<<<< HEAD
<<<<<<< HEAD
		ChallengeResponseRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
			validator: T::AccountId,
		},
		TraverseResponseRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
			validator: T::AccountId,
		},
		EmptyConsistentGroup,
<<<<<<< HEAD
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		ValidatorKeySet {
			validator: T::AccountId,
		},
>>>>>>> 932271b3 (Add ocw information (#404))
=======
		TotalNodeUsageLessThanZero {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
		},
<<<<<<< HEAD
>>>>>>> 00eed38c (Changes to accept stored_bytes as signed input (#410))
=======
		EraValidationRootsPosted {
			cluster_id: ClusterId,
			era_id: DdcEra,
			validator: T::AccountId,
			payers_merkle_root_hash: ActivityHash,
			payees_merkle_root_hash: ActivityHash,
			payers_batch_merkle_root_hashes: Vec<ActivityHash>,
			payees_batch_merkle_root_hashes: Vec<ActivityHash>,
		},
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		NotEnoughNodeAggregatesForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			bucket_id: BucketId,
			node_id: String,
			validator: T::AccountId,
		},
<<<<<<< HEAD
		BucketAggregateActivityNotInConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			id: ActivityHash,
			node_ids: Vec<String>,
			validator: T::AccountId,
		},
<<<<<<< HEAD
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
=======
>>>>>>> 8149e899 (Changes for challenging node sub aggregates)
=======
>>>>>>> f34e347c (feat: grouping aggregates by consistent categories)
		ChallengeResponseRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
			validator: T::AccountId,
		},
		TraverseResponseRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
			validator: T::AccountId,
		},
<<<<<<< HEAD
>>>>>>> 18369b0b (Challenge sub trees and make them ready for payout (#434))
=======
		EmptyConsistentGroup,
>>>>>>> f34e347c (feat: grouping aggregates by consistent categories)
=======
		FailedToFetchVerifiedDeltaUsage,
		FailedToFetchVerifiedPayableUsage,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
	}

	/// Consensus Errors
	#[derive(Debug, Encode, Decode, Clone, TypeInfo, PartialEq)]
	pub enum OCWError {
<<<<<<< HEAD
<<<<<<< HEAD
=======
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
<<<<<<< HEAD
		/// No activity in consensus.
		ActivityNotInConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			id: ActivityHash,
		},
<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		/// No Bucket Aggregate activity in consensus.
		BucketAggregateActivityNotInConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			id: ActivityHash,
			node_ids: Vec<String>,
		},
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
>>>>>>> 8149e899 (Changes for challenging node sub aggregates)
=======
>>>>>>> f34e347c (feat: grouping aggregates by consistent categories)
		/// Node Usage Retrieval Error.
		NodeUsageRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			node_pub_key: NodePubKey,
		},
<<<<<<< HEAD
<<<<<<< HEAD
		/// Bucket aggregates Retrieval Error.
		BucketAggregatesRetrievalError {
=======
		/// Customer Usage Retrieval Error.
		CustomerUsageRetrievalError {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		/// Bucket aggregates Retrieval Error.
		BucketAggregatesRetrievalError {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
			cluster_id: ClusterId,
			era_id: DdcEra,
			node_pub_key: NodePubKey,
		},
		EraRetrievalError {
			cluster_id: ClusterId,
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
			node_pub_key: Option<NodePubKey>,
		},
		/// Bucket aggregate Retrieval Error.
		BucketAggregateRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			bucket_id: BucketId,
			node_pub_key: NodePubKey,
		},
<<<<<<< HEAD
<<<<<<< HEAD
		/// Challenge Response Retrieval Error.
		ChallengeResponseRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
		},
		/// Traverse Response Retrieval Error.
		TraverseResponseRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
		},
=======
			node_pub_key: NodePubKey,
		},
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		/// Bucket aggregates Retrieval Error.
		BucketAggregatesRetrievalError {
=======
		/// Bucket aggregate Retrieval Error.
		BucketAggregateRetrievalError {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
			cluster_id: ClusterId,
			era_id: DdcEra,
			bucket_id: BucketId,
			node_pub_key: NodePubKey,
		},
<<<<<<< HEAD
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
		/// Challenge Response Retrieval Error.
		ChallengeResponseRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
		},
		/// Traverse Response Retrieval Error.
		TraverseResponseRetrievalError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
		},
>>>>>>> 18369b0b (Challenge sub trees and make them ready for payout (#434))
		PrepareEraTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			payers_merkle_root_hash: DeltaUsageHash,
			payees_merkle_root_hash: DeltaUsageHash,
		},
		CommitBillingFingerprintTransactionError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			payers_root: PayableUsageHash,
			payees_root: PayableUsageHash,
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
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		FailedToCollectVerificationKey,
		FailedToFetchVerificationKey,
=======
		FailedToRetrieveVerificationKey,
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
=======
		FailedToCollectVerificationKey,
		FailedToFetchVerificationKey,
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)
		FailedToFetchNodeProvider,
		FailedToFetchClusterNodes,
		FailedToFetchDacNodes,
		FailedToFetchNodeTotalUsage {
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
		},
<<<<<<< HEAD
<<<<<<< HEAD
		EmptyConsistentGroup,
<<<<<<< HEAD
=======
		FailedToFetchCurrentValidator,
		FailedToFetchNodeProvider,
<<<<<<< HEAD
<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		TotalNodeUsageLessThanZero {
=======
		FailedToFetchNodeTotalUsage {
>>>>>>> e0ce0e5b (node integer delta usage (#412))
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
		},
>>>>>>> 00eed38c (Changes to accept stored_bytes as signed input (#410))
=======
		/// Not enough subaggregates for consensus.
		NotEnoughNodeAggregatesForConsensus {
			cluster_id: ClusterId,
			era_id: DdcEra,
			bucket_id: BucketId,
			node_id: String,
		},
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
		EmptyConsistentGroup,
>>>>>>> f34e347c (feat: grouping aggregates by consistent categories)
=======
		FailedToFetchVerifiedDeltaUsage,
		FailedToFetchVerifiedPayableUsage,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		/// Bad verification key.
		BadVerificationKey,
		/// Bad requests.
		BadRequest,
		/// Not a validator.
<<<<<<< HEAD
<<<<<<< HEAD
		Unauthorized,
=======
		Unauthorised,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		Unauthorized,
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
		/// Already signed era.
		AlreadySignedEra,
		NotExpectedState,
		/// Already signed payout batch.
		AlreadySignedPayoutBatch,
		/// Node Retrieval Error.
		NodeRetrievalError,
<<<<<<< HEAD
<<<<<<< HEAD
=======
		/// Cluster To Validate Retrieval Error.
		ClusterToValidateRetrievalError,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
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
		FailedToGenerateProof,
		/// Fail to verify merkle proof
		FailedToVerifyMerkleProof,
		/// No Era Validation exist
		NoEraValidation,
<<<<<<< HEAD
<<<<<<< HEAD
		/// Given era is already validated and paid.
		EraAlreadyPaid,
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		/// Given era is already validated and paid.
		EraAlreadyPaid,
>>>>>>> 64c47a4b (New `skip_dac_validation_to_era` extrinsic)
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

<<<<<<< HEAD
<<<<<<< HEAD
=======
	/// Cluster id storage
	#[pallet::storage]
	#[pallet::getter(fn cluster_to_validate)]
	pub type ClusterToValidate<T: Config> = StorageValue<_, ClusterId>;

>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 732d4a4c (chore: migration for deprecated storage item)
	/// List of validators.
	#[pallet::storage]
	#[pallet::getter(fn validator_set)]
	pub type ValidatorSet<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	/// Validator stash key mapping
	#[pallet::storage]
	#[pallet::getter(fn get_stash_for_ddc_validator)]
	pub type ValidatorToStashKey<T: Config> = StorageMap<_, Identity, T::AccountId, T::AccountId>;
<<<<<<< HEAD
<<<<<<< HEAD
=======
=======

<<<<<<< HEAD
>>>>>>> 18369b0b (Challenge sub trees and make them ready for payout (#434))
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
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))

=======
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
	/// Era activity of a node.
	#[derive(
		Debug,
		Serialize,
		Deserialize,
		Clone,
		Copy,
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

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
	impl From<aggregator_client::json::AggregationEraResponse> for EraActivity {
		fn from(era: aggregator_client::json::AggregationEraResponse) -> Self {
=======
	impl From<AggregationEraResponse> for EraActivity {
		fn from(era: AggregationEraResponse) -> Self {
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
=======
	impl From<aggregator_client::json::AggregationEraResponse> for EraActivity {
		fn from(era: aggregator_client::json::AggregationEraResponse) -> Self {
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
			Self { id: era.id, start: era.start, end: era.end }
		}
	}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
	#[derive(Clone)]
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
	pub struct CustomerBatch {
		pub(crate) batch_index: BatchIndex,
		pub(crate) payers: Vec<(BucketId, BucketUsage)>,
		pub(crate) batch_proof: MMRProof,
	}

	#[derive(Clone)]
	pub struct ProviderBatch {
		pub(crate) batch_index: BatchIndex,
		pub(crate) payees: Vec<(NodePubKey, NodeUsage)>,
		pub(crate) batch_proof: MMRProof,
	}

	/// The `ConsolidatedAggregate` struct represents a merging result of multiple aggregates
	/// that have reached consensus on the usage criteria. This result should be taken into
	/// consideration when choosing the intensity of the challenge.
	#[derive(Debug, Clone, PartialEq)]
	pub(crate) struct ConsolidatedAggregate<A: Aggregate> {
		/// The representative aggregate after consolidation
		pub(crate) aggregate: A,
		/// Number of aggregates that were consistent
		pub(crate) count: u16,
		/// Aggregators that provided consistent aggregates
		pub(crate) aggregators: Vec<AggregatorInfo>,
	}

	impl<A: Aggregate> ConsolidatedAggregate<A> {
		pub(crate) fn new(aggregate: A, count: u16, aggregators: Vec<AggregatorInfo>) -> Self {
			ConsolidatedAggregate { aggregate, count, aggregators }
		}
	}

	#[derive(Debug, Clone, PartialEq)]
	pub(crate) struct ConsistencyGroups<A: Aggregate> {
		pub(crate) consensus: Vec<ConsolidatedAggregate<A>>,
		pub(crate) quorum: Vec<ConsolidatedAggregate<A>>,
		pub(crate) others: Vec<ConsolidatedAggregate<A>>,
	}

	#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq)]
	pub enum AggregateKey {
		NodeAggregateKey(String),
		BucketSubAggregateKey(BucketId, String),
	}

	pub(crate) trait Hashable {
		/// Hash of the entity
		fn hash<T: Config>(&self) -> H256;
	}

	/// The 'Aggregate' trait defines a set of members common to activity aggregates, which reflect
	/// the usage of a node or bucket within an Era..
	pub(crate) trait Aggregate:
		Hashable + Clone + Ord + PartialEq + Eq + Serialize + for<'de> Deserialize<'de> + Debug
	{
		/// Aggregation key of this aggregate, i.e. bucket composite key or node key
		fn get_key(&self) -> AggregateKey;
		/// Number of activity records this aggregated by this aggregate
		fn get_number_of_leaves(&self) -> u64;
		/// Aggregator provided this aggregate
		fn get_aggregator(&self) -> AggregatorInfo;
	}

	impl Hashable for aggregator_client::json::BucketSubAggregate {
		fn hash<T: Config>(&self) -> DeltaUsageHash {
			let mut data = self.bucket_id.encode();
			data.extend_from_slice(&self.node_id.encode());
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			data.extend_from_slice(&self.number_of_puts.encode());
			data.extend_from_slice(&self.number_of_gets.encode());
			T::Hasher::hash(&data)
		}
	}

	impl Aggregate for aggregator_client::json::BucketSubAggregate {
		fn get_key(&self) -> AggregateKey {
			AggregateKey::BucketSubAggregateKey(self.bucket_id, self.node_id.clone())
		}

		fn get_number_of_leaves(&self) -> u64 {
			self.number_of_gets.saturating_add(self.number_of_puts)
		}

		fn get_aggregator(&self) -> AggregatorInfo {
			self.aggregator.clone()
		}
	}

	impl Hashable for aggregator_client::json::NodeAggregate {
		fn hash<T: Config>(&self) -> DeltaUsageHash {
			let mut data = self.node_id.encode();
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			data.extend_from_slice(&self.number_of_puts.encode());
			data.extend_from_slice(&self.number_of_gets.encode());
			T::Hasher::hash(&data)
		}
	}

	impl Aggregate for aggregator_client::json::NodeAggregate {
		fn get_key(&self) -> AggregateKey {
			AggregateKey::NodeAggregateKey(self.node_id.clone())
		}

		fn get_aggregator(&self) -> AggregatorInfo {
			self.aggregator.clone()
		}

		fn get_number_of_leaves(&self) -> u64 {
			self.number_of_gets.saturating_add(self.number_of_puts)
		}
	}
	pub trait NodeAggregateLeaf:
		Clone + Ord + PartialEq + Eq + Serialize + for<'de> Deserialize<'de>
	{
		fn leaf_hash<T: Config>(&self) -> DeltaUsageHash;
	}

	pub trait BucketSubAggregateLeaf:
		Clone + Ord + PartialEq + Eq + Serialize + for<'de> Deserialize<'de>
	{
		fn leaf_hash<T: Config>(&self) -> DeltaUsageHash;
	}

	impl NodeAggregateLeaf for aggregator_client::json::Leaf {
		fn leaf_hash<T: Config>(&self) -> DeltaUsageHash {
			let mut data = self.record.id.encode();
			data.extend_from_slice(&self.record.upstream.request.requestType.encode());
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			T::Hasher::hash(&data)
		}
	}

	impl BucketSubAggregateLeaf for aggregator_client::json::Leaf {
		fn leaf_hash<T: Config>(&self) -> DeltaUsageHash {
			let mut data = self.record.upstream.request.bucketId.encode();
			data.extend_from_slice(&self.record.encode());
			data.extend_from_slice(&self.record.upstream.request.requestType.encode());
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
<<<<<<< HEAD
			T::ActivityHasher::hash(&data).into()
=======
=======
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
	pub struct CustomerBatch<T: Config> {
=======
	pub struct CustomerBatch {
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		pub(crate) batch_index: BatchIndex,
		pub(crate) payers: Vec<(NodePubKey, BucketId, CustomerUsage)>,
		pub(crate) batch_proof: MMRProof,
	}

	pub struct ProviderBatch {
		pub(crate) batch_index: BatchIndex,
		pub(crate) payees: Vec<(NodePubKey, NodeUsage)>,
		pub(crate) batch_proof: MMRProof,
	}

	/// The `ConsolidatedAggregate` struct represents a merging result of multiple aggregates
	/// that have reached consensus on the usage criteria. This result should be taken into
	/// consideration when choosing the intensity of the challenge.
	#[derive(Debug, Clone, PartialEq)]
	pub(crate) struct ConsolidatedAggregate<A: Aggregate> {
		/// The representative aggregate after consolidation
		pub(crate) aggregate: A,
		/// Number of aggregates that were consistent
		pub(crate) count: u16,
		/// Aggregators that provided consistent aggregates
		pub(crate) aggregators: Vec<AggregatorInfo>,
	}

	impl<A: Aggregate> ConsolidatedAggregate<A> {
		pub(crate) fn new(aggregate: A, count: u16, aggregators: Vec<AggregatorInfo>) -> Self {
			ConsolidatedAggregate { aggregate, count, aggregators }
		}
	}

	#[derive(Debug, Clone, PartialEq)]
	pub(crate) struct ConsistencyGroups<A: Aggregate> {
		pub(crate) consensus: Vec<ConsolidatedAggregate<A>>,
		pub(crate) quorum: Vec<ConsolidatedAggregate<A>>,
		pub(crate) others: Vec<ConsolidatedAggregate<A>>,
	}

	#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq)]
	pub enum AggregateKey {
		NodeAggregateKey(String),
		BucketSubAggregateKey(BucketId, String),
	}

	/// The 'Aggregate' trait defines a set of members common to activity aggregates, which reflect
	/// the usage of a node or bucket within an Era..
	pub(crate) trait Aggregate:
		Clone + Ord + PartialEq + Eq + Serialize + for<'de> Deserialize<'de> + Debug
	{
		/// Hash of the aggregate that is defined by it 'usage' values
		fn hash<T: Config>(&self) -> ActivityHash;
		/// Aggregation key of this aggregate, i.e. bucket composite key or node key
		fn get_key(&self) -> AggregateKey;
		/// Number of activity records this aggregated by this aggregate
		fn get_number_of_leaves(&self) -> u64;
		/// Aggregator provided this aggregate
		fn get_aggregator(&self) -> AggregatorInfo;
	}

	impl Aggregate for aggregator_client::json::BucketSubAggregate {
		fn hash<T: Config>(&self) -> ActivityHash {
<<<<<<< HEAD
			T::ActivityHasher::hash(&self.encode()).into()
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			let mut data = self.bucket_id.encode();
			data.extend_from_slice(&self.node_id.encode());
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			data.extend_from_slice(&self.number_of_puts.encode());
			data.extend_from_slice(&self.number_of_gets.encode());
			T::ActivityHasher::hash(&data).into()
>>>>>>> c54cb6d0 (refactoring)
		}

		fn get_key(&self) -> AggregateKey {
			AggregateKey::BucketSubAggregateKey(self.bucket_id, self.node_id.clone())
		}

		fn get_number_of_leaves(&self) -> u64 {
			self.number_of_gets.saturating_add(self.number_of_puts)
		}

		fn get_aggregator(&self) -> AggregatorInfo {
			self.aggregator.clone()
		}
	}

	impl Aggregate for aggregator_client::json::NodeAggregate {
		fn hash<T: Config>(&self) -> ActivityHash {
			let mut data = self.node_id.encode();
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			data.extend_from_slice(&self.number_of_puts.encode());
			data.extend_from_slice(&self.number_of_gets.encode());
			T::ActivityHasher::hash(&data).into()
		}

		fn get_key(&self) -> AggregateKey {
			AggregateKey::NodeAggregateKey(self.node_id.clone())
		}

		fn get_aggregator(&self) -> AggregatorInfo {
			self.aggregator.clone()
		}

		fn get_number_of_leaves(&self) -> u64 {
			self.number_of_gets.saturating_add(self.number_of_puts)
		}
	}
	pub trait NodeAggregateLeaf:
		Clone + Ord + PartialEq + Eq + Serialize + for<'de> Deserialize<'de>
	{
		fn leaf_hash<T: Config>(&self) -> ActivityHash;
	}

	pub trait BucketSubAggregateLeaf:
		Clone + Ord + PartialEq + Eq + Serialize + for<'de> Deserialize<'de>
	{
		fn leaf_hash<T: Config>(&self) -> ActivityHash;
	}

	impl NodeAggregateLeaf for aggregator_client::json::Leaf {
		fn leaf_hash<T: Config>(&self) -> ActivityHash {
			let mut data = self.record.id.encode();
			data.extend_from_slice(&self.record.upstream.request.requestType.encode());
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			T::ActivityHasher::hash(&data).into()
		}
	}

	impl BucketSubAggregateLeaf for aggregator_client::json::Leaf {
		fn leaf_hash<T: Config>(&self) -> ActivityHash {
			let mut data = self.record.upstream.request.bucketId.encode();
			data.extend_from_slice(&self.record.encode());
			data.extend_from_slice(&self.record.upstream.request.requestType.encode());
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			T::ActivityHasher::hash(&data).into()
=======
			T::Hasher::hash(&data)
		}
	}

	impl Hashable for BucketPayableUsage {
		fn hash<T: Config>(&self) -> PayableUsageHash {
			let mut data = self.0.encode(); // bucket_id
			data.extend_from_slice(&self.1.stored_bytes.encode());
			data.extend_from_slice(&self.1.transferred_bytes.encode());
			data.extend_from_slice(&self.1.number_of_puts.encode());
			data.extend_from_slice(&self.1.number_of_gets.encode());
			T::Hasher::hash(&data)
		}
	}

	impl Hashable for NodePayableUsage {
		fn hash<T: Config>(&self) -> PayableUsageHash {
			let mut data = self.0.encode(); // node_key
			data.extend_from_slice(&self.1.stored_bytes.encode());
			data.extend_from_slice(&self.1.transferred_bytes.encode());
			data.extend_from_slice(&self.1.number_of_puts.encode());
			data.extend_from_slice(&self.1.number_of_gets.encode());
			T::Hasher::hash(&data)
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
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
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
			if block_number.saturated_into::<u32>() % T::BLOCK_TO_START as u32 != 0 {
				return;
			}

<<<<<<< HEAD
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
			if !sp_io::offchain::is_validator() {
				return;
			}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 6b1cea0d (Mutex for `pallet-ddc-verification` OCW)
			// Allow only one instance of the offchain worker to run at a time.
			if !local_storage_compare_and_set(
				StorageKind::PERSISTENT,
				IS_RUNNING_KEY,
				None,
				IS_RUNNING_VALUE,
			) {
				return;
			}

<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)
=======
>>>>>>> 6b1cea0d (Mutex for `pallet-ddc-verification` OCW)
			let verification_key = unwrap_or_log_error!(
=======
			let verification_account = unwrap_or_log_error!(
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
				Self::collect_verification_pub_key(),
				"❌ Error collecting validator verification key"
			);

			let signer = Signer::<T, T::OffchainIdentifierId>::any_account()
				.with_filter(vec![verification_account.public.clone()]);

<<<<<<< HEAD
			if !signer.can_sign() {
				log::error!("🚨 OCW signer is not available");
				return;
			}

			Self::store_verification_account_id(verification_account.public.clone().into_account());

			let clusters_ids = unwrap_or_log_error!(
				T::ClusterManager::get_clusters(ClusterStatus::Activated),
				"❌ Error retrieving clusters to validate"
			);
			log::info!("🎡 {:?} of 'Activated' clusters found", clusters_ids.len());

			for cluster_id in clusters_ids {
				let mut errors: Vec<OCWError> = Vec::new();

				let validation_result =
					Self::start_validation_phase(&cluster_id, &verification_account, &signer);

<<<<<<< HEAD
				match dac_era_result {
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
=======
			let signer = Signer::<T, T::OffchainIdentifierId>::any_account();
=======
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)
			if !signer.can_sign() {
				log::error!("🚨 OCW signer is not available");
				return;
			}

			Self::store_verification_account_id(verification_key.clone().into_account());

			let clusters_ids = unwrap_or_log_error!(
				T::ClusterManager::get_clusters(ClusterStatus::Activated),
				"❌ Error retrieving clusters to validate"
			);
			log::info!("🎡 {:?} of 'Activated' clusters found", clusters_ids.len());

			for cluster_id in clusters_ids {
				let batch_size = T::MAX_PAYOUT_BATCH_SIZE;
				let mut errors: Vec<OCWError> = Vec::new();

				let dac_era_result = Self::process_dac_era(&cluster_id, None, batch_size.into());

				match dac_era_result {
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

<<<<<<< HEAD
					for (_, res) in &results {
						match res {
							Ok(()) => {
								log::info!(
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						for (_, res) in &results {
							match res {
								Ok(()) => {
									log::info!(
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
										"🏭⛳️ Merkle roots posted on-chain for cluster_id: {:?}, era: {:?}",
										cluster_id,
										era_activity.clone()
									);
<<<<<<< HEAD
<<<<<<< HEAD
								},
								Err(e) => {
									log::error!(
=======
							},
							Err(e) => {
								log::error!(
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
								},
								Err(e) => {
									log::error!(
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
										"🏭❌ Error to post merkle roots on-chain for cluster_id: {:?}, era: {:?}: {:?}",
										cluster_id,
										era_activity.clone(),
										e
									);
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
									// Extrinsic call failed
									errors.push(OCWError::PrepareEraTransactionError {
										cluster_id,
										era_id: era_activity.id,
										payers_merkle_root_hash,
										payees_merkle_root_hash,
									});
								},
							}
<<<<<<< HEAD
						}
					},
					Ok(None) => {
						log::info!("🏭ℹ️  No eras for DAC process for cluster_id: {:?}", cluster_id);
					},
					Err(process_errors) => {
						errors.extend(process_errors);
					},
				};

				// todo! factor out as macro as this is repetitive
				match Self::prepare_begin_billing_report(&cluster_id) {
					Ok(Some((era_id, start_era, end_era))) => {
						log::info!(
=======
								// Extrinsic call failed
								errors.push(OCWError::PrepareEraTransactionError {
									cluster_id,
									era_id: era_activity.id,
									payers_merkle_root_hash,
									payees_merkle_root_hash,
								});
							},
=======
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
						}
					},
					Ok(None) => {
						log::info!("🏭ℹ️ No eras for DAC process for cluster_id: {:?}", cluster_id);
					},
					Err(process_errors) => {
						errors.extend(process_errors);
					},
				};

<<<<<<< HEAD
			// todo! factor out as macro as this is repetitive
			match Self::prepare_begin_billing_report(&cluster_id) {
				Ok(Some((era_id, start_era, end_era))) => {
					log::info!(
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				// todo! factor out as macro as this is repetitive
				match Self::prepare_begin_billing_report(&cluster_id) {
					Ok(Some((era_id, start_era, end_era))) => {
						log::info!(
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
						"🏭🚀 process_start_payout processed successfully for cluster_id: {:?}, era_id: {:?},  start_era: {:?},  end_era: {:?} ",
						cluster_id,
						era_id,
						start_era,
						end_era
					);
<<<<<<< HEAD
<<<<<<< HEAD
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
=======
				if let Err(errs) = validation_result {
					errors.extend(errs);
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
				}

				let payouts_result =
					Self::start_payouts_phase(&cluster_id, &verification_account, &signer);

				if let Err(errs) = payouts_result {
					errors.extend(errs);
				}

<<<<<<< HEAD
				// todo! factor out as macro as this is repetitive
				match Self::prepare_send_charging_customers_batch(&cluster_id, batch_size.into()) {
					Ok(Some((era_id, batch_payout))) => {
<<<<<<< HEAD
<<<<<<< HEAD
						log::info!(
							"🎁 prepare_send_charging_customers_batch processed successfully for cluster_id: {:?}, era_id: {:?}",
							cluster_id,
							era_id
						);

						if let Some((_, res)) = signer.send_signed_transaction(|_acc| {
							Call::send_charging_customers_batch {
								cluster_id,
								era_id,
								batch_index: batch_payout.batch_index,
								payers: batch_payout.payers.clone(),
								batch_proof: batch_payout.batch_proof.clone(),
							}
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
									errors.push(
										OCWError::SendChargingCustomersBatchTransactionError {
											cluster_id,
											era_id,
											batch_index: batch_payout.batch_index,
										},
									);
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
				match Self::prepare_begin_rewarding_providers(&cluster_id, batch_size.into()) {
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
									errors.push(
										OCWError::BeginRewardingProvidersTransactionError {
											cluster_id,
											era_id,
										},
									);
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
				match Self::prepare_send_rewarding_providers_batch(&cluster_id, batch_size.into()) {
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
=======
					let results = signer.send_signed_transaction(|_account| {
						Call::begin_billing_report { cluster_id, era_id, start_era, end_era }
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
					});

					for (_, res) in &results {
						match res {
							Ok(()) => {
<<<<<<< HEAD
								log::info!("✅ Successfully submitted emit_consensus_errors tx")
							},
							Err(_) => log::error!("🏭❌ Failed to submit emit_consensus_errors tx"),
						}
					}
				}
			}

			// Allow the next invocation of the offchain worker hook to run.
			local_storage_clear(StorageKind::PERSISTENT, IS_RUNNING_KEY);
=======
								log::info!(
=======
						let results = signer.send_signed_transaction(|_account| {
							Call::begin_billing_report { cluster_id, era_id, start_era, end_era }
						});

						for (_, res) in &results {
							match res {
								Ok(()) => {
									log::info!(
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
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
				match Self::prepare_begin_charging_customers(&cluster_id, batch_size.into()) {
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
				match Self::prepare_send_charging_customers_batch(&cluster_id, batch_size.into()) {
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
=======
						let payers_log: Vec<(String, String, BucketId, CustomerUsage)> =
							batch_payout
								.payers
								.clone()
								.into_iter()
								.map(|(acc_id, node_id, bucket_id, customer_usage)| {
									let account_id: T::AccountIdConverter = acc_id.into();
									let account_id_32: AccountId32 = account_id.into();
									let account_ref: &[u8; 32] = account_id_32.as_ref();
									(hex::encode(account_ref), node_id, bucket_id, customer_usage)
								})
								.collect();
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
						log::info!(
							"🎁 prepare_send_charging_customers_batch processed successfully for cluster_id: {:?}, era_id: {:?}",
							cluster_id,
							era_id
						);

						if let Some((_, res)) = signer.send_signed_transaction(|_acc| {
							Call::send_charging_customers_batch {
								cluster_id,
								era_id,
								batch_index: batch_payout.batch_index,
								payers: batch_payout.payers.clone(),
								batch_proof: batch_payout.batch_proof.clone(),
							}
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
									errors.push(
										OCWError::SendChargingCustomersBatchTransactionError {
											cluster_id,
											era_id,
											batch_index: batch_payout.batch_index,
										},
									);
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
				match Self::prepare_begin_rewarding_providers(&cluster_id, batch_size.into()) {
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
									errors.push(
										OCWError::BeginRewardingProvidersTransactionError {
											cluster_id,
											era_id,
										},
									);
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
				match Self::prepare_send_rewarding_providers_batch(&cluster_id, batch_size.into()) {
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

									Self::clear_validation_activities(&cluster_id, era_id);
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
							Ok(()) => {
								log::info!("✅ Successfully submitted emit_consensus_errors tx")
							},
							Err(_) => log::error!("🏭❌ Failed to submit emit_consensus_errors tx"),
						}
					}
				}
=======
				Self::submit_errors(&errors, &verification_account, &signer);
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
			}
<<<<<<< HEAD
<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======

<<<<<<< HEAD
			Self::clear_validation_activities();
>>>>>>> 2c0c7a8a (Clear validation activities cache when OCW is done)
=======

=======
>>>>>>> 7c239abb (Clear OCW cache for a specific cluster and era)
			// Allow the next invocation of the offchain worker hook to run.
			local_storage_clear(StorageKind::PERSISTENT, IS_RUNNING_KEY);
>>>>>>> 6b1cea0d (Mutex for `pallet-ddc-verification` OCW)
		}
	}

	macro_rules! define_payout_step_function {
		(
			$func_name:ident,
			$prepare_fn:ident,
			$call_variant:expr,
			$era_variant:expr,
			$log_prefix:literal,
			$error_variant:expr
		) => {
			#[allow(clippy::redundant_closure_call)]
			pub(crate) fn $func_name(
				cluster_id: &ClusterId,
				account: &Account<T>,
				signer: &Signer<T, T::OffchainIdentifierId>,
			) -> Result<Option<DdcEra>, Vec<OCWError>> {
				match Self::$prepare_fn(&cluster_id) {
					Ok(Some(prepared_data)) => {

						let era_id = $era_variant(&prepared_data);

						log::info!(
							concat!($log_prefix, " Initializing '{}' call for cluster_id: {:?}, era_id: {:?}"),
							stringify!($func_name),
							cluster_id,
							era_id,
						);

						let call = $call_variant(cluster_id, prepared_data.clone());
						let result = signer.send_single_signed_transaction(account, call);

						match result {
							Some(Ok(_)) => {
								log::info!(
									concat!($log_prefix, " Successfully sent '{}' call for cluster_id: {:?}, era_id: {:?}"),
									stringify!($func_name),
									cluster_id,
									era_id,
								);
								Ok(Some(era_id))
							}
							_ => {
								log::error!(
									concat!($log_prefix, " Failed to send '{}' call for cluster_id: {:?}, era_id: {:?}"),
									stringify!($func_name),
									cluster_id,
									era_id,
								);
								Err(vec![$error_variant(cluster_id, prepared_data)])
							}
						}
					}
					Ok(None) => {
						log::info!(
							concat!($log_prefix, " Skipping '{}' call as there is no era for payout for cluster_id: {:?}"),
							stringify!($func_name),
							cluster_id,
						);
						Ok(None)
					}
					Err(errs) => Err(errs),
				}
			}
		};
	}

	impl<T: Config> Pallet<T> {
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		pub(crate) fn do_skip_era_validation(
=======
		pub(crate) fn do_set_era_validations(
>>>>>>> 1b745359 (Separate `set_era_validations` body)
=======
		pub(crate) fn do_skip_era_validation(
>>>>>>> dc1c3a91 (Better name for validation skipping fn)
			cluster_id: &ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			let era_validations = <EraValidations<T>>::get(cluster_id, era_id);

			if era_validations.is_none() {
				let mut era_validation = EraValidation {
					status: EraValidationStatus::PayoutSkipped,
					..Default::default()
				};

				let signed_validators = era_validation
					.validators
					.entry((DeltaUsageHash::default(), DeltaUsageHash::default()))
					.or_insert_with(Vec::new);

				let validators = <ValidatorSet<T>>::get();

				signed_validators.extend(validators);

				<EraValidations<T>>::insert(cluster_id, era_id, era_validation);
			}

			Ok(())
		}

		#[allow(clippy::type_complexity)]
		pub(crate) fn process_dac_era(
			cluster_id: &ClusterId,
			era_id_to_process: Option<EraActivity>,
		) -> Result<
			Option<(
				EraActivity,
				DeltaUsageHash,
				DeltaUsageHash,
				Vec<DeltaUsageHash>,
				Vec<DeltaUsageHash>,
			)>,
			Vec<OCWError>,
		> {
			let batch_size = T::MAX_PAYOUT_BATCH_SIZE;

			let dac_nodes = Self::get_dac_nodes(cluster_id).map_err(|_| {
				log::error!("❌ Error retrieving dac nodes to validate cluster {:?}", cluster_id);
				vec![OCWError::FailedToFetchDacNodes]
			})?;
=======
=======
		#[allow(clippy::type_complexity)]
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		pub(crate) fn process_dac_data(
			cluster_id: &ClusterId,
			era_id_to_process: Option<EraActivity>,
			batch_size: usize,
		) -> Result<
			Option<(EraActivity, ActivityHash, ActivityHash, Vec<ActivityHash>, Vec<ActivityHash>)>,
			Vec<OCWError>,
		> {
			log::info!("🚀 Processing dac data for cluster_id: {:?}", cluster_id);
<<<<<<< HEAD
<<<<<<< HEAD
			if dac_nodes.len().ilog2() < min_nodes.into() {
				return Err(vec![OCWError::NotEnoughDACNodes { num_nodes: min_nodes }]);
			}
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			// todo! Need to debug follwing condition. Why it is not working on Devnet

			// if dac_nodes.len().ilog2() < min_nodes.into() {
			// 	return Err(vec![OCWError::NotEnoughDACNodes { num_nodes: min_nodes }]);
			// }
>>>>>>> a11d78d6 (Commented out min node condition (#398))
=======
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)

			let dac_nodes = Self::get_dac_nodes(cluster_id).map_err(|_| {
				log::error!("🏭❌ Error retrieving dac nodes to validate cluster {:?}", cluster_id);
				vec![OCWError::FailedToFetchDacNodes]
			})?;

			let era_activity = if let Some(era_activity) = era_id_to_process {
				EraActivity {
					id: era_activity.id,
					start: era_activity.start,
					end: era_activity.end,
				}
			} else {
<<<<<<< HEAD
<<<<<<< HEAD
				match Self::get_era_for_validation(cluster_id, &dac_nodes) {
=======
				match Self::get_era_for_validation(cluster_id, dac_nodes) {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				match Self::get_era_for_validation(cluster_id, &dac_nodes) {
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
					Ok(Some(era_activity)) => era_activity,
					Ok(None) => return Ok(None),
					Err(err) => return Err(vec![err]),
				}
			};

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
=======
			log::info!(
				"👁️‍🗨️  Start processing DAC for cluster_id: {:?} era_id; {:?}",
				cluster_id,
				era_activity.id
			);

>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
			// todo: move to cluster protocol parameters
			let dac_redundancy_factor = T::DAC_REDUNDANCY_FACTOR;
			let aggregators_quorum = T::AggregatorsQuorum::get();

			let nodes_aggregates_by_aggregator =
				Self::fetch_nodes_aggregates_for_era(cluster_id, era_activity.id, &dac_nodes)
<<<<<<< HEAD
					.map_err(|err| vec![err])?;

			let buckets_aggregates_by_aggregator =
				Self::fetch_buckets_aggregates_for_era(cluster_id, era_activity.id, &dac_nodes)
					.map_err(|err| vec![err])?;

			let buckets_sub_aggregates_groups = Self::group_buckets_sub_aggregates_by_consistency(
				cluster_id,
				era_activity.id,
				buckets_aggregates_by_aggregator,
				dac_redundancy_factor,
				aggregators_quorum,
			);

			let total_buckets_usage = Self::get_total_usage(
				cluster_id,
				era_activity.id,
				buckets_sub_aggregates_groups,
				true,
			)?;

			let customer_activity_hashes: Vec<DeltaUsageHash> =
				total_buckets_usage.clone().into_iter().map(|c| c.hash::<T>()).collect();

			let customer_activity_hashes_string: Vec<String> =
				customer_activity_hashes.clone().into_iter().map(hex::encode).collect();

			log::info!(
				"👁️‍🗨️  Customer Activity hashes for cluster_id: {:?} era_id: {:?} is: {:?}",
				cluster_id,
				era_activity.id,
				customer_activity_hashes_string
			);
			let customers_activity_batch_roots = Self::convert_to_batch_merkle_roots(
				cluster_id,
				era_activity.id,
				Self::split_to_batches(&total_buckets_usage, batch_size.into()),
			)
			.map_err(|err| vec![err])?;

			let customer_batch_roots_string: Vec<String> =
				customers_activity_batch_roots.clone().into_iter().map(hex::encode).collect();

			for (pos, batch_root) in customer_batch_roots_string.iter().enumerate() {
				log::info!(
				"👁️‍🗨️‍  Customer Activity batches for cluster_id: {:?} era_id: {:?} is: batch {:?} with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
					pos + 1,
					batch_root,
					customer_activity_hashes_string
				);
			}

=======
			let nodes_usage =
=======
			let nodes_aggregates_by_aggregator =
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
				Self::fetch_nodes_usage_for_era(cluster_id, era_activity.id, dac_nodes)
=======
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
					.map_err(|err| vec![err])?;

			let buckets_aggregates_by_aggregator =
				Self::fetch_buckets_aggregates_for_era(cluster_id, era_activity.id, &dac_nodes)
					.map_err(|err| vec![err])?;

			let buckets_sub_aggregates_groups = Self::group_buckets_sub_aggregates_by_consistency(
				cluster_id,
				era_activity.id,
				buckets_aggregates_by_aggregator,
				dac_redundancy_factor,
				aggregators_quorum,
			);

			let total_buckets_usage = Self::get_total_usage(
				cluster_id,
				era_activity.id,
				buckets_sub_aggregates_groups,
				true,
			)?;

			let customer_activity_hashes: Vec<ActivityHash> =
				total_buckets_usage.clone().into_iter().map(|c| c.hash::<T>()).collect();

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
				Self::split_to_batches(&total_buckets_usage, batch_size),
			)
			.map_err(|err| vec![err])?;

<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
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

>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
			let customers_activity_root = Self::create_merkle_root(
				cluster_id,
				era_activity.id,
				&customers_activity_batch_roots,
			)
			.map_err(|err| vec![err])?;

<<<<<<< HEAD
<<<<<<< HEAD
			log::info!(
				"👁️‍🗨️‍  Customer Activity batches tree for cluster_id: {:?} era_id: {:?} is: batch with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
				hex::encode(customers_activity_root),
					customer_batch_roots_string,
			);

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 0641acde (chore: classification by consistency)
			let nodes_aggregates_groups = Self::group_nodes_aggregates_by_consistency(
				cluster_id,
				era_activity.id,
				nodes_aggregates_by_aggregator,
				dac_redundancy_factor,
				aggregators_quorum,
			);
<<<<<<< HEAD

			let total_nodes_usage =
<<<<<<< HEAD
<<<<<<< HEAD
				Self::get_total_usage(cluster_id, era_activity.id, nodes_aggregates_groups, true)?;
=======
			let (nodes_activities_in_consensus, _nodes_activities_not_in_consensus) =
				Self::classify_nodes_aggregates_by_consistency(
					cluster_id,
					era_activity.id,
					nodes_aggregates_by_aggregator,
					dac_redundancy_factor,
					aggregators_quorum,
				);
=======
>>>>>>> 0641acde (chore: classification by consistency)

<<<<<<< HEAD
			// let mut nodes_activities_passed_challenges: Vec<NodeAggregate> = vec![];
			// if !nodes_activities_passed_challenges.is_empty() {
			// 	nodes_activities_passed_challenges =
			// 		Self::challenge_and_find_valid_node_aggregates_not_in_consensus(
			// 			cluster_id,
			// 			era_activity.id,
			// 			nodes_activities_not_in_consensus,
			// 		)?;
			// }
			// total_nodes_usage.extend(nodes_activities_passed_challenges);
>>>>>>> ff2caa48 (chore: aggregate challenging is disabled for now)

=======
>>>>>>> 14ebbbc0 (chore: adding verified usage to payouts phase)
			let total_nodes_usage = Self::get_total_usage(nodes_aggregates_groups)?;
=======
				Self::get_total_usage(cluster_id, era_activity.id, nodes_aggregates_groups)?;
>>>>>>> 11d3ff37 (feat: challenging and traversing deviating aggregates (disabled for now))
=======
				Self::get_total_usage(cluster_id, era_activity.id, nodes_aggregates_groups, true)?;
>>>>>>> 7c18b215 (Optional challenge for total usage)

			let node_activity_hashes: Vec<DeltaUsageHash> =
				total_nodes_usage.clone().into_iter().map(|c| c.hash::<T>()).collect();

			let node_activity_hashes_string: Vec<String> =
				node_activity_hashes.clone().into_iter().map(hex::encode).collect();

			log::info!(
				"👁️‍🗨️  Node Activity hashes for cluster_id: {:?} era_id: {:?} is: {:?}",
				cluster_id,
				era_activity.id,
				node_activity_hashes_string
			);

			let nodes_activity_batch_roots = Self::convert_to_batch_merkle_roots(
				cluster_id,
				era_activity.id,
				Self::split_to_batches(&total_nodes_usage, batch_size.into()),
			)
			.map_err(|err| vec![err])?;

			let nodes_activity_batch_roots_string: Vec<String> =
				nodes_activity_batch_roots.clone().into_iter().map(hex::encode).collect();

			for (pos, batch_root) in nodes_activity_batch_roots_string.iter().enumerate() {
				log::info!(
				"👁️‍🗨️  Node Activity batches for cluster_id: {:?} era_id: {:?} are: batch {:?} with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
					pos + 1,
					batch_root,
					node_activity_hashes_string
				);
			}

=======
=======
			log::info!(
				"🧗‍  Customer Activity batches tree for ClusterId: {:?} EraId: {:?}  is: batch with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
				hex::encode(customers_activity_root),
					customer_batch_roots_string,
			);

>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
			let nodes_activity_in_consensus = Self::get_consensus_for_activities(
				cluster_id,
				era_activity.id,
				&nodes_usage,
				min_nodes,
				Percent::from_percent(T::MAJORITY),
			)?;
=======
			let (nodes_activity_in_consensus, nodes_activity_not_in_consensus) =
				Self::get_consensus_for_activities(
=======
			let (nodes_activities_in_consensus, nodes_activities_not_in_consensus) =
				Self::classify_nodes_aggregates_by_consistency(
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
					cluster_id,
					era_activity.id,
					nodes_aggregates_by_aggregator,
					redundancy_factor,
					T::AggregatorsQuorum::get(),
				);

			let mut nodes_activities_passed_challenges: Vec<NodeActivity> = vec![];

			if !nodes_activities_passed_challenges.is_empty() {
				nodes_activities_passed_challenges =
					Self::challenge_and_find_valid_node_aggregates_not_in_consensus(
						cluster_id,
						era_activity.id,
						nodes_activities_not_in_consensus,
					)?;
			}

<<<<<<< HEAD
			let mut total_node_aggregates = nodes_activity_in_consensus.clone();
			total_node_aggregates.extend(node_aggregates_passed_challenges);
>>>>>>> 8149e899 (Changes for challenging node sub aggregates)
=======
			let mut total_nodes_usage = nodes_activities_in_consensus.clone();
			total_nodes_usage.extend(nodes_activities_passed_challenges);
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)

			let node_activity_hashes: Vec<ActivityHash> =
				total_nodes_usage.clone().into_iter().map(|c| c.hash::<T>()).collect();

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
				Self::split_to_batches(&total_nodes_usage, batch_size),
			)
			.map_err(|err| vec![err])?;

<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			let nodes_activity_batch_roots_string: Vec<String> =
				nodes_activity_batch_roots.clone().into_iter().map(hex::encode).collect();

			for (pos, batch_root) in nodes_activity_batch_roots_string.iter().enumerate() {
				log::info!(
				"🧗‍  Node Activity batches for ClusterId: {:?} EraId: {:?}  is: batch {:?} with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
<<<<<<< HEAD
					nodes_activity_batch_roots_string,
					node_activity_hashes
			);
<<<<<<< HEAD
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
=======
					pos + 1,
					batch_root,
					node_activity_hashes_string
				);
			}
>>>>>>> f164ec01 (Added OCW activity logs (#415))

>>>>>>> 92e08a3b (OCW additional logs (#413))
			let nodes_activity_root =
				Self::create_merkle_root(cluster_id, era_activity.id, &nodes_activity_batch_roots)
					.map_err(|err| vec![err])?;

<<<<<<< HEAD
<<<<<<< HEAD
			log::info!(
				"👁️‍🗨️  Node Activity batches tree for cluster_id: {:?} era_id: {:?} are: batch with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
				hex::encode(nodes_activity_root),
					nodes_activity_batch_roots_string,
			);

			Self::store_verified_delta_usage(
				cluster_id,
				era_activity.id,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				&total_buckets_usage,
				customers_activity_root,
				&customers_activity_batch_roots,
				&total_nodes_usage,
=======
=======
			log::info!(
				"🧗‍  Node Activity batches tree for ClusterId: {:?} EraId: {:?}  is: batch 1 with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
				hex::encode(nodes_activity_root),
					nodes_activity_batch_roots_string,
			);

>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
			Self::store_validation_activities(
				cluster_id,
				era_activity.id,
				&customers_activity_in_consensus,
=======
				&bucket_node_aggregates_in_consensus,
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
				customers_activity_root,
				&customers_activity_batch_roots,
				&nodes_activity_in_consensus,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				&total_bucket_aggregates,
				customers_activity_root,
				&customers_activity_batch_roots,
				&total_node_aggregates,
>>>>>>> 8149e899 (Changes for challenging node sub aggregates)
=======
				&total_buckets_usage,
				customers_activity_root,
				&customers_activity_batch_roots,
				&total_nodes_usage,
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
				nodes_activity_root,
				&nodes_activity_batch_roots,
			);
<<<<<<< HEAD
			log::info!("🙇‍ Dac data processing completed for cluster_id: {:?}", cluster_id);
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			log::info!(
				"👁️‍🗨️‍  End processing DAC for cluster_id: {:?} era_id: {:?}",
				cluster_id,
				era_activity.id
			);
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
			Ok(Some((
				era_activity,
				customers_activity_root,
				nodes_activity_root,
				customers_activity_batch_roots,
				nodes_activity_batch_roots,
			)))
<<<<<<< HEAD
		}

		pub(crate) fn get_total_usage<A: Aggregate>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			consistency_groups: ConsistencyGroups<A>,
			should_challenge: bool,
		) -> Result<Vec<A>, Vec<OCWError>> {
			let mut total_usage = vec![];
			let mut total_usage_keys = vec![];

			// todo: implement 'challenge_consensus' fn and run a light challenge for unanimous
			// consensus
			let in_consensus_usage = consistency_groups
				.consensus
				.clone()
				.into_iter()
				.map(|ca| ca.aggregate.clone())
				.collect::<Vec<_>>();
			total_usage.extend(in_consensus_usage.clone());
			total_usage_keys
				.extend(in_consensus_usage.into_iter().map(|a| a.get_key()).collect::<Vec<_>>());

			// todo: implement 'challenge_quorum' fn and run a light challenge for the quorum, i.e.
			// for majority
			let in_quorum_usage = consistency_groups
				.quorum
				.clone()
				.into_iter()
				.map(|ca| ca.aggregate.clone())
				.collect::<Vec<_>>();
			total_usage.extend(in_quorum_usage.clone());
			total_usage_keys
				.extend(in_quorum_usage.into_iter().map(|a| a.get_key()).collect::<Vec<_>>());

			let verified_usage = Self::challenge_others(
				cluster_id,
				era_id,
				consistency_groups,
				&mut total_usage_keys,
				should_challenge,
			)?;

			if !verified_usage.is_empty() {
				total_usage.extend(verified_usage.clone());
			}

			Ok(total_usage)
		}

		pub(crate) fn challenge_others<A: Aggregate>(
			_cluster_id: &ClusterId,
			_era_id: DdcEra,
			consistency_groups: ConsistencyGroups<A>,
			accepted_keys: &mut Vec<AggregateKey>,
			should_challenge: bool,
		) -> Result<Vec<A>, Vec<OCWError>> {
			let redundancy_factor = T::DAC_REDUNDANCY_FACTOR;
			let mut verified_usage: Vec<A> = vec![];

			for consolidated_aggregate in consistency_groups.others {
				let aggregate_key = consolidated_aggregate.aggregate.get_key();

				if accepted_keys.contains(&aggregate_key) {
					log::warn!(
						"⚠️ The aggregate {:?} is inconsistent between aggregators.",
						aggregate_key
					);

					// This prevents the double spending in case of inconsistencies between
					// aggregators for the same aggregation key
					continue;
				}

				if consolidated_aggregate.count > redundancy_factor {
					let excessive_aggregate = consolidated_aggregate.aggregate.clone();

					log::warn!(
						"⚠️ Number of consistent aggregates with key {:?} exceeds the redundancy factor",
						aggregate_key
					);

					log::info!(
						"🔎‍ Challenging excessive aggregate with key {:?} and hash {:?}",
						aggregate_key,
						excessive_aggregate.hash::<T>()
					);

					// todo: run a challenge dedicated to the excessive number of aggregates.
					// we assume it won't happen at the moment, so we just take the aggregate to
					// payouts stage
					verified_usage.push(excessive_aggregate);
					accepted_keys.push(aggregate_key);
				} else {
					let defective_aggregate = consolidated_aggregate.aggregate.clone();

					log::info!(
						"🔎‍ Challenging defective aggregate with key {:?} and hash {:?}",
						aggregate_key,
						defective_aggregate.hash::<T>()
					);

					let mut is_passed = true;
					// todo: run an intensive challenge for deviating aggregate
					// let is_passed = Self::_challenge_aggregate(_cluster_id, _era_id,
					// &defective_aggregate)?;
					if should_challenge {
						is_passed = Self::challenge_aggregate_proto(
							_cluster_id,
							_era_id,
							&defective_aggregate,
						)?;
					}
					if is_passed {
						// we assume all aggregates are valid at the moment, so we just take the
						// aggregate to payouts stage
						verified_usage.push(defective_aggregate);
						accepted_keys.push(aggregate_key);
					}
				}
			}

			Ok(verified_usage)
		}

		pub(crate) fn _challenge_aggregate<A: Aggregate>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate: &A,
		) -> Result<bool, Vec<OCWError>> {
			let number_of_identifiers = T::MAX_MERKLE_NODE_IDENTIFIER;

			log::info!(
				"🚀 Challenge process starts when bucket sub aggregates are not in consensus!"
			);

			let aggregate_key = aggregate.get_key();
			let merkle_node_ids = Self::find_random_merkle_node_ids(
				number_of_identifiers.into(),
				aggregate.get_number_of_leaves(),
				aggregate_key.clone(),
			);

			log::info!(
				"🚀 Merkle Node Identifiers for aggregate key: {:?} identifiers: {:?}",
				aggregate_key,
				merkle_node_ids
			);

			let aggregator = aggregate.get_aggregator();

			let challenge_response = Self::_fetch_challenge_responses(
				cluster_id,
				era_id,
				aggregate_key.clone(),
				merkle_node_ids,
				aggregator.clone(),
			)
			.map_err(|err| vec![err])?;

			log::info!(
				"🚀 Fetched challenge response for aggregate key: {:?}, challenge_response: {:?}",
				aggregate_key,
				challenge_response
			);

			let calculated_merkle_root = Self::_get_hash_from_merkle_path(
				challenge_response,
				cluster_id,
				era_id,
				aggregate_key.clone(),
			)?;

			log::info!(
				"🚀 Calculated merkle root for aggregate key: {:?}, calculated_merkle_root: {:?}",
				aggregate_key,
				calculated_merkle_root
			);

			let root_merkle_node = Self::_fetch_traverse_response(
				era_id,
				aggregate_key.clone(),
				1,
				1,
				&aggregator.node_params,
			)
			.map_err(|_| {
				vec![OCWError::TraverseResponseRetrievalError {
					cluster_id: *cluster_id,
					era_id,
					aggregate_key: aggregate_key.clone(),
					aggregator: aggregator.node_pub_key,
				}]
			})?;

			let mut merkle_root_buf = [0u8; _BUF_SIZE];
			let bytes =
				Base64::decode(root_merkle_node.hash.clone(), &mut merkle_root_buf).unwrap(); // todo! remove unwrap
			let traversed_merkle_root = ActivityHash::from(sp_core::H256::from_slice(bytes));

			log::info!(
				"🚀 Fetched merkle root for aggregate key: {:?} traversed_merkle_root: {:?}",
				aggregate_key,
				traversed_merkle_root
			);

			let is_matched = if calculated_merkle_root == traversed_merkle_root {
				log::info!(
					"🚀👍 The aggregate with hash {:?} and key {:?} has passed the challenge.",
					aggregate.hash::<T>(),
					aggregate_key,
				);

				true
			} else {
				log::info!(
					"🚀👎 The aggregate with hash {:?} and key {:?} has not passed the challenge.",
					aggregate.hash::<T>(),
					aggregate_key,
				);

				false
			};

			Ok(is_matched)
		}

		pub(crate) fn challenge_aggregate_proto<A: Aggregate>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate: &A,
		) -> Result<bool, Vec<OCWError>> {
			let number_of_identifiers = T::MAX_MERKLE_NODE_IDENTIFIER;

			log::info!(
				"🚀 Challenge process starts when bucket sub aggregates are not in consensus!"
			);

			let aggregate_key = aggregate.get_key();
			let merkle_node_ids = Self::find_random_merkle_node_ids(
				number_of_identifiers.into(),
				aggregate.get_number_of_leaves(),
				aggregate_key.clone(),
			);

			log::info!(
				"🚀 Merkle Node Identifiers for aggregate key: {:?} identifiers: {:?}",
				aggregate_key,
				merkle_node_ids
			);

			let aggregator = aggregate.get_aggregator();

			let challenge_response = Self::_fetch_challenge_responses_proto(
				cluster_id,
				era_id,
				aggregate_key.clone(),
				merkle_node_ids.iter().map(|id| *id as u32).collect(),
				aggregator.clone(),
			)
			.map_err(|err| vec![err])?;

			log::info!(
				"🚀 Fetched challenge response for aggregate key: {:?}, challenge_response: {:?}",
				aggregate_key,
				challenge_response
			);

			let are_signatures_valid = signature::Verify::verify(&challenge_response);

			if are_signatures_valid {
				log::info!("👍 Valid challenge signatures for aggregate key: {:?}", aggregate_key,);
			} else {
				log::info!("👎 Invalid challenge signatures at aggregate key: {:?}", aggregate_key,);
			}

			Ok(are_signatures_valid)
		}

		pub(crate) fn _get_hash_from_merkle_path(
			challenge_response: aggregator_client::json::ChallengeAggregateResponse,
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
		) -> Result<ActivityHash, Vec<OCWError>> {
			log::info!("Getting hash from merkle tree path for aggregate key: {:?}", aggregate_key);

			let mut resulting_hash = ActivityHash::default();

			for proof in challenge_response.proofs {
				let leaf_record_hashes: Vec<ActivityHash> = match aggregate_key {
					AggregateKey::BucketSubAggregateKey(_, _) => proof
						.leafs
						.into_iter()
						.map(|p| NodeAggregateLeaf::leaf_hash::<T>(&p))
						.collect(),
					AggregateKey::NodeAggregateKey(_) => proof
						.leafs
						.into_iter()
						.map(|p| BucketSubAggregateLeaf::leaf_hash::<T>(&p))
						.collect(),
				};

				let leaf_record_hashes_string: Vec<String> =
					leaf_record_hashes.clone().into_iter().map(hex::encode).collect();

				log::info!(
					"🚀 Fetched leaf record hashes aggregate key: {:?} leaf_record_hashes: {:?}",
					aggregate_key,
					leaf_record_hashes_string
				);

				let leaf_node_root =
					Self::create_merkle_root(cluster_id, era_id, &leaf_record_hashes)
						.map_err(|err| vec![err])?;

				log::info!(
					"🚀 Fetched leaf record root aggregate key: {:?} leaf_record_root_hash: {:?}",
					aggregate_key,
					hex::encode(leaf_node_root)
				);

				let paths = proof.path.iter().rev();

				resulting_hash = leaf_node_root;
				for path in paths {
					let mut dec_buf = [0u8; _BUF_SIZE];
					let bytes = Base64::decode(path, &mut dec_buf).unwrap(); // todo! remove unwrap
					let path_hash: ActivityHash =
						ActivityHash::from(sp_core::H256::from_slice(bytes));

					let node_root =
						Self::create_merkle_root(cluster_id, era_id, &[resulting_hash, path_hash])
							.map_err(|err| vec![err])?;

					log::info!("🚀 Fetched leaf node root aggregate_key: {:?} for path:{:?} leaf_node_hash: {:?}",
						aggregate_key, path, hex::encode(node_root));

					resulting_hash = node_root;
				}
			}

			Ok(resulting_hash)
		}

		pub(crate) fn find_random_merkle_node_ids(
			number_of_identifiers: usize,
			number_of_leaves: u64,
			aggregate_key: AggregateKey,
		) -> Vec<u64> {
			let nonce_key = match aggregate_key {
				AggregateKey::NodeAggregateKey(node_id) => node_id,
				AggregateKey::BucketSubAggregateKey(.., node_id) => node_id,
			};

			let nonce = Self::_store_and_fetch_nonce(nonce_key);
			let mut small_rng = SmallRng::seed_from_u64(nonce);

			let total_levels = number_of_leaves.ilog2() + 1;
			let int_list: Vec<u64> = (0..total_levels as u64).collect();

			let ids: Vec<u64> = int_list
				.choose_multiple(&mut small_rng, number_of_identifiers)
				.cloned()
				.collect::<Vec<u64>>();

			ids
		}

		/// Computes the consensus for a set of partial activities across multiple buckets within a
		/// given cluster and era.
		///
		/// This function collects activities from various buckets, groups them by their consensus
		/// ID, and then determines if a consensus is reached for each group based on the minimum
		/// number of nodes and a given threshold. If the consensus is reached, the activity is
		/// included in the result. Otherwise, appropriate errors are returned.
		///
		/// # Input Parameters
		/// - `cluster_id: &ClusterId`: The ID of the cluster for which consensus is being computed.
		/// - `era_id: DdcEra`: The era ID within the cluster.
		/// - `buckets_aggregates_by_aggregator: &[(NodePubKey, Vec<A>)]`: A list of tuples, where
		///   each tuple contains a node's public key and a vector of activities reported for that
		///   bucket.
		/// - `redundancy_factor: u16`: The number of aggregators that should report total activity
		///   for a node or a bucket
		/// - `quorum: Percent`: The threshold percentage that determines if an activity is in
		///   consensus.
		///
		/// # Output
		/// - `Result<Vec<A>, Vec<OCWError>>`:
		///   - `Ok(Vec<A>)`: A vector of activities that have reached consensus.
		///   - `Err(Vec<OCWError>)`: A vector of errors indicating why consensus was not reached
		///     for some activities.
		pub(crate) fn group_buckets_sub_aggregates_by_consistency(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			buckets_aggregates_by_aggregator: Vec<(
				AggregatorInfo,
				Vec<aggregator_client::json::BucketAggregateResponse>,
			)>,
			redundancy_factor: u16,
			quorum: Percent,
		) -> ConsistencyGroups<aggregator_client::json::BucketSubAggregate> {
			let mut buckets_sub_aggregates: Vec<aggregator_client::json::BucketSubAggregate> =
				Vec::new();

			log::info!(
				"🏠⏳ Starting fetching bucket sub-aggregates for cluster_id: {:?} for era_id: {:?}",
				cluster_id,
				era_id
			);
			for (aggregator_info, buckets_aggregates_resp) in
				buckets_aggregates_by_aggregator.clone()
			{
				for bucket_aggregate_resp in buckets_aggregates_resp {
					for bucket_sub_aggregate_resp in bucket_aggregate_resp.sub_aggregates.clone() {
						let bucket_sub_aggregate = aggregator_client::json::BucketSubAggregate {
							bucket_id: bucket_aggregate_resp.bucket_id,
							node_id: bucket_sub_aggregate_resp.NodeID,
							stored_bytes: bucket_sub_aggregate_resp.stored_bytes,
							transferred_bytes: bucket_sub_aggregate_resp.transferred_bytes,
							number_of_puts: bucket_sub_aggregate_resp.number_of_puts,
							number_of_gets: bucket_sub_aggregate_resp.number_of_gets,
							aggregator: aggregator_info.clone(),
						};

						buckets_sub_aggregates.push(bucket_sub_aggregate);
					}

					log::info!("🏠🚀 Fetched Bucket sub-aggregates for cluster_id: {:?} for era_id: {:?} for bucket_id {:?}::: Bucket Sub-Aggregates are {:?}", cluster_id, era_id, bucket_aggregate_resp.bucket_id, bucket_aggregate_resp.sub_aggregates);
				}
			}

			let buckets_sub_aggregates_groups =
				Self::group_by_consistency(buckets_sub_aggregates, redundancy_factor, quorum);

			log::info!("🏠🌕 Bucket Sub-Aggregates, which are in consensus for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, buckets_sub_aggregates_groups.consensus);
			log::info!("🏠🌗 Bucket Sub-Aggregates, which are in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, buckets_sub_aggregates_groups.quorum);
			log::info!("🏠🌘 Bucket Sub-Aggregates, which are neither in consensus nor in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, buckets_sub_aggregates_groups.others);

			buckets_sub_aggregates_groups
=======
			Ok(Some((era_activity, customers_activity_root, nodes_activity_root)))
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		}

		pub(crate) fn start_validation_phase(
			cluster_id: &ClusterId,
			verification_account: &Account<T>,
			signer: &Signer<T, T::OffchainIdentifierId>,
		) -> Result<(), Vec<OCWError>> {
			let validation_output = Self::process_dac_era(cluster_id, None)?;

			match validation_output {
				Some((
					era_activity,
					payers_merkle_root_hash,
					payees_merkle_root_hash,
					payers_batch_merkle_root_hashes,
					payees_batch_merkle_root_hashes,
				)) => {
					let call = Call::set_prepare_era_for_payout {
						cluster_id: *cluster_id,
						era_activity,
						payers_merkle_root_hash,
						payees_merkle_root_hash,
						payers_batch_merkle_root_hashes: payers_batch_merkle_root_hashes.clone(),
						payees_batch_merkle_root_hashes: payees_batch_merkle_root_hashes.clone(),
					};

					let result = signer.send_single_signed_transaction(verification_account, call);

					match result {
						Some(Ok(_)) => {
							log::info!(
								"👁️‍🗨️  DAC Validation merkle roots posted on-chain for cluster_id: {:?}, era: {:?}",
								cluster_id,
								era_activity.clone()
							);
							Ok(())
						},
						_ => Err(vec![OCWError::PrepareEraTransactionError {
							cluster_id: *cluster_id,
							era_id: era_activity.id,
							payers_merkle_root_hash,
							payees_merkle_root_hash,
						}]),
					}
				},
				None => {
					log::info!("👁️‍🗨️  No eras for DAC processing for cluster_id: {:?}", cluster_id);
					Ok(())
				},
			}
		}

		pub(crate) fn start_payouts_phase(
			cluster_id: &ClusterId,
			account: &Account<T>,
			signer: &Signer<T, T::OffchainIdentifierId>,
		) -> Result<(), Vec<OCWError>> {
			let mut errors: Vec<OCWError> = Vec::new();

			if let Err(errs) = Self::step_commit_billing_fingerprint(cluster_id, account, signer) {
				errors.extend(errs);
			}

			if let Err(errs) = Self::step_begin_billing_report(cluster_id, account, signer) {
				errors.extend(errs);
			}

			if let Err(errs) = Self::step_begin_charging_customers(cluster_id, account, signer) {
				errors.extend(errs);
			}

			if let Err(errs) = Self::step_send_charging_customers(cluster_id, account, signer) {
				errors.extend(errs);
			}

			if let Err(errs) = Self::step_end_charging_customers(cluster_id, account, signer) {
				errors.extend(errs);
			}

			if let Err(errs) = Self::step_begin_rewarding_providers(cluster_id, account, signer) {
				errors.extend(errs);
			}

			if let Err(errs) = Self::step_send_rewarding_providers(cluster_id, account, signer) {
				errors.extend(errs);
			}

			if let Err(errs) = Self::step_end_rewarding_providers(cluster_id, account, signer) {
				errors.extend(errs);
			}

<<<<<<< HEAD
			if let Err(errs) = Self::step_end_billing_report(cluster_id, account, signer) {
				errors.extend(errs);
=======
			match Self::step_end_billing_report(cluster_id, account, signer) {
				Ok(Some(era_id)) => {
					Self::clear_verified_delta_usage(cluster_id, era_id);
				},
				Err(errs) => errors.extend(errs),
				_ => {},
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			}

			if !errors.is_empty() {
				Err(errors)
			} else {
				Ok(())
			}
		}

		define_payout_step_function!(
			step_commit_billing_fingerprint,
			prepare_commit_billing_fingerprint,
			|cluster_id: &ClusterId, (era, era_payable_usage): (EraActivity, PayableEraUsage)| {
				Call::commit_billing_fingerprint {
					cluster_id: *cluster_id,
					era_id: era.id,
					start_era: era.start,
					end_era: era.end,
					payers_root: era_payable_usage.payers_root,
					payees_root: era_payable_usage.payees_root,
					cluster_usage: era_payable_usage.cluster_usage,
				}
			},
			|prepared_data: &(EraActivity, PayableEraUsage)| prepared_data.0.id,
			"🔑",
			|cluster_id: &ClusterId, (era, era_payable_usage): (EraActivity, PayableEraUsage)| {
				OCWError::CommitBillingFingerprintTransactionError {
					cluster_id: *cluster_id,
					era_id: era.id,
					payers_root: era_payable_usage.payers_root,
					payees_root: era_payable_usage.payees_root,
				}
			}
		);

		define_payout_step_function!(
			step_begin_billing_report,
			prepare_begin_billing_report,
			|cluster_id: &ClusterId, (era_id, fingerprint)| Call::begin_billing_report {
				cluster_id: *cluster_id,
				era_id,
				fingerprint
			},
			|prepared_data: &(DdcEra, _)| prepared_data.0,
			"🗓️ ",
			|cluster_id: &ClusterId, (era_id, _)| OCWError::BeginBillingReportTransactionError {
				cluster_id: *cluster_id,
				era_id,
			}
		);

		define_payout_step_function!(
			step_begin_charging_customers,
			prepare_begin_charging_customers,
			|cluster_id: &ClusterId, (era_id, max_batch_index)| Call::begin_charging_customers {
				cluster_id: *cluster_id,
				era_id,
				max_batch_index,
			},
			|prepared_data: &(DdcEra, _)| prepared_data.0,
			"📥",
			|cluster_id: &ClusterId, (era_id, _)| {
				OCWError::BeginChargingCustomersTransactionError { cluster_id: *cluster_id, era_id }
			}
		);

		define_payout_step_function!(
			step_send_charging_customers,
			prepare_send_charging_customers_batch,
			|cluster_id: &ClusterId, (era_id, batch_payout): (DdcEra, CustomerBatch)| {
				Call::send_charging_customers_batch {
					cluster_id: *cluster_id,
					era_id,
					batch_index: batch_payout.batch_index,
					payers: batch_payout.payers.clone(),
					batch_proof: batch_payout.batch_proof.clone(),
				}
			},
			|prepared_data: &(DdcEra, _)| prepared_data.0,
			"🧾",
			|cluster_id: &ClusterId, (era_id, batch_payout): (DdcEra, CustomerBatch)| {
				OCWError::SendChargingCustomersBatchTransactionError {
					cluster_id: *cluster_id,
					era_id,
					batch_index: batch_payout.batch_index,
				}
			}
		);

		define_payout_step_function!(
			step_end_charging_customers,
			prepare_end_charging_customers,
			|cluster_id: &ClusterId, era_id| Call::end_charging_customers {
				cluster_id: *cluster_id,
				era_id
			},
			|prepared_data: &DdcEra| *prepared_data,
			"📪",
			|cluster_id: &ClusterId, era_id| OCWError::EndChargingCustomersTransactionError {
				cluster_id: *cluster_id,
				era_id,
			}
		);

		define_payout_step_function!(
			step_begin_rewarding_providers,
			prepare_begin_rewarding_providers,
			|cluster_id: &ClusterId, (era_id, max_batch_index): (DdcEra, u16)| {
				Call::begin_rewarding_providers { cluster_id: *cluster_id, era_id, max_batch_index }
			},
			|prepared_data: &(DdcEra, _)| prepared_data.0,
			"📤",
			|cluster_id: &ClusterId, (era_id, _)| {
				OCWError::BeginRewardingProvidersTransactionError {
					cluster_id: *cluster_id,
					era_id,
				}
			}
		);

		define_payout_step_function!(
			step_send_rewarding_providers,
			prepare_send_rewarding_providers_batch,
			|cluster_id: &ClusterId, (era_id, batch_payout): (DdcEra, ProviderBatch)| {
				Call::send_rewarding_providers_batch {
					cluster_id: *cluster_id,
					era_id,
					batch_index: batch_payout.batch_index,
					payees: batch_payout.payees.clone(),
					batch_proof: batch_payout.batch_proof.clone(),
				}
			},
			|prepared_data: &(DdcEra, _)| prepared_data.0,
			"💸",
			|cluster_id: &ClusterId, (era_id, batch_payout): (DdcEra, ProviderBatch)| {
				OCWError::SendRewardingProvidersBatchTransactionError {
					cluster_id: *cluster_id,
					era_id,
					batch_index: batch_payout.batch_index,
				}
			}
		);

		define_payout_step_function!(
			step_end_rewarding_providers,
			prepare_end_rewarding_providers,
			|cluster_id: &ClusterId, era_id| Call::end_rewarding_providers {
				cluster_id: *cluster_id,
				era_id
			},
			|prepared_data: &DdcEra| *prepared_data,
			"📭",
			|cluster_id: &ClusterId, era_id| OCWError::EndRewardingProvidersTransactionError {
				cluster_id: *cluster_id,
				era_id,
			}
		);

		define_payout_step_function!(
			step_end_billing_report,
			prepare_end_billing_report,
			|cluster_id: &ClusterId, era_id| Call::end_billing_report {
				cluster_id: *cluster_id,
				era_id
			},
			|prepared_data: &DdcEra| *prepared_data,
			"🧮",
			|cluster_id: &ClusterId, era_id| OCWError::EndBillingReportTransactionError {
				cluster_id: *cluster_id,
				era_id,
			}
		);

		pub(crate) fn submit_errors(
			errors: &Vec<OCWError>,
			verification_account: &Account<T>,
			signer: &Signer<T, T::OffchainIdentifierId>,
		) {
			if !errors.is_empty() {
				let call = Call::emit_consensus_errors { errors: errors.to_owned() };
				let result = signer.send_single_signed_transaction(verification_account, call);

				if let Some(Ok(_)) = result {
					log::info!("✔️ Successfully sent 'emit_consensus_errors' call");
				} else {
					log::error!("❌ Failed to send 'emit_consensus_errors' call");
				};
			}
		}

		pub(crate) fn get_total_usage<A: Aggregate>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			consistency_groups: ConsistencyGroups<A>,
			should_challenge: bool,
		) -> Result<Vec<A>, Vec<OCWError>> {
			let mut total_usage = vec![];
			let mut total_usage_keys = vec![];

			// todo: implement 'challenge_consensus' fn and run a light challenge for unanimous
			// consensus
			let in_consensus_usage = consistency_groups
				.consensus
				.clone()
				.into_iter()
				.map(|ca| ca.aggregate.clone())
				.collect::<Vec<_>>();
			total_usage.extend(in_consensus_usage.clone());
			total_usage_keys
				.extend(in_consensus_usage.into_iter().map(|a| a.get_key()).collect::<Vec<_>>());

			// todo: implement 'challenge_quorum' fn and run a light challenge for the quorum, i.e.
			// for majority
			let in_quorum_usage = consistency_groups
				.quorum
				.clone()
				.into_iter()
				.map(|ca| ca.aggregate.clone())
				.collect::<Vec<_>>();
			total_usage.extend(in_quorum_usage.clone());
			total_usage_keys
				.extend(in_quorum_usage.into_iter().map(|a| a.get_key()).collect::<Vec<_>>());

			let verified_usage = Self::challenge_others(
				cluster_id,
				era_id,
				consistency_groups,
				&mut total_usage_keys,
				should_challenge,
			)?;

			if !verified_usage.is_empty() {
				total_usage.extend(verified_usage.clone());
			}

			Ok(total_usage)
		}

		pub(crate) fn challenge_others<A: Aggregate>(
			_cluster_id: &ClusterId,
			_era_id: DdcEra,
			consistency_groups: ConsistencyGroups<A>,
			accepted_keys: &mut Vec<AggregateKey>,
			should_challenge: bool,
		) -> Result<Vec<A>, Vec<OCWError>> {
			let redundancy_factor = T::DAC_REDUNDANCY_FACTOR;
			let mut verified_usage: Vec<A> = vec![];

			for consolidated_aggregate in consistency_groups.others {
				let aggregate_key = consolidated_aggregate.aggregate.get_key();

				if accepted_keys.contains(&aggregate_key) {
					log::warn!(
						"⚠️ The aggregate {:?} is inconsistent between aggregators.",
						aggregate_key
					);

					// This prevents the double spending in case of inconsistencies between
					// aggregators for the same aggregation key
					continue;
				}

				if consolidated_aggregate.count > redundancy_factor {
					let excessive_aggregate = consolidated_aggregate.aggregate.clone();

					log::warn!(
						"⚠️ Number of consistent aggregates with key {:?} exceeds the redundancy factor",
						aggregate_key
					);

					log::info!(
						"🔎‍ Challenging excessive aggregate with key {:?} and hash {:?}",
						aggregate_key,
						excessive_aggregate.hash::<T>()
					);

					// todo: run a challenge dedicated to the excessive number of aggregates.
					// we assume it won't happen at the moment, so we just take the aggregate to
					// payouts stage
					verified_usage.push(excessive_aggregate);
					accepted_keys.push(aggregate_key);
				} else {
					let defective_aggregate = consolidated_aggregate.aggregate.clone();

					log::info!(
						"🔎‍ Challenging defective aggregate with key {:?} and hash {:?}",
						aggregate_key,
						defective_aggregate.hash::<T>()
					);

					let mut is_passed = true;
					// todo: run an intensive challenge for deviating aggregate
					// let is_passed = Self::_challenge_aggregate(_cluster_id, _era_id,
					// &defective_aggregate)?;
					if should_challenge {
						is_passed = Self::challenge_aggregate_proto(
							_cluster_id,
							_era_id,
							&defective_aggregate,
						)?;
					}
					if is_passed {
						// we assume all aggregates are valid at the moment, so we just take the
						// aggregate to payouts stage
						verified_usage.push(defective_aggregate);
						accepted_keys.push(aggregate_key);
					}
				}
			}

			Ok(verified_usage)
		}

		pub(crate) fn _challenge_aggregate<A: Aggregate>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate: &A,
		) -> Result<bool, Vec<OCWError>> {
			let number_of_identifiers = T::MAX_MERKLE_NODE_IDENTIFIER;

			log::info!(
				"👁️‍🗨️  Challenge process starts when bucket sub aggregates are not in consensus!"
			);

			let aggregate_key = aggregate.get_key();
			let merkle_node_ids = Self::find_random_merkle_node_ids(
				number_of_identifiers.into(),
				aggregate.get_number_of_leaves(),
				aggregate_key.clone(),
			);

			log::info!(
				"👁️‍🗨️  Merkle Node Identifiers for aggregate key: {:?} identifiers: {:?}",
				aggregate_key,
				merkle_node_ids
			);

			let aggregator = aggregate.get_aggregator();

			let challenge_response = Self::_fetch_challenge_responses(
				cluster_id,
				era_id,
				aggregate_key.clone(),
				merkle_node_ids,
				aggregator.clone(),
			)
			.map_err(|err| vec![err])?;

			log::info!(
				"👁️‍🗨️  Fetched challenge response for aggregate key: {:?}, challenge_response: {:?}",
				aggregate_key,
				challenge_response
			);

			let calculated_merkle_root = Self::_get_hash_from_merkle_path(
				challenge_response,
				cluster_id,
				era_id,
				aggregate_key.clone(),
			)?;

			log::info!(
				"👁️‍🗨️  Calculated merkle root for aggregate key: {:?}, calculated_merkle_root: {:?}",
				aggregate_key,
				calculated_merkle_root
			);

			let root_merkle_node = Self::_fetch_traverse_response(
				era_id,
				aggregate_key.clone(),
				1,
				1,
				&aggregator.node_params,
			)
			.map_err(|_| {
				vec![OCWError::TraverseResponseRetrievalError {
					cluster_id: *cluster_id,
					era_id,
					aggregate_key: aggregate_key.clone(),
					aggregator: aggregator.node_pub_key,
				}]
			})?;

			let mut merkle_root_buf = [0u8; _BUF_SIZE];
			let bytes =
				Base64::decode(root_merkle_node.hash.clone(), &mut merkle_root_buf).unwrap(); // todo! remove unwrap
			let traversed_merkle_root = DeltaUsageHash::from(sp_core::H256::from_slice(bytes));

			log::info!(
				"👁️‍🗨️  Fetched merkle root for aggregate key: {:?} traversed_merkle_root: {:?}",
				aggregate_key,
				traversed_merkle_root
			);

			let is_matched = if calculated_merkle_root == traversed_merkle_root {
				log::info!(
					"👁️‍🗨️👍 The aggregate with hash {:?} and key {:?} has passed the challenge.",
					aggregate.hash::<T>(),
					aggregate_key,
				);

				true
			} else {
				log::info!(
					"👁️‍🗨️👎 The aggregate with hash {:?} and key {:?} has not passed the challenge.",
					aggregate.hash::<T>(),
					aggregate_key,
				);

				false
			};

			Ok(is_matched)
		}

		pub(crate) fn challenge_aggregate_proto<A: Aggregate>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate: &A,
		) -> Result<bool, Vec<OCWError>> {
			let number_of_identifiers = T::MAX_MERKLE_NODE_IDENTIFIER;

			log::info!(
				"👁️‍🗨️  Challenge process starts when bucket sub aggregates are not in consensus!"
			);

			let aggregate_key = aggregate.get_key();
			let merkle_node_ids = Self::find_random_merkle_node_ids(
				number_of_identifiers.into(),
				aggregate.get_number_of_leaves(),
				aggregate_key.clone(),
			);

			log::info!(
				"👁️‍🗨️  Merkle Node Identifiers for aggregate key: {:?} identifiers: {:?}",
				aggregate_key,
				merkle_node_ids
			);

			let aggregator = aggregate.get_aggregator();

			let challenge_response = Self::_fetch_challenge_responses_proto(
				cluster_id,
				era_id,
				aggregate_key.clone(),
				merkle_node_ids.iter().map(|id| *id as u32).collect(),
				aggregator.clone(),
			)
			.map_err(|err| vec![err])?;

			log::info!(
				"👁️‍🗨️  Fetched challenge response for aggregate key: {:?}, challenge_response: {:?}",
				aggregate_key,
				challenge_response
			);

			let are_signatures_valid = signature::Verify::verify(&challenge_response);

			if are_signatures_valid {
				log::info!("👍 Valid challenge signatures for aggregate key: {:?}", aggregate_key,);
			} else {
				log::info!("👎 Invalid challenge signatures at aggregate key: {:?}", aggregate_key,);
			}

			Ok(are_signatures_valid)
		}

		pub(crate) fn _get_hash_from_merkle_path(
			challenge_response: aggregator_client::json::ChallengeAggregateResponse,
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
		) -> Result<DeltaUsageHash, Vec<OCWError>> {
			log::info!("Getting hash from merkle tree path for aggregate key: {:?}", aggregate_key);

			let mut resulting_hash = DeltaUsageHash::default();

			for proof in challenge_response.proofs {
				let leaf_record_hashes: Vec<DeltaUsageHash> = match aggregate_key {
					AggregateKey::BucketSubAggregateKey(_, _) => proof
						.leafs
						.into_iter()
						.map(|p| NodeAggregateLeaf::leaf_hash::<T>(&p))
						.collect(),
					AggregateKey::NodeAggregateKey(_) => proof
						.leafs
						.into_iter()
						.map(|p| BucketSubAggregateLeaf::leaf_hash::<T>(&p))
						.collect(),
				};

				let leaf_record_hashes_string: Vec<String> =
					leaf_record_hashes.clone().into_iter().map(hex::encode).collect();

				log::info!(
					"👁️‍🗨️  Fetched leaf record hashes aggregate key: {:?} leaf_record_hashes: {:?}",
					aggregate_key,
					leaf_record_hashes_string
				);

				let leaf_node_root =
					Self::create_merkle_root(cluster_id, era_id, &leaf_record_hashes)
						.map_err(|err| vec![err])?;

				log::info!(
					"👁️‍🗨️  Fetched leaf record root aggregate key: {:?} leaf_record_root_hash: {:?}",
					aggregate_key,
					hex::encode(leaf_node_root)
				);

				let paths = proof.path.iter().rev();

				resulting_hash = leaf_node_root;
				for path in paths {
					let mut dec_buf = [0u8; _BUF_SIZE];
					let bytes = Base64::decode(path, &mut dec_buf).unwrap(); // todo! remove unwrap
					let path_hash: DeltaUsageHash = DeltaUsageHash::from(H256::from_slice(bytes));

					let node_root =
						Self::create_merkle_root(cluster_id, era_id, &[resulting_hash, path_hash])
							.map_err(|err| vec![err])?;

					log::info!("👁️‍🗨️  Fetched leaf node root aggregate_key: {:?} for path:{:?} leaf_node_hash: {:?}",
						aggregate_key, path, hex::encode(node_root));

					resulting_hash = node_root;
				}
			}

			Ok(resulting_hash)
		}

		pub(crate) fn find_random_merkle_node_ids(
			number_of_identifiers: usize,
			number_of_leaves: u64,
			aggregate_key: AggregateKey,
		) -> Vec<u64> {
			let nonce_key = match aggregate_key {
				AggregateKey::NodeAggregateKey(node_id) => node_id,
				AggregateKey::BucketSubAggregateKey(.., node_id) => node_id,
			};

			let nonce = Self::_store_and_fetch_nonce(nonce_key);
			let mut small_rng = SmallRng::seed_from_u64(nonce);

			let total_levels = number_of_leaves.ilog2() + 1;
			let int_list: Vec<u64> = (0..total_levels as u64).collect();

			let ids: Vec<u64> = int_list
				.choose_multiple(&mut small_rng, number_of_identifiers)
				.cloned()
				.collect::<Vec<u64>>();

			ids
		}

		/// Computes the consensus for a set of partial activities across multiple buckets within a
		/// given cluster and era.
		///
		/// This function collects activities from various buckets, groups them by their consensus
		/// ID, and then determines if a consensus is reached for each group based on the minimum
		/// number of nodes and a given threshold. If the consensus is reached, the activity is
		/// included in the result. Otherwise, appropriate errors are returned.
		///
		/// # Input Parameters
		/// - `cluster_id: &ClusterId`: The ID of the cluster for which consensus is being computed.
		/// - `era_id: DdcEra`: The era ID within the cluster.
		/// - `buckets_aggregates_by_aggregator: &[(NodePubKey, Vec<A>)]`: A list of tuples, where
		///   each tuple contains a node's public key and a vector of activities reported for that
		///   bucket.
		/// - `redundancy_factor: u16`: The number of aggregators that should report total activity
		///   for a node or a bucket
		/// - `quorum: Percent`: The threshold percentage that determines if an activity is in
		///   consensus.
		///
		/// # Output
		/// - `Result<Vec<A>, Vec<OCWError>>`:
		///   - `Ok(Vec<A>)`: A vector of activities that have reached consensus.
		///   - `Err(Vec<OCWError>)`: A vector of errors indicating why consensus was not reached
		///     for some activities.
		pub(crate) fn group_buckets_sub_aggregates_by_consistency(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			buckets_aggregates_by_aggregator: Vec<(
				AggregatorInfo,
				Vec<aggregator_client::json::BucketAggregateResponse>,
			)>,
			redundancy_factor: u16,
			quorum: Percent,
		) -> ConsistencyGroups<aggregator_client::json::BucketSubAggregate> {
			let mut buckets_sub_aggregates: Vec<aggregator_client::json::BucketSubAggregate> =
				Vec::new();

			log::info!(
				"👁️‍🗨️‍  Starting fetching bucket sub-aggregates for cluster_id: {:?} for era_id: {:?}",
				cluster_id,
				era_id
			);
			for (aggregator_info, buckets_aggregates_resp) in
				buckets_aggregates_by_aggregator.clone()
			{
				for bucket_aggregate_resp in buckets_aggregates_resp {
					for bucket_sub_aggregate_resp in bucket_aggregate_resp.sub_aggregates.clone() {
						let bucket_sub_aggregate = aggregator_client::json::BucketSubAggregate {
							bucket_id: bucket_aggregate_resp.bucket_id,
							node_id: bucket_sub_aggregate_resp.NodeID,
							stored_bytes: bucket_sub_aggregate_resp.stored_bytes,
							transferred_bytes: bucket_sub_aggregate_resp.transferred_bytes,
							number_of_puts: bucket_sub_aggregate_resp.number_of_puts,
							number_of_gets: bucket_sub_aggregate_resp.number_of_gets,
							aggregator: aggregator_info.clone(),
						};

						buckets_sub_aggregates.push(bucket_sub_aggregate);
					}

					log::info!("👁️‍🗨️‍  Fetched Bucket sub-aggregates for cluster_id: {:?} for era_id: {:?} for bucket_id {:?}::: Bucket Sub-Aggregates are {:?}", cluster_id, era_id, bucket_aggregate_resp.bucket_id, bucket_aggregate_resp.sub_aggregates);
				}
			}

			let buckets_sub_aggregates_groups =
				Self::group_by_consistency(buckets_sub_aggregates, redundancy_factor, quorum);

			log::info!("👁️‍🗨️‍🌕 Bucket Sub-Aggregates, which are in consensus for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, buckets_sub_aggregates_groups.consensus);
			log::info!("👁️‍🗨️‍🌗 Bucket Sub-Aggregates, which are in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, buckets_sub_aggregates_groups.quorum);
			log::info!("👁️‍🗨️‍🌘 Bucket Sub-Aggregates, which are neither in consensus nor in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, buckets_sub_aggregates_groups.others);

			buckets_sub_aggregates_groups
		}

		pub(crate) fn build_and_store_payable_usage(
			cluster_id: &ClusterId,
			era: EraActivity,
		) -> Result<(), Vec<OCWError>> {
			let batch_size = T::MAX_PAYOUT_BATCH_SIZE;

			let (buckets_delta_usage, nodes_delta_usage) =
				Self::fetch_verified_delta_usage_or_retry(cluster_id, era.id, era.start, era.end)?;

			let payers_usage =
				Self::calculate_buckets_payable_usage(cluster_id, buckets_delta_usage);

			let payees_usage = Self::calculate_nodes_payable_usage(cluster_id, nodes_delta_usage);

			let payers_batch_roots = Self::convert_to_batch_merkle_roots(
				cluster_id,
				era.id,
				Self::split_to_batches(&payers_usage, batch_size.into()),
			)
			.map_err(|err| vec![err])?;

			let payees_batch_roots = Self::convert_to_batch_merkle_roots(
				cluster_id,
				era.id,
				Self::split_to_batches(&payees_usage, batch_size.into()),
			)
			.map_err(|err| vec![err])?;

			let payers_root = Self::create_merkle_root(cluster_id, era.id, &payers_batch_roots)
				.map_err(|err| vec![err])?;

			let payees_root = Self::create_merkle_root(cluster_id, era.id, &payees_batch_roots)
				.map_err(|err| vec![err])?;

			Self::store_payable_usage(
				cluster_id,
				era,
				payers_usage,
				payers_root,
				payers_batch_roots,
				payees_usage,
				payees_root,
				payees_batch_roots,
			);

			Ok(())
		}

		fn fetch_payable_usage_or_retry(
			cluster_id: &ClusterId,
			era: EraActivity,
		) -> Result<PayableEraUsage, Vec<OCWError>> {
			if let Some(payble_usage) = Self::fetch_payable_usage(cluster_id, era.id) {
				Ok(payble_usage)
			} else {
				Self::build_and_store_payable_usage(cluster_id, era)?;
				if let Some(payble_usage) = Self::fetch_payable_usage(cluster_id, era.id) {
					Ok(payble_usage)
				} else {
					Err(vec![OCWError::FailedToFetchVerifiedPayableUsage])
				}
			}
		}

		fn calculate_buckets_payable_usage(
			cluster_id: &ClusterId,
			buckets_delta_usage: Vec<BucketDeltaUsage>,
		) -> Vec<BucketPayableUsage> {
			let mut result = Vec::new();

			let delta_usage_map: BTreeMap<BucketId, BucketDeltaUsage> = buckets_delta_usage
				.into_iter()
				.map(|delta_usage| (delta_usage.bucket_id, delta_usage))
				.collect();

			let mut merged_bucket_ids: BTreeSet<BucketId> = BTreeSet::new();

			for current_usage in T::BucketsStorageUsageProvider::iter_storage_usage(cluster_id) {
				if let Some(delta_usage) = delta_usage_map.get(&current_usage.bucket_id) {
					// Intersection: Charge for the sum of the current usage and delta usage.
					let payable_usage = BucketPayableUsage(
						current_usage.bucket_id,
						BucketUsage {
							transferred_bytes: delta_usage.transferred_bytes,
							stored_bytes: current_usage
								.stored_bytes
								.saturating_add(delta_usage.stored_bytes),
							number_of_puts: delta_usage.number_of_puts,
							number_of_gets: delta_usage.number_of_gets,
						},
					);

					result.push(payable_usage);
				} else {
					// No Intersection: Charge for the current usage only. There was no activity for
					// this bucket in the operating era.
					let payable_usage = BucketPayableUsage(
						current_usage.bucket_id,
						BucketUsage {
							transferred_bytes: 0,
							stored_bytes: current_usage.stored_bytes,
							number_of_puts: 0,
							number_of_gets: 0,
						},
					);
					result.push(payable_usage);
				}
				merged_bucket_ids.insert(current_usage.bucket_id);
			}

			for delta_usage in delta_usage_map.values() {
				if !merged_bucket_ids.contains(&delta_usage.bucket_id) {
					// No Intersection: Charge for the delta usage only. Possibly, this is a new
					// bucket that is charged after its first operating era.
					let payable_usage = BucketPayableUsage(
						delta_usage.bucket_id,
						BucketUsage {
							transferred_bytes: delta_usage.transferred_bytes,
							stored_bytes: delta_usage.stored_bytes,
							number_of_puts: delta_usage.number_of_puts,
							number_of_gets: delta_usage.number_of_gets,
						},
					);
					result.push(payable_usage);
				}
			}

			result
		}

		fn calculate_nodes_payable_usage(
			cluster_id: &ClusterId,
			nodes_delta_usage: Vec<NodeDeltaUsage>,
		) -> Vec<NodePayableUsage> {
			let mut result = Vec::new();

			let delta_usage_map: BTreeMap<NodePubKey, NodeDeltaUsage> = nodes_delta_usage
				.into_iter()
				.filter_map(|delta_usage| {
					if let Ok(node_key) = Self::node_key_from_hex(delta_usage.node_id.clone()) {
						Option::Some((node_key, delta_usage))
					} else {
						Option::None
					}
				})
				.collect();

			let mut merged_nodes_keys: BTreeSet<NodePubKey> = BTreeSet::new();

			for current_usage in T::NodesStorageUsageProvider::iter_storage_usage(cluster_id) {
				if let Some(delta_usage) = delta_usage_map.get(&current_usage.node_key) {
					// Intersection: Reward for the sum of the current usage and delta usage.
					let payable_usage = NodePayableUsage(
						current_usage.node_key.clone(),
						NodeUsage {
							transferred_bytes: delta_usage.transferred_bytes,
							stored_bytes: current_usage
								.stored_bytes
								.saturating_add(delta_usage.stored_bytes),
							number_of_puts: delta_usage.number_of_puts,
							number_of_gets: delta_usage.number_of_gets,
						},
					);

					result.push(payable_usage);
				} else {
					// No Intersection: Reward for the current usage only. There was no activity for
					// this node in the operating era.
					let payable_usage = NodePayableUsage(
						current_usage.node_key.clone(),
						NodeUsage {
							transferred_bytes: 0,
							stored_bytes: current_usage.stored_bytes,
							number_of_puts: 0,
							number_of_gets: 0,
						},
					);
					result.push(payable_usage);
				}
				merged_nodes_keys.insert(current_usage.node_key);
			}

			for (node_key, delta_usage) in delta_usage_map.into_iter() {
				if !merged_nodes_keys.contains(&node_key) {
					// No Intersection: Reward for the delta usage only. Possibly, this is a new
					// node that is rewarded after its first operating era.
					let payable_usage = NodePayableUsage(
						node_key,
						NodeUsage {
							transferred_bytes: delta_usage.transferred_bytes,
							stored_bytes: delta_usage.stored_bytes,
							number_of_puts: delta_usage.number_of_puts,
							number_of_gets: delta_usage.number_of_gets,
						},
					);
					result.push(payable_usage);
				}
			}

			result
		}

		fn fetch_verified_delta_usage_or_retry(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			start: i64,
			end: i64,
		) -> Result<(Vec<BucketDeltaUsage>, Vec<NodeDeltaUsage>), Vec<OCWError>> {
			if let Some((buckets_deltas, _, _, nodes_deltas, _, _)) =
				Self::fetch_verified_delta_usage(cluster_id, era_id)
			{
				Ok((buckets_deltas, nodes_deltas))
			} else {
				let era_activity = EraActivity { id: era_id, start, end };
				Self::process_dac_era(cluster_id, Some(era_activity))?;
				if let Some((buckets_deltas, _, _, nodes_deltas, _, _)) =
					Self::fetch_verified_delta_usage(cluster_id, era_id)
				{
					Ok((buckets_deltas, nodes_deltas))
				} else {
					Err(vec![OCWError::FailedToFetchVerifiedDeltaUsage])
				}
			}
		}

		pub(crate) fn prepare_commit_billing_fingerprint(
			cluster_id: &ClusterId,
		) -> Result<Option<(EraActivity, PayableEraUsage)>, Vec<OCWError>> {
			if let Some(era) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::ReadyForPayout)
			{
				let era_payable_usage = Self::fetch_payable_usage_or_retry(cluster_id, era)?;
				Ok(Some((era, era_payable_usage)))
			} else {
				Ok(None)
			}
		}

		#[allow(dead_code)]
		pub(crate) fn prepare_begin_billing_report(
			cluster_id: &ClusterId,
<<<<<<< HEAD
		) -> Result<Option<(DdcEra, i64, i64)>, Vec<OCWError>> {
			Ok(Self::get_era_for_payout(cluster_id, EraValidationStatus::ReadyForPayout))
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
			// todo! get start and end values based on result
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
=======
				.map_err(|e| vec![e])
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
=======
		) -> Result<Option<(DdcEra, Fingerprint)>, Vec<OCWError>> {
			if let Some(era) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::ReadyForPayout)
			{
				let era_payable_usage = Self::fetch_payable_usage_or_retry(cluster_id, era)?;
				Ok(Some((era.id, era_payable_usage.fingerprint())))
			} else {
				Ok(None)
			}
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		}

		pub(crate) fn prepare_begin_charging_customers(
			cluster_id: &ClusterId,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
<<<<<<< HEAD
			min_nodes: u16,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			redundancy_factor: u16,
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
=======
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
			batch_size: usize,
=======
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
		) -> Result<Option<(DdcEra, BatchIndex)>, Vec<OCWError>> {
			if let Some(era) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::Initialized
				{
					if let Some((_, _, customers_activity_batch_roots, _, _, _)) =
						Self::fetch_validation_activities::<
							aggregator_client::json::BucketSubAggregate,
							aggregator_client::json::NodeAggregate,
						>(cluster_id, era_id)
					{
=======
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::Initialized
				{
					if let Some((_, _, customers_activity_batch_roots, _, _, _)) =
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
						Self::fetch_validation_activities::<CustomerActivity, NodeActivity>(
							cluster_id, era_id,
						) {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						Self::fetch_validation_activities::<
							BucketNodeAggregatesActivity,
							NodeActivity,
						>(cluster_id, era_id)
					{
>>>>>>> 5288a1a7 (Fixing billing activity issue (#431))
=======
						Self::fetch_validation_activities::<BucketActivityPerNode, NodeActivity>(
=======
						Self::fetch_validation_activities::<BucketSubAggregate, NodeAggregate>(
>>>>>>> 23acfcfc (chore: activities renamed to aggregates, redundant traits impls are removed)
							cluster_id, era_id,
						) {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
=======
						Self::fetch_validation_activities::<
							aggregator_client::json::BucketSubAggregate,
							aggregator_client::json::NodeAggregate,
						>(cluster_id, era_id)
					{
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
						Self::fetch_customer_activity(
							cluster_id,
							era_id,
							customers_activity_batch_roots,
						)
					} else {
						let era_activity = EraActivity { id: era_id, start, end };

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 6a46f4d2 (chore: renaming for eliminating ambiguity)
						let _ = Self::process_dac_era(cluster_id, Some(era_activity), batch_size)?;
=======
						let _ = Self::process_dac_era(cluster_id, Some(era_activity))?;
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)

						if let Some((_, _, customers_activity_batch_roots, _, _, _)) =
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
							Self::fetch_validation_activities::<
								aggregator_client::json::BucketSubAggregate,
								aggregator_client::json::NodeAggregate,
							>(cluster_id, era_id)
						{
<<<<<<< HEAD
=======
						let _ = Self::process_dac_data(
							cluster_id,
							Some(era_activity),
							dac_nodes,
							redundancy_factor,
							batch_size,
						)?;
=======
						let _ = Self::process_dac_data(cluster_id, Some(era_activity), batch_size)?;
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)

						if let Some((_, _, customers_activity_batch_roots, _, _, _)) =
<<<<<<< HEAD
<<<<<<< HEAD
							Self::fetch_validation_activities::<CustomerActivity, NodeActivity>(
								cluster_id, era_id,
							) {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
							Self::fetch_validation_activities::<
								BucketNodeAggregatesActivity,
								NodeActivity,
							>(cluster_id, era_id)
						{
>>>>>>> 5288a1a7 (Fixing billing activity issue (#431))
=======
							Self::fetch_validation_activities::<BucketActivityPerNode, NodeActivity>(
=======
							Self::fetch_validation_activities::<BucketSubAggregate, NodeAggregate>(
>>>>>>> 23acfcfc (chore: activities renamed to aggregates, redundant traits impls are removed)
								cluster_id, era_id,
							) {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
							Self::fetch_customer_activity(
								cluster_id,
								era_id,
								customers_activity_batch_roots,
							)
						} else {
							Ok(None)
						}
					}
=======
=======
>>>>>>> c7dd3fb3 (fix: runtime version is increased)
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era.id) ==
					PayoutState::Initialized
				{
					let era_payable_usage = Self::fetch_payable_usage_or_retry(cluster_id, era)?;
					Self::fetch_charging_loop_input(
						cluster_id,
						era.id,
						era_payable_usage.payers_batch_roots,
					)
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
				} else {
					Ok(None)
				}
			} else {
				Ok(None)
			}
		}

		pub(crate) fn fetch_charging_loop_input(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			payers_batch_roots: Vec<PayableUsageHash>,
		) -> Result<Option<(DdcEra, BatchIndex)>, Vec<OCWError>> {
			if let Some(max_batch_index) = payers_batch_roots.len().checked_sub(1) {
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
<<<<<<< HEAD
			batch_size: usize,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
		) -> Result<Option<(DdcEra, CustomerBatch)>, Vec<OCWError>> {
			let batch_size = T::MAX_PAYOUT_BATCH_SIZE;

			if let Some(era) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
<<<<<<< HEAD
<<<<<<< HEAD
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era_id) ==
<<<<<<< HEAD
					PayoutState::ChargingCustomers
				{
					if let Some((
						customers_total_activity,
=======
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
			redundancy_factor: u16,
=======
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
		) -> Result<Option<(DdcEra, CustomerBatch<T>)>, Vec<OCWError>> {
=======
		) -> Result<Option<(DdcEra, CustomerBatch)>, Vec<OCWError>> {
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
			if let Some((era_id, start, end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
					PayoutState::ChargingCustomers
				{
					if let Some((
<<<<<<< HEAD
<<<<<<< HEAD
						customers_activity_in_consensus,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						bucket_nodes_activity_in_consensus,
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
						customers_total_activity,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
						_,
						customers_activity_batch_roots,
						_,
						_,
						_,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
					)) = Self::fetch_validation_activities::<
						aggregator_client::json::BucketSubAggregate,
						aggregator_client::json::NodeAggregate,
					>(cluster_id, era_id)
					{
<<<<<<< HEAD
=======
					)) = Self::fetch_validation_activities::<CustomerActivity, NodeActivity>(
						cluster_id, era_id,
					) {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
					)) = Self::fetch_validation_activities::<
						BucketNodeAggregatesActivity,
						NodeActivity,
					>(cluster_id, era_id)
					{
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
					)) = Self::fetch_validation_activities::<BucketActivityPerNode, NodeActivity>(
=======
					)) = Self::fetch_validation_activities::<BucketSubAggregate, NodeAggregate>(
>>>>>>> 23acfcfc (chore: activities renamed to aggregates, redundant traits impls are removed)
						cluster_id, era_id,
					) {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
						Self::fetch_charging_activities(
							cluster_id,
							batch_size.into(),
							era_id,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
							customers_total_activity,
=======
							customers_activity_in_consensus,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
							bucket_nodes_activity_in_consensus,
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
							customers_total_activity,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
							customers_activity_batch_roots,
						)
					} else {
						let era_activity = EraActivity { id: era_id, start, end };

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 6a46f4d2 (chore: renaming for eliminating ambiguity)
						let _ = Self::process_dac_era(cluster_id, Some(era_activity), batch_size)?;
=======
						let _ = Self::process_dac_era(cluster_id, Some(era_activity))?;
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)

						if let Some((
							customers_total_activity,
<<<<<<< HEAD
=======
						let _ = Self::process_dac_data(
							cluster_id,
							Some(era_activity),
							dac_nodes,
							redundancy_factor,
							batch_size,
						)?;
=======
						let _ = Self::process_dac_data(cluster_id, Some(era_activity), batch_size)?;
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)

						if let Some((
<<<<<<< HEAD
							customers_activity_in_consensus,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
							bucket_nodes_activity_in_consensus,
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
							_,
							customers_activity_batch_roots,
							_,
							_,
							_,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
						)) = Self::fetch_validation_activities::<
							aggregator_client::json::BucketSubAggregate,
							aggregator_client::json::NodeAggregate,
						>(cluster_id, era_id)
						{
<<<<<<< HEAD
=======
						)) = Self::fetch_validation_activities::<CustomerActivity, NodeActivity>(
							cluster_id, era_id,
						) {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						)) = Self::fetch_validation_activities::<
							BucketNodeAggregatesActivity,
							NodeActivity,
						>(cluster_id, era_id)
						{
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
						)) = Self::fetch_validation_activities::<BucketActivityPerNode, NodeActivity>(
=======
						)) = Self::fetch_validation_activities::<BucketSubAggregate, NodeAggregate>(
>>>>>>> 23acfcfc (chore: activities renamed to aggregates, redundant traits impls are removed)
							cluster_id, era_id,
						) {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
							Self::fetch_charging_activities(
								cluster_id,
								batch_size.into(),
								era_id,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
								customers_total_activity,
=======
								customers_activity_in_consensus,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
								bucket_nodes_activity_in_consensus,
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
								customers_total_activity,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
								customers_activity_batch_roots,
							)
						} else {
							Ok(None)
						}
					}
=======
=======
>>>>>>> c7dd3fb3 (fix: runtime version is increased)
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era.id) ==
					PayoutState::ChargingCustomers
				{
					let era_payable_usage = Self::fetch_payable_usage_or_retry(cluster_id, era)?;
					Self::fetch_charging_customers_batch(
						cluster_id,
						batch_size.into(),
						era.id,
						era_payable_usage.payers_usage,
						era_payable_usage.payers_batch_roots,
					)
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
				} else {
					Ok(None)
				}
			} else {
				Ok(None)
			}
		}

		fn fetch_charging_customers_batch(
			cluster_id: &ClusterId,
			batch_size: usize,
			era_id: DdcEra,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			customers_total_activity: Vec<aggregator_client::json::BucketSubAggregate>,
			customers_activity_batch_roots: Vec<ActivityHash>,
=======
			payers_usage: Vec<BucketPayableUsage>,
			payers_batch_roots: Vec<PayableUsageHash>,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		) -> Result<Option<(DdcEra, CustomerBatch)>, Vec<OCWError>> {
			let batch_index =
				T::PayoutProcessor::get_next_customer_batch_for_payment(cluster_id, era_id)
					.map_err(|_| {
						vec![OCWError::BillingReportDoesNotExist {
							cluster_id: *cluster_id,
							era_id,
						}]
					})?;
<<<<<<< HEAD
=======
			customers_activity_in_consensus: Vec<CustomerActivity>,
=======
			bucket_nodes_activity_in_consensus: Vec<BucketNodeAggregatesActivity>,
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
			bucket_nodes_activity_in_consensus: Vec<BucketActivityPerNode>,
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
=======
			bucket_nodes_activity_in_consensus: Vec<BucketSubAggregate>,
>>>>>>> 23acfcfc (chore: activities renamed to aggregates, redundant traits impls are removed)
=======
			customers_total_activity: Vec<BucketSubAggregate>,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
			customers_total_activity: Vec<aggregator_client::json::BucketSubAggregate>,
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
			customers_activity_batch_roots: Vec<ActivityHash>,
		) -> Result<Option<(DdcEra, CustomerBatch)>, Vec<OCWError>> {
			let batch_index = T::PayoutVisitor::get_next_customer_batch_for_payment(
				cluster_id, era_id,
			)
			.map_err(|_| {
				vec![OCWError::BillingReportDoesNotExist { cluster_id: *cluster_id, era_id }]
			})?;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

			if let Some(index) = batch_index {
				let i: usize = index.into();
				// todo! store batched activity to avoid splitting it again each time
<<<<<<< HEAD
				let customers_activity_batched =
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
					Self::split_to_batches(&customers_total_activity, batch_size);
=======
					Self::split_to_batches(&customers_activity_in_consensus, batch_size);
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
					Self::split_to_batches(&bucket_nodes_activity_in_consensus, batch_size);
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
					Self::split_to_batches(&customers_total_activity, batch_size);
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
				let payers_batches = Self::split_to_batches(&payers_usage, batch_size);
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)

				let batch_root = payers_batch_roots[i];
				let store = MemStore::default();
				let mut mmr: MMR<DeltaUsageHash, MergeMMRHash, &MemStore<DeltaUsageHash>> =
					MemMMR::<_, MergeMMRHash>::new(0, &store);

				let leaf_position_map: Vec<(DeltaUsageHash, u64)> =
					payers_batch_roots.iter().map(|a| (*a, mmr.push(*a).unwrap())).collect();

				let leaf_position: Vec<(u64, DeltaUsageHash)> = leaf_position_map
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

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				let batch_proof = MMRProof { proof };
=======
				let batch_proof = MMRProof {
					mmr_size: mmr.mmr_size(),
					proof,
					leaf_with_position: leaf_position[0],
				};
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				let batch_proof = MMRProof { mmr_size: mmr.mmr_size(), proof };
>>>>>>> 29b2c1bf (chore: hash and position of the leaf is removed from tx parameters)
=======
				let batch_proof = MMRProof { proof };
>>>>>>> 0c3d2872 (chore: calculating mmr size based on max batch index)
				Ok(Some((
					era_id,
					CustomerBatch {
						batch_index: index,
						payers: payers_batches[i]
							.iter()
<<<<<<< HEAD
							.map(|activity| {
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
								let node_key = Self::node_key_from_hex(activity.node_id.clone())
									.expect("Node Public Key to be decoded");
								let bucket_id = activity.bucket_id;
								let customer_usage = BucketUsage {
<<<<<<< HEAD
=======
=======
								let customer_id = activity.clone().customer_id.into_bytes();
>>>>>>> 1f5e092b (Fixing Customer Id Ecoding Issue (#406))
								let account_id =
									T::AccountId::decode(&mut &customer_id[..]).unwrap(); // todo! Remove Unwrap
=======
								let binding = activity.clone();
								let customer_id = binding.customer_id.as_str();
								let key = customer_id.trim_start_matches("0x");
								let h_key = hex::decode(key).unwrap();
								let account_id = T::AccountId::decode(&mut &h_key[..]).unwrap(); // todo! Remove Unwrap
>>>>>>> 396e017d (Fixing customer id encoding (#407))
=======
								let account_id =
									T::CustomerVisitor::get_bucket_owner(&activity.bucket_id)
										.unwrap();
<<<<<<< HEAD
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
								let node_id = activity.node_id.clone();
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
								let node_key = Self::node_key_from_hex(activity.node_id.clone())
									.expect("Node Public Key to be decoded");
								let bucket_id = activity.bucket_id;
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
								let customer_usage = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
									transferred_bytes: activity.transferred_bytes,
									stored_bytes: activity.stored_bytes,
									number_of_puts: activity.number_of_puts,
									number_of_gets: activity.number_of_gets,
								};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
								(node_key, bucket_id, customer_usage)
=======
								(account_id, activity.bucket_id, customer_usage)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
								(account_id, node_id, activity.bucket_id, customer_usage)
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
								(node_key, bucket_id, customer_usage)
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
							.map(|payable_usage| {
								let bucket_id = payable_usage.0;
								let customer_usage = BucketUsage {
									transferred_bytes: payable_usage.1.transferred_bytes,
									stored_bytes: payable_usage.1.stored_bytes,
									number_of_puts: payable_usage.1.number_of_puts,
									number_of_gets: payable_usage.1.number_of_gets,
								};
								(bucket_id, customer_usage)
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
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
		) -> Result<Option<DdcEra>, Vec<OCWError>> {
			if let Some(era) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::ChargingCustomers &&
					T::PayoutProcessor::all_customer_batches_processed(cluster_id, era_id)
=======
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::ChargingCustomers &&
					T::PayoutVisitor::all_customer_batches_processed(cluster_id, era_id)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::ChargingCustomers &&
					T::PayoutProcessor::all_customer_batches_processed(cluster_id, era_id)
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
=======
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era.id) ==
					PayoutState::ChargingCustomers &&
					T::PayoutProcessor::all_customer_batches_processed(cluster_id, era.id)
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
=======
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era.id) ==
					PayoutState::ChargingCustomers &&
					T::PayoutProcessor::all_customer_batches_processed(cluster_id, era.id)
>>>>>>> c7dd3fb3 (fix: runtime version is increased)
				{
					return Ok(Some(era.id));
				}
			}
			Ok(None)
		}

		pub(crate) fn prepare_begin_rewarding_providers(
			cluster_id: &ClusterId,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
<<<<<<< HEAD
			min_nodes: u16,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			redundancy_factor: u16,
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
=======
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
			batch_size: usize,
=======
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
		) -> Result<Option<(DdcEra, BatchIndex, NodeUsage)>, Vec<OCWError>> {
			if let Some((era_id, start, end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				let nodes_total_usages = Self::get_nodes_total_usage(cluster_id)?;
=======
				let nodes_total_usages = Self::get_nodes_total_usage(cluster_id, dac_nodes)?;
>>>>>>> e0ce0e5b (node integer delta usage (#412))
=======
				let nodes_total_usages = Self::get_nodes_total_usage(cluster_id)?;
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)

				let nodes_total_usage: i64 = nodes_total_usages
					.iter()
					.filter_map(|usage| usage.as_ref().map(|u| u.stored_bytes))
					.sum();

<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::CustomersChargedWithFees
				{
					if let Some((_, _, _, nodes_total_activity, _, nodes_activity_batch_roots)) =
						Self::fetch_validation_activities::<
							aggregator_client::json::BucketSubAggregate,
							aggregator_client::json::NodeAggregate,
						>(cluster_id, era_id)
					{
						Self::fetch_reward_activities(
							cluster_id,
							era_id,
							nodes_total_activity,
							nodes_activity_batch_roots,
							nodes_total_usage,
=======
=======
>>>>>>> e0ce0e5b (node integer delta usage (#412))
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::CustomersChargedWithFees
				{
					if let Some((_, _, _, nodes_total_activity, _, nodes_activity_batch_roots)) =
						Self::fetch_validation_activities::<
							aggregator_client::json::BucketSubAggregate,
							aggregator_client::json::NodeAggregate,
						>(cluster_id, era_id)
					{
						Self::fetch_reward_activities(
							cluster_id,
							era_id,
							nodes_total_activity,
							nodes_activity_batch_roots,
<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
							nodes_total_usage,
>>>>>>> e0ce0e5b (node integer delta usage (#412))
						)
					} else {
						let era_activity = EraActivity { id: era_id, start, end };

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
						let _ = Self::process_dac_era(cluster_id, Some(era_activity), batch_size)?;
=======
						let _ = Self::process_dac_data(
							cluster_id,
							Some(era_activity),
							dac_nodes,
							redundancy_factor,
							batch_size,
						)?;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						let _ = Self::process_dac_data(cluster_id, Some(era_activity), batch_size)?;
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
=======
						let _ = Self::process_dac_era(cluster_id, Some(era_activity), batch_size)?;
>>>>>>> 6a46f4d2 (chore: renaming for eliminating ambiguity)
=======
						let _ = Self::process_dac_era(cluster_id, Some(era_activity))?;
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)

						if let Some((
							_,
							_,
							_,
<<<<<<< HEAD
<<<<<<< HEAD
							nodes_total_activity,
							_,
							nodes_activity_batch_roots,
						)) = Self::fetch_validation_activities::<
							aggregator_client::json::BucketSubAggregate,
							aggregator_client::json::NodeAggregate,
						>(cluster_id, era_id)
						{
<<<<<<< HEAD
							Self::fetch_reward_activities(
								cluster_id,
								era_id,
								nodes_total_activity,
								nodes_activity_batch_roots,
								nodes_total_usage,
=======
							nodes_activity_in_consensus,
=======
							nodes_total_activity,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
							_,
							nodes_activity_batch_roots,
						)) = Self::fetch_validation_activities::<BucketSubAggregate, NodeAggregate>(
							cluster_id, era_id,
						) {
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
							Self::fetch_reward_activities(
								cluster_id,
								era_id,
								nodes_total_activity,
								nodes_activity_batch_roots,
<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
								nodes_total_usage,
>>>>>>> e0ce0e5b (node integer delta usage (#412))
							)
						} else {
							Ok(None)
						}
					}
=======
		) -> Result<Option<(DdcEra, BatchIndex)>, Vec<OCWError>> {
			if let Some(era) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era.id) ==
					PayoutState::CustomersChargedWithFees
				{
					let era_payable_usage = Self::fetch_payable_usage_or_retry(cluster_id, era)?;
					Self::fetch_rewarding_loop_input(
						cluster_id,
						era.id,
						era_payable_usage.payees_batch_roots,
					)
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
				} else {
					Ok(None)
				}
			} else {
				Ok(None)
			}
		}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		pub(crate) fn fetch_reward_activities(
			cluster_id: &ClusterId,
			era_id: DdcEra,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			nodes_total_activity: Vec<aggregator_client::json::NodeAggregate>,
=======
			nodes_total_activity: Vec<NodeAggregate>,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
			nodes_total_activity: Vec<aggregator_client::json::NodeAggregate>,
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
			nodes_activity_batch_roots: Vec<ActivityHash>,
			current_nodes_total_usage: i64,
=======
		fn fetch_reward_activities(
=======
		pub(crate) fn fetch_reward_activities(
>>>>>>> 00eed38c (Changes to accept stored_bytes as signed input (#410))
			cluster_id: &ClusterId,
			era_id: DdcEra,
			nodes_activity_in_consensus: Vec<NodeActivity>,
=======
			nodes_activity_in_consensus: Vec<NodeAggregate>,
>>>>>>> 23acfcfc (chore: activities renamed to aggregates, redundant traits impls are removed)
			nodes_activity_batch_roots: Vec<ActivityHash>,
<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			current_nodes_total_usage: i64,
>>>>>>> e0ce0e5b (node integer delta usage (#412))
		) -> Result<Option<(DdcEra, BatchIndex, NodeUsage)>, Vec<OCWError>> {
			if let Some(max_batch_index) = nodes_activity_batch_roots.len().checked_sub(1)
			// -1 cause payout expects max_index, not length
			{
=======
		pub(crate) fn fetch_rewarding_loop_input(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			payees_batch_roots: Vec<PayableUsageHash>,
		) -> Result<Option<(DdcEra, BatchIndex)>, Vec<OCWError>> {
			if let Some(max_batch_index) = payees_batch_roots.len().checked_sub(1) {
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
				let max_batch_index: u16 = max_batch_index.try_into().map_err(|_| {
					vec![OCWError::BatchIndexConversionFailed { cluster_id: *cluster_id, era_id }]
				})?;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> e0ce0e5b (node integer delta usage (#412))
				let mut total_node_usage = NodeUsage {
					transferred_bytes: 0,
					stored_bytes: current_nodes_total_usage,
					number_of_puts: 0,
					number_of_gets: 0,
				};
<<<<<<< HEAD

				for activity in nodes_total_activity {
					total_node_usage.transferred_bytes += activity.transferred_bytes;
					total_node_usage.stored_bytes += activity.stored_bytes;
					total_node_usage.number_of_puts += activity.number_of_puts;
					total_node_usage.number_of_gets += activity.number_of_gets;
				}
=======
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
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				let total_node_usage = nodes_activity_in_consensus
					.into_iter()
					.try_fold(
						NodeUsage {
							transferred_bytes: 0,
							stored_bytes: 0,
							number_of_puts: 0,
							number_of_gets: 0,
						},
						|mut acc: NodeUsage, activity| {
							let total_stored_bytes = acc.stored_bytes + activity.stored_bytes;

							if total_stored_bytes < 0 {
								Err(OCWError::TotalNodeUsageLessThanZero {
									cluster_id: *cluster_id,
									era_id,
								})
							} else {
								acc.transferred_bytes += activity.transferred_bytes;
								acc.stored_bytes = total_stored_bytes;
								acc.number_of_puts += activity.number_of_puts;
								acc.number_of_gets += activity.number_of_gets;
								Ok(acc)
							}
						},
					)
					.map_err(|e| vec![e])?;
>>>>>>> 00eed38c (Changes to accept stored_bytes as signed input (#410))
=======

				for activity in nodes_total_activity {
					total_node_usage.transferred_bytes += activity.transferred_bytes;
					total_node_usage.stored_bytes += activity.stored_bytes;
					total_node_usage.number_of_puts += activity.number_of_puts;
					total_node_usage.number_of_gets += activity.number_of_gets;
				}
>>>>>>> e0ce0e5b (node integer delta usage (#412))

				Ok(Some((era_id, max_batch_index, total_node_usage)))
=======
				Ok(Some((era_id, max_batch_index)))
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			} else {
				Err(vec![OCWError::EmptyCustomerActivity { cluster_id: *cluster_id, era_id }])
			}
		}

		pub(crate) fn prepare_send_rewarding_providers_batch(
			cluster_id: &ClusterId,
<<<<<<< HEAD
			batch_size: usize,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
		) -> Result<Option<(DdcEra, ProviderBatch)>, Vec<OCWError>> {
			let batch_size = T::MAX_PAYOUT_BATCH_SIZE;

			if let Some(era) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era.id) ==
					PayoutState::RewardingProviders
				{
<<<<<<< HEAD
					if let Some((_, _, _, nodes_total_activity, _, nodes_activity_batch_roots)) =
						Self::fetch_validation_activities::<
							aggregator_client::json::BucketSubAggregate,
							aggregator_client::json::NodeAggregate,
						>(cluster_id, era_id)
					{
=======
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
			redundancy_factor: u16,
=======
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
		) -> Result<Option<(DdcEra, ProviderBatch<T>)>, Vec<OCWError>> {
=======
		) -> Result<Option<(DdcEra, ProviderBatch)>, Vec<OCWError>> {
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
			if let Some((era_id, start, end)) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::RewardingProviders
				{
<<<<<<< HEAD
					if let Some((
						_,
						_,
						_,
						nodes_activity_in_consensus,
						_,
						nodes_activity_batch_roots,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
					)) = Self::fetch_validation_activities::<CustomerActivity, NodeActivity>(
						cluster_id, era_id,
					) {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
					)) = Self::fetch_validation_activities::<
						BucketNodeAggregatesActivity,
						NodeActivity,
					>(cluster_id, era_id)
					{
>>>>>>> 5288a1a7 (Fixing billing activity issue (#431))
=======
					)) = Self::fetch_validation_activities::<BucketActivityPerNode, NodeActivity>(
=======
					)) = Self::fetch_validation_activities::<BucketSubAggregate, NodeAggregate>(
>>>>>>> 23acfcfc (chore: activities renamed to aggregates, redundant traits impls are removed)
						cluster_id, era_id,
					) {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
=======
					if let Some((_, _, _, nodes_total_activity, _, nodes_activity_batch_roots)) =
<<<<<<< HEAD
						Self::fetch_validation_activities::<BucketSubAggregate, NodeAggregate>(
							cluster_id, era_id,
						) {
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
						Self::fetch_validation_activities::<
							aggregator_client::json::BucketSubAggregate,
							aggregator_client::json::NodeAggregate,
						>(cluster_id, era_id)
					{
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
						Self::fetch_reward_provider_batch(
							cluster_id,
							batch_size.into(),
							era_id,
<<<<<<< HEAD
<<<<<<< HEAD
							nodes_total_activity,
=======
							nodes_activity_in_consensus,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
							nodes_total_activity,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
							nodes_activity_batch_roots,
						)
					} else {
						let era_activity = EraActivity { id: era_id, start, end };

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
						let _ = Self::process_dac_era(cluster_id, Some(era_activity), batch_size)?;
=======
						let _ = Self::process_dac_data(
							cluster_id,
							Some(era_activity),
							dac_nodes,
							redundancy_factor,
							batch_size,
						)?;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						let _ = Self::process_dac_data(cluster_id, Some(era_activity), batch_size)?;
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
=======
						let _ = Self::process_dac_era(cluster_id, Some(era_activity), batch_size)?;
>>>>>>> 6a46f4d2 (chore: renaming for eliminating ambiguity)
=======
						let _ = Self::process_dac_era(cluster_id, Some(era_activity))?;
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)

						if let Some((
							_,
							_,
							_,
<<<<<<< HEAD
<<<<<<< HEAD
							nodes_total_activity,
							_,
							nodes_activity_batch_roots,
						)) = Self::fetch_validation_activities::<
							aggregator_client::json::BucketSubAggregate,
							aggregator_client::json::NodeAggregate,
						>(cluster_id, era_id)
						{
<<<<<<< HEAD
=======
							nodes_activity_in_consensus,
=======
							nodes_total_activity,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
							_,
							nodes_activity_batch_roots,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
						)) = Self::fetch_validation_activities::<CustomerActivity, NodeActivity>(
							cluster_id, era_id,
						) {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						)) = Self::fetch_validation_activities::<
							BucketNodeAggregatesActivity,
							NodeActivity,
						>(cluster_id, era_id)
						{
>>>>>>> 5288a1a7 (Fixing billing activity issue (#431))
=======
						)) = Self::fetch_validation_activities::<BucketActivityPerNode, NodeActivity>(
=======
						)) = Self::fetch_validation_activities::<BucketSubAggregate, NodeAggregate>(
>>>>>>> 23acfcfc (chore: activities renamed to aggregates, redundant traits impls are removed)
							cluster_id, era_id,
						) {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
							Self::fetch_reward_provider_batch(
								cluster_id,
								batch_size.into(),
								era_id,
<<<<<<< HEAD
<<<<<<< HEAD
								nodes_total_activity,
=======
								nodes_activity_in_consensus,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
								nodes_total_activity,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
								nodes_activity_batch_roots,
							)
						} else {
							Ok(None)
						}
					}
=======
					let era_payable_usage = Self::fetch_payable_usage_or_retry(cluster_id, era)?;
					Self::fetch_rewarding_providers_batch(
						cluster_id,
						batch_size.into(),
						era.id,
						era_payable_usage.payees_usage,
						era_payable_usage.payees_batch_roots,
					)
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
				} else {
					Ok(None)
				}
			} else {
				Ok(None)
			}
		}

		fn fetch_rewarding_providers_batch(
			cluster_id: &ClusterId,
			batch_size: usize,
			era_id: DdcEra,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			nodes_total_activity: Vec<aggregator_client::json::NodeAggregate>,
			nodes_activity_batch_roots: Vec<ActivityHash>,
=======
			payees_usage: Vec<NodePayableUsage>,
			payees_batch_roots: Vec<PayableUsageHash>,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		) -> Result<Option<(DdcEra, ProviderBatch)>, Vec<OCWError>> {
			let batch_index =
				T::PayoutProcessor::get_next_provider_batch_for_payment(cluster_id, era_id)
					.map_err(|_| {
						vec![OCWError::BillingReportDoesNotExist {
							cluster_id: *cluster_id,
							era_id,
						}]
					})?;
<<<<<<< HEAD
=======
			nodes_activity_in_consensus: Vec<NodeActivity>,
=======
			nodes_activity_in_consensus: Vec<NodeAggregate>,
>>>>>>> 23acfcfc (chore: activities renamed to aggregates, redundant traits impls are removed)
=======
			nodes_total_activity: Vec<NodeAggregate>,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
			nodes_total_activity: Vec<aggregator_client::json::NodeAggregate>,
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
			nodes_activity_batch_roots: Vec<ActivityHash>,
		) -> Result<Option<(DdcEra, ProviderBatch)>, Vec<OCWError>> {
			let batch_index = T::PayoutVisitor::get_next_provider_batch_for_payment(
				cluster_id, era_id,
			)
			.map_err(|_| {
				vec![OCWError::BillingReportDoesNotExist { cluster_id: *cluster_id, era_id }]
			})?;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

			if let Some(index) = batch_index {
				let i: usize = index.into();
				// todo! store batched activity to avoid splitting it again each time
<<<<<<< HEAD
				let nodes_activity_batched =
<<<<<<< HEAD
<<<<<<< HEAD
					Self::split_to_batches(&nodes_total_activity, batch_size);
=======
					Self::split_to_batches(&nodes_activity_in_consensus, batch_size);
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
					Self::split_to_batches(&nodes_total_activity, batch_size);
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
				let nodes_activity_batched = Self::split_to_batches(&payees_usage, batch_size);
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)

				let batch_root = payees_batch_roots[i];
				let store = MemStore::default();
				let mut mmr: MMR<DeltaUsageHash, MergeMMRHash, &MemStore<DeltaUsageHash>> =
					MemMMR::<_, MergeMMRHash>::new(0, &store);

				let leaf_position_map: Vec<(DeltaUsageHash, u64)> =
					payees_batch_roots.iter().map(|a| (*a, mmr.push(*a).unwrap())).collect();

				let leaf_position: Vec<(u64, DeltaUsageHash)> = leaf_position_map
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

<<<<<<< HEAD
<<<<<<< HEAD
				let batch_proof = MMRProof { proof };
=======
				// todo! attend [i] through get(i).ok_or()
				// todo! attend accountid conversion
<<<<<<< HEAD
				let batch_proof = MMRProof {
					mmr_size: mmr.mmr_size(),
					proof,
					leaf_with_position: leaf_position[0],
				};
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				let batch_proof = MMRProof { mmr_size: mmr.mmr_size(), proof };
>>>>>>> 29b2c1bf (chore: hash and position of the leaf is removed from tx parameters)
=======
				let batch_proof = MMRProof { proof };
>>>>>>> 0c3d2872 (chore: calculating mmr size based on max batch index)
				Ok(Some((
					era_id,
					ProviderBatch {
						batch_index: index,
						payees: nodes_activity_batched[i]
							.iter()
<<<<<<< HEAD
							.map(|activity| {
<<<<<<< HEAD
<<<<<<< HEAD
								let node_key = Self::node_key_from_hex(activity.node_id.clone())
									.expect("Node Public Key to be decoded");
=======
								let node_id = activity.clone().node_id;
<<<<<<< HEAD
								let provider_id = Self::fetch_provider_id(node_id).unwrap(); // todo! remove unwrap
<<<<<<< HEAD

>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 99095ecd (verified copy of PR#393 (#402))
=======
								let provider_id = Self::fetch_provider_id(node_id.clone()).unwrap(); // todo! remove unwrap
>>>>>>> acbb6a47 (fix: checking MMR proof for a batch)
=======
								let node_key = Self::node_key_from_hex(activity.node_id.clone())
									.expect("Node Public Key to be decoded");
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
								let node_usage = NodeUsage {
									transferred_bytes: activity.transferred_bytes,
									stored_bytes: activity.stored_bytes,
									number_of_puts: activity.number_of_puts,
									number_of_gets: activity.number_of_gets,
								};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
								(node_key, node_usage)
=======
								(provider_id, node_usage)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
								(provider_id, node_id, node_usage)
>>>>>>> acbb6a47 (fix: checking MMR proof for a batch)
=======
								(node_key, node_usage)
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
=======
							.map(|payable_usage| {
								let node_key = payable_usage.0.clone();
								let provider_usage = NodeUsage {
									transferred_bytes: payable_usage.1.transferred_bytes,
									stored_bytes: payable_usage.1.stored_bytes,
									number_of_puts: payable_usage.1.number_of_puts,
									number_of_gets: payable_usage.1.number_of_gets,
								};
								(node_key, provider_usage)
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
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
		) -> Result<Option<DdcEra>, Vec<OCWError>> {
			if let Some(era) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::RewardingProviders &&
					T::PayoutProcessor::all_provider_batches_processed(cluster_id, era_id)
=======
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::RewardingProviders &&
					T::PayoutVisitor::all_provider_batches_processed(cluster_id, era_id)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era_id) ==
					PayoutState::RewardingProviders &&
					T::PayoutProcessor::all_provider_batches_processed(cluster_id, era_id)
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
=======
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era.id) ==
					PayoutState::RewardingProviders &&
					T::PayoutProcessor::all_provider_batches_processed(cluster_id, era.id)
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
=======
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era.id) ==
					PayoutState::RewardingProviders &&
					T::PayoutProcessor::all_provider_batches_processed(cluster_id, era.id)
>>>>>>> c7dd3fb3 (fix: runtime version is increased)
				{
					return Ok(Some(era.id));
				}
			}
			Ok(None)
		}

		pub(crate) fn prepare_end_billing_report(
			cluster_id: &ClusterId,
		) -> Result<Option<DdcEra>, Vec<OCWError>> {
			if let Some(era) =
				Self::get_era_for_payout(cluster_id, EraValidationStatus::PayoutInProgress)
			{
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era_id) ==
=======
				if T::PayoutVisitor::get_billing_report_status(cluster_id, era_id) ==
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era_id) ==
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
=======
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era.id) ==
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
=======
				if T::PayoutProcessor::get_billing_report_status(cluster_id, era.id) ==
>>>>>>> c7dd3fb3 (fix: runtime version is increased)
					PayoutState::ProvidersRewarded
				{
					return Ok(Some(era.id));
				}
			}
			Ok(None)
		}

		pub(crate) fn derive_delta_usage_key(cluster_id: &ClusterId, era_id: DdcEra) -> Vec<u8> {
			format!("offchain::activities::{:?}::{:?}", cluster_id, era_id).into_bytes()
		}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)
		pub(crate) fn collect_verification_pub_key() -> Result<T::Public, OCWError> {
=======
=======
		pub(crate) fn derive_paybale_usage_key(cluster_id: &ClusterId, era_id: DdcEra) -> Vec<u8> {
			format!("offchain::paybale_usage::{:?}::{:?}", cluster_id, era_id).into_bytes()
		}

>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		pub(crate) fn collect_verification_pub_key() -> Result<Account<T>, OCWError> {
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
			let session_verification_keys = <T::OffchainIdentifierId as AppCrypto<
				T::Public,
				T::Signature,
			>>::RuntimeAppPublic::all()
			.into_iter()
			.enumerate()
			.filter_map(|(i, key)| {
				let generic_public = <T::OffchainIdentifierId as AppCrypto<
					T::Public,
					T::Signature,
				>>::GenericPublic::from(key);
				let public_key: T::Public = generic_public.into();
				let account_id = public_key.clone().into_account();

				if <ValidatorSet<T>>::get().contains(&account_id) {
					let account = Account::new(i, account_id, public_key);
					Option::Some(account)
				} else {
					Option::None
				}
			})
			.collect::<Vec<_>>();
<<<<<<< HEAD
=======
		pub(crate) fn map_validator_verification_key() -> Result<T::AccountId, OCWError> {
			let verification_keys = sr25519_public_keys(DAC_VERIFICATION_KEY_TYPE)
				.into_iter()
				.map(|pub_key| {
					let key: [u8; 32] = pub_key.into();
					T::AccountId::decode(&mut &key[..]).expect("Verification key to be decoded")
				})
				.collect::<Vec<_>>();

			let session_verification_keys = verification_keys
				.into_iter()
				.filter(|account_id| <ValidatorSet<T>>::get().contains(account_id))
				.collect::<Vec<_>>();
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
=======
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)

			if session_verification_keys.len() != 1 {
				log::error!(
					"🚨 Unexpected number of session verification keys is found. Expected: 1, Actual: {:?}",
					session_verification_keys.len()
				);
<<<<<<< HEAD
<<<<<<< HEAD
				return Err(OCWError::FailedToCollectVerificationKey);
=======
				return Err(OCWError::FailedToRetrieveVerificationKey);
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
=======
				return Err(OCWError::FailedToCollectVerificationKey);
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)
			}

			session_verification_keys
				.into_iter()
				.next() // first
<<<<<<< HEAD
<<<<<<< HEAD
				.ok_or(OCWError::FailedToCollectVerificationKey)
		}

		pub(crate) fn store_verification_account_id(account_id: T::AccountId) {
			let validator: Vec<u8> = account_id.encode();
			let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();
			local_storage_set(StorageKind::PERSISTENT, &key, &validator);
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> a58b0e83 (Import offchain storage fns)
		}

		pub(crate) fn fetch_verification_account_id() -> Result<T::AccountId, OCWError> {
			let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();

			match local_storage_get(StorageKind::PERSISTENT, &key) {
				Some(data) => {
					let account_id = T::AccountId::decode(&mut &data[..])
						.map_err(|_| OCWError::FailedToFetchVerificationKey)?;
					Ok(account_id)
				},
				None => Err(OCWError::FailedToFetchVerificationKey),
=======
		pub(crate) fn store_current_validator(validator: Vec<u8>) {
			let key = format!("offchain::validator::{:?}", KEY_TYPE).into_bytes();
=======
				.ok_or(OCWError::FailedToRetrieveVerificationKey)
=======
				.ok_or(OCWError::FailedToCollectVerificationKey)
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)
		}

		pub(crate) fn store_verification_account_id(account_id: T::AccountId) {
			let validator: Vec<u8> = account_id.encode();
			let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
			sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, &key, &validator);
=======
>>>>>>> 14793e95 (Import offchain storage functions)
		}

		pub(crate) fn fetch_verification_account_id() -> Result<T::AccountId, OCWError> {
			let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();

<<<<<<< HEAD
			match sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
<<<<<<< HEAD
				Some(data) => Ok(data),
				None => Err(OCWError::FailedToFetchCurrentValidator),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
=======
			match local_storage_get(StorageKind::PERSISTENT, &key) {
>>>>>>> 14793e95 (Import offchain storage functions)
				Some(data) => {
					let account_id = T::AccountId::decode(&mut &data[..])
						.map_err(|_| OCWError::FailedToFetchVerificationKey)?;
					Ok(account_id)
				},
<<<<<<< HEAD
				None => Err(OCWError::FailedToRetrieveVerificationKey),
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
=======
				None => Err(OCWError::FailedToFetchVerificationKey),
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)
			}
		}

		#[allow(clippy::too_many_arguments)]
		pub(crate) fn store_verified_delta_usage<A: Encode, B: Encode>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			customers_total_activity: &[A],
			customers_activity_root: ActivityHash,
			customers_activity_batch_roots: &[ActivityHash],
			nodes_total_activity: &[B],
=======
			customers_activity_in_consensus: &[A],
=======
			bucket_nodes_activity_in_consensus: &[A],
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
			customers_activity_root: ActivityHash,
			customers_activity_batch_roots: &[ActivityHash],
			nodes_activity_in_consensus: &[B],
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			customers_total_activity: &[A],
			customers_activity_root: ActivityHash,
			customers_activity_batch_roots: &[ActivityHash],
			nodes_total_activity: &[B],
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
			nodes_activity_root: ActivityHash,
			nodes_activity_batch_roots: &[ActivityHash],
=======
			buckets_deltas: &[A],
			buckets_deltas_root: DeltaUsageHash,
			buckets_deltas_batch_roots: &[DeltaUsageHash],
			nodes_deltas: &[B],
			nodes_deltas_root: DeltaUsageHash,
			nodes_deltas_batch_roots: &[DeltaUsageHash],
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		) {
			let key = Self::derive_delta_usage_key(cluster_id, era_id);
			let encoded_tuple = (
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				customers_total_activity,
				customers_activity_root,
				customers_activity_batch_roots,
				nodes_total_activity,
=======
				customers_activity_in_consensus,
=======
				bucket_nodes_activity_in_consensus,
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
				customers_activity_root,
				customers_activity_batch_roots,
				nodes_activity_in_consensus,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customers_total_activity,
				customers_activity_root,
				customers_activity_batch_roots,
				nodes_total_activity,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
				nodes_activity_root,
				nodes_activity_batch_roots,
=======
				buckets_deltas,
				buckets_deltas_root,
				buckets_deltas_batch_roots,
				nodes_deltas,
				nodes_deltas_root,
				nodes_deltas_batch_roots,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			)
				.encode();

			// Store the serialized data in local offchain storage
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			local_storage_set(StorageKind::PERSISTENT, &key, &encoded_tuple);
<<<<<<< HEAD
		}

		pub(crate) fn get_nodes_total_usage(
			cluster_id: &ClusterId,
		) -> Result<Vec<Option<NodeUsage>>, Vec<OCWError>> {
			let mut results: Vec<Option<NodeUsage>> = Vec::new();
			let mut errors: Vec<OCWError> = Vec::new();

			let nodes = match T::ClusterManager::get_nodes(cluster_id) {
				Ok(nodes_pub_keys) => nodes_pub_keys,
				Err(_) => {
					errors.push(OCWError::FailedToFetchClusterNodes);
					return Err(errors);
				},
			};

			for node_pub_key in nodes.iter() {
				match T::NodeManager::get_total_usage(node_pub_key) {
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
=======
			sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, &key, &encoded_tuple);
<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
=======
			local_storage_set(StorageKind::PERSISTENT, &key, &encoded_tuple);
>>>>>>> 14793e95 (Import offchain storage functions)

			let maybe_existing_activities_keys =
				local_storage_get(StorageKind::PERSISTENT, VALIDATION_ACTIVITIES_KEY);

			let existing_activities_keys = match maybe_existing_activities_keys {
				Some(v) => v,
				None => Vec::new(),
			};

			let new_activity_keys = if !existing_activities_keys.is_empty() {
				itertools::concat(vec![
					existing_activities_keys,
					vec![VALIDATION_ACTIVITIES_KEYS_SEP],
					key,
				])
			} else {
				key
			};

			local_storage_set(
				StorageKind::PERSISTENT,
				VALIDATION_ACTIVITIES_KEY,
				&new_activity_keys,
			);
<<<<<<< HEAD
>>>>>>> 2c0c7a8a (Clear validation activities cache when OCW is done)
=======
			log::debug!(
				"Activity keys extended, now {:?}",
				from_utf8(&new_activity_keys).unwrap_or("parsing failed"),
			);
>>>>>>> 00ee6d9c (Log OCW activity-related keys writes)
=======
			local_storage_set(StorageKind::PERSISTENT, &key, &encoded_tuple);
>>>>>>> a58b0e83 (Import offchain storage fns)
=======
>>>>>>> 7c239abb (Clear OCW cache for a specific cluster and era)
		}

		#[allow(clippy::type_complexity)]
		pub(crate) fn fetch_verified_delta_usage(
			cluster_id: &ClusterId,
			era_id: DdcEra,
		) -> Option<(
			Vec<BucketDeltaUsage>,
			DeltaUsageHash,
			Vec<DeltaUsageHash>,
			Vec<NodeDeltaUsage>,
			DeltaUsageHash,
			Vec<DeltaUsageHash>,
		)> {
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
			log::info!(
				"👁️‍🗨️🏠 Off-chain cache hit for Verified Delta in cluster_id: {:?} era_id: {:?}",
				cluster_id,
				era_id
			);
<<<<<<< HEAD
<<<<<<< HEAD
			let key = Self::derive_key(cluster_id, era_id);
=======
			let key = Self::derive_delta_usage_key(cluster_id, era_id);
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)

			// Retrieve encoded tuple from local storage
			let encoded_tuple = match local_storage_get(StorageKind::PERSISTENT, &key) {
				Some(data) => data,
				None => return None,
			};
<<<<<<< HEAD
=======
=======
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
			let key = Self::derive_key(cluster_id, era_id);

			// Retrieve encoded tuple from local storage
<<<<<<< HEAD
			let encoded_tuple =
				match sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
					Some(data) => data,
					None => return None,
				};
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			let encoded_tuple = match local_storage_get(StorageKind::PERSISTENT, &key) {
				Some(data) => data,
				None => return None,
			};
>>>>>>> 14793e95 (Import offchain storage functions)
=======
>>>>>>> a58b0e83 (Import offchain storage fns)

			// Attempt to decode tuple from bytes
			match Decode::decode(&mut &encoded_tuple[..]) {
				Ok((
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
					customers_total_activity,
					customers_activity_root,
					customers_activity_batch_roots,
					nodes_total_activity,
					nodes_activity_root,
					nodes_activity_batch_roots,
				)) => Some((
					customers_total_activity,
					customers_activity_root,
					customers_activity_batch_roots,
					nodes_total_activity,
=======
					customers_activity_in_consensus,
=======
					bucket_nodes_activity_in_consensus,
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
					customers_total_activity,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
					customers_activity_root,
					customers_activity_batch_roots,
					nodes_total_activity,
					nodes_activity_root,
					nodes_activity_batch_roots,
				)) => Some((
					customers_total_activity,
					customers_activity_root,
					customers_activity_batch_roots,
<<<<<<< HEAD
					nodes_activity_in_consensus,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
					nodes_total_activity,
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
					nodes_activity_root,
					nodes_activity_batch_roots,
=======
					buckets_deltas,
					buckets_deltas_root,
					buckets_deltas_batch_roots,
					nodes_deltas,
					nodes_deltas_root,
					nodes_deltas_batch_roots,
				)) => Some((
					buckets_deltas,
					buckets_deltas_root,
					buckets_deltas_batch_roots,
					nodes_deltas,
					nodes_deltas_root,
					nodes_deltas_batch_roots,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
				)),
				Err(err) => {
					// Print error message with details of the decoding error
					log::error!("Decoding error: {:?}", err);
					None
				},
			}
		}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
		#[allow(clippy::type_complexity)]
=======
>>>>>>> a7221a8d (Remove redundant clippy lint silencing)
		pub(crate) fn clear_validation_activities() {
			log::debug!("Clearing validation_activities");

			let maybe_validation_activities_keys =
				local_storage_get(StorageKind::PERSISTENT, VALIDATION_ACTIVITIES_KEY);

			let validation_activities_keys = match maybe_validation_activities_keys {
				Some(activities_keys) => activities_keys,
				None => return,
			};

			let validation_activities_keys_separated = validation_activities_keys
				.split(|&x| x == VALIDATION_ACTIVITIES_KEYS_SEP)
				.collect::<Vec<_>>();

			for key in validation_activities_keys_separated {
				local_storage_clear(StorageKind::PERSISTENT, key);
				log::debug!(
					"Clearing validation_activities, key {:?}",
					from_utf8(key).unwrap_or("parsing failed")
				);
			}

			local_storage_clear(StorageKind::PERSISTENT, VALIDATION_ACTIVITIES_KEY);
=======
		pub(crate) fn clear_validation_activities(cluster_id: &ClusterId, era_id: DdcEra) {
			let key = Self::derive_key(cluster_id, era_id);
>>>>>>> 7c239abb (Clear OCW cache for a specific cluster and era)
=======
		#[allow(clippy::too_many_arguments)]
		pub(crate) fn store_payable_usage(
			cluster_id: &ClusterId,
			era: EraActivity,
			payers_usage: Vec<BucketPayableUsage>,
			payers_root: PayableUsageHash,
			payers_batch_roots: Vec<PayableUsageHash>,
			payees_usage: Vec<NodePayableUsage>,
			payees_root: PayableUsageHash,
			payees_batch_roots: Vec<PayableUsageHash>,
		) {
			let key = Self::derive_paybale_usage_key(cluster_id, era.id);

			let mut cluster_usage = NodeUsage {
				transferred_bytes: 0,
				stored_bytes: 0,
				number_of_puts: 0,
				number_of_gets: 0,
			};

			for usage in payees_usage.clone() {
				cluster_usage.transferred_bytes += usage.1.transferred_bytes;
				cluster_usage.stored_bytes += usage.1.stored_bytes;
				cluster_usage.number_of_puts += usage.1.number_of_puts;
				cluster_usage.number_of_gets += usage.1.number_of_gets;
			}
			let era_paybale_usage = PayableEraUsage {
				cluster_id: *cluster_id,
				era,
				payers_usage,
				payers_root,
				payers_batch_roots,
				payees_usage,
				payees_root,
				payees_batch_roots,
				cluster_usage,
			};
			let encoded_era_paybale_usage = era_paybale_usage.encode();

			// Store the serialized data in local offchain storage
			local_storage_set(StorageKind::PERSISTENT, &key, &encoded_era_paybale_usage);
		}

		#[allow(clippy::type_complexity)]
		pub(crate) fn fetch_payable_usage(
			cluster_id: &ClusterId,
			era_id: DdcEra,
		) -> Option<PayableEraUsage> {
			log::info!(
				"🪙🏠 Off-chain cache hit for Payable Usage in cluster_id: {:?} era_id: {:?}",
				cluster_id,
				era_id
			);
			let key = Self::derive_paybale_usage_key(cluster_id, era_id);

			let encoded_era_paybale_usage = match local_storage_get(StorageKind::PERSISTENT, &key) {
				Some(encoded_data) => encoded_data,
				None => return None,
			};

			match Decode::decode(&mut &encoded_era_paybale_usage[..]) {
				Ok(era_paybale_usage) => Some(era_paybale_usage),
				Err(err) => {
					log::error!("Decoding error: {:?}", err);
					None
				},
			}
		}

		pub(crate) fn clear_verified_delta_usage(cluster_id: &ClusterId, era_id: DdcEra) {
			let key = Self::derive_delta_usage_key(cluster_id, era_id);
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			log::debug!(
				"Clearing validation activities for cluster {:?} at era {:?}, key {:?}",
				cluster_id,
				era_id,
				key,
			);

			local_storage_clear(StorageKind::PERSISTENT, &key);
		}

>>>>>>> 2c0c7a8a (Clear validation activities cache when OCW is done)
		pub(crate) fn _store_and_fetch_nonce(node_id: String) -> u64 {
			let key = format!("offchain::activities::nonce::{:?}", node_id).into_bytes();
			let encoded_nonce =
				local_storage_get(StorageKind::PERSISTENT, &key).unwrap_or_else(|| 0.encode());
<<<<<<< HEAD
<<<<<<< HEAD
=======
		pub(crate) fn store_and_fetch_nonce(node_id: String) -> u64 {
=======
		pub(crate) fn _store_and_fetch_nonce(node_id: String) -> u64 {
>>>>>>> ff2caa48 (chore: aggregate challenging is disabled for now)
			let key = format!("offchain::activities::nonce::{:?}", node_id).into_bytes();
			let encoded_nonce = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key)
				.unwrap_or_else(|| 0.encode());
>>>>>>> 18369b0b (Challenge sub trees and make them ready for payout (#434))
=======
>>>>>>> 14793e95 (Import offchain storage functions)
=======
>>>>>>> a58b0e83 (Import offchain storage fns)

			let nonce_data = match Decode::decode(&mut &encoded_nonce[..]) {
				Ok(nonce) => nonce,
				Err(err) => {
					log::error!("Decoding error while fetching nonce: {:?}", err);
					0
				},
			};

			let new_nonce = nonce_data + 1;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 14793e95 (Import offchain storage functions)
=======
>>>>>>> a58b0e83 (Import offchain storage fns)
			local_storage_set(StorageKind::PERSISTENT, &key, &new_nonce.encode());
			nonce_data
		}

<<<<<<< HEAD
=======
=======
			sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, &key, &new_nonce.encode());
			nonce_data
		}
<<<<<<< HEAD
>>>>>>> 18369b0b (Challenge sub trees and make them ready for payout (#434))
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
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======

>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		/// Converts a vector of activity batches into their corresponding Merkle roots.
=======
		/// Converts a vector of hashable batches into their corresponding Merkle roots.
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		///
		/// This function takes a vector of hashable batches, where each batch is a vector of
		/// hashable items. It computes the Merkle root for each batch by first hashing each
		/// activity and then combining these hashes into a single Merkle root.
		///
		/// # Input Parameters
		/// - `batches: Vec<Vec<A>>`: A vector of vectors, where each inner vector represents a
		///   batch of hashable items..
		///
		/// # Output
<<<<<<< HEAD
		/// - `Vec<ActivityHash>`: A vector of Merkle roots, one for each batch of activities.
<<<<<<< HEAD
<<<<<<< HEAD
		pub(crate) fn convert_to_batch_merkle_roots<A: Aggregate>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			batches: Vec<Vec<A>>,
<<<<<<< HEAD
		) -> Result<Vec<ActivityHash>, OCWError> {
=======
		/// - `Vec<H256>`: A vector of Merkle roots, one for each batch of items.
		pub(crate) fn convert_to_batch_merkle_roots<A: Hashable>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			batches: Vec<Vec<A>>,
		) -> Result<Vec<H256>, OCWError> {
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			batches
				.into_iter()
				.map(|batch| {
					let activity_hashes: Vec<H256> =
						batch.into_iter().map(|a| a.hash::<T>()).collect();
=======
		pub(crate) fn convert_to_batch_merkle_roots<A: Activity>(
=======
		pub(crate) fn convert_to_batch_merkle_roots<A: Aggregate>(
>>>>>>> f34e347c (feat: grouping aggregates by consistent categories)
			cluster_id: &ClusterId,
			era_id: DdcEra,
			activities: Vec<Vec<A>>,
=======
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
		) -> Result<Vec<ActivityHash>, OCWError> {
			batches
				.into_iter()
				.map(|batch| {
					let activity_hashes: Vec<ActivityHash> =
<<<<<<< HEAD
						inner_vec.into_iter().map(|a| a.hash::<T>()).collect();
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						batch.into_iter().map(|a| a.hash::<T>()).collect();
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
					Self::create_merkle_root(cluster_id, era_id, &activity_hashes).map_err(|_| {
						OCWError::FailedToCreateMerkleRoot { cluster_id: *cluster_id, era_id }
					})
				})
				.collect::<Result<Vec<H256>, OCWError>>()
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
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		pub(crate) fn split_to_batches<A: Aggregate>(
=======
		pub(crate) fn split_to_batches<A: Activity>(
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		pub(crate) fn split_to_batches<A: Aggregate>(
>>>>>>> f34e347c (feat: grouping aggregates by consistent categories)
=======
		pub(crate) fn split_to_batches<A: Ord + Clone>(
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
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

		/// Creates a Merkle root from a list of hashes.
		///
		/// This function takes a slice of `H256` and constructs a Merkle tree
		/// using an in-memory store. It returns a tuple containing the Merkle root hash,
		/// the size of the Merkle tree, and a vector mapping each input leaf to its position
		/// in the Merkle tree.
		///
		/// # Input Parameters
		///
		/// * `leaves` - A slice of `H256` representing the leaves of the Merkle tree.
		///
		/// # Output
		///
		/// A `Result` containing:
		/// * A tuple with the Merkle root `H256`, the size of the Merkle tree, and a vector mapping
		///   each input leaf to its position in the Merkle tree.
		/// * `OCWError::FailedToCreateMerkleRoot` if there is an error creating the Merkle root.
		pub(crate) fn create_merkle_root(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			leaves: &[H256],
		) -> Result<H256, OCWError> {
			if leaves.is_empty() {
				return Ok(H256::default());
			}

			let store = MemStore::default();
			let mut mmr: MMR<H256, MergeMMRHash, &MemStore<H256>> =
				MemMMR::<_, MergeMMRHash>::new(0, &store);

			let mut leaves_with_position: Vec<(u64, H256)> = Vec::with_capacity(leaves.len());

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

<<<<<<< HEAD
		/// Verify whether leaf is part of tree
		///
		/// Parameters:
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 29b2c1bf (chore: hash and position of the leaf is removed from tx parameters)
		/// - `root_hash`: merkle root hash
		/// - `batch_hash`: hash of the batch
		/// - `batch_index`: index of the batch
		/// - `batch_proof`: MMR proofs
		pub(crate) fn proof_merkle_leaf(
			root_hash: ActivityHash,
			batch_hash: ActivityHash,
			batch_index: BatchIndex,
<<<<<<< HEAD
<<<<<<< HEAD
			max_batch_index: BatchIndex,
			batch_proof: &MMRProof,
		) -> Result<bool, Error<T>> {
			let batch_position = leaf_index_to_pos(batch_index.into());
			let mmr_size = leaf_index_to_mmr_size(max_batch_index.into());
			let proof: MerkleProof<ActivityHash, MergeActivityHash> =
				MerkleProof::new(mmr_size, batch_proof.proof.clone());
			proof
				.verify(root_hash, vec![(batch_position, batch_hash)])
=======
		/// - `root`: merkle root
		/// - `leaf`: Leaf of the tree
		pub(crate) fn _proof_merkle_leaf(
			root: ActivityHash,
=======
>>>>>>> 29b2c1bf (chore: hash and position of the leaf is removed from tx parameters)
=======
			max_batch_index: BatchIndex,
>>>>>>> 0c3d2872 (chore: calculating mmr size based on max batch index)
			batch_proof: &MMRProof,
		) -> Result<bool, Error<T>> {
			let batch_position = leaf_index_to_pos(batch_index.into());
			let mmr_size = leaf_index_to_mmr_size(max_batch_index.into());
			let proof: MerkleProof<ActivityHash, MergeActivityHash> =
				MerkleProof::new(mmr_size, batch_proof.proof.clone());
			proof
<<<<<<< HEAD
				.verify(root, vec![batch_proof.leaf_with_position])
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				.verify(root_hash, vec![(batch_position, batch_hash)])
>>>>>>> 29b2c1bf (chore: hash and position of the leaf is removed from tx parameters)
				.map_err(|_| Error::<T>::FailToVerifyMerkleProof)
		}

		// todo! simplify method by removing start/end from the result
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		pub(crate) fn get_era_for_payout(
			cluster_id: &ClusterId,
			status: EraValidationStatus,
		) -> Option<EraActivity> {
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

			smallest_era_id.map(|era_id| EraActivity { id: era_id, start: start_era, end: end_era })
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
<<<<<<< HEAD
<<<<<<< HEAD
		pub(crate) fn get_last_paid_era(
=======
		pub(crate) fn get_last_validated_era(
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		pub(crate) fn get_last_paid_era(
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
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
<<<<<<< HEAD
<<<<<<< HEAD
			cluster_id: &ClusterId,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Option<EraActivity>, OCWError> {
			let this_validator = Self::fetch_verification_account_id()?;

			let last_validated_era_by_this_validator =
				Self::get_last_paid_era(cluster_id, this_validator)?
					.unwrap_or_else(DdcEra::default);

			let last_paid_era_for_cluster =
				T::ClusterValidator::get_last_paid_era(cluster_id).map_err(|_| {
					OCWError::EraRetrievalError { cluster_id: *cluster_id, node_pub_key: None }
				})?;

			log::info!(
				"ℹ️  The last era validated by this specific validator for cluster_id: {:?} is {:?}. The last paid era for the cluster is {:?}",
				cluster_id,
				last_validated_era_by_this_validator,
				last_paid_era_for_cluster
			);

			// we want to fetch processed eras from all available validators
			let available_processed_eras =
				Self::fetch_processed_era_for_nodes(cluster_id, dac_nodes)?;

			// we want to let the current validator to validate available processed/completed eras
			// that are greater than the last validated era in the cluster
			let processed_eras_to_validate: Vec<EraActivity> = available_processed_eras
				.iter()
				.flat_map(|eras| {
					eras.iter()
						.filter(|&ids| {
							ids.id > last_validated_era_by_this_validator &&
								ids.id > last_paid_era_for_cluster
						})
						.cloned()
				})
				.sorted()
				.collect::<Vec<EraActivity>>();

			// we want to process only eras reported by quorum of validators
			let mut processed_eras_with_quorum: Vec<EraActivity> = vec![];

			let quorum = T::AggregatorsQuorum::get();
			let threshold = quorum * dac_nodes.len();
			for (era_key, candidates) in
				&processed_eras_to_validate.into_iter().chunk_by(|elt| elt.clone())
			{
				let count = candidates.count();
				if count >= threshold {
					processed_eras_with_quorum.push(era_key);
				} else {
					log::warn!(
						"⚠️ Era {:?} in cluster_id: {:?} has been reported with unmet quorum. Desired: {:?} Actual: {:?}",
						era_key,
						cluster_id,
						threshold,
						count
					);
				}
			}

			Ok(processed_eras_with_quorum.iter().cloned().min_by_key(|n| n.id))
=======
			// todo! this needs to be rewriten - too complex and inefficient
=======
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
			cluster_id: &ClusterId,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Option<EraActivity>, OCWError> {
			let this_validator = Self::fetch_verification_account_id()?;

			let last_validated_era_by_this_validator =
				Self::get_last_paid_era(cluster_id, this_validator)?
					.unwrap_or_else(DdcEra::default);

			let last_paid_era_for_cluster =
				T::ClusterValidator::get_last_paid_era(cluster_id).map_err(|_| {
					OCWError::EraRetrievalError { cluster_id: *cluster_id, node_pub_key: None }
				})?;

			log::info!(
				"👁️‍🗨️  The last era validated by this specific validator for cluster_id: {:?} is {:?}. The last paid era for the cluster is {:?}",
				cluster_id,
				last_validated_era_by_this_validator,
				last_paid_era_for_cluster
			);

			// we want to fetch processed eras from all available validators
			let available_processed_eras =
				Self::fetch_processed_era_for_nodes(cluster_id, dac_nodes)?;

			// we want to let the current validator to validate available processed/completed eras
			// that are greater than the last validated era in the cluster
			let processed_eras_to_validate: Vec<EraActivity> = available_processed_eras
				.iter()
				.flat_map(|eras| {
					eras.iter()
						.filter(|&ids| {
							ids.id > last_validated_era_by_this_validator &&
								ids.id > last_paid_era_for_cluster
						})
						.cloned()
				})
				.sorted()
				.collect::<Vec<EraActivity>>();

			// we want to process only eras reported by quorum of validators
			let mut processed_eras_with_quorum: Vec<EraActivity> = vec![];

			let quorum = T::AggregatorsQuorum::get();
			let threshold = quorum * dac_nodes.len();
			for (era_key, candidates) in
				&processed_eras_to_validate.into_iter().chunk_by(|elt| *elt)
			{
				let count = candidates.count();
				if count >= threshold {
					processed_eras_with_quorum.push(era_key);
				} else {
					log::warn!(
						"⚠️ Era {:?} in cluster_id: {:?} has been reported with unmet quorum. Desired: {:?} Actual: {:?}",
						era_key,
						cluster_id,
						threshold,
						count
					);
				}
			}

			Ok(processed_eras_with_quorum.iter().cloned().min_by_key(|n| n.id))
		}

<<<<<<< HEAD
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
		pub(crate) fn _reach_consensus<A: Aggregate>(
			activities: Vec<A>,
			threshold: usize,
		) -> Option<A> {
			let mut count_map: BTreeMap<ActivityHash, Vec<A>> = BTreeMap::new();

			for activity in activities {
				count_map.entry(activity.hash::<T>()).or_default().push(activity);
			}

			count_map
				.into_iter()
<<<<<<< HEAD
				.find(|&(_, count)| count >= threshold)
				.map(|(activity, _)| activity)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				.find(|(_, count)| count.len() >= threshold)
				.map(|(_, same_activities)| same_activities.clone().first().cloned())
				.unwrap_or_else(|| None)
>>>>>>> c54cb6d0 (refactoring)
		}

=======
>>>>>>> 90b6b897 (test: additional tests scenarios are added)
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
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 3ae6cdbd (chore: generic function for consistency classification)
		/// - `nodes_aggregates_by_aggregator: &[(NodePubKey, Vec<A>)]`: A list of tuples, where
		///   each tuple contains a node's public key and a vector of activities reported by that
		///   node.
		/// - `redundancy_factor: u16`: The number of aggregators that should report total activity
<<<<<<< HEAD
		///   for a node or a bucket
		/// - `quorum: Percent`: The threshold percentage that determines if an activity is in
=======
		/// - `activities: &[(NodePubKey, Vec<A>)]`: A list of tuples, where each tuple contains a
		///   node's public key and a vector of activities reported by that node.
		/// - `dac_redundancy_factor: u16`: The number of aggregators that should report an activity
		///   for a node or a bucket
		/// - `threshold: Percent`: The threshold percentage that determines if an activity is in
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		///   for a node or a bucket
		/// - `quorum: Percent`: The threshold percentage that determines if an activity is in
>>>>>>> 3ae6cdbd (chore: generic function for consistency classification)
		///   consensus.
		///
		/// # Output
		/// - `Result<Vec<A>, Vec<OCWError>>`:
		///   - `Ok(Vec<A>)`: A vector of activities that have reached consensus.
		///   - `Err(Vec<OCWError>)`: A vector of errors indicating why consensus was not reached
		///     for some activities.
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		pub(crate) fn group_nodes_aggregates_by_consistency(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			nodes_aggregates_by_aggregator: Vec<(
				AggregatorInfo,
				Vec<aggregator_client::json::NodeAggregateResponse>,
			)>,
<<<<<<< HEAD
			redundancy_factor: u16,
			quorum: Percent,
		) -> ConsistencyGroups<aggregator_client::json::NodeAggregate> {
			let mut nodes_aggregates: Vec<aggregator_client::json::NodeAggregate> = Vec::new();

			log::info!(
				"👁️‍🗨️‍  Starting fetching node aggregates for cluster_id: {:?} for era_id: {:?}",
				cluster_id,
				era_id
			);

			for (aggregator_info, nodes_aggregates_resp) in nodes_aggregates_by_aggregator.clone() {
				for node_aggregate_resp in nodes_aggregates_resp.clone() {
					let node_aggregate = aggregator_client::json::NodeAggregate {
						node_id: node_aggregate_resp.node_id,
						stored_bytes: node_aggregate_resp.stored_bytes,
						transferred_bytes: node_aggregate_resp.transferred_bytes,
						number_of_puts: node_aggregate_resp.number_of_puts,
						number_of_gets: node_aggregate_resp.number_of_gets,
						aggregator: aggregator_info.clone(),
					};
					nodes_aggregates.push(node_aggregate);
				}

				log::info!("👁️‍🗨️‍  Fetched Node-aggregates for cluster_id: {:?} for era_id: {:?} :::Node Aggregates are {:?}", cluster_id, era_id, nodes_aggregates);
			}

			let nodes_aggregates_groups =
				Self::group_by_consistency(nodes_aggregates, redundancy_factor, quorum);

			log::info!("👁️‍🗨️‍🌕 Node Aggregates, which are in consensus for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, nodes_aggregates_groups.consensus);
			log::info!("👁️‍🗨️‍🌗 Node Aggregates, which are in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, nodes_aggregates_groups.quorum);
			log::info!("👁️‍🗨️‍🌘 Node Aggregates, which are neither in consensus nor in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, nodes_aggregates_groups.others);

			nodes_aggregates_groups
		}

		pub(crate) fn group_by_consistency<A>(
			aggregates: Vec<A>,
			redundancy_factor: u16,
			quorum: Percent,
		) -> ConsistencyGroups<A>
		where
			A: Aggregate + Clone,
		{
			let mut consistent_aggregates: BTreeMap<DeltaUsageHash, Vec<A>> = BTreeMap::new();

			for aggregate in aggregates.iter() {
				consistent_aggregates
					.entry(aggregate.hash::<T>())
					.or_default()
					.push(aggregate.clone());
			}

			let mut consensus_group = Vec::new();
			let mut quorum_group = Vec::new();
			let mut others_group = Vec::new();

			let max_aggregates_count = redundancy_factor;
			let quorum_threshold = quorum * max_aggregates_count;

			for (_hash, group) in consistent_aggregates {
				let aggregate = group.first().unwrap();
				let aggregates_count = u16::try_from(group.len()).unwrap_or(u16::MAX);
				let aggregators: Vec<AggregatorInfo> =
					group.clone().into_iter().map(|a| a.get_aggregator()).collect();

				let consolidated_aggregate = ConsolidatedAggregate::<A>::new(
					aggregate.clone(),
					aggregates_count,
					aggregators,
				);

				if aggregates_count == max_aggregates_count {
					consensus_group.push(consolidated_aggregate);
				} else if aggregates_count >= quorum_threshold {
					quorum_group.push(consolidated_aggregate);
				} else {
					others_group.push(consolidated_aggregate);
				}
			}

			ConsistencyGroups {
				consensus: consensus_group,
				quorum: quorum_group,
				others: others_group,
			}
		}

		/// Fetch Challenge node aggregate or bucket sub-aggregate.
		pub(crate) fn _fetch_challenge_responses(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_node_identifiers: Vec<u64>,
			aggregator: AggregatorInfo,
		) -> Result<aggregator_client::json::ChallengeAggregateResponse, OCWError> {
			let response = Self::_fetch_challenge_response(
				era_id,
				aggregate_key.clone(),
				merkle_node_identifiers.clone(),
				&aggregator.node_params,
			)
			.map_err(|_| OCWError::ChallengeResponseRetrievalError {
				cluster_id: *cluster_id,
				era_id,
				aggregate_key,
				aggregator: aggregator.node_pub_key,
			})?;

			Ok(response)
		}

		/// Challenge node aggregate or bucket sub-aggregate.
		pub(crate) fn _fetch_challenge_responses_proto(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_tree_node_id: Vec<u32>,
			aggregator: AggregatorInfo,
		) -> Result<proto::ChallengeResponse, OCWError> {
			let response = Self::_fetch_challenge_response_proto(
				era_id,
				aggregate_key.clone(),
				merkle_tree_node_id.clone(),
				&aggregator.node_params,
			)
			.map_err(|_| OCWError::ChallengeResponseRetrievalError {
				cluster_id: *cluster_id,
				era_id,
				aggregate_key,
				aggregator: aggregator.node_pub_key,
			})?;

			Ok(response)
		}

		/// Fetch challenge response.
		///
		/// Parameters:
		/// - `era_id`: era id
		/// - `aggregate_key`: key of the aggregate to challenge
		/// - `merkle_node_identifiers`: set of merkle node identifiers to challenge
		/// - `node_params`: aggregator node parameters
		pub(crate) fn _fetch_challenge_response(
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_node_identifiers: Vec<u64>,
			node_params: &StorageNodeParams,
		) -> Result<aggregator_client::json::ChallengeAggregateResponse, http::Error> {
			let scheme = "http";
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;

			let ids = merkle_node_identifiers
				.iter()
				.map(|x| format!("{}", x.clone()))
				.collect::<Vec<_>>()
				.join(",");

			let url = match aggregate_key {
                AggregateKey::NodeAggregateKey(node_id) => format!(
                    "{}://{}:{}/activity/nodes/{}/challenge?eraId={}&merkleTreeNodeId={}",
                    scheme, host, node_params.http_port, node_id, era_id, ids
                ),
                AggregateKey::BucketSubAggregateKey(bucket_id, node_id) => format!(
                    "{}://{}:{}/activity/buckets/{}/challenge?eraId={}&nodeId={}&merkleTreeNodeId={}",
                    scheme, host, node_params.http_port, bucket_id, era_id, node_id, ids
                ),
            };

			let request = http::Request::get(&url);
			let timeout = sp_io::offchain::timestamp()
				.add(sp_runtime::offchain::Duration::from_millis(RESPONSE_TIMEOUT));
			let pending = request.deadline(timeout).send().map_err(|_| http::Error::IoError)?;

			let response =
				pending.try_wait(timeout).map_err(|_| http::Error::DeadlineReached)??;
			if response.code != _SUCCESS_CODE {
				return Err(http::Error::Unknown);
			}

			let body = response.body().collect::<Vec<u8>>();
			serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)
		}

		/// Fetch protobuf challenge response.
		pub(crate) fn _fetch_challenge_response_proto(
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_tree_node_id: Vec<u32>,
			node_params: &StorageNodeParams,
		) -> Result<proto::ChallengeResponse, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				3,
				T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
			);

			match aggregate_key {
				AggregateKey::BucketSubAggregateKey(bucket_id, node_id) => client
					.challenge_bucket_sub_aggregate(
						era_id,
						bucket_id,
						&node_id,
						merkle_tree_node_id,
					),
				AggregateKey::NodeAggregateKey(node_id) =>
					client.challenge_node_aggregate(era_id, &node_id, merkle_tree_node_id),
			}
		}

		/// Fetch traverse response.
		///
		/// Parameters:
		/// - `era_id`: era id
		/// - `aggregate_key`: key of the aggregate to challenge
		/// - `merkle_node_identifiers`: set of merkle node identifiers to challenge
		/// - `levels`: a number of levels to raverse
		/// - `node_params`: aggregator node parameters
		pub(crate) fn _fetch_traverse_response(
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_tree_node_id: u32,
			levels: u16,
			node_params: &StorageNodeParams,
		) -> Result<aggregator_client::json::MerkleTreeNodeResponse, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				3,
				T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
			);

			let response = match aggregate_key {
				AggregateKey::BucketSubAggregateKey(bucket_id, node_id) => client
					.traverse_bucket_sub_aggregate(
						era_id,
						bucket_id,
						&node_id,
						merkle_tree_node_id,
						levels,
					),
				AggregateKey::NodeAggregateKey(node_id) =>
					client.traverse_node_aggregate(era_id, &node_id, merkle_tree_node_id, levels),
<<<<<<< HEAD
			}?;

			Ok(response)
=======
		pub(crate) fn get_consensus_for_activities<A: Activity>(
=======
		pub(crate) fn get_consensus_for_activities(
>>>>>>> 8149e899 (Changes for challenging node sub aggregates)
=======
		pub(crate) fn classify_nodes_aggregates_by_consistency(
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
=======
		pub(crate) fn group_nodes_aggregates_by_consistency(
>>>>>>> 0641acde (chore: classification by consistency)
			cluster_id: &ClusterId,
			era_id: DdcEra,
			nodes_aggregates_by_aggregator: Vec<(AggregatorInfo, Vec<NodeAggregateResponse>)>,
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
			redundancy_factor: u16,
			quorum: Percent,
		) -> ConsistencyGroups<aggregator_client::json::NodeAggregate> {
			let mut nodes_aggregates: Vec<aggregator_client::json::NodeAggregate> = Vec::new();

			log::info!(
				"🏠⏳ Starting fetching node aggregates for cluster_id: {:?} for era_id: {:?}",
				cluster_id,
				era_id
			);

			for (aggregator_info, nodes_aggregates_resp) in nodes_aggregates_by_aggregator.clone() {
				for node_aggregate_resp in nodes_aggregates_resp.clone() {
					let node_aggregate = aggregator_client::json::NodeAggregate {
						node_id: node_aggregate_resp.node_id,
						stored_bytes: node_aggregate_resp.stored_bytes,
						transferred_bytes: node_aggregate_resp.transferred_bytes,
						number_of_puts: node_aggregate_resp.number_of_puts,
						number_of_gets: node_aggregate_resp.number_of_gets,
						aggregator: aggregator_info.clone(),
					};
					nodes_aggregates.push(node_aggregate);
				}

				log::info!("🏠🚀 Fetched Node-aggregates for cluster_id: {:?} for era_id: {:?} :::Node Aggregates are {:?}", cluster_id, era_id, nodes_aggregates);
			}

			let nodes_aggregates_groups =
				Self::group_by_consistency(nodes_aggregates, redundancy_factor, quorum);

			log::info!("🏠🌕 Node Aggregates, which are in consensus for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, nodes_aggregates_groups.consensus);
			log::info!("🏠🌗 Node Aggregates, which are in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, nodes_aggregates_groups.quorum);
			log::info!("🏠🌘 Node Aggregates, which are neither in consensus nor in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, nodes_aggregates_groups.others);

			nodes_aggregates_groups
		}

		pub(crate) fn group_by_consistency<A>(
			aggregates: Vec<A>,
			redundancy_factor: u16,
			quorum: Percent,
		) -> ConsistencyGroups<A>
		where
			A: Aggregate + Clone,
		{
			let mut consistent_aggregates: BTreeMap<ActivityHash, Vec<A>> = BTreeMap::new();

			for aggregate in aggregates.iter() {
				consistent_aggregates
					.entry(aggregate.hash::<T>())
					.or_default()
					.push(aggregate.clone());
			}

			let mut consensus_group = Vec::new();
			let mut quorum_group = Vec::new();
			let mut others_group = Vec::new();

			let max_aggregates_count = redundancy_factor;
			let quorum_threshold = quorum * max_aggregates_count;

			for (_hash, group) in consistent_aggregates {
				let aggregate = group.first().unwrap();
				let aggregates_count = u16::try_from(group.len()).unwrap_or(u16::MAX);
				let aggregators: Vec<AggregatorInfo> =
					group.clone().into_iter().map(|a| a.get_aggregator()).collect();

				let consolidated_aggregate = ConsolidatedAggregate::<A>::new(
					aggregate.clone(),
					aggregates_count,
					aggregators,
				);

				if aggregates_count == max_aggregates_count {
					consensus_group.push(consolidated_aggregate);
				} else if aggregates_count >= quorum_threshold {
					quorum_group.push(consolidated_aggregate);
				} else {
					others_group.push(consolidated_aggregate);
				}
			}

			ConsistencyGroups {
				consensus: consensus_group,
				quorum: quorum_group,
				others: others_group,
			}
		}

<<<<<<< HEAD
		/// Fetch cluster to validate.
		fn get_cluster_to_validate() -> Result<ClusterId, Error<T>> {
			// todo! to implement
			Self::cluster_to_validate().ok_or(Error::ClusterToValidateRetrievalError)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
		}

=======
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
		/// Fetch Challenge node aggregate or bucket sub-aggregate.
		pub(crate) fn _fetch_challenge_responses(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_node_identifiers: Vec<u64>,
			aggregator: AggregatorInfo,
		) -> Result<aggregator_client::json::ChallengeAggregateResponse, OCWError> {
			let response = Self::_fetch_challenge_response(
				era_id,
				aggregate_key.clone(),
				merkle_node_identifiers.clone(),
				&aggregator.node_params,
			)
			.map_err(|_| OCWError::ChallengeResponseRetrievalError {
				cluster_id: *cluster_id,
				era_id,
				aggregate_key,
				aggregator: aggregator.node_pub_key,
			})?;

			Ok(response)
		}

		/// Challenge node aggregate or bucket sub-aggregate.
		pub(crate) fn _fetch_challenge_responses_proto(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_tree_node_id: Vec<u32>,
			aggregator: AggregatorInfo,
		) -> Result<proto::ChallengeResponse, OCWError> {
			let response = Self::_fetch_challenge_response_proto(
				era_id,
				aggregate_key.clone(),
				merkle_tree_node_id.clone(),
				&aggregator.node_params,
			)
			.map_err(|_| OCWError::ChallengeResponseRetrievalError {
				cluster_id: *cluster_id,
				era_id,
				aggregate_key,
				aggregator: aggregator.node_pub_key,
			})?;

			Ok(response)
		}

		/// Fetch challenge response.
		///
		/// Parameters:
		/// - `era_id`: era id
		/// - `aggregate_key`: key of the aggregate to challenge
		/// - `merkle_node_identifiers`: set of merkle node identifiers to challenge
		/// - `node_params`: aggregator node parameters
		pub(crate) fn _fetch_challenge_response(
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_node_identifiers: Vec<u64>,
			node_params: &StorageNodeParams,
		) -> Result<aggregator_client::json::ChallengeAggregateResponse, http::Error> {
			let scheme = "http";
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;

			let ids = merkle_node_identifiers
				.iter()
				.map(|x| format!("{}", x.clone()))
				.collect::<Vec<_>>()
				.join(",");

			let url = match aggregate_key {
				AggregateKey::NodeAggregateKey(node_id) => format!(
					"{}://{}:{}/activity/nodes/{}/challenge?eraId={}&merkleTreeNodeId={}",
					scheme, host, node_params.http_port, node_id, era_id, ids
				),
				AggregateKey::BucketSubAggregateKey(bucket_id, node_id) => format!(
					"{}://{}:{}/activity/buckets/{}/challenge?eraId={}&nodeId={}&merkleTreeNodeId={}",
					scheme, host, node_params.http_port, bucket_id, era_id, node_id, ids
				),
			};

			let request = http::Request::get(&url);
			let timeout = sp_io::offchain::timestamp()
				.add(sp_runtime::offchain::Duration::from_millis(RESPONSE_TIMEOUT));
			let pending = request.deadline(timeout).send().map_err(|_| http::Error::IoError)?;

			let response =
				pending.try_wait(timeout).map_err(|_| http::Error::DeadlineReached)??;
			if response.code != _SUCCESS_CODE {
				return Err(http::Error::Unknown);
			}

			let body = response.body().collect::<Vec<u8>>();
			serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)
		}

		/// Fetch protobuf challenge response.
		pub(crate) fn _fetch_challenge_response_proto(
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_tree_node_id: Vec<u32>,
			node_params: &StorageNodeParams,
		) -> Result<proto::ChallengeResponse, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				3,
			);

			match aggregate_key {
				AggregateKey::BucketSubAggregateKey(bucket_id, node_id) => client
					.challenge_bucket_sub_aggregate(
						era_id,
						bucket_id,
						&node_id,
						merkle_tree_node_id,
					),
				AggregateKey::NodeAggregateKey(node_id) =>
					client.challenge_node_aggregate(era_id, &node_id, merkle_tree_node_id),
			}
		}

		/// Fetch traverse response.
		///
		/// Parameters:
		/// - `era_id`: era id
		/// - `aggregate_key`: key of the aggregate to challenge
		/// - `merkle_node_identifiers`: set of merkle node identifiers to challenge
		/// - `levels`: a number of levels to raverse
		/// - `node_params`: aggregator node parameters
		pub(crate) fn _fetch_traverse_response(
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_tree_node_id: u32,
			levels: u16,
			node_params: &StorageNodeParams,
		) -> Result<aggregator_client::json::MerkleTreeNodeResponse, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				3,
			);

			let response = match aggregate_key {
				AggregateKey::BucketSubAggregateKey(bucket_id, node_id) => client
					.traverse_bucket_sub_aggregate(
						era_id,
						bucket_id,
						&node_id,
						merkle_tree_node_id,
						levels,
					),
				AggregateKey::NodeAggregateKey(node_id) =>
					client.traverse_node_aggregate(era_id, &node_id, merkle_tree_node_id, levels),
=======
>>>>>>> c7dd3fb3 (fix: runtime version is increased)
			}?;

			Ok(response)
		}

		/// Fetch processed era.
		///
		/// Parameters:
		/// - `node_params`: DAC node parameters
		#[allow(dead_code)]
<<<<<<< HEAD
<<<<<<< HEAD
		pub(crate) fn fetch_processed_eras(
			node_params: &StorageNodeParams,
		) -> Result<Vec<aggregator_client::json::AggregationEraResponse>, http::Error> {
<<<<<<< HEAD
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				3,
				T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
			);

			let response = client.eras()?;

			Ok(response.into_iter().filter(|e| e.status == "PROCESSED").collect::<Vec<_>>())
=======
		pub(crate) fn fetch_processed_era(
=======
		pub(crate) fn fetch_processed_eras(
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
			node_params: &StorageNodeParams,
		) -> Result<Vec<AggregationEraResponse>, http::Error> {
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				3,
			);

			let response = client.eras()?;

<<<<<<< HEAD
<<<<<<< HEAD
			serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			let processed_status = String::from("PROCESSED");
			Ok(res.into_iter().filter(|e| e.status == processed_status).collect::<Vec<_>>())
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
=======
			Ok(response.into_iter().filter(|e| e.status == "PROCESSED").collect::<Vec<_>>())
>>>>>>> 2ae69e4d (Remove redundant string alloc)
		}
		/// Fetch customer usage.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
<<<<<<< HEAD
<<<<<<< HEAD
		pub(crate) fn fetch_bucket_aggregates(
			_cluster_id: &ClusterId,
			era_id: DdcEra,
			node_params: &StorageNodeParams,
		) -> Result<Vec<aggregator_client::json::BucketAggregateResponse>, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				3,
				T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
			);

			let mut buckets_aggregates = Vec::new();
			let mut prev_token = None;

			loop {
				let response = client.buckets_aggregates(
					era_id,
					Some(BUCKETS_AGGREGATES_FETCH_BATCH_SIZE as u32),
					prev_token,
				)?;

				let response_len = response.len();
				prev_token = response.last().map(|a| a.bucket_id);

				buckets_aggregates.extend(response);

				if response_len < BUCKETS_AGGREGATES_FETCH_BATCH_SIZE {
					break;
				}
			}

			Ok(buckets_aggregates)
=======
		pub(crate) fn fetch_customers_usage(
=======
		pub(crate) fn fetch_bucket_aggregates(
>>>>>>> 73cca90f (tests: additional test scenarious are added)
			_cluster_id: &ClusterId,
			era_id: DdcEra,
			node_params: &StorageNodeParams,
		) -> Result<Vec<BucketAggregateResponse>, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				3,
			);

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
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
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			client.buckets_aggregates(era_id)
>>>>>>> cf535d86 (Use aggregator client to fetch aggregates)
=======
			client.buckets_aggregates(era_id, None, None)
>>>>>>> ded69fb6 (Add pagination for aggregates fetching)
=======
			let mut buckets_aggregates = Vec::new();
			let mut prev_token = None;

			loop {
				let response = client.buckets_aggregates(
					era_id,
					Some(BUCKETS_AGGREGATES_FETCH_BATCH_SIZE as u32),
					prev_token,
				)?;

				let response_len = response.len();
				prev_token = response.last().map(|a| a.bucket_id);

				buckets_aggregates.extend(response);

				if response_len < BUCKETS_AGGREGATES_FETCH_BATCH_SIZE {
					break;
				}
			}

			Ok(buckets_aggregates)
>>>>>>> 991b5815 (Use pagination for aggregates fetching)
		}

		/// Fetch node usage.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
<<<<<<< HEAD
<<<<<<< HEAD
		pub(crate) fn fetch_node_aggregates(
			_cluster_id: &ClusterId,
			era_id: DdcEra,
			node_params: &StorageNodeParams,
		) -> Result<Vec<aggregator_client::json::NodeAggregateResponse>, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				3,
				T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
			);

			let mut nodes_aggregates = Vec::new();
			let mut prev_token = None;

			loop {
				let response = client.nodes_aggregates(
					era_id,
					Some(NODES_AGGREGATES_FETCH_BATCH_SIZE as u32),
					prev_token,
				)?;

				let response_len = response.len();
				prev_token = response.last().map(|a| a.node_id.clone());

				nodes_aggregates.extend(response);

				if response_len < NODES_AGGREGATES_FETCH_BATCH_SIZE {
					break;
				}
			}

			Ok(nodes_aggregates)
=======
		pub(crate) fn fetch_node_usage(
=======
		pub(crate) fn fetch_node_aggregates(
>>>>>>> 73cca90f (tests: additional test scenarious are added)
			_cluster_id: &ClusterId,
			era_id: DdcEra,
			node_params: &StorageNodeParams,
		) -> Result<Vec<NodeAggregateResponse>, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				3,
			);

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
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
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			client.nodes_aggregates(era_id)
>>>>>>> cf535d86 (Use aggregator client to fetch aggregates)
=======
			client.nodes_aggregates(era_id, None, None)
>>>>>>> ded69fb6 (Add pagination for aggregates fetching)
=======
			let mut nodes_aggregates = Vec::new();
			let mut prev_token = None;

			loop {
				let response = client.nodes_aggregates(
					era_id,
					Some(NODES_AGGREGATES_FETCH_BATCH_SIZE as u32),
					prev_token,
				)?;

				let response_len = response.len();
				prev_token = response.last().map(|a| a.node_id.clone());

				nodes_aggregates.extend(response);

				if response_len < NODES_AGGREGATES_FETCH_BATCH_SIZE {
					break;
				}
			}

			Ok(nodes_aggregates)
>>>>>>> 991b5815 (Use pagination for aggregates fetching)
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
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
					T::NodeManager::get_node_params(&node_pub_key)
				{
					log::info!(
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
						"🏭📝 Obtained DAC Node for cluster_id: {:?} and with key: {:?}",
=======
						"🏭 Obtained DAC Node for cluster_id: {:?} and with key: {:?}",
>>>>>>> 2ad19b5e (refactor: allocating validation and payouts phases to individual functions; introducing macro for payout repetitive steps)
						cluster_id,
						node_pub_key.get_hex()
					);

=======
						"🏭📝Get DAC Node for cluster_id: {:?} and node_pub_key: {:?}",
=======
						"🏭📝Get DAC Node for cluster_id: {:?} and node_pub_key: {:#?}",
>>>>>>> d215ac72 (Added debug information in payout (#405))
=======
						"🏭📝Get DAC Node for cluster_id: {:?} and node_pub_key: {:?}",
>>>>>>> 1f5e092b (Fixing Customer Id Ecoding Issue (#406))
=======
						"🏭📝 Obtained DAC Node for cluster_id: {:?} and with key: {:?}",
>>>>>>> 492c0466 (chore: more logging info for tracing)
						cluster_id,
						node_pub_key.get_hex()
					);
<<<<<<< HEAD
>>>>>>> 932271b3 (Add ocw information (#404))
=======

>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
					// Add to the results if the mode matches
					dac_nodes.push((node_pub_key, storage_params));
=======
					T::NodeVisitor::get_node_params(&node_pub_key)
				{
<<<<<<< HEAD
					// Check if the mode is StorageNodeMode::DAC
					if storage_params.mode == StorageNodeMode::DAC {
						// Add to the results if the mode matches
						dac_nodes.push((node_pub_key, storage_params));
					}
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
					// Add to the results if the mode matches
					dac_nodes.push((node_pub_key, storage_params));
>>>>>>> 7d165a87 (Allow all nodes to participate (#399))
				}
			}

			Ok(dac_nodes)
		}

<<<<<<< HEAD
<<<<<<< HEAD
=======
		fn get_node_provider_id(node_pub_key: &NodePubKey) -> Result<T::AccountId, OCWError> {
			T::NodeVisitor::get_node_provider_id(node_pub_key)
				.map_err(|_| OCWError::FailedToFetchNodeProvider)
		}

>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		/// Fetch node usage of an era.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		pub(crate) fn fetch_nodes_aggregates_for_era(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<
			Vec<(AggregatorInfo, Vec<aggregator_client::json::NodeAggregateResponse>)>,
			OCWError,
		> {
<<<<<<< HEAD
			let mut nodes_aggregates = Vec::new();

			for (node_key, node_params) in dac_nodes {
				let aggregates_res = Self::fetch_node_aggregates(cluster_id, era_id, node_params);
				if aggregates_res.is_err() {
					log::warn!(
						"Aggregator from cluster {:?} is unavailable while fetching nodes aggregates. Key: {:?} Host: {:?}",
						cluster_id,
						node_key.get_hex(),
						String::from_utf8(node_params.host.clone())
					);
					// skip unavailable aggregators and continue with available ones
					continue;
				}

				let aggregates = aggregates_res.expect("Nodes Aggregates Response to be available");

				nodes_aggregates.push((
					AggregatorInfo {
						node_pub_key: node_key.clone(),
						node_params: node_params.clone(),
					},
					aggregates,
				));
			}

			Ok(nodes_aggregates)
=======
		fn fetch_nodes_usage_for_era(
=======
		fn fetch_nodes_aggregates_for_era(
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
=======
		pub(crate) fn fetch_nodes_aggregates_for_era(
>>>>>>> 73cca90f (tests: additional test scenarious are added)
			cluster_id: &ClusterId,
			era_id: DdcEra,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<(AggregatorInfo, Vec<NodeAggregateResponse>)>, OCWError> {
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
			let mut nodes_aggregates = Vec::new();

			for (node_key, node_params) in dac_nodes {
				let aggregates_res = Self::fetch_node_aggregates(cluster_id, era_id, node_params);
				if aggregates_res.is_err() {
					log::warn!(
						"Aggregator from cluster {:?} is unavailable while fetching nodes aggregates. Key: {:?} Host: {:?}",
						cluster_id,
						node_key.get_hex(),
						String::from_utf8(node_params.host.clone())
					);
					// skip unavailable aggregators and continue with available ones
					continue;
				}

				let aggregates = aggregates_res.expect("Nodes Aggregates Response to be available");

				nodes_aggregates.push((
					AggregatorInfo {
						node_pub_key: node_key.clone(),
						node_params: node_params.clone(),
					},
					aggregates,
				));
			}

<<<<<<< HEAD
			Ok(node_usages)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			Ok(nodes_aggregates)
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
		}

		/// Fetch customer usage for an era.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		pub(crate) fn fetch_buckets_aggregates_for_era(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 247f460a (Move aggregator's JSON API structs out of pallet)
		) -> Result<
			Vec<(AggregatorInfo, Vec<aggregator_client::json::BucketAggregateResponse>)>,
			OCWError,
		> {
			let mut bucket_aggregates: Vec<(
				AggregatorInfo,
				Vec<aggregator_client::json::BucketAggregateResponse>,
			)> = Vec::new();

			for (node_key, node_params) in dac_nodes {
				let aggregates_res = Self::fetch_bucket_aggregates(cluster_id, era_id, node_params);
				if aggregates_res.is_err() {
					log::warn!(
						"Aggregator from cluster {:?} is unavailable while fetching buckets aggregates. Key: {:?} Host: {:?}",
						cluster_id,
						node_key.get_hex(),
						String::from_utf8(node_params.host.clone())
					);
					// skip unavailable aggregators and continue with available ones
					continue;
				}

				let aggregates =
					aggregates_res.expect("Buckets Aggregates Response to be available");

				bucket_aggregates.push((
					AggregatorInfo {
						node_pub_key: node_key.clone(),
						node_params: node_params.clone(),
					},
					aggregates,
				));
			}

			Ok(bucket_aggregates)
=======
		fn fetch_customers_usage_for_era(
=======
		pub(crate) fn fetch_customers_usage_for_era(
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
		pub(crate) fn fetch_buckets_aggregates_for_era(
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
			cluster_id: &ClusterId,
			era_id: DdcEra,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<(AggregatorInfo, Vec<BucketAggregate>)>, OCWError> {
			let mut bucket_aggregates: Vec<(AggregatorInfo, Vec<BucketAggregate>)> = Vec::new();
=======
		) -> Result<Vec<(AggregatorInfo, Vec<BucketAggregateResponse>)>, OCWError> {
			let mut bucket_aggregates: Vec<(AggregatorInfo, Vec<BucketAggregateResponse>)> =
				Vec::new();
>>>>>>> 23acfcfc (chore: activities renamed to aggregates, redundant traits impls are removed)

			for (node_key, node_params) in dac_nodes {
				let aggregates_res = Self::fetch_bucket_aggregates(cluster_id, era_id, node_params);
				if aggregates_res.is_err() {
					log::warn!(
						"Aggregator from cluster {:?} is unavailable while fetching buckets aggregates. Key: {:?} Host: {:?}",
						cluster_id,
						node_key.get_hex(),
						String::from_utf8(node_params.host.clone())
					);
					// skip unavailable aggregators and continue with available ones
					continue;
				}

				let aggregates =
					aggregates_res.expect("Buckets Aggregates Response to be available");

				bucket_aggregates.push((
					AggregatorInfo {
						node_pub_key: node_key.clone(),
						node_params: node_params.clone(),
					},
					aggregates,
				));
			}

<<<<<<< HEAD
			Ok(customers_usages)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			Ok(bucket_aggregates)
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
		}

		/// Fetch processed era for across all nodes.
		///
		/// Parameters:
		/// - `cluster_id`: Cluster id
		/// - `node_params`: DAC node parameters
<<<<<<< HEAD
<<<<<<< HEAD
		fn fetch_processed_era_for_nodes(
			cluster_id: &ClusterId,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<Vec<EraActivity>>, OCWError> {
			let mut processed_eras_by_nodes: Vec<Vec<EraActivity>> = Vec::new();

			for (node_key, node_params) in dac_nodes {
				let processed_eras_by_node = Self::fetch_processed_eras(node_params);
				if processed_eras_by_node.is_err() {
					log::warn!(
						"Aggregator from cluster {:?} is unavailable while fetching processed eras. Key: {:?} Host: {:?}",
<<<<<<< HEAD
						cluster_id,
						node_key.get_hex(),
						String::from_utf8(node_params.host.clone())
					);
					// skip unavailable aggregators and continue with available ones
					continue;
				} else {
					let eras = processed_eras_by_node.expect("Era Response to be available");
					if !eras.is_empty() {
						processed_eras_by_nodes
							.push(eras.into_iter().map(|e| e.into()).collect::<Vec<_>>());
					}
				}
			}

			Ok(processed_eras_by_nodes)
		}

		pub fn node_key_from_hex(hex_str: String) -> Result<NodePubKey, hex::FromHexError> {
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> e6993a6e (fix: cutting off '0x' prefix while decoding)
			let bytes_vec = if hex_str.len() == 66 {
				// cut `0x` prefix
				hex::decode(&hex_str[2..])?
			} else {
				hex::decode(hex_str)?
			};

<<<<<<< HEAD
=======
			let bytes_vec = hex::decode(hex_str)?;
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
=======
>>>>>>> e6993a6e (fix: cutting off '0x' prefix while decoding)
			let bytes_arr: [u8; 32] =
				bytes_vec.try_into().map_err(|_| hex::FromHexError::InvalidStringLength)?;
			let pub_key = AccountId32::from(bytes_arr);
			Ok(NodePubKey::StoragePubKey(pub_key))
<<<<<<< HEAD
=======
		fn fetch_processed_era_for_node(
			cluster_id: &ClusterId,
=======
		fn fetch_processed_era_for_nodes(
<<<<<<< HEAD
			_cluster_id: &ClusterId,
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
=======
			cluster_id: &ClusterId,
>>>>>>> 492c0466 (chore: more logging info for tracing)
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<Vec<EraActivity>>, OCWError> {
			let mut processed_eras_by_nodes: Vec<Vec<EraActivity>> = Vec::new();

			for (node_key, node_params) in dac_nodes {
				let processed_eras_by_node = Self::fetch_processed_eras(node_params);
				if processed_eras_by_node.is_err() {
					log::warn!(
						"Aggregator from cluster {:?} is unavailable while fetching nodes aggregates. Key: {:?} Host: {:?}",
=======
>>>>>>> 33b0438a (fix: log message)
						cluster_id,
						node_key.get_hex(),
						String::from_utf8(node_params.host.clone())
					);
					// skip unavailable aggregators and continue with available ones
					continue;
				} else {
					let eras = processed_eras_by_node.expect("Era Response to be available");
					if !eras.is_empty() {
						processed_eras_by_nodes
							.push(eras.into_iter().map(|e| e.into()).collect::<Vec<_>>());
					}
				}
			}

<<<<<<< HEAD
			Ok(eras)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			Ok(processed_eras_by_nodes)
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
=======
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		}

		/// Verify whether leaf is part of tree
		///
		/// Parameters:
		/// - `root_hash`: merkle root hash
		/// - `batch_hash`: hash of the batch
		/// - `batch_index`: index of the batch
		/// - `batch_proof`: MMR proofs
		pub(crate) fn _proof_merkle_leaf(
			root_hash: PayableUsageHash,
			batch_hash: PayableUsageHash,
			batch_index: BatchIndex,
			max_batch_index: BatchIndex,
			batch_proof: &MMRProof,
		) -> Result<bool, Error<T>> {
			let batch_position = leaf_index_to_pos(batch_index.into());
			let mmr_size = leaf_index_to_mmr_size(max_batch_index.into());
			let proof: MerkleProof<PayableUsageHash, MergeMMRHash> =
				MerkleProof::new(mmr_size, batch_proof.proof.clone());
			proof
				.verify(root_hash, vec![(batch_position, batch_hash)])
				.map_err(|_| Error::<T>::FailedToVerifyMerkleProof)
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
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_prepare_era_for_payout(payers_batch_merkle_root_hashes.len() as u32 + payees_batch_merkle_root_hashes.len() as u32))]
=======
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports() + T::DbWeight::get().reads_writes(2, 5))] // todo! implement weights
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
=======
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_prepare_era_for_payout(payers_batch_merkle_root_hashes.len() as u32 + payees_batch_merkle_root_hashes.len() as u32))]
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
		pub fn set_prepare_era_for_payout(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_activity: EraActivity,
<<<<<<< HEAD
			payers_merkle_root_hash: ActivityHash,
			payees_merkle_root_hash: ActivityHash,
<<<<<<< HEAD
<<<<<<< HEAD
			payers_batch_merkle_root_hashes: Vec<ActivityHash>,
			payees_batch_merkle_root_hashes: Vec<ActivityHash>,
=======
			payers_merkle_root_hash: DeltaUsageHash,
			payees_merkle_root_hash: DeltaUsageHash,
			payers_batch_merkle_root_hashes: Vec<DeltaUsageHash>,
			payees_batch_merkle_root_hashes: Vec<DeltaUsageHash>,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorized);
<<<<<<< HEAD
=======
=======
			payers_batch_merkle_root_hashes: Vec<ActivityHash>,
			payees_batch_merkle_root_hashes: Vec<ActivityHash>,
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorised);
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
			let mut era_validation = {
				let era_validations = <EraValidations<T>>::get(cluster_id, era_activity.id);

				if era_validations.is_none() {
					EraValidation {
						payers_merkle_root_hash: DeltaUsageHash::default(),
						payees_merkle_root_hash: DeltaUsageHash::default(),
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

			let validators_quorum = T::ValidatorsQuorum::get();
			let threshold = validators_quorum * <ValidatorSet<T>>::get().len();

			let mut should_deposit_ready_event = false;
<<<<<<< HEAD
<<<<<<< HEAD
			if threshold <= signed_validators.len() {
=======
			if threshold < signed_validators.len() {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			if threshold <= signed_validators.len() {
>>>>>>> 5288a1a7 (Fixing billing activity issue (#431))
				// Update payers_merkle_root_hash and payees_merkle_root_hash as ones passed the
				// threshold
				era_validation.payers_merkle_root_hash = payers_merkle_root_hash;
				era_validation.payees_merkle_root_hash = payees_merkle_root_hash;
				era_validation.start_era = era_activity.start; // todo! start/end is set by the last validator and is not in consensus
				era_validation.end_era = era_activity.end;

<<<<<<< HEAD
<<<<<<< HEAD
				if payers_merkle_root_hash == ActivityHash::default() &&
<<<<<<< HEAD
<<<<<<< HEAD
					payees_merkle_root_hash == ActivityHash::default()
=======
				if payers_merkle_root_hash == DeltaUsageHash::default() &&
					payees_merkle_root_hash == DeltaUsageHash::default()
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
=======
				if payers_merkle_root_hash == DeltaUsageHash::default() &&
					payees_merkle_root_hash == DeltaUsageHash::default()
>>>>>>> c7dd3fb3 (fix: runtime version is increased)
				{
					// this condition is satisfied when there is no activity within era, i.e. when a
					// validator posts empty roots
					era_validation.status = EraValidationStatus::PayoutSkipped;
=======
					payees_merkle_root_hash == payers_merkle_root_hash
				{
					era_validation.status = EraValidationStatus::PayoutSuccess;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
					payees_merkle_root_hash == ActivityHash::default()
				{
					// this condition is satisfied when there is no activity within era, i.e. when a
					// validator posts empty roots
					era_validation.status = EraValidationStatus::PayoutSkipped;
>>>>>>> d9dfd7ab (fix: skipping payouts in case there is no activity)
				} else {
					era_validation.status = EraValidationStatus::ReadyForPayout;
				}

				should_deposit_ready_event = true;
			}

			// Update the EraValidations storage
			<EraValidations<T>>::insert(cluster_id, era_activity.id, era_validation);
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
			Self::deposit_event(Event::<T>::EraValidationRootsPosted {
				cluster_id,
				era_id: era_activity.id,
				validator: caller,
				payers_merkle_root_hash,
				payees_merkle_root_hash,
				payers_batch_merkle_root_hashes,
				payees_batch_merkle_root_hashes,
			});
<<<<<<< HEAD
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
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

<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
		/// Set validator key.
		///
		/// The origin must be a validator.
		///
		/// Parameters:
		/// - `ddc_validator_pub`: validator Key
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_validator_key())]
		pub fn set_validator_key(
			origin: OriginFor<T>,
			ddc_validator_pub: T::AccountId,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			let stash = T::ValidatorStaking::stash_by_ctrl(&controller)
				.map_err(|_| Error::<T>::NotController)?;

			ensure!(
				<ValidatorSet<T>>::get().contains(&ddc_validator_pub),
				Error::<T>::NotValidatorStash
			);

			ValidatorToStashKey::<T>::insert(&ddc_validator_pub, &stash);
			Self::deposit_event(Event::<T>::ValidatorKeySet { validator: ddc_validator_pub });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::commit_billing_fingerprint())]
		#[allow(clippy::too_many_arguments)]
		pub fn commit_billing_fingerprint(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			start_era: i64,
			end_era: i64,
			payers_root: PayableUsageHash,
			payees_root: PayableUsageHash,
			cluster_usage: NodeUsage,
		) -> DispatchResult {
<<<<<<< HEAD
			let sender = ensure_signed(origin)?;
<<<<<<< HEAD
<<<<<<< HEAD
=======
			let sender = ensure_signed(origin.clone())?;
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)

			T::PayoutProcessor::commit_billing_fingerprint(
				sender,
				cluster_id,
				era_id,
				start_era,
				end_era,
				payers_root,
				payees_root,
				cluster_usage,
			)
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::begin_billing_report())]
		pub fn begin_billing_report(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			fingerprint: Fingerprint,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);

			T::PayoutProcessor::begin_billing_report(cluster_id, era_id, fingerprint)?;

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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::begin_charging_customers())]
		pub fn begin_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
<<<<<<< HEAD
<<<<<<< HEAD
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
			T::PayoutProcessor::begin_charging_customers(cluster_id, era_id, max_batch_index)
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::send_charging_customers_batch(payers.len() as u32))]
		pub fn send_charging_customers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			payers: Vec<(NodePubKey, BucketId, BucketUsage)>,
=======
			payers: Vec<(BucketId, BucketUsage)>,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			batch_proof: MMRProof,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
=======
			payers: Vec<(NodePubKey, BucketId, CustomerUsage)>,
			batch_proof: MMRProof,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
<<<<<<< HEAD
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
			T::PayoutProcessor::send_charging_customers_batch(
				cluster_id,
				era_id,
				batch_index,
				&payers,
				batch_proof,
			)
		}

		#[pallet::call_index(6)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::end_charging_customers())]
		pub fn end_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
<<<<<<< HEAD
<<<<<<< HEAD
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
			T::PayoutProcessor::end_charging_customers(cluster_id, era_id)
		}

		#[pallet::call_index(7)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::begin_rewarding_providers())]
		pub fn begin_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
<<<<<<< HEAD
<<<<<<< HEAD
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
<<<<<<< HEAD
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
			T::PayoutProcessor::begin_rewarding_providers(
				cluster_id,
				era_id,
				max_batch_index,
				total_node_usage,
			)
=======
			T::PayoutProcessor::begin_rewarding_providers(cluster_id, era_id, max_batch_index)
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		}

		#[pallet::call_index(8)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::send_rewarding_providers_batch(payees.len() as u32))]
		pub fn send_rewarding_providers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
			payees: Vec<(NodePubKey, NodeUsage)>,
			batch_proof: MMRProof,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
<<<<<<< HEAD
<<<<<<< HEAD
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
			T::PayoutProcessor::send_rewarding_providers_batch(
				cluster_id,
				era_id,
				batch_index,
				&payees,
				batch_proof,
			)
		}

		#[pallet::call_index(9)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::end_rewarding_providers())]
		pub fn end_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
<<<<<<< HEAD
<<<<<<< HEAD
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
			T::PayoutProcessor::end_rewarding_providers(cluster_id, era_id)
		}

		#[pallet::call_index(10)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::end_billing_report())]
		pub fn end_billing_report(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
<<<<<<< HEAD
<<<<<<< HEAD
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
			T::PayoutProcessor::end_billing_report(cluster_id, era_id)?;

			let mut era_validation = <EraValidations<T>>::get(cluster_id, era_id).unwrap(); // should exist
			era_validation.status = EraValidationStatus::PayoutSuccess;
			<EraValidations<T>>::insert(cluster_id, era_id, era_validation);

<<<<<<< HEAD
<<<<<<< HEAD
			T::ClusterValidator::set_last_paid_era(&cluster_id, era_id)
		}

=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			// todo(yahortsaryk): this should be renamed to `last_paid_era` to eliminate ambiguity,
			// as the validation step is decoupled from payout step.
			T::ClusterValidator::set_last_validated_era(&cluster_id, era_id)
=======
			T::ClusterValidator::set_last_paid_era(&cluster_id, era_id)
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
		}

>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
		/// Emit consensus errors.
		///
		/// The origin must be a validator.
		///
		/// Parameters:
		/// - errors`: List of consensus errors
		///
		/// Emits `NotEnoughNodesForConsensus`  OR `ActivityNotInConsensus` event depend of error
		/// type, when successful.
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
		#[pallet::call_index(10)]
=======
		#[pallet::call_index(11)]
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		#[pallet::weight(<T as pallet::Config>::WeightInfo::emit_consensus_errors(errors.len() as u32))]
=======
		#[pallet::call_index(1)]
<<<<<<< HEAD
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		#[pallet::weight(<T as pallet::Config>::WeightInfo::emit_consensus_errors(errors.len() as u32))]
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
		pub fn emit_consensus_errors(
			origin: OriginFor<T>,
			errors: Vec<OCWError>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
<<<<<<< HEAD
<<<<<<< HEAD
			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorized);

			for error in errors {
				match error {
=======
			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorised);
=======
			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorized);
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)

			for error in errors {
				match error {
<<<<<<< HEAD
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
<<<<<<< HEAD
					OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
						Self::deposit_event(Event::ActivityNotInConsensus {
							cluster_id,
							era_id,
							id,
							validator: caller.clone(),
						});
					},
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 8149e899 (Changes for challenging node sub aggregates)
=======
>>>>>>> f34e347c (feat: grouping aggregates by consistent categories)
					OCWError::NodeUsageRetrievalError { cluster_id, era_id, node_pub_key } => {
						Self::deposit_event(Event::NodeUsageRetrievalError {
							cluster_id,
							era_id,
							node_pub_key,
							validator: caller.clone(),
						});
					},
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
					OCWError::BucketAggregatesRetrievalError {
						cluster_id,
						era_id,
						node_pub_key,
					} => {
						Self::deposit_event(Event::BucketAggregatesRetrievalError {
<<<<<<< HEAD
=======
					OCWError::CustomerUsageRetrievalError { cluster_id, era_id, node_pub_key } => {
						Self::deposit_event(Event::CustomerUsageRetrievalError {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
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
					OCWError::CommitBillingFingerprintTransactionError {
						cluster_id,
						era_id,
						payers_root,
						payees_root,
					} => {
						Self::deposit_event(Event::CommitBillingFingerprintTransactionError {
							cluster_id,
							era_id,
							payers_root,
							payees_root,
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
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)
					OCWError::FailedToCollectVerificationKey => {
						Self::deposit_event(Event::FailedToCollectVerificationKey {
							validator: caller.clone(),
						});
					},
					OCWError::FailedToFetchVerificationKey => {
						Self::deposit_event(Event::FailedToFetchVerificationKey {
<<<<<<< HEAD
=======
					OCWError::FailedToFetchCurrentValidator => {
						Self::deposit_event(Event::FailedToFetchCurrentValidator {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
					OCWError::FailedToRetrieveVerificationKey => {
						Self::deposit_event(Event::FailedToRetrieveVerificationKey {
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
=======
>>>>>>> 2c0f1952 (chore: fetching verification public keys in generic way)
							validator: caller.clone(),
						});
					},
					OCWError::FailedToFetchNodeProvider => {
						Self::deposit_event(Event::FailedToFetchNodeProvider {
							validator: caller.clone(),
						});
					},
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
					OCWError::FailedToFetchNodeTotalUsage { cluster_id, node_pub_key } => {
						Self::deposit_event(Event::FailedToFetchNodeTotalUsage {
							cluster_id,
							node_pub_key,
							validator: caller.clone(),
						});
					},
<<<<<<< HEAD
<<<<<<< HEAD
					OCWError::BucketAggregateRetrievalError {
=======
					OCWError::BucketAggregatesRetrievalError {
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
					OCWError::BucketAggregateRetrievalError {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
						cluster_id,
						era_id,
						bucket_id,
						node_pub_key,
					} => {
<<<<<<< HEAD
<<<<<<< HEAD
						Self::deposit_event(Event::BucketAggregateRetrievalError {
=======
						Self::deposit_event(Event::BucketAggregatesRetrievalError {
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
						Self::deposit_event(Event::BucketAggregateRetrievalError {
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
							cluster_id,
							era_id,
							bucket_id,
							node_pub_key,
							validator: caller.clone(),
						});
					},
<<<<<<< HEAD
<<<<<<< HEAD
					OCWError::ChallengeResponseRetrievalError {
						cluster_id,
						era_id,
						aggregate_key,
						aggregator,
					} => {
						Self::deposit_event(Event::ChallengeResponseRetrievalError {
							cluster_id,
							era_id,
							aggregate_key,
							aggregator,
							validator: caller.clone(),
						});
					},
					OCWError::TraverseResponseRetrievalError {
						cluster_id,
						era_id,
						aggregate_key,
						aggregator,
					} => {
						Self::deposit_event(Event::TraverseResponseRetrievalError {
							cluster_id,
							era_id,
							aggregate_key,
							aggregator,
							validator: caller.clone(),
						});
					},
					OCWError::FailedToFetchClusterNodes => {
						Self::deposit_event(Event::FailedToFetchClusterNodes {
							validator: caller.clone(),
						});
					},
					OCWError::FailedToFetchDacNodes => {
						Self::deposit_event(Event::FailedToFetchDacNodes {
							validator: caller.clone(),
						});
					},
					OCWError::EmptyConsistentGroup => {
						Self::deposit_event(Event::EmptyConsistentGroup);
					},
<<<<<<< HEAD
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
					OCWError::TotalNodeUsageLessThanZero { cluster_id, era_id } => {
						Self::deposit_event(Event::TotalNodeUsageLessThanZero {
=======
					OCWError::FailedToFetchNodeTotalUsage { cluster_id, node_pub_key } => {
						Self::deposit_event(Event::FailedToFetchNodeTotalUsage {
>>>>>>> e0ce0e5b (node integer delta usage (#412))
							cluster_id,
							node_pub_key,
							validator: caller.clone(),
						});
					},
>>>>>>> 00eed38c (Changes to accept stored_bytes as signed input (#410))
=======
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
<<<<<<< HEAD
					OCWError::BucketAggregateActivityNotInConsensus {
						cluster_id,
						era_id,
						id,
						node_ids,
					} => {
						Self::deposit_event(Event::BucketAggregateActivityNotInConsensus {
							cluster_id,
							era_id,
							id,
							node_ids,
							validator: caller.clone(),
						});
					},
<<<<<<< HEAD
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
=======
>>>>>>> 8149e899 (Changes for challenging node sub aggregates)
=======
>>>>>>> f34e347c (feat: grouping aggregates by consistent categories)
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
<<<<<<< HEAD
					OCWError::NotEnoughRecordsForConsensus { cluster_id, era_id, record_id } => {
						Self::deposit_event(Event::NotEnoughRecordsForConsensus {
							cluster_id,
							era_id,
							record_id,
							validator: caller.clone(),
						});
					},
<<<<<<< HEAD
>>>>>>> 18369b0b (Challenge sub trees and make them ready for payout (#434))
=======
=======
>>>>>>> f34e347c (feat: grouping aggregates by consistent categories)
					OCWError::FailedToFetchClusterNodes => {
						Self::deposit_event(Event::FailedToFetchClusterNodes {
							validator: caller.clone(),
						});
					},
					OCWError::FailedToFetchDacNodes => {
						Self::deposit_event(Event::FailedToFetchDacNodes {
							validator: caller.clone(),
						});
					},
<<<<<<< HEAD
>>>>>>> f462869c (chore: excessive parameters removed from DAC processing)
=======
					OCWError::EmptyConsistentGroup => {
						Self::deposit_event(Event::EmptyConsistentGroup);
					},
>>>>>>> f34e347c (feat: grouping aggregates by consistent categories)
=======
					OCWError::FailedToFetchVerifiedDeltaUsage => {
						Self::deposit_event(Event::FailedToFetchVerifiedDeltaUsage);
					},
					OCWError::FailedToFetchVerifiedPayableUsage => {
						Self::deposit_event(Event::FailedToFetchVerifiedPayableUsage);
					},
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
				}
			}

			Ok(())
		}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 95c6f7aa (Add doc comment for `set_era_validations` call)
		/// Set PayoutSkipped state of a given era if it is not validated yet. Otherwise does
		/// nothing.
		///
		/// Emits `EraValidationReady`.
<<<<<<< HEAD
<<<<<<< HEAD
		#[pallet::call_index(11)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_era_validations())]
		pub fn set_era_validations(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_skip_era_validation(&cluster_id, era_id)?;
			Self::deposit_event(Event::<T>::EraValidationReady { cluster_id, era_id });
=======
		/// Set validator key.
		///
		/// The origin must be a validator.
		///
		/// Parameters:
		/// - `ddc_validator_pub`: validator Key
		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_validator_key())]
		pub fn set_validator_key(
			origin: OriginFor<T>,
			ddc_validator_pub: T::AccountId,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			let stash = T::ValidatorStaking::stash_by_ctrl(&controller)
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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::begin_billing_report())]
		pub fn begin_billing_report(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			start_era: i64,
			end_era: i64,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);

			T::PayoutProcessor::begin_billing_report(cluster_id, era_id, start_era, end_era)?;

			EraValidations::<T>::try_mutate(
				cluster_id,
				era_id,
				|maybe_era_validations| -> DispatchResult {
					maybe_era_validations.as_mut().ok_or(Error::<T>::NoEraValidation)?.status =
						EraValidationStatus::PayoutInProgress;
					Ok(())
				},
			)?;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))

			Ok(())
		}

<<<<<<< HEAD
		/// Continue DAC validation from an era after a given one. It updates `last_paid_era` of a
		/// given cluster, creates an empty billing report with a finalized state, and sets an empty
		/// validation result on validators (in case it does not exist yet).
		#[pallet::call_index(12)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::skip_dac_validation_to_era())]
		pub fn skip_dac_validation_to_era(
=======
		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::begin_charging_customers())]
		pub fn begin_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutProcessor::begin_charging_customers(cluster_id, era_id, max_batch_index)
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::send_charging_customers_batch(payers.len() as u32))]
		pub fn send_charging_customers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
			payers: Vec<(NodePubKey, BucketId, CustomerUsage)>,
			batch_proof: MMRProof,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutProcessor::send_charging_customers_batch(
				cluster_id,
				era_id,
				batch_index,
				&payers,
				batch_proof,
			)
		}

		#[pallet::call_index(6)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::end_charging_customers())]
		pub fn end_charging_customers(
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
<<<<<<< HEAD
			ensure_root(origin)?;
			ensure!(
				era_id > T::ClusterValidator::get_last_paid_era(&cluster_id)?,
				Error::<T>::EraAlreadyPaid
			);

			Self::do_skip_era_validation(&cluster_id, era_id)?;

			let billing_report_params = BillingReportParams {
				cluster_id,
				era: era_id,
				state: PayoutState::Finalized,
				..Default::default()
			};

			T::PayoutProcessor::create_billing_report(
				T::AccountId::decode(&mut [0u8; 32].as_slice()).unwrap(),
				billing_report_params,
			);

			T::ClusterValidator::set_last_paid_era(&cluster_id, era_id)?;
=======
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutProcessor::end_charging_customers(cluster_id, era_id)
		}

		#[pallet::call_index(7)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::begin_rewarding_providers())]
		pub fn begin_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			max_batch_index: BatchIndex,
			total_node_usage: NodeUsage,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutProcessor::begin_rewarding_providers(
				cluster_id,
				era_id,
				max_batch_index,
				total_node_usage,
			)
		}

		#[pallet::call_index(8)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::send_rewarding_providers_batch(payees.len() as u32))]
		pub fn send_rewarding_providers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
			payees: Vec<(NodePubKey, NodeUsage)>,
			batch_proof: MMRProof,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutProcessor::send_rewarding_providers_batch(
				cluster_id,
				era_id,
				batch_index,
				&payees,
				batch_proof,
			)
		}

		#[pallet::call_index(9)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::end_rewarding_providers())]
		pub fn end_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutProcessor::end_rewarding_providers(cluster_id, era_id)
		}

		#[pallet::call_index(10)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::end_billing_report())]
		pub fn end_billing_report(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorised);
			T::PayoutProcessor::end_billing_report(cluster_id, era_id)?;

			let mut era_validation = <EraValidations<T>>::get(cluster_id, era_id).unwrap(); // should exist
			era_validation.status = EraValidationStatus::PayoutSuccess;
			<EraValidations<T>>::insert(cluster_id, era_id, era_validation);

			// todo(yahortsaryk): this should be renamed to `last_paid_era` to eliminate ambiguity,
			// as the validation step is decoupled from payout step.
			T::ClusterValidator::set_last_validated_era(&cluster_id, era_id)
		}

<<<<<<< HEAD
<<<<<<< HEAD
		#[pallet::call_index(11)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn set_cluster_to_validate(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
		) -> DispatchResult {
			ensure_root(origin)?;
			ClusterToValidate::<T>::put(cluster_id);
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))

			Ok(())
		}

=======
>>>>>>> 732d4a4c (chore: migration for deprecated storage item)
		// todo! Need to remove this
		#[pallet::call_index(11)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_billing_reports())] // todo! implement weights
		pub fn set_current_validator(origin: OriginFor<T>) -> DispatchResult {
			let validator = ensure_signed(origin)?;

			if !<ValidatorSet<T>>::get().contains(&validator) {
				ValidatorSet::<T>::append(validator);
			}

			Ok(())
		}

=======
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
		// todo! remove this after devnet testing
=======
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
>>>>>>> 95c6f7aa (Add doc comment for `set_era_validations` call)
		#[pallet::call_index(11)]
=======
		#[pallet::call_index(12)]
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_era_validations())]
		pub fn set_era_validations(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_skip_era_validation(&cluster_id, era_id)?;
			Self::deposit_event(Event::<T>::EraValidationReady { cluster_id, era_id });

			Ok(())
		}

		/// Continue DAC validation from an era after a given one. It updates `last_paid_era` of a
		/// given cluster, creates an empty billing report with a finalized state, and sets an empty
		/// validation result on validators (in case it does not exist yet).
		#[pallet::call_index(13)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::skip_dac_validation_to_era())]
		pub fn skip_dac_validation_to_era(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: DdcEra,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				era_id > T::ClusterValidator::get_last_paid_era(&cluster_id)?,
				Error::<T>::EraAlreadyPaid
			);

			Self::do_skip_era_validation(&cluster_id, era_id)?;

			let fingerprint =
				T::PayoutProcessor::create_billing_fingerprint(BillingFingerprintParams {
					cluster_id,
					era: era_id,
					start_era: Default::default(),
					end_era: Default::default(),
					payers_merkle_root: Default::default(),
					payees_merkle_root: Default::default(),
					cluster_usage: Default::default(),
					validators: BTreeSet::new(),
				});

			let billing_report_params = BillingReportParams {
				cluster_id,
				era: era_id,
				state: PayoutState::Finalized,
				fingerprint,
				..Default::default()
			};

			T::PayoutProcessor::create_billing_report(
				T::AccountId::decode(&mut [0u8; 32].as_slice()).unwrap(),
				billing_report_params,
			);

			T::ClusterValidator::set_last_paid_era(&cluster_id, era_id)?;

			Ok(())
		}
	}

	impl<T: Config> ValidatorVisitor<T> for Pallet<T> {
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		fn is_ocw_validator(caller: T::AccountId) -> bool {
			if ValidatorToStashKey::<T>::contains_key(caller.clone()) {
				<ValidatorSet<T>>::get().contains(&caller)
=======
=======
		#[cfg(feature = "runtime-benchmarks")]
<<<<<<< HEAD
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
		fn setup_validators(validators: Vec<T::AccountId>) {
=======
		fn setup_validators(validators_with_keys: Vec<(T::AccountId, T::AccountId)>) {
			let mut validators = vec![];
			for (validator, verification_key) in validators_with_keys {
				ValidatorToStashKey::<T>::insert(&verification_key, &validator);
				validators.push(validator);
			}

>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
			ValidatorSet::<T>::put(validators);
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn setup_validation_era(
			cluster_id: ClusterId,
			era_id: DdcEra,
			era_validation: EraValidation<T>,
		) {
			<EraValidations<T>>::insert(cluster_id, era_id, era_validation);
		}

=======
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
		fn is_ocw_validator(caller: T::AccountId) -> bool {
<<<<<<< HEAD
			if let Some(stash) = ValidatorToStashKey::<T>::get(caller) {
				<ValidatorSet<T>>::get().contains(&stash)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			if ValidatorToStashKey::<T>::contains_key(caller.clone()) {
				<ValidatorSet<T>>::get().contains(&caller)
>>>>>>> 00eed38c (Changes to accept stored_bytes as signed input (#410))
			} else {
				false
			}
		}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 0c3d2872 (chore: calculating mmr size based on max batch index)
		fn is_customers_batch_valid(
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
<<<<<<< HEAD
<<<<<<< HEAD
			max_batch_index: BatchIndex,
			payers: &[(NodePubKey, BucketId, BucketUsage)],
<<<<<<< HEAD
=======
		// todo! use batch_index and payers as part of the validation
		fn is_customers_batch_valid(
			cluster_id: ClusterId,
			era_id: DdcEra,
<<<<<<< HEAD
			_batch_index: BatchIndex,
			_payers: &[(T::AccountId, BucketId, CustomerUsage)],
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			batch_index: BatchIndex,
			payers: &[(T::AccountId, String, BucketId, CustomerUsage)],
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
=======
			max_batch_index: BatchIndex,
>>>>>>> 0c3d2872 (chore: calculating mmr size based on max batch index)
			payers: &[(NodePubKey, BucketId, CustomerUsage)],
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			batch_proof: &MMRProof,
		) -> bool {
			let validation_era = EraValidations::<T>::get(cluster_id, era_id);

			match validation_era {
				Some(valid_era) => {
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
					let root_hash = valid_era.payers_merkle_root_hash;

					let activity_hashes = payers
						.iter()
						.map(|(node_key, bucket_id, usage)| {
<<<<<<< HEAD
							let mut data = bucket_id.encode();
							let node_id = format!("0x{}", node_key.get_hex());
=======
=======
					let root_hash = valid_era.payers_merkle_root_hash;

>>>>>>> 29b2c1bf (chore: hash and position of the leaf is removed from tx parameters)
					let activity_hashes = payers
						.iter()
						.map(|(_bucket_owner, node_id, bucket_id, usage)| {
							let mut data = bucket_id.encode();
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
							let mut data = bucket_id.encode();
							let node_id = format!("0x{}", node_key.get_hex());
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
							data.extend_from_slice(&node_id.encode());
							data.extend_from_slice(&usage.stored_bytes.encode());
							data.extend_from_slice(&usage.transferred_bytes.encode());
							data.extend_from_slice(&usage.number_of_puts.encode());
							data.extend_from_slice(&usage.number_of_gets.encode());
							T::ActivityHasher::hash(&data).into()
						})
						.collect::<Vec<_>>();
<<<<<<< HEAD

					let batch_hash =
						Self::create_merkle_root(&cluster_id, era_id, activity_hashes.as_slice())
							.expect("batch_hash to be created");

					Self::proof_merkle_leaf(
						root_hash,
						batch_hash,
						batch_index,
						max_batch_index,
						batch_proof,
					)
					.unwrap_or(false)
=======
					//Self::create_merkle_root(leaves)

					let root = valid_era.payers_merkle_root_hash;
					Self::proof_merkle_leaf(root, batch_proof).unwrap_or(false)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======

					let batch_hash =
						Self::create_merkle_root(&cluster_id, era_id, activity_hashes.as_slice())
							.expect("batch_hash to be created");

<<<<<<< HEAD
					Self::proof_merkle_leaf(root_hash, batch_hash, batch_index, batch_proof)
						.unwrap_or(false)
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
					Self::proof_merkle_leaf(
						root_hash,
						batch_hash,
						batch_index,
						max_batch_index,
						batch_proof,
					)
					.unwrap_or(false)
>>>>>>> 0c3d2872 (chore: calculating mmr size based on max batch index)
				},
				None => false,
			}
		}

<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 0c3d2872 (chore: calculating mmr size based on max batch index)
		fn is_providers_batch_valid(
			cluster_id: ClusterId,
			era_id: DdcEra,
			batch_index: BatchIndex,
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 0c3d2872 (chore: calculating mmr size based on max batch index)
			max_batch_index: BatchIndex,
			payees: &[(NodePubKey, NodeUsage)],
=======
		// todo! use batch_index and payees as part of the validation
		fn is_providers_batch_valid(
			cluster_id: ClusterId,
			era_id: DdcEra,
<<<<<<< HEAD
			_batch_index: BatchIndex,
			_payees: &[(T::AccountId, NodeUsage)],
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			batch_index: BatchIndex,
			payees: &[(T::AccountId, String, NodeUsage)],
>>>>>>> acbb6a47 (fix: checking MMR proof for a batch)
=======
			payees: &[(NodePubKey, NodeUsage)],
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
			batch_proof: &MMRProof,
		) -> bool {
			let validation_era = EraValidations::<T>::get(cluster_id, era_id);

			match validation_era {
				Some(valid_era) => {
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
					let root_hash = valid_era.payees_merkle_root_hash;

					let activity_hashes = payees
						.iter()
						.map(|(node_key, usage)| {
							let mut data = format!("0x{}", node_key.get_hex()).encode();
<<<<<<< HEAD
=======
=======
					let root_hash = valid_era.payees_merkle_root_hash;

>>>>>>> 29b2c1bf (chore: hash and position of the leaf is removed from tx parameters)
					let activity_hashes = payees
						.iter()
						.map(|(_node_provider, node_id, usage)| {
							let mut data = node_id.encode();
>>>>>>> acbb6a47 (fix: checking MMR proof for a batch)
=======
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
							data.extend_from_slice(&usage.stored_bytes.encode());
							data.extend_from_slice(&usage.transferred_bytes.encode());
							data.extend_from_slice(&usage.number_of_puts.encode());
							data.extend_from_slice(&usage.number_of_gets.encode());
							T::ActivityHasher::hash(&data).into()
						})
						.collect::<Vec<_>>();

					let batch_hash =
						Self::create_merkle_root(&cluster_id, era_id, activity_hashes.as_slice())
							.expect("batch_hash to be created");
<<<<<<< HEAD
<<<<<<< HEAD

					Self::proof_merkle_leaf(
						root_hash,
						batch_hash,
						batch_index,
						max_batch_index,
						batch_proof,
					)
					.unwrap_or(false)
=======
					let root = valid_era.payees_merkle_root_hash;
					Self::proof_merkle_leaf(root, batch_proof).unwrap_or(false)
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
					let batch_position = leaf_index_to_pos(batch_index.into()); // batch_proof.leaf_with_position.0;
=======
>>>>>>> 29b2c1bf (chore: hash and position of the leaf is removed from tx parameters)

<<<<<<< HEAD
					Self::proof_merkle_leaf(root_hash, batch_hash, batch_index, batch_proof)
						.unwrap_or(false)
>>>>>>> acbb6a47 (fix: checking MMR proof for a batch)
=======
					Self::proof_merkle_leaf(
						root_hash,
						batch_hash,
						batch_index,
						max_batch_index,
						batch_proof,
					)
					.unwrap_or(false)
>>>>>>> 0c3d2872 (chore: calculating mmr size based on max batch index)
				},
				None => false,
			}
=======
		fn is_quorum_reached(quorum: Percent, members_count: usize) -> bool {
			let threshold = quorum * <ValidatorSet<T>>::get().len();
			threshold <= members_count
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		}
	}

	impl<T: Config> sp_application_crypto::BoundToRuntimeAppPublic for Pallet<T> {
		type Public = T::AuthorityId;
	}

	impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
		type Key = T::AuthorityId;

<<<<<<< HEAD
		#[allow(clippy::multiple_bound_locations)]
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
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

<<<<<<< HEAD
		#[allow(clippy::multiple_bound_locations)]
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
		fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_authorities: I)
		where
			I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
		{
			log::info!("🙌Adding Validator from new session.");
			let validators = validators
				.map(|(_, k)| T::AccountId::decode(&mut &k.into().encode()[..]).unwrap())
				.collect::<Vec<_>>();
<<<<<<< HEAD
<<<<<<< HEAD
			log::info!("🙌Total validator from new session. {:?}", validators.len());
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			log::info!("🙌Total validator from new session. {:?}", validators.len());
>>>>>>> 626be780 (Changes to fix unprocessed eras (#423))
			ValidatorSet::<T>::put(validators);
		}

		fn on_disabled(_i: u32) {}
	}
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub validators: Vec<T::AccountId>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { validators: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		fn build(&self) {
			for validator in &self.validators {
				<ValidatorSet<T>>::append(validator);
			}
		}
	}
<<<<<<< HEAD
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
}
