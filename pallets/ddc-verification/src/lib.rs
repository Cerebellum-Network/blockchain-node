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
#![allow(clippy::manual_inspect)]
use core::str;

use base64ct::{Base64, Encoding};
#[cfg(feature = "runtime-benchmarks")]
use ddc_primitives::traits::{ClusterCreator, CustomerDepositor};
use ddc_primitives::{
	ocw_mutex::OcwMutex,
	traits::{
		BucketManager, ClusterManager, ClusterProtocol, ClusterValidator, CustomerVisitor,
		NodeManager, PayoutProcessor, StorageUsageProvider, ValidatorVisitor,
	},
	BatchIndex, BucketStorageUsage, BucketUsage, ClusterFeesParams, ClusterId,
	ClusterPricingParams, ClusterStatus, CustomerCharge as CustomerCosts, DdcEra, EHDId, EhdEra,
	MMRProof, NodeParams, NodePubKey, NodeStorageUsage, NodeUsage, PHDId, PayableUsageHash,
	PaymentEra, PayoutFingerprintParams, PayoutReceiptParams, PayoutState,
	ProviderReward as ProviderProfits, StorageNodeParams, StorageNodePubKey, AVG_SECONDS_MONTH,
};
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, Get, OneSessionHandler, StorageVersion},
};
use frame_system::{
	offchain::{Account, AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer},
	pallet_prelude::*,
};
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
use sp_io::hashing::blake2_256;
pub use sp_io::{
	crypto::sr25519_public_keys,
	offchain::{
		local_storage_clear, local_storage_compare_and_set, local_storage_get, local_storage_set,
	},
};
use sp_runtime::{
	offchain::{http, Duration, StorageKind},
	traits::{Hash, IdentifyAccount},
	ArithmeticError, Percent, Perquintill,
};
use sp_staking::StakingInterface;
use sp_std::{
	collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	fmt::Debug,
	prelude::*,
};
pub mod weights;
use crate::weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

pub mod migrations;

mod aggregator_client;

pub mod aggregate_tree;
use aggregate_tree::{
	calculate_sample_size_fin, calculate_sample_size_inf, get_leaves_ids, D_099, P_001,
};

pub mod proto {
	include!(concat!(env!("OUT_DIR"), "/activity.rs"));
}

mod signature;
use signature::Verify;

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
	use sp_runtime::SaturatedConversion;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	const _SUCCESS_CODE: u16 = 200;
	const _BUF_SIZE: usize = 128;
	const RESPONSE_TIMEOUT: u64 = 20000;
	pub const MAX_RETRIES_COUNT: u32 = 3;
	pub const BUCKETS_AGGREGATES_FETCH_BATCH_SIZE: usize = 100;
	pub const NODES_AGGREGATES_FETCH_BATCH_SIZE: usize = 10;
	pub const OCW_MUTEX_ID: &[u8] = b"inspection_lock";

	/// This is overall amount that the bucket owner will be charged for his buckets within a
	/// payment Era.
	#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Encode, Decode)]
	pub(crate) struct CustomerCharge(AccountId32, u128);

	/// This is overall amount of bytes that the node owner will be
	#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Encode, Decode)]
	pub(crate) struct ProviderReward(AccountId32, u128);

	/// Payable usage of a Payment Era (EHD) includes all the batches of customers and providers
	/// that will be processed during the payout process along with merkle root hashes and proofs.
	/// To calculate the same payout fingerprint and let the payouts to start the required quorum
	/// of validators need to agree on the same values for the Era usage and commit the same payout
	/// fingerprint.
	#[derive(Clone, Encode, Decode)]
	pub(crate) struct PayableEHDUsage {
		cluster_id: ClusterId,
		ehd_id: String,
		payers_usage: Vec<CustomerCharge>,
		payers_root: PayableUsageHash,
		payers_batch_roots: Vec<PayableUsageHash>,
		payees_usage: Vec<ProviderReward>,
		payees_root: PayableUsageHash,
		payees_batch_roots: Vec<PayableUsageHash>,
	}

	impl PayableEHDUsage {
		fn fingerprint(&self) -> Fingerprint {
			let mut data = self.cluster_id.encode();
			data.extend_from_slice(&self.ehd_id.encode());
			data.extend_from_slice(&self.payers_root.encode());
			data.extend_from_slice(&self.payees_root.encode());
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
		type ClusterValidator: ClusterValidator<Self>;
		type ClusterManager: ClusterManager<Self>;
		type PayoutProcessor: PayoutProcessor<Self>;
		/// DDC nodes read-only registry.
		type NodeManager: NodeManager<Self>;
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
		const DAC_REDUNDANCY_FACTOR: u16;

		#[pallet::constant]
		type AggregatorsQuorum: Get<Percent>;
		#[pallet::constant]
		type ValidatorsQuorum: Get<Percent>;

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
		const VERIFY_AGGREGATOR_RESPONSE_SIGNATURE: bool;
		const DISABLE_PAYOUTS_CUTOFF: bool;
		const DEBUG_MODE: bool;
		type ClusterProtocol: ClusterProtocol<Self, BalanceOf<Self>>;
		#[cfg(feature = "runtime-benchmarks")]
		type CustomerDepositor: CustomerDepositor<Self>;
		#[cfg(feature = "runtime-benchmarks")]
		type ClusterCreator: ClusterCreator<Self, BalanceOf<Self>>;
		type BucketManager: BucketManager<Self>;
	}

	/// The event type.
	#[pallet::event]
	/// The `generate_deposit` macro generates a function on `Pallet` called `deposit_event` which
	/// will properly convert the error type of your pallet into `RuntimeEvent` (recall `type
	/// RuntimeEvent: From<Event<Self>>`, so it can be converted) and deposit it via
	/// `frame_system::Pallet::deposit_event`.
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		PaidEraRetrievalError {
			cluster_id: ClusterId,
			validator: T::AccountId,
		},
		CommitPayoutFingerprintTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			payers_root: PayableUsageHash,
			payees_root: PayableUsageHash,
			validator: T::AccountId,
		},
		BeginPayoutTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			validator: T::AccountId,
		},
		BeginChargingCustomersTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			validator: T::AccountId,
		},
		SendChargingCustomersBatchTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			batch_index: BatchIndex,
			validator: T::AccountId,
		},
		SendRewardingProvidersBatchTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			batch_index: BatchIndex,
			validator: T::AccountId,
		},
		EndChargingCustomersTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			validator: T::AccountId,
		},
		BeginRewardingProvidersTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			validator: T::AccountId,
		},
		EndRewardingProvidersTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			validator: T::AccountId,
		},
		EndPayoutTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			validator: T::AccountId,
		},
		PayoutReceiptDoesNotExist {
			cluster_id: ClusterId,
			era_id: EhdEra,
			validator: T::AccountId,
		},
		BatchIndexUnderflow {
			cluster_id: ClusterId,
			era_id: EhdEra,
			validator: T::AccountId,
		},
		BatchIndexOverflow {
			cluster_id: ClusterId,
			era_id: EhdEra,
			validator: T::AccountId,
		},
		FailedToCreateMerkleRoot {
			cluster_id: ClusterId,
			era_id: EhdEra,
			validator: T::AccountId,
		},
		FailedToCreateMerkleProof {
			cluster_id: ClusterId,
			era_id: EhdEra,
			validator: T::AccountId,
		},
		FailedToCollectVerificationKey {
			validator: T::AccountId,
		},
		FailedToFetchVerificationKey {
			validator: T::AccountId,
		},
		ValidatorKeySet {
			validator: T::AccountId,
		},
		FailedToFetchCollectors {
			cluster_id: ClusterId,
			validator: T::AccountId,
		},
		FailedToFetchGCollectors {
			cluster_id: ClusterId,
			validator: T::AccountId,
		},
		ChallengeResponseError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
			validator: T::AccountId,
		},
		TraverseResponseError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
			validator: T::AccountId,
		},
		FailedToFetchVerifiedDeltaUsage,
		FailedToFetchVerifiedPayableUsage,
		FailedToParseEHDId {
			ehd_id: String,
		},
		FailedToParsePHDId {
			phd_id: String,
		},
		FailedToParseNodeKey {
			node_key: String,
		},
		FailedToFetchProviderId {
			node_key: NodePubKey,
		},
		FailedToParseBucketId {
			bucket_id: String,
		},
		FailedToFetchCustomerId {
			bucket_id: BucketId,
		},
		FailedToParseCustomerId {
			customer_id: AccountId32,
		},
		TimeCapsuleError,
		FailedToFetchTraversedEHD,
		FailedToFetchTraversedPHD,
		FailedToFetchNodeChallenge,
		FailedToFetchBucketChallenge,
		FailedToSaveInspectionReceipt,
		FailedToFetchInspectionReceipt,
		FailedToFetchPaymentEra,
		FailedToCalculatePayersBatches {
			era_id: EhdEra,
		},
		FailedToCalculatePayeesBatches {
			era_id: EhdEra,
		},
		FailedToFetchProtocolParams {
			cluster_id: ClusterId,
		},
		NodesInspectionError,
		BucketsInspectionError,
		FailedToSignInspectionReceipt,
	}

	/// Consensus Errors
	#[derive(Debug, Encode, Decode, Clone, TypeInfo, PartialEq)]
	pub enum OCWError {
		PaidEraRetrievalError {
			cluster_id: ClusterId,
		},
		/// Challenge Response Retrieval Error.
		ChallengeResponseError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
		},
		/// Traverse Response Retrieval Error.
		TraverseResponseError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			aggregator: NodePubKey,
		},
		CommitPayoutFingerprintTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			payers_root: PayableUsageHash,
			payees_root: PayableUsageHash,
		},
		BeginPayoutTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
		},
		BeginChargingCustomersTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
		},
		SendChargingCustomersBatchTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			batch_index: BatchIndex,
		},
		SendRewardingProvidersBatchTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
			batch_index: BatchIndex,
		},
		EndChargingCustomersTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
		},
		BeginRewardingProvidersTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
		},
		EndRewardingProvidersTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
		},
		EndPayoutTransactionError {
			cluster_id: ClusterId,
			era_id: EhdEra,
		},
		PayoutReceiptDoesNotExist {
			cluster_id: ClusterId,
			era_id: EhdEra,
		},
		BatchIndexUnderflow {
			cluster_id: ClusterId,
			era_id: EhdEra,
		},
		BatchIndexOverflow {
			cluster_id: ClusterId,
			era_id: EhdEra,
		},
		FailedToCreateMerkleRoot {
			cluster_id: ClusterId,
			era_id: EhdEra,
		},
		FailedToCreateMerkleProof {
			cluster_id: ClusterId,
			era_id: EhdEra,
		},
		FailedToCollectVerificationKey,
		FailedToFetchVerificationKey,
		FailedToFetchCollectors {
			cluster_id: ClusterId,
		},
		FailedToFetchGCollectors {
			cluster_id: ClusterId,
		},
		FailedToFetchVerifiedDeltaUsage,
		FailedToFetchVerifiedPayableUsage,
		FailedToParseEHDId {
			ehd_id: String,
		},
		FailedToParsePHDId {
			phd_id: String,
		},
		FailedToParseNodeKey {
			node_key: String,
		},
		FailedToFetchProviderId {
			node_key: NodePubKey,
		},
		FailedToParseBucketId {
			bucket_id: String,
		},
		FailedToFetchCustomerId {
			bucket_id: BucketId,
		},
		FailedToParseCustomerId {
			customer_id: AccountId32,
		},
		TimeCapsuleError,
		FailedToFetchTraversedEHD,
		FailedToFetchTraversedPHD,
		FailedToFetchNodeChallenge,
		FailedToFetchBucketChallenge,
		FailedToSaveInspectionReceipt,
		FailedToFetchInspectionReceipt,
		FailedToFetchPaymentEra,
		FailedToCalculatePayersBatches {
			era_id: EhdEra,
		},
		FailedToCalculatePayeesBatches {
			era_id: EhdEra,
		},
		FailedToFetchProtocolParams {
			cluster_id: ClusterId,
		},
		NodesInspectionError,
		BucketsInspectionError,
		FailedToSignInspectionReceipt,
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		/// Bad verification key.
		BadVerificationKey,
		/// Bad requests.
		BadRequest,
		/// Not a validator.
		Unauthorized,
		/// Already signed era.
		AlreadySignedEra,
		NotExpectedState,
		/// Already signed payout batch.
		AlreadySignedPayoutBatch,
		/// Node Retrieval Error.
		NodeRetrievalError,
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
		/// Fail to generate proof
		FailedToGenerateProof,
		/// Fail to verify merkle proof
		FailedToVerifyMerkleProof,
		/// No Era Validation exist
		NoEraValidation,
		/// Given era is already validated and paid.
		EraAlreadyPaid,
		VerificationKeyAlreadySet,
		VerificationKeyAlreadyPaired,
	}

	/// List of validators.
	#[pallet::storage]
	pub type ValidatorSet<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	/// Validator stash key mapping
	#[pallet::storage]
	pub type ValidatorToStashKey<T: Config> = StorageMap<_, Identity, T::AccountId, T::AccountId>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::commit_payout_fingerprint())]
		#[allow(clippy::too_many_arguments)]
		pub fn commit_payout_fingerprint(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			ehd_id: String,
			payers_root: PayableUsageHash,
			payees_root: PayableUsageHash,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);

			T::PayoutProcessor::commit_payout_fingerprint(
				sender,
				cluster_id,
				ehd_id,
				payers_root,
				payees_root,
			)
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::begin_payout())]
		pub fn begin_payout(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
			fingerprint: Fingerprint,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
			T::PayoutProcessor::begin_payout(cluster_id, era_id, fingerprint)?;
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::begin_charging_customers())]
		pub fn begin_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
			T::PayoutProcessor::begin_charging_customers(cluster_id, era_id, max_batch_index)
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::send_charging_customers_batch(payers.len() as u32))]
		pub fn send_charging_customers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
			batch_index: BatchIndex,
			payers: Vec<(T::AccountId, u128)>,
			batch_proof: MMRProof,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
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
			era_id: PaymentEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
			T::PayoutProcessor::end_charging_customers(cluster_id, era_id)
		}

		#[pallet::call_index(7)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::begin_rewarding_providers())]
		pub fn begin_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
			T::PayoutProcessor::begin_rewarding_providers(cluster_id, era_id, max_batch_index)
		}

		#[pallet::call_index(8)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::send_rewarding_providers_batch(payees.len() as u32))]
		pub fn send_rewarding_providers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
			batch_index: BatchIndex,
			payees: Vec<(T::AccountId, u128)>,
			batch_proof: MMRProof,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
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
			era_id: PaymentEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
			T::PayoutProcessor::end_rewarding_providers(cluster_id, era_id)
		}

		#[pallet::call_index(10)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::end_payout())]
		pub fn end_payout(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized);
			T::PayoutProcessor::end_payout(cluster_id, era_id)?;
			T::ClusterValidator::set_last_paid_era(&cluster_id, era_id)
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
		#[pallet::call_index(11)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::emit_consensus_errors(errors.len() as u32))]
		pub fn emit_consensus_errors(
			origin: OriginFor<T>,
			errors: Vec<OCWError>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorized);

			for error in errors {
				match error {
					OCWError::PaidEraRetrievalError { cluster_id } => {
						Self::deposit_event(Event::PaidEraRetrievalError {
							cluster_id,
							validator: caller.clone(),
						});
					},
					OCWError::CommitPayoutFingerprintTransactionError {
						cluster_id,
						era_id,
						payers_root,
						payees_root,
					} => {
						Self::deposit_event(Event::CommitPayoutFingerprintTransactionError {
							cluster_id,
							era_id,
							payers_root,
							payees_root,
							validator: caller.clone(),
						});
					},
					OCWError::BeginPayoutTransactionError { cluster_id, era_id } => {
						Self::deposit_event(Event::BeginPayoutTransactionError {
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
					OCWError::EndPayoutTransactionError { cluster_id, era_id } => {
						Self::deposit_event(Event::EndPayoutTransactionError {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::PayoutReceiptDoesNotExist { cluster_id, era_id } => {
						Self::deposit_event(Event::PayoutReceiptDoesNotExist {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::BatchIndexUnderflow { cluster_id, era_id } => {
						Self::deposit_event(Event::BatchIndexUnderflow {
							cluster_id,
							era_id,
							validator: caller.clone(),
						});
					},
					OCWError::BatchIndexOverflow { cluster_id, era_id } => {
						Self::deposit_event(Event::BatchIndexOverflow {
							cluster_id,
							era_id,
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
					OCWError::FailedToCollectVerificationKey => {
						Self::deposit_event(Event::FailedToCollectVerificationKey {
							validator: caller.clone(),
						});
					},
					OCWError::FailedToFetchVerificationKey => {
						Self::deposit_event(Event::FailedToFetchVerificationKey {
							validator: caller.clone(),
						});
					},
					OCWError::ChallengeResponseError {
						cluster_id,
						era_id,
						aggregate_key,
						aggregator,
					} => {
						Self::deposit_event(Event::ChallengeResponseError {
							cluster_id,
							era_id,
							aggregate_key,
							aggregator,
							validator: caller.clone(),
						});
					},
					OCWError::TraverseResponseError {
						cluster_id,
						era_id,
						aggregate_key,
						aggregator,
					} => {
						Self::deposit_event(Event::TraverseResponseError {
							cluster_id,
							era_id,
							aggregate_key,
							aggregator,
							validator: caller.clone(),
						});
					},
					OCWError::FailedToFetchCollectors { cluster_id } => {
						Self::deposit_event(Event::FailedToFetchCollectors {
							validator: caller.clone(),
							cluster_id,
						});
					},
					OCWError::FailedToFetchGCollectors { cluster_id } => {
						Self::deposit_event(Event::FailedToFetchGCollectors {
							validator: caller.clone(),
							cluster_id,
						});
					},
					OCWError::FailedToFetchVerifiedDeltaUsage => {
						Self::deposit_event(Event::FailedToFetchVerifiedDeltaUsage);
					},
					OCWError::FailedToFetchVerifiedPayableUsage => {
						Self::deposit_event(Event::FailedToFetchVerifiedPayableUsage);
					},
					OCWError::FailedToParseEHDId { ehd_id } => {
						Self::deposit_event(Event::FailedToParseEHDId { ehd_id });
					},
					OCWError::FailedToParsePHDId { phd_id } => {
						Self::deposit_event(Event::FailedToParsePHDId { phd_id });
					},
					OCWError::FailedToParseNodeKey { node_key } => {
						Self::deposit_event(Event::FailedToParseNodeKey { node_key });
					},
					OCWError::FailedToParseBucketId { bucket_id } => {
						Self::deposit_event(Event::FailedToParseBucketId { bucket_id });
					},
					OCWError::FailedToFetchCustomerId { bucket_id } => {
						Self::deposit_event(Event::FailedToFetchCustomerId { bucket_id });
					},
					OCWError::FailedToFetchProviderId { node_key } => {
						Self::deposit_event(Event::FailedToFetchProviderId { node_key });
					},
					OCWError::TimeCapsuleError => {
						Self::deposit_event(Event::TimeCapsuleError);
					},
					OCWError::FailedToSignInspectionReceipt => {
						Self::deposit_event(Event::FailedToSignInspectionReceipt);
					},
					OCWError::FailedToFetchTraversedEHD => {
						Self::deposit_event(Event::FailedToFetchTraversedEHD);
					},
					OCWError::FailedToFetchTraversedPHD => {
						Self::deposit_event(Event::FailedToFetchTraversedPHD);
					},
					OCWError::FailedToFetchNodeChallenge => {
						Self::deposit_event(Event::FailedToFetchNodeChallenge);
					},
					OCWError::FailedToFetchBucketChallenge => {
						Self::deposit_event(Event::FailedToFetchBucketChallenge);
					},
					OCWError::FailedToSaveInspectionReceipt => {
						Self::deposit_event(Event::FailedToSaveInspectionReceipt);
					},
					OCWError::FailedToFetchInspectionReceipt => {
						Self::deposit_event(Event::FailedToFetchInspectionReceipt);
					},
					OCWError::FailedToFetchPaymentEra => {
						Self::deposit_event(Event::FailedToFetchPaymentEra);
					},
					OCWError::FailedToCalculatePayersBatches { era_id } => {
						Self::deposit_event(Event::FailedToCalculatePayersBatches { era_id });
					},
					OCWError::FailedToCalculatePayeesBatches { era_id } => {
						Self::deposit_event(Event::FailedToCalculatePayeesBatches { era_id });
					},
					OCWError::FailedToFetchProtocolParams { cluster_id } => {
						Self::deposit_event(Event::FailedToFetchProtocolParams { cluster_id });
					},
					OCWError::FailedToParseCustomerId { customer_id } => {
						Self::deposit_event(Event::FailedToParseCustomerId { customer_id });
					},
					OCWError::NodesInspectionError => {
						Self::deposit_event(Event::NodesInspectionError);
					},
					OCWError::BucketsInspectionError => {
						Self::deposit_event(Event::BucketsInspectionError);
					},
				}
			}

			Ok(())
		}

		/// Continue DAC inspection from an era after a given one. It updates `last_paid_era` of a
		/// given cluster, creates an empty payout receipt with a finalized state, and sets an empty
		/// validation result on validators (in case it does not exist yet).
		#[pallet::call_index(12)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::force_skip_inspection())]
		pub fn force_skip_inspection(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			ehd_id: String,
		) -> DispatchResult {
			let era_id = EHDId::try_from(ehd_id.clone()).map_err(|_| Error::<T>::BadRequest)?.2;

			ensure_root(origin)?;
			ensure!(
				era_id > T::ClusterValidator::get_last_paid_era(&cluster_id)?,
				Error::<T>::EraAlreadyPaid
			);

			let fingerprint =
				T::PayoutProcessor::create_payout_fingerprint(PayoutFingerprintParams {
					cluster_id,
					ehd_id: ehd_id.clone(),
					payers_merkle_root: Default::default(),
					payees_merkle_root: Default::default(),
					validators: BTreeSet::new(),
				});

			let payout_receipt_params = PayoutReceiptParams {
				cluster_id,
				era: era_id,
				state: PayoutState::Finalized,
				fingerprint,
				..Default::default()
			};

			T::PayoutProcessor::create_payout_receipt(
				T::AccountId::decode(&mut [0u8; 32].as_slice()).unwrap(),
				payout_receipt_params,
			);

			T::ClusterValidator::set_last_paid_era(&cluster_id, era_id)?;

			Ok(())
		}

		/// For Devnet usage only
		#[pallet::call_index(13)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::force_set_validator_key())]
		pub fn force_set_validator_key(
			origin: OriginFor<T>,
			controller: T::AccountId,
			ddc_validator_pub: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;

			let stash = T::ValidatorStaking::stash_by_ctrl(&controller)
				.map_err(|_| Error::<T>::NotController)?;

			ensure!(
				!<ValidatorSet<T>>::get().contains(&ddc_validator_pub),
				Error::<T>::VerificationKeyAlreadySet
			);
			ensure!(
				!<ValidatorToStashKey<T>>::contains_key(&ddc_validator_pub),
				Error::<T>::VerificationKeyAlreadyPaired
			);

			<ValidatorSet<T>>::append(ddc_validator_pub.clone());
			ValidatorToStashKey::<T>::insert(&ddc_validator_pub, &stash);

			Self::deposit_event(Event::<T>::ValidatorKeySet { validator: ddc_validator_pub });
			Ok(())
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
			if block_number.saturated_into::<u32>() % T::BLOCK_TO_START as u32 != 0 {
				return;
			}

			if !sp_io::offchain::is_validator() {
				return;
			}

			// Allow only one instance of the off-chain worker to run at the same time.
			let mut ocw_mutex = OcwMutex::new(OCW_MUTEX_ID.to_vec());
			if !ocw_mutex.try_lock() {
				log::debug!("Another inspection OCW is already running, terminating...",);
				return;
			}
			log::debug!("OCW mutex 0x{} locked.", hex::encode(ocw_mutex.local_storage_key()));

			let verification_account = unwrap_or_log_error!(
				Self::collect_verification_pub_key(),
				"❌ Error collecting validator verification key"
			);

			let signer = Signer::<T, T::OffchainIdentifierId>::any_account()
				.with_filter(vec![verification_account.public.clone()]);

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

				let inspection_result =
					Self::start_inspection_phase(&cluster_id, &verification_account, &signer);

				if let Err(errs) = inspection_result {
					errors.extend(errs);
				}

				let payout_result =
					Self::start_payouts_phase(&cluster_id, &verification_account, &signer);

				if let Err(errs) = payout_result {
					errors.extend(errs);
				}

				if T::DEBUG_MODE {
					Self::submit_errors(&errors, &verification_account, &signer);
				}
			}
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
		#[allow(clippy::collapsible_else_if)]
		pub(crate) fn start_inspection_phase(
			cluster_id: &ClusterId,
			verification_account: &Account<T>,
			_signer: &Signer<T, T::OffchainIdentifierId>,
		) -> Result<(), Vec<OCWError>> {
			let g_collectors = Self::get_g_collectors_nodes(cluster_id).map_err(|_| {
				vec![OCWError::FailedToFetchGCollectors { cluster_id: *cluster_id }]
			})?;
			// todo(yahortsaryk): infer the node deterministically
			let Some(g_collector) = g_collectors.first() else {
				log::warn!("⚠️ No Grouping Collector found in cluster {:?}", cluster_id);
				return Ok(());
			};

			if let Some(ehd_era) =
				Self::get_ehd_era_for_inspection(cluster_id, vec![g_collector.clone()].as_slice())
					.map_err(|e| vec![e])?
			{
				let tcaa_start =
					ehd_era.era_start.ok_or_else(|| vec![OCWError::TimeCapsuleError])?;
				let tcaa_end = ehd_era.era_end.ok_or_else(|| vec![OCWError::TimeCapsuleError])?;

				let ehd_root = Self::get_ehd_root(
					cluster_id,
					EHDId(*cluster_id, g_collector.0.clone(), ehd_era.id),
				)
				.map_err(|e| vec![e])?;

				let ehd_id = EHDId::try_from(ehd_root.ehd_id.clone()).map_err(|_| {
					vec![OCWError::FailedToParseEHDId { ehd_id: ehd_root.ehd_id.clone() }]
				})?;

				let mut phd_roots = vec![];
				for phd_id in &ehd_root.pdh_ids {
					let phd_id = PHDId::try_from(phd_id.clone()).map_err(|_| {
						vec![OCWError::FailedToParsePHDId { phd_id: phd_id.clone() }]
					})?;

					let phd_root = Self::get_phd_root(cluster_id, phd_id).map_err(|e| vec![e])?;
					phd_roots.push(phd_root.clone());
				}

				let nodes_inspection =
					Self::inspect_nodes_aggregates(cluster_id, &phd_roots, tcaa_start, tcaa_end)?;

				let buckets_inspection =
					Self::inspect_buckets_aggregates(cluster_id, &phd_roots, tcaa_start, tcaa_end)?;

				let payload = (
					Into::<String>::into(ehd_id.clone()),
					nodes_inspection.clone(),
					buckets_inspection.clone(),
				)
					.encode();

				let signature =
					<T::OffchainIdentifierId as AppCrypto<T::Public, T::Signature>>::sign(
						&payload,
						verification_account.public.clone(),
					)
					.ok_or([OCWError::FailedToSignInspectionReceipt])?;

				let inspector_pub_key: Vec<u8> = verification_account.public.encode();
				let inspector = format!("0x{}", hex::encode(&inspector_pub_key[1..])); // skip byte of SCALE encoding
				let signature = format!("0x{}", hex::encode(signature.encode()));

				let receipt = aggregator_client::json::InspectionReceipt {
					ehd_id: ehd_id.clone().into(),
					inspector,
					signature,
					nodes_inspection,
					buckets_inspection,
				};

				Self::send_inspection_receipt(cluster_id, g_collector, receipt)
					.map_err(|e| vec![e])?;

				Self::store_last_inspected_ehd(cluster_id, ehd_id);
			}

			Ok(())
		}

		#[allow(clippy::collapsible_else_if)]
		pub(crate) fn inspect_nodes_aggregates(
			cluster_id: &ClusterId,
			phd_roots: &Vec<aggregator_client::json::TraversedPHDResponse>,
			tcaa_start: DdcEra,
			tcaa_end: DdcEra,
		) -> Result<aggregator_client::json::InspectionResult, Vec<OCWError>> {
			#[allow(clippy::type_complexity)]
			let mut era_leaves_map: BTreeMap<
				NodePubKey, // node aggrehate key
				BTreeMap<(PHDId, (u64, u64)), BTreeMap<DdcEra, Vec<u64>>>,
			> = BTreeMap::new();

			let mut tcaas_map: BTreeMap<NodePubKey, DdcEra> = BTreeMap::new();

			for phd_root in phd_roots {
				let phd_id = PHDId::try_from(phd_root.phd_id.clone()).map_err(|_| {
					vec![OCWError::FailedToParsePHDId { phd_id: phd_root.phd_id.clone() }]
				})?;

				let collector_key = phd_id.0.clone();
				for node_aggregate in &phd_root.nodes_aggregates {
					let node_key =
						NodePubKey::try_from(node_aggregate.node_key.clone()).map_err(|_| {
							vec![OCWError::FailedToParseNodeKey {
								node_key: node_aggregate.node_key.clone(),
							}]
						})?;

					if tcaas_map.contains_key(&node_key.clone()) {
						// Currently, we inspect node aggregation from a single (first
						// responded) collector. We may compare the aggregation for the same
						// node between different collectors in the next iterations to ensure
						// redundancy.
						continue;
					}

					let mut node_leaves_count: u64 = 0;
					let mut node_tcaa_count: u64 = 0;

					let mut tcaa_leaves = BTreeMap::new();
					for tcaa_id in tcaa_start..=tcaa_end {
						if let Ok(challenge_res) = Self::fetch_node_challenge_response(
							cluster_id,
							tcaa_id,
							collector_key.clone(),
							node_key.clone(),
							vec![1],
						) {
							let tcaa_root = challenge_res
								.proofs
								.first()
								.ok_or([OCWError::FailedToFetchNodeChallenge])?;

							let tcaa_leaves_count = tcaa_root
								.usage
								.ok_or([OCWError::FailedToFetchNodeChallenge])?
								.puts + tcaa_root
								.usage
								.ok_or([OCWError::FailedToFetchNodeChallenge])?
								.gets;

							let ids = get_leaves_ids(tcaa_leaves_count);
							tcaa_leaves.insert(tcaa_id, ids);

							node_leaves_count += tcaa_leaves_count;
							node_tcaa_count += 1;

							tcaas_map.insert(node_key.clone(), tcaa_id);
						}
					}

					if node_tcaa_count > 0 {
						era_leaves_map.insert(
							node_key,
							BTreeMap::from([(
								(phd_id.clone(), (node_leaves_count, node_tcaa_count)),
								tcaa_leaves,
							)]),
						);
					}
				}
			}

			let mut unverified_phd_parts = vec![];
			let mut verified_phd_parts = vec![];

			let n0 = calculate_sample_size_inf(D_099, P_001);

			for (node_key, mut val) in era_leaves_map {
				let ((phd_id, (node_leaves_count, node_tcaa_count)), tcaa_leaves) =
					val.pop_first().ok_or([OCWError::NodesInspectionError])?;
				let collector_key = phd_id.0.clone();

				let n = match calculate_sample_size_fin(n0, node_leaves_count) {
					Ok(n) => n,
					Err(_) => continue,
				};

				let n_per_tcaa = n / node_tcaa_count;
				if n_per_tcaa == 0 {
					continue;
				}

				let mut remainder = n % node_tcaa_count;

				let mut verified_tcaas: BTreeMap<DdcEra, Vec<u64>> = BTreeMap::new();
				let mut unverified_tcaas: BTreeMap<DdcEra, Vec<u64>> = BTreeMap::new();

				for (tcaa_id, ids) in tcaa_leaves {
					let ids_count: u64 = ids.len().try_into().unwrap();

					let leaves_to_inspect = if n_per_tcaa < ids_count {
						let sample_size = if remainder > 0 && (n_per_tcaa + remainder) <= ids_count
						{
							let size = n_per_tcaa + remainder;
							remainder = 0;
							size
						} else {
							n_per_tcaa
						};

						Self::select_random_leaves(sample_size, ids, node_key.clone().into())
					} else {
						remainder += n_per_tcaa - ids_count;
						ids
					};

					log::info!(
						"Node {:?} - TCAA {:?}. Selecting {:?} leaves out of {:?} for inspection. Selected leaves {:?}. Additional reminder is {:?}.",
						node_key.clone(),
						tcaa_id,
						n_per_tcaa,
						ids_count,
						leaves_to_inspect,
						remainder
					);

					if let Ok(challenge_res) = Self::fetch_node_challenge_response(
						cluster_id,
						tcaa_id,
						collector_key.clone(),
						node_key.clone(),
						leaves_to_inspect.clone(),
					) {
						if challenge_res.verify() {
							if verified_tcaas.contains_key(&tcaa_id) {
								let mut verified_ids = verified_tcaas
									.get(&tcaa_id)
									.ok_or([OCWError::TimeCapsuleError])?
									.clone();
								verified_ids.extend(leaves_to_inspect);
								verified_tcaas.insert(tcaa_id, verified_ids.clone());
							} else {
								verified_tcaas.insert(tcaa_id, leaves_to_inspect);
							};
						} else {
							if unverified_tcaas.contains_key(&tcaa_id) {
								let mut unverified_ids = unverified_tcaas
									.get(&tcaa_id)
									.ok_or([OCWError::TimeCapsuleError])?
									.clone();
								unverified_ids.extend(leaves_to_inspect);
								unverified_tcaas.insert(tcaa_id, unverified_ids);
							} else {
								unverified_tcaas.insert(tcaa_id, leaves_to_inspect);
							};
						}
					}
				}

				if !verified_tcaas.is_empty() {
					verified_phd_parts.push(aggregator_client::json::InspectedTreePart {
						collector: phd_id.0.clone().into(),
						aggregate: AggregateKey::NodeAggregateKey(node_key.clone().into()),
						nodes: Vec::new(), /* todo: re-calculate aggregations in branch nodes
						                    * of PHD merkle tree */
						leafs: verified_tcaas,
					});
				}

				if !unverified_tcaas.is_empty() {
					unverified_phd_parts.push(aggregator_client::json::InspectedTreePart {
						collector: phd_id.0.clone().into(),
						aggregate: AggregateKey::NodeAggregateKey(node_key.clone().into()),
						nodes: Vec::new(), /* todo: re-calculate aggregations in branch nodes
						                    * of PHD merkle tree */
						leafs: unverified_tcaas,
					});
				}
			}

			let nodes_inspection_result = aggregator_client::json::InspectionResult {
				unverified_branches: unverified_phd_parts,
				verified_branches: verified_phd_parts,
			};

			Ok(nodes_inspection_result)
		}

		#[allow(clippy::collapsible_else_if)]
		pub(crate) fn inspect_buckets_aggregates(
			cluster_id: &ClusterId,
			phd_roots: &Vec<aggregator_client::json::TraversedPHDResponse>,
			tcaa_start: DdcEra,
			tcaa_end: DdcEra,
		) -> Result<aggregator_client::json::InspectionResult, Vec<OCWError>> {
			#[allow(clippy::type_complexity)]
			let mut era_leaves_map: BTreeMap<
				(BucketId, NodePubKey), // bucket sub-aggegate key
				BTreeMap<(PHDId, (u64, u64)), BTreeMap<DdcEra, Vec<u64>>>,
			> = BTreeMap::new();

			let mut tcaas_map: BTreeMap<(BucketId, NodePubKey), DdcEra> = BTreeMap::new();

			for phd_root in phd_roots {
				let phd_id = PHDId::try_from(phd_root.phd_id.clone()).map_err(|_| {
					vec![OCWError::FailedToParsePHDId { phd_id: phd_root.phd_id.clone() }]
				})?;

				let collector_key = phd_id.0.clone();

				for bucket_aggregate in &phd_root.buckets_aggregates {
					let bucket_id = bucket_aggregate.bucket_id;

					for node_aggregate in &phd_root.nodes_aggregates {
						let node_key = NodePubKey::try_from(node_aggregate.node_key.clone())
							.map_err(|_| {
								vec![OCWError::FailedToParseNodeKey {
									node_key: node_aggregate.node_key.clone(),
								}]
							})?;

						if tcaas_map.contains_key(&(bucket_id, node_key.clone())) {
							// Currently, we inspect node aggregation from a single (first
							// responded) collector. We may compare the aggregation for the same
							// node between different collectors in the next iterations to ensure
							// redundancy.
							continue;
						}

						let mut bucket_sub_leaves_count: u64 = 0;
						let mut bucket_sub_tcaa_count: u64 = 0;

						let mut tcaa_leaves = BTreeMap::new();
						for tcaa_id in tcaa_start..=tcaa_end {
							if let Ok(challenge_res) = Self::fetch_bucket_challenge_response(
								cluster_id,
								tcaa_id,
								collector_key.clone(),
								node_key.clone(),
								bucket_id,
								vec![1],
							) {
								let tcaa_root = challenge_res
									.proofs
									.first()
									.ok_or([OCWError::FailedToFetchBucketChallenge])?;

								let tcaa_leaves_count = tcaa_root
									.usage
									.ok_or([OCWError::FailedToFetchBucketChallenge])?
									.puts + tcaa_root
									.usage
									.ok_or([OCWError::FailedToFetchBucketChallenge])?
									.gets;

								let ids = get_leaves_ids(tcaa_leaves_count);
								tcaa_leaves.insert(tcaa_id, ids);

								bucket_sub_leaves_count += tcaa_leaves_count;
								bucket_sub_tcaa_count += 1;

								tcaas_map.insert((bucket_id, node_key.clone()), tcaa_id);
							}
						}

						if bucket_sub_tcaa_count > 0 {
							era_leaves_map.insert(
								(bucket_id, node_key.clone()),
								BTreeMap::from([(
									(
										phd_id.clone(),
										(bucket_sub_leaves_count, bucket_sub_tcaa_count),
									),
									tcaa_leaves,
								)]),
							);
						}
					}
				}
			}

			let mut unverified_phd_parts = vec![];
			let mut verified_phd_parts = vec![];

			let n0 = calculate_sample_size_inf(D_099, P_001);

			for ((bucket_id, node_key), mut val) in era_leaves_map {
				let ((phd_id, (bucket_sub_leaves_count, bucket_sub_tcaa_count)), tcaa_leaves) =
					val.pop_first().ok_or([OCWError::BucketsInspectionError])?;

				let collector_key = phd_id.0.clone();

				let n = match calculate_sample_size_fin(n0, bucket_sub_leaves_count) {
					Ok(n) => n,
					Err(_) => continue,
				};

				let n_per_tcaa = n / bucket_sub_tcaa_count;
				if n_per_tcaa == 0 {
					continue;
				}

				let mut remainder = n % bucket_sub_tcaa_count;

				let mut verified_tcaas: BTreeMap<DdcEra, Vec<u64>> = BTreeMap::new();
				let mut unverified_tcaas: BTreeMap<DdcEra, Vec<u64>> = BTreeMap::new();

				for (tcaa_id, ids) in tcaa_leaves {
					let ids_count: u64 = ids.len().try_into().unwrap();

					let leaves_to_inspect = if n_per_tcaa < ids_count {
						let sample_size = if remainder > 0 && (n_per_tcaa + remainder) <= ids_count
						{
							let size = n_per_tcaa + remainder;
							remainder = 0;
							size
						} else {
							n_per_tcaa
						};

						Self::select_random_leaves(sample_size, ids, node_key.clone().into())
					} else {
						remainder += n_per_tcaa - ids_count;
						ids
					};

					log::info!(
						"Bucket {:?}/{:?} - TCAA {:?}. Selecting {:?} leaves out of {:?} for inspection. Selected leaves {:?}. Additional reminder is {:?}.",
						bucket_id,
						node_key.clone(),
						tcaa_id,
						n_per_tcaa,
						ids_count,
						leaves_to_inspect,
						remainder
					);

					if let Ok(challenge_res) = Self::fetch_bucket_challenge_response(
						cluster_id,
						tcaa_id,
						collector_key.clone(),
						node_key.clone(),
						bucket_id,
						leaves_to_inspect.clone(),
					) {
						if challenge_res.verify() {
							if verified_tcaas.contains_key(&tcaa_id) {
								let mut verified_ids = verified_tcaas
									.get(&tcaa_id)
									.ok_or([OCWError::TimeCapsuleError])?
									.clone();
								verified_ids.extend(leaves_to_inspect);
								verified_tcaas.insert(tcaa_id, verified_ids.clone());
							} else {
								verified_tcaas.insert(tcaa_id, leaves_to_inspect);
							};
						} else {
							if unverified_tcaas.contains_key(&tcaa_id) {
								let mut unverified_ids = unverified_tcaas
									.get(&tcaa_id)
									.ok_or([OCWError::TimeCapsuleError])?
									.clone();
								unverified_ids.extend(leaves_to_inspect);
								unverified_tcaas.insert(tcaa_id, unverified_ids);
							} else {
								unverified_tcaas.insert(tcaa_id, leaves_to_inspect);
							};
						}
					}
				}

				if !verified_tcaas.is_empty() {
					verified_phd_parts.push(aggregator_client::json::InspectedTreePart {
						collector: phd_id.0.clone().into(),
						aggregate: AggregateKey::BucketSubAggregateKey(
							bucket_id,
							node_key.clone().into(),
						),
						nodes: Vec::new(), /* todo: re-calculate aggregations in branch nodes
						                    * of PHD merkle tree */
						leafs: verified_tcaas,
					});
				}

				if !unverified_tcaas.is_empty() {
					unverified_phd_parts.push(aggregator_client::json::InspectedTreePart {
						collector: phd_id.0.clone().into(),
						aggregate: AggregateKey::BucketSubAggregateKey(
							bucket_id,
							node_key.clone().into(),
						),
						nodes: Vec::new(), /* todo: re-calculate aggregations in branch nodes
						                    * of PHD merkle tree */
						leafs: unverified_tcaas,
					});
				}
			}

			let buckets_inspection_result = aggregator_client::json::InspectionResult {
				unverified_branches: unverified_phd_parts,
				verified_branches: verified_phd_parts,
			};

			Ok(buckets_inspection_result)
		}

		pub(crate) fn start_payouts_phase(
			cluster_id: &ClusterId,
			account: &Account<T>,
			signer: &Signer<T, T::OffchainIdentifierId>,
		) -> Result<(), Vec<OCWError>> {
			let mut errors: Vec<OCWError> = Vec::new();

			if let Err(errs) = Self::step_commit_payout_fingerprint(cluster_id, account, signer) {
				errors.extend(errs);
			}

			if let Err(errs) = Self::step_begin_payout(cluster_id, account, signer) {
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

			match Self::step_end_payout(cluster_id, account, signer) {
				Ok(Some(_ehd_id)) => {
					// Self::clear_verified_delta_usage(cluster_id, era_id);
				},
				Err(errs) => errors.extend(errs),
				_ => {},
			}

			if !errors.is_empty() {
				Err(errors)
			} else {
				Ok(())
			}
		}

		define_payout_step_function!(
			step_commit_payout_fingerprint,
			prepare_commit_payout_fingerprint,
			|cluster_id: &ClusterId, (ehd_id, ehd_payable_usage): (EHDId, PayableEHDUsage)| {
				Call::commit_payout_fingerprint {
					cluster_id: *cluster_id,
					ehd_id: ehd_id.into(),
					payers_root: ehd_payable_usage.payers_root,
					payees_root: ehd_payable_usage.payees_root,
				}
			},
			|prepared_data: &(EHDId, PayableEHDUsage)| prepared_data.0 .2,
			"🔑",
			|cluster_id: &ClusterId, (ehd_id, era_payable_usage): (EHDId, PayableEHDUsage)| {
				OCWError::CommitPayoutFingerprintTransactionError {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
					payers_root: era_payable_usage.payers_root,
					payees_root: era_payable_usage.payees_root,
				}
			}
		);

		define_payout_step_function!(
			step_begin_payout,
			prepare_begin_payout,
			|cluster_id: &ClusterId, (ehd_id, fingerprint): (EHDId, PayableUsageHash)| {
				Call::begin_payout { cluster_id: *cluster_id, era_id: ehd_id.2, fingerprint }
			},
			|prepared_data: &(EHDId, _)| prepared_data.0 .2,
			"🗓️ ",
			|cluster_id: &ClusterId, (ehd_id, _): (EHDId, _)| {
				OCWError::BeginPayoutTransactionError { cluster_id: *cluster_id, era_id: ehd_id.2 }
			}
		);

		define_payout_step_function!(
			step_begin_charging_customers,
			prepare_begin_charging_customers,
			|cluster_id: &ClusterId, (ehd_id, max_batch_index): (EHDId, u16)| {
				Call::begin_charging_customers {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
					max_batch_index,
				}
			},
			|prepared_data: &(EHDId, _)| prepared_data.0 .2,
			"📥",
			|cluster_id: &ClusterId, (ehd_id, _): (EHDId, _)| {
				OCWError::BeginChargingCustomersTransactionError {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
				}
			}
		);

		define_payout_step_function!(
			step_send_charging_customers,
			prepare_send_charging_customers_batch,
			|cluster_id: &ClusterId, (ehd_id, batch_payout): (EHDId, CustomerBatch)| {
				Call::send_charging_customers_batch {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
					batch_index: batch_payout.batch_index,
					payers: batch_payout
						.payers
						.into_iter()
						.map(|payer| {
							(
								T::AccountId::decode(&mut &payer.0.encode()[..])
									.expect("CustomerId to be parsed"),
								payer.1,
							)
						})
						.collect(),
					batch_proof: batch_payout.batch_proof.clone(),
				}
			},
			|prepared_data: &(EHDId, _)| prepared_data.0 .2,
			"🧾",
			|cluster_id: &ClusterId, (ehd_id, batch_payout): (EHDId, CustomerBatch)| {
				OCWError::SendChargingCustomersBatchTransactionError {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
					batch_index: batch_payout.batch_index,
				}
			}
		);

		define_payout_step_function!(
			step_end_charging_customers,
			prepare_end_charging_customers,
			|cluster_id: &ClusterId, ehd_id: EHDId| Call::end_charging_customers {
				cluster_id: *cluster_id,
				era_id: ehd_id.2
			},
			|prepared_data: &EHDId| prepared_data.2,
			"📪",
			|cluster_id: &ClusterId, ehd_id: EHDId| {
				OCWError::EndChargingCustomersTransactionError {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
				}
			}
		);

		define_payout_step_function!(
			step_begin_rewarding_providers,
			prepare_begin_rewarding_providers,
			|cluster_id: &ClusterId, (ehd_id, max_batch_index): (EHDId, u16)| {
				Call::begin_rewarding_providers {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
					max_batch_index,
				}
			},
			|prepared_data: &(EHDId, _)| prepared_data.0 .2,
			"📤",
			|cluster_id: &ClusterId, (ehd_id, _): (EHDId, _)| {
				OCWError::BeginRewardingProvidersTransactionError {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
				}
			}
		);

		define_payout_step_function!(
			step_send_rewarding_providers,
			prepare_send_rewarding_providers_batch,
			|cluster_id: &ClusterId, (ehd_id, batch_payout): (EHDId, ProviderBatch)| {
				Call::send_rewarding_providers_batch {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
					batch_index: batch_payout.batch_index,
					payees: batch_payout
						.payees
						.into_iter()
						.map(|payee| {
							(
								T::AccountId::decode(&mut &payee.0.encode()[..])
									.expect("ProviderId to be parsed"),
								payee.1,
							)
						})
						.collect(),
					batch_proof: batch_payout.batch_proof.clone(),
				}
			},
			|prepared_data: &(EHDId, _)| prepared_data.0 .2,
			"💸",
			|cluster_id: &ClusterId, (ehd_id, batch_payout): (EHDId, ProviderBatch)| {
				OCWError::SendRewardingProvidersBatchTransactionError {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
					batch_index: batch_payout.batch_index,
				}
			}
		);

		define_payout_step_function!(
			step_end_rewarding_providers,
			prepare_end_rewarding_providers,
			|cluster_id: &ClusterId, ehd_id: EHDId| Call::end_rewarding_providers {
				cluster_id: *cluster_id,
				era_id: ehd_id.2,
			},
			|prepared_data: &EHDId| prepared_data.2,
			"📭",
			|cluster_id: &ClusterId, ehd_id: EHDId| {
				OCWError::EndRewardingProvidersTransactionError {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
				}
			}
		);

		define_payout_step_function!(
			step_end_payout,
			prepare_end_payout,
			|cluster_id: &ClusterId, ehd_id: EHDId| Call::end_payout {
				cluster_id: *cluster_id,
				era_id: ehd_id.2,
			},
			|prepared_data: &EHDId| prepared_data.2,
			"🧮",
			|cluster_id: &ClusterId, ehd_id: EHDId| OCWError::EndPayoutTransactionError {
				cluster_id: *cluster_id,
				era_id: ehd_id.2,
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

		pub(crate) fn select_random_leaves(
			sample_size: u64,
			leaves_ids: Vec<u64>,
			nonce_key: String,
		) -> Vec<u64> {
			let nonce = Self::store_and_fetch_nonce(nonce_key);
			let mut small_rng = SmallRng::seed_from_u64(nonce);

			leaves_ids
				.choose_multiple(&mut small_rng, sample_size.try_into().unwrap())
				.cloned()
				.sorted()
				.collect::<Vec<u64>>()
		}

		pub(crate) fn build_and_store_ehd_payable_usage(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
		) -> Result<(), OCWError> {
			let batch_size = T::MAX_PAYOUT_BATCH_SIZE;

			// todo(yahortsaryk): infer g-collectors deterministically
			let g_collector = Self::get_g_collectors_nodes(cluster_id)
				.map_err(|_| OCWError::FailedToFetchGCollectors { cluster_id: *cluster_id })?
				.first()
				.cloned()
				.ok_or(OCWError::FailedToFetchGCollectors { cluster_id: *cluster_id })?;

			let ehd = Self::get_ehd_root(cluster_id, ehd_id.clone())?;

			let era =
				Self::fetch_processed_ehd_era_from_collector(cluster_id, ehd_id.2, &g_collector)?;

			let pricing = T::ClusterProtocol::get_pricing_params(cluster_id)
				.map_err(|_| OCWError::FailedToFetchProtocolParams { cluster_id: *cluster_id })?;

			let fees = T::ClusterProtocol::get_fees_params(cluster_id)
				.map_err(|_| OCWError::FailedToFetchProtocolParams { cluster_id: *cluster_id })?;

			let inspection_receipts = Self::fetch_inspection_receipts(cluster_id, ehd_id.clone())?;

			let customers_usage_cutoff = if !T::DISABLE_PAYOUTS_CUTOFF {
				Self::calculate_customers_usage_cutoff(cluster_id, &inspection_receipts)?
			} else {
				Default::default()
			};

			let (payers, cluster_costs) = Self::calculate_ehd_customers_charges(
				&ehd.customers,
				&customers_usage_cutoff,
				&pricing,
				era.time_start.ok_or(OCWError::TimeCapsuleError)?,
				era.time_end.ok_or(OCWError::TimeCapsuleError)?,
			)
			.map_err(|_| OCWError::FailedToCalculatePayersBatches { era_id: ehd_id.2 })?;

			let cluster_usage = ehd.get_cluster_usage();

			let providers_usage_cutoff = if !T::DISABLE_PAYOUTS_CUTOFF {
				Self::calculate_providers_usage_cutoff(cluster_id, &inspection_receipts)?
			} else {
				Default::default()
			};

			let payees = Self::calculate_ehd_providers_rewards(
				&ehd.providers,
				&providers_usage_cutoff,
				&fees,
				&cluster_usage,
				&cluster_costs,
			)
			.map_err(|_| OCWError::FailedToCalculatePayeesBatches { era_id: ehd_id.2 })?;

			let payers_batch_roots = Self::convert_to_batch_merkle_roots(
				cluster_id,
				Self::split_to_batches(&payers, batch_size.into()),
				ehd_id.2,
			)?;

			let payees_batch_roots = Self::convert_to_batch_merkle_roots(
				cluster_id,
				Self::split_to_batches(&payees, batch_size.into()),
				ehd_id.2,
			)?;

			let payers_root = Self::create_merkle_root(cluster_id, &payers_batch_roots, ehd_id.2)?;
			let payees_root = Self::create_merkle_root(cluster_id, &payees_batch_roots, ehd_id.2)?;

			Self::store_ehd_payable_usage(
				cluster_id,
				ehd_id,
				payers,
				payers_root,
				payers_batch_roots,
				payees,
				payees_root,
				payees_batch_roots,
			);

			Ok(())
		}

		fn fetch_ehd_payable_usage_or_retry(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
		) -> Result<PayableEHDUsage, Vec<OCWError>> {
			if let Some(ehd_paybale_usage) =
				Self::fetch_ehd_payable_usage(cluster_id, ehd_id.clone())
			{
				Ok(ehd_paybale_usage)
			} else {
				Self::build_and_store_ehd_payable_usage(cluster_id, ehd_id.clone())
					.map_err(|e| [e])?;
				if let Some(ehd_paybale_usage) = Self::fetch_ehd_payable_usage(cluster_id, ehd_id) {
					Ok(ehd_paybale_usage)
				} else {
					Err(vec![OCWError::FailedToFetchVerifiedPayableUsage])
				}
			}
		}

		fn calculate_ehd_customers_charges(
			customers: &Vec<aggregator_client::json::TraversedEHDCustomer>,
			cutoff_usage_map: &BTreeMap<T::AccountId, BucketUsage>,
			pricing: &ClusterPricingParams,
			time_start: i64,
			time_end: i64,
		) -> Result<(Vec<CustomerCharge>, CustomerCosts), ArithmeticError> {
			let mut customers_charges = Vec::new();
			let mut cluster_costs = CustomerCosts::default();

			for customer in customers {
				if let Ok(customer_id) = customer.parse_customer_id() {
					let customer_costs = Self::get_customer_costs(
						pricing,
						&customer.consumed_usage,
						cutoff_usage_map.get(
							&T::AccountId::decode(&mut &customer_id.encode()[..])
								.map_err(|_| ArithmeticError::Overflow)?,
						),
						time_start,
						time_end,
					)?;

					// todo: cut off unverified activity of buckets if it is detected during
					// inspection

					cluster_costs.storage = cluster_costs
						.storage
						.checked_add(customer_costs.storage)
						.ok_or(ArithmeticError::Overflow)?;

					cluster_costs.transfer = cluster_costs
						.transfer
						.checked_add(customer_costs.transfer)
						.ok_or(ArithmeticError::Overflow)?;

					cluster_costs.puts = cluster_costs
						.puts
						.checked_add(customer_costs.puts)
						.ok_or(ArithmeticError::Overflow)?;

					cluster_costs.gets = cluster_costs
						.gets
						.checked_add(customer_costs.gets)
						.ok_or(ArithmeticError::Overflow)?;

					let charge_amount = (|| -> Option<u128> {
						customer_costs
							.transfer
							.checked_add(customer_costs.storage)?
							.checked_add(customer_costs.puts)?
							.checked_add(customer_costs.gets)
					})()
					.ok_or(ArithmeticError::Overflow)?;

					customers_charges.push(CustomerCharge(customer_id, charge_amount));
				}
			}

			Ok((customers_charges, cluster_costs))
		}

		#[allow(clippy::field_reassign_with_default)]
		fn get_customer_costs(
			pricing: &ClusterPricingParams,
			consumed_usage: &aggregator_client::json::EHDUsage,
			cutoff_usage: Option<&BucketUsage>,
			time_start: i64,
			time_end: i64,
		) -> Result<CustomerCosts, ArithmeticError> {
			#[allow(clippy::field_reassign_with_default)]
			let mut customer_costs = CustomerCosts::default();
			let no_cutoff = Default::default();
			let cutoff = cutoff_usage.unwrap_or(&no_cutoff);

			customer_costs.transfer = (consumed_usage.transferred_bytes as u128)
				.checked_sub(cutoff.transferred_bytes as u128)
				.ok_or(ArithmeticError::Underflow)?
				.checked_mul(pricing.unit_per_mb_streamed)
				.ok_or(ArithmeticError::Overflow)?
				.checked_div(byte_unit::MEBIBYTE)
				.ok_or(ArithmeticError::Underflow)?;

			// Calculate the duration of the period in seconds
			let duration_seconds = time_end - time_start;
			let fraction_of_month =
				Perquintill::from_rational(duration_seconds as u64, AVG_SECONDS_MONTH as u64);

			customer_costs.storage = fraction_of_month *
				((consumed_usage.stored_bytes as u128)
					.checked_sub(cutoff.stored_bytes as u128)
					.ok_or(ArithmeticError::Underflow)?
					.checked_mul(pricing.unit_per_mb_stored)
					.ok_or(ArithmeticError::Overflow)?
					.checked_div(byte_unit::MEBIBYTE)
					.ok_or(ArithmeticError::Underflow)?);

			customer_costs.gets = (consumed_usage.number_of_gets as u128)
				.checked_sub(cutoff.number_of_gets as u128)
				.ok_or(ArithmeticError::Underflow)?
				.checked_mul(pricing.unit_per_get_request)
				.ok_or(ArithmeticError::Overflow)?;

			customer_costs.puts = (consumed_usage.number_of_puts as u128)
				.checked_sub(cutoff.number_of_puts as u128)
				.ok_or(ArithmeticError::Underflow)?
				.checked_mul(pricing.unit_per_put_request)
				.ok_or(ArithmeticError::Overflow)?;

			Ok(customer_costs)
		}

		fn get_provider_profits(
			provided_usage: &aggregator_client::json::EHDUsage,
			cutoff_usage: Option<&NodeUsage>,
			cluster_usage: &aggregator_client::json::EHDUsage,
			cluster_costs: &CustomerCosts,
		) -> Result<ProviderProfits, ArithmeticError> {
			let mut provider_profits = ProviderProfits::default();
			let no_cutoff = Default::default();
			let cutoff = cutoff_usage.unwrap_or(&no_cutoff);

			let mut ratio = Perquintill::from_rational(
				(provided_usage.transferred_bytes as u128)
					.checked_sub(cutoff.transferred_bytes as u128)
					.ok_or(ArithmeticError::Underflow)?,
				cluster_usage.transferred_bytes as u128,
			);

			// ratio multiplied by X will be > 0, < X no overflow
			provider_profits.transfer = ratio * cluster_costs.transfer;

			ratio = Perquintill::from_rational(
				(provided_usage.stored_bytes as u128)
					.checked_sub(cutoff.stored_bytes as u128)
					.ok_or(ArithmeticError::Underflow)?,
				cluster_usage.stored_bytes as u128,
			);
			provider_profits.storage = ratio * cluster_costs.storage;

			ratio = Perquintill::from_rational(
				provided_usage
					.number_of_puts
					.checked_sub(cutoff.number_of_puts)
					.ok_or(ArithmeticError::Underflow)?,
				cluster_usage.number_of_puts,
			);
			provider_profits.puts = ratio * cluster_costs.puts;

			ratio = Perquintill::from_rational(
				provided_usage
					.number_of_gets
					.checked_sub(cutoff.number_of_gets)
					.ok_or(ArithmeticError::Underflow)?,
				cluster_usage.number_of_gets,
			);
			provider_profits.gets = ratio * cluster_costs.gets;

			Ok(provider_profits)
		}

		fn calculate_ehd_providers_rewards(
			providers: &Vec<aggregator_client::json::TraversedEHDProvider>,
			cutoff_usage_map: &BTreeMap<T::AccountId, NodeUsage>,
			fees: &ClusterFeesParams,
			cluster_usage: &aggregator_client::json::EHDUsage,
			cluster_costs: &CustomerCosts,
		) -> Result<Vec<ProviderReward>, ArithmeticError> {
			let mut providers_profits = Vec::new();

			for provider in providers {
				if let Ok(provider_id) = provider.parse_provider_id() {
					let provider_profits = Self::get_provider_profits(
						&provider.provided_usage,
						cutoff_usage_map.get(
							&T::AccountId::decode(&mut &provider_id.encode()[..])
								.map_err(|_| ArithmeticError::Overflow)?,
						),
						cluster_usage,
						cluster_costs,
					)?;

					let reward_amount = (|| -> Option<u128> {
						provider_profits
							.transfer
							.checked_add(provider_profits.storage)?
							.checked_add(provider_profits.puts)?
							.checked_add(provider_profits.gets)
					})()
					.ok_or(ArithmeticError::Overflow)?;

					let treasury_fee_amount = fees.treasury_share * reward_amount;
					let validators_fee_amount = fees.validators_share * reward_amount;
					let cluster_reserve_fee_amount = fees.cluster_reserve_share * reward_amount;

					let profit_amount = (|| -> Option<u128> {
						reward_amount
							.checked_sub(treasury_fee_amount)?
							.checked_sub(validators_fee_amount)?
							.checked_sub(cluster_reserve_fee_amount)
					})()
					.ok_or(ArithmeticError::Overflow)?;

					providers_profits.push(ProviderReward(provider_id, profit_amount));
				}
			}

			Ok(providers_profits)
		}

		pub(crate) fn prepare_commit_payout_fingerprint(
			cluster_id: &ClusterId,
		) -> Result<Option<(EHDId, PayableEHDUsage)>, Vec<OCWError>> {
			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				let era_payable_usage =
					Self::fetch_ehd_payable_usage_or_retry(cluster_id, ehd_id.clone())?;
				Ok(Some((ehd_id, era_payable_usage)))
			} else {
				Ok(None)
			}
		}

		#[allow(dead_code)]
		pub(crate) fn prepare_begin_payout(
			cluster_id: &ClusterId,
		) -> Result<Option<(EHDId, Fingerprint)>, Vec<OCWError>> {
			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				let ehd_payable_usage =
					Self::fetch_ehd_payable_usage_or_retry(cluster_id, ehd_id.clone())?;
				Ok(Some((ehd_id, ehd_payable_usage.fingerprint())))
			} else {
				Ok(None)
			}
		}

		pub(crate) fn prepare_begin_charging_customers(
			cluster_id: &ClusterId,
		) -> Result<Option<(EHDId, BatchIndex)>, Vec<OCWError>> {
			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				if T::PayoutProcessor::get_payout_state(cluster_id, ehd_id.2) ==
					PayoutState::Initialized
				{
					let era_payable_usage =
						Self::fetch_ehd_payable_usage_or_retry(cluster_id, ehd_id.clone())?;
					Self::fetch_ehd_charging_loop_input(
						cluster_id,
						ehd_id,
						era_payable_usage.payers_batch_roots,
					)
				} else {
					Ok(None)
				}
			} else {
				Ok(None)
			}
		}

		pub(crate) fn fetch_ehd_charging_loop_input(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
			payers_batch_roots: Vec<PayableUsageHash>,
		) -> Result<Option<(EHDId, BatchIndex)>, Vec<OCWError>> {
			if let Some(max_batch_index) = payers_batch_roots.len().checked_sub(1) {
				let max_batch_index: u16 = max_batch_index.try_into().map_err(|_| {
					vec![OCWError::BatchIndexOverflow { cluster_id: *cluster_id, era_id: ehd_id.2 }]
				})?;
				Ok(Some((ehd_id, max_batch_index)))
			} else {
				Err(vec![OCWError::BatchIndexUnderflow {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
				}])
			}
		}

		pub(crate) fn prepare_send_charging_customers_batch(
			cluster_id: &ClusterId,
		) -> Result<Option<(EHDId, CustomerBatch)>, Vec<OCWError>> {
			let batch_size = T::MAX_PAYOUT_BATCH_SIZE;

			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				if T::PayoutProcessor::get_payout_state(cluster_id, ehd_id.2) ==
					PayoutState::ChargingCustomers
				{
					let era_payable_usage =
						Self::fetch_ehd_payable_usage_or_retry(cluster_id, ehd_id.clone())?;
					Self::fetch_charging_customers_batch(
						cluster_id,
						batch_size.into(),
						ehd_id,
						era_payable_usage.payers_usage,
						era_payable_usage.payers_batch_roots,
					)
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
			ehd_id: EHDId,
			payers_usage: Vec<CustomerCharge>,
			payers_batch_roots: Vec<PayableUsageHash>,
		) -> Result<Option<(EHDId, CustomerBatch)>, Vec<OCWError>> {
			let batch_index = T::PayoutProcessor::get_next_customers_batch(cluster_id, ehd_id.2)
				.map_err(|_| {
					vec![OCWError::PayoutReceiptDoesNotExist {
						cluster_id: *cluster_id,
						era_id: ehd_id.2,
					}]
				})?;

			if let Some(index) = batch_index {
				let i: usize = index.into();
				// todo! store batched activity to avoid splitting it again each time
				let payers_batches = Self::split_to_batches(&payers_usage, batch_size);

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
						era_id: ehd_id.2,
					})
					.map_err(|e| vec![e])?
					.proof_items()
					.to_vec();

				let batch_proof = MMRProof { proof };
				Ok(Some((
					ehd_id.clone(),
					CustomerBatch {
						batch_index: index,
						payers: payers_batches[i].clone(),
						batch_proof,
					},
				)))
			} else {
				Ok(None)
			}
		}

		pub(crate) fn prepare_end_charging_customers(
			cluster_id: &ClusterId,
		) -> Result<Option<EHDId>, Vec<OCWError>> {
			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				if T::PayoutProcessor::get_payout_state(cluster_id, ehd_id.2) ==
					PayoutState::ChargingCustomers &&
					T::PayoutProcessor::is_customers_charging_finished(cluster_id, ehd_id.2)
				{
					return Ok(Some(ehd_id));
				}
			}
			Ok(None)
		}

		pub(crate) fn prepare_begin_rewarding_providers(
			cluster_id: &ClusterId,
		) -> Result<Option<(EHDId, BatchIndex)>, Vec<OCWError>> {
			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				if T::PayoutProcessor::get_payout_state(cluster_id, ehd_id.2) ==
					PayoutState::CustomersChargedWithFees
				{
					let ehd_payable_usage =
						Self::fetch_ehd_payable_usage_or_retry(cluster_id, ehd_id.clone())?;
					Self::fetch_ehd_rewarding_loop_input(
						cluster_id,
						ehd_id,
						ehd_payable_usage.payees_batch_roots,
					)
				} else {
					Ok(None)
				}
			} else {
				Ok(None)
			}
		}

		pub(crate) fn fetch_ehd_rewarding_loop_input(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
			payees_batch_roots: Vec<PayableUsageHash>,
		) -> Result<Option<(EHDId, BatchIndex)>, Vec<OCWError>> {
			if let Some(max_batch_index) = payees_batch_roots.len().checked_sub(1) {
				let max_batch_index: u16 = max_batch_index.try_into().map_err(|_| {
					vec![OCWError::BatchIndexOverflow { cluster_id: *cluster_id, era_id: ehd_id.2 }]
				})?;

				Ok(Some((ehd_id, max_batch_index)))
			} else {
				Err(vec![OCWError::BatchIndexUnderflow {
					cluster_id: *cluster_id,
					era_id: ehd_id.2,
				}])
			}
		}

		pub(crate) fn prepare_send_rewarding_providers_batch(
			cluster_id: &ClusterId,
		) -> Result<Option<(EHDId, ProviderBatch)>, Vec<OCWError>> {
			let batch_size = T::MAX_PAYOUT_BATCH_SIZE;

			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				if T::PayoutProcessor::get_payout_state(cluster_id, ehd_id.2) ==
					PayoutState::RewardingProviders
				{
					let era_payable_usage =
						Self::fetch_ehd_payable_usage_or_retry(cluster_id, ehd_id.clone())?;
					Self::fetch_rewarding_providers_batch(
						cluster_id,
						batch_size.into(),
						ehd_id,
						era_payable_usage.payees_usage,
						era_payable_usage.payees_batch_roots,
					)
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
			ehd_id: EHDId,
			payees_usage: Vec<ProviderReward>,
			payees_batch_roots: Vec<PayableUsageHash>,
		) -> Result<Option<(EHDId, ProviderBatch)>, Vec<OCWError>> {
			let batch_index = T::PayoutProcessor::get_next_providers_batch(cluster_id, ehd_id.2)
				.map_err(|_| {
					vec![OCWError::PayoutReceiptDoesNotExist {
						cluster_id: *cluster_id,
						era_id: ehd_id.2,
					}]
				})?;

			if let Some(index) = batch_index {
				let i: usize = index.into();
				// todo! store batched activity to avoid splitting it again each time
				let nodes_activity_batched = Self::split_to_batches(&payees_usage, batch_size);

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
							era_id: ehd_id.2,
						}]
					})?
					.proof_items()
					.to_vec();

				let batch_proof = MMRProof { proof };
				Ok(Some((
					ehd_id,
					ProviderBatch {
						batch_index: index,
						payees: nodes_activity_batched[i].clone(),
						batch_proof,
					},
				)))
			} else {
				Ok(None)
			}
		}

		pub(crate) fn prepare_end_rewarding_providers(
			cluster_id: &ClusterId,
		) -> Result<Option<EHDId>, Vec<OCWError>> {
			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				if T::PayoutProcessor::get_payout_state(cluster_id, ehd_id.2) ==
					PayoutState::RewardingProviders &&
					T::PayoutProcessor::is_providers_rewarding_finished(cluster_id, ehd_id.2)
				{
					return Ok(Some(ehd_id));
				}
			}
			Ok(None)
		}

		pub(crate) fn prepare_end_payout(
			cluster_id: &ClusterId,
		) -> Result<Option<EHDId>, Vec<OCWError>> {
			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				if T::PayoutProcessor::get_payout_state(cluster_id, ehd_id.2) ==
					PayoutState::ProvidersRewarded
				{
					return Ok(Some(ehd_id));
				}
			}
			Ok(None)
		}

		pub(crate) fn derive_ehd_paybale_usage_key(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
		) -> Vec<u8> {
			format!("offchain::paybale_usage::{:?}::{:?}", cluster_id, Into::<String>::into(ehd_id))
				.into_bytes()
		}

		pub(crate) fn collect_verification_pub_key() -> Result<Account<T>, OCWError> {
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

			if session_verification_keys.len() != 1 {
				log::error!(
					"🚨 Unexpected number of session verification keys is found. Expected: 1, Actual: {:?}",
					session_verification_keys.len()
				);
				return Err(OCWError::FailedToCollectVerificationKey);
			}

			session_verification_keys
				.into_iter()
				.next() // first
				.ok_or(OCWError::FailedToCollectVerificationKey)
		}

		pub(crate) fn store_verification_account_id(account_id: T::AccountId) {
			let validator: Vec<u8> = account_id.encode();
			let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();
			local_storage_set(StorageKind::PERSISTENT, &key, &validator);
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
			}
		}

		#[allow(clippy::too_many_arguments)]
		pub(crate) fn store_ehd_payable_usage(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
			payers_usage: Vec<CustomerCharge>,
			payers_root: PayableUsageHash,
			payers_batch_roots: Vec<PayableUsageHash>,
			payees_usage: Vec<ProviderReward>,
			payees_root: PayableUsageHash,
			payees_batch_roots: Vec<PayableUsageHash>,
		) {
			let key = Self::derive_ehd_paybale_usage_key(cluster_id, ehd_id.clone());

			let ehd_paybale_usage = PayableEHDUsage {
				cluster_id: *cluster_id,
				ehd_id: ehd_id.into(),
				payers_usage,
				payers_root,
				payers_batch_roots,
				payees_usage,
				payees_root,
				payees_batch_roots,
			};
			let encoded_ehd_paybale_usage = ehd_paybale_usage.encode();

			// Store the serialized data in local offchain storage
			local_storage_set(StorageKind::PERSISTENT, &key, &encoded_ehd_paybale_usage);
		}

		#[allow(clippy::type_complexity)]
		pub(crate) fn fetch_ehd_payable_usage(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
		) -> Option<PayableEHDUsage> {
			log::info!(
				"🪙🏠 Off-chain cache hit for Payable Usage in cluster_id: {:?} ehd_id: {:?}",
				cluster_id,
				ehd_id
			);
			let key = Self::derive_ehd_paybale_usage_key(cluster_id, ehd_id);

			let encoded_ehd_paybale_usage = match local_storage_get(StorageKind::PERSISTENT, &key) {
				Some(encoded_data) => encoded_data,
				None => return None,
			};

			match Decode::decode(&mut &encoded_ehd_paybale_usage[..]) {
				Ok(ehd_paybale_usage) => Some(ehd_paybale_usage),
				Err(err) => {
					log::error!("Decoding error: {:?}", err);
					None
				},
			}
		}

		pub(crate) fn store_and_fetch_nonce(node_id: String) -> u64 {
			let key = format!("offchain::activities::nonce::{:?}", node_id).into_bytes();
			let encoded_nonce =
				local_storage_get(StorageKind::PERSISTENT, &key).unwrap_or_else(|| 0.encode());

			let nonce_data = match Decode::decode(&mut &encoded_nonce[..]) {
				Ok(nonce) => nonce,
				Err(err) => {
					log::error!("Decoding error while fetching nonce: {:?}", err);
					0
				},
			};

			let new_nonce = nonce_data + 1;

			local_storage_set(StorageKind::PERSISTENT, &key, &new_nonce.encode());
			nonce_data
		}

		/// Converts a vector of hashable batches into their corresponding Merkle roots.
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
		/// - `Vec<H256>`: A vector of Merkle roots, one for each batch of items.
		pub(crate) fn convert_to_batch_merkle_roots<A: Hashable>(
			cluster_id: &ClusterId,
			batches: Vec<Vec<A>>,
			era_id: EhdEra,
		) -> Result<Vec<H256>, OCWError> {
			batches
				.into_iter()
				.map(|batch| {
					let activity_hashes: Vec<H256> =
						batch.into_iter().map(|a| a.hash::<T>()).collect();
					Self::create_merkle_root(cluster_id, &activity_hashes, era_id).map_err(|_| {
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
		pub(crate) fn split_to_batches<A: Ord + Clone>(
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
			leaves: &[H256],
			era_id: EhdEra,
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

		pub(crate) fn get_ehd_root(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
		) -> Result<aggregator_client::json::TraversedEHDResponse, OCWError> {
			Self::fetch_traversed_era_historical_document(cluster_id, ehd_id, 1, 1)?
				.first()
				.ok_or(OCWError::FailedToFetchTraversedEHD)
				.cloned()
		}

		pub(crate) fn get_phd_root(
			cluster_id: &ClusterId,
			phd_id: PHDId,
		) -> Result<aggregator_client::json::TraversedPHDResponse, OCWError> {
			Self::fetch_traversed_partial_historical_document(cluster_id, phd_id, 1, 1)?
				.first()
				.ok_or(OCWError::FailedToFetchTraversedPHD)
				.cloned()
		}

		pub(crate) fn get_tcaa_bucket_usage(
			cluster_id: &ClusterId,
			collector_key: NodePubKey,
			tcaa_id: DdcEra,
			node_key: NodePubKey,
			bucket_id: BucketId,
		) -> Result<BucketUsage, OCWError> {
			let challenge_res = Self::fetch_bucket_challenge_response(
				cluster_id,
				tcaa_id,
				collector_key,
				node_key,
				bucket_id,
				vec![1],
			)?;

			let tcaa_root =
				challenge_res.proofs.first().ok_or(OCWError::FailedToFetchBucketChallenge)?;
			let tcaa_usage = tcaa_root.usage.ok_or(OCWError::FailedToFetchBucketChallenge)?;

			Ok(BucketUsage {
				stored_bytes: tcaa_usage.stored,
				transferred_bytes: tcaa_usage.delivered,
				number_of_puts: tcaa_usage.puts,
				number_of_gets: tcaa_usage.gets,
			})
		}

		pub(crate) fn get_tcaa_node_usage(
			cluster_id: &ClusterId,
			collector_key: NodePubKey,
			tcaa_id: DdcEra,
			node_key: NodePubKey,
		) -> Result<NodeUsage, OCWError> {
			let challenge_res = Self::fetch_node_challenge_response(
				cluster_id,
				tcaa_id,
				collector_key,
				node_key,
				vec![1],
			)?;

			let tcaa_root =
				challenge_res.proofs.first().ok_or(OCWError::FailedToFetchNodeChallenge)?;
			let tcaa_usage = tcaa_root.usage.ok_or(OCWError::FailedToFetchNodeChallenge)?;

			Ok(NodeUsage {
				stored_bytes: tcaa_usage.stored,
				transferred_bytes: tcaa_usage.delivered,
				number_of_puts: tcaa_usage.puts,
				number_of_gets: tcaa_usage.gets,
			})
		}

		pub(crate) fn get_ehd_id_for_payout(cluster_id: &ClusterId) -> Option<EHDId> {
			match T::ClusterValidator::get_last_paid_era(cluster_id) {
				Ok(last_paid_era_for_cluster) => {
					if let Some(inspected_ehds) = Self::fetch_last_inspected_ehds(cluster_id) {
						for inspected_ehd in inspected_ehds.clone().into_iter().sorted() {
							if inspected_ehd.2 > last_paid_era_for_cluster {
								let ehd_root =
									Self::get_ehd_root(cluster_id, inspected_ehd.clone()).ok()?;

								let cluster_usage = ehd_root.get_cluster_usage();
								if cluster_usage == Default::default() {
									continue;
								}

								let receipts_by_inspector = Self::fetch_inspection_receipts(
									cluster_id,
									inspected_ehd.clone(),
								)
								.map_err(|e| vec![e])
								.ok()?;

								let inspectors_quorum = T::ValidatorsQuorum::get();
								let threshold = inspectors_quorum * <ValidatorSet<T>>::get().len();

								if threshold <= receipts_by_inspector.len() {
									return Some(inspected_ehd);
								}
							}
						}
					}
					None
				},
				Err(_) => None,
			}
		}

		pub(crate) fn get_last_inspected_ehd(cluster_id: &ClusterId) -> Option<EHDId> {
			if let Some(last_inspected_ehds) = Self::fetch_last_inspected_ehds(cluster_id) {
				last_inspected_ehds.iter().max_by_key(|ehd| ehd.2).cloned()
			} else {
				None
			}
		}

		pub(crate) fn derive_last_inspected_ehd_key(cluster_id: &ClusterId) -> Vec<u8> {
			format!("offchain::inspected_ehds::v1::{:?}", cluster_id).into_bytes()
		}

		pub(crate) fn store_last_inspected_ehd(cluster_id: &ClusterId, ehd_id: EHDId) {
			let key = Self::derive_last_inspected_ehd_key(cluster_id);

			if let Some(mut inspected_ehds) = Self::fetch_last_inspected_ehds(cluster_id) {
				inspected_ehds.push(ehd_id);

				let encoded_ehds_ids =
					inspected_ehds.into_iter().sorted().collect::<Vec<EHDId>>().encode();
				local_storage_set(StorageKind::PERSISTENT, &key, &encoded_ehds_ids);
			} else {
				log::warn!(
					"🗄️  Failed to retrieve last inspected ehds from offchain storage for cluster_id: {:?}",
					cluster_id,
				);
			}
		}

		pub(crate) fn fetch_last_inspected_ehds(cluster_id: &ClusterId) -> Option<Vec<EHDId>> {
			log::info!("🗄️  Trying to fetch last inspected ehds for cluster_id: {:?}", cluster_id,);

			let key = Self::derive_last_inspected_ehd_key(cluster_id);

			let encoded_last_inspected_ehd: Vec<u8> =
				match local_storage_get(StorageKind::PERSISTENT, &key) {
					Some(encoded_data) => encoded_data,
					None => return Some(vec![]),
				};

			match Decode::decode(&mut &encoded_last_inspected_ehd[..]) {
				Ok(last_inspected_ehd) => Some(last_inspected_ehd),
				Err(err) => {
					log::error!("🗄️  Error occured while decoding last inspected ehds in cluster_id: {:?} {:?}", cluster_id, err);
					None
				},
			}
		}

		/// Fetch current era across all DAC nodes to validate.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `g_collectors`: List of G-Collectors nodes
		pub(crate) fn get_ehd_era_for_inspection(
			cluster_id: &ClusterId,
			g_collectors: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Option<aggregator_client::json::EHDEra>, OCWError> {
			let _this_validator = Self::fetch_verification_account_id()?;

			let last_validated_ehd_by_this_validator = Self::get_last_inspected_ehd(cluster_id);

			let last_validated_era_by_this_validator: EhdEra =
				if let Some(ehd) = last_validated_ehd_by_this_validator {
					ehd.2
				} else {
					Default::default()
				};

			let last_paid_era_for_cluster = T::ClusterValidator::get_last_paid_era(cluster_id)
				.map_err(|_| OCWError::PaidEraRetrievalError { cluster_id: *cluster_id })?;

			log::info!(
				"👁️‍🗨️  The last era inspected by this specific validator for cluster_id: {:?} is {:?}. The last paid era for the cluster is {:?}",
				cluster_id,
				last_validated_era_by_this_validator,
				last_paid_era_for_cluster
			);

			// we want to fetch processed eras from all available validators
			let available_processed_ehd_eras =
				Self::fetch_processed_ehd_eras_from_collector(cluster_id, g_collectors)?;

			// we want to let the current validator to validate available processed/completed eras
			// that are greater than the last validated era in the cluster
			let processed_ehd_eras_to_inspect: Vec<aggregator_client::json::EHDEra> =
				available_processed_ehd_eras
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
					.collect::<Vec<aggregator_client::json::EHDEra>>();

			// We want to process only eras reported by quorum of validators
			let mut processed_ehd_eras_with_quorum: Vec<aggregator_client::json::EHDEra> = vec![];

			// let quorum = T::AggregatorsQuorum::get();
			// let threshold = quorum * collectors_nodes.len();

			// At the moment we have only one G-collector
			let threshold = 1;
			for (era_key, candidates) in
				&processed_ehd_eras_to_inspect.into_iter().chunk_by(|elt| elt.clone())
			{
				let count = candidates.count();
				if count >= threshold {
					processed_ehd_eras_with_quorum.push(era_key);
				} else {
					log::warn!(
						"⚠️  Era {:?} in cluster_id: {:?} has been reported with unmet quorum. Desired: {:?} Actual: {:?}",
						era_key,
						cluster_id,
						threshold,
						count
					);
				}
			}

			let ehd_era_to_inspect =
				processed_ehd_eras_with_quorum.iter().cloned().min_by_key(|n| n.id);

			log::info!(
				"👁️‍🗨️  Era {:?} has been selected for inspection in cluster_id: {:?}",
				ehd_era_to_inspect,
				cluster_id,
			);

			Ok(ehd_era_to_inspect)
		}

		/// Fetch collectors nodes of a cluster.
		/// Parameters:
		/// - `cluster_id`: Cluster id of a cluster.
		fn get_collectors_nodes(
			cluster_id: &ClusterId,
		) -> Result<Vec<(NodePubKey, StorageNodeParams)>, Error<T>> {
			let mut collectors = Vec::new();

			let nodes = T::ClusterManager::get_nodes(cluster_id)
				.map_err(|_| Error::<T>::NodeRetrievalError)?;

			for node_pub_key in nodes {
				if let Ok(NodeParams::StorageParams(storage_params)) =
					T::NodeManager::get_node_params(&node_pub_key)
				{
					collectors.push((node_pub_key, storage_params));
				}
			}

			Ok(collectors)
		}

		/// Fetch grouping collectors nodes of a cluster.
		/// Parameters:
		/// - `cluster_id`: Cluster id of a cluster.
		fn get_g_collectors_nodes(
			cluster_id: &ClusterId,
		) -> Result<Vec<(NodePubKey, StorageNodeParams)>, Error<T>> {
			let mut g_collectors = Vec::new();

			let collectors = Self::get_collectors_nodes(cluster_id)?;
			for (node_key, node_params) in collectors {
				if Self::check_grouping_collector(&node_params)
					.map_err(|_| Error::<T>::NodeRetrievalError)?
				{
					g_collectors.push((node_key, node_params))
				}
			}

			Ok(g_collectors)
		}

		/// Fetch processed payment era for across all nodes.
		///
		/// Parameters:
		/// - `cluster_id`: Cluster Id
		/// - `g_collector_key`: G-collector node key to fetch the payment eras from
		/// - `node_params`: DAC node parameters
		fn fetch_processed_ehd_eras_from_collector(
			cluster_id: &ClusterId,
			g_collectors: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<Vec<aggregator_client::json::EHDEra>>, OCWError> {
			let mut processed_eras_by_nodes: Vec<Vec<aggregator_client::json::EHDEra>> = Vec::new();

			for (collector_key, node_params) in g_collectors {
				let processed_payment_eras = Self::fetch_processed_ehd_eras(node_params);
				if processed_payment_eras.is_err() {
					log::warn!(
						"Aggregator from cluster {:?} is unavailable while fetching processed eras. Key: {:?} Host: {:?}",
						cluster_id,
						collector_key,
						String::from_utf8(node_params.host.clone())
					);
					// Skip unavailable aggregators and continue with available ones
					continue;
				} else {
					let eras =
						processed_payment_eras.map_err(|_| OCWError::FailedToFetchPaymentEra)?;
					if !eras.is_empty() {
						processed_eras_by_nodes.push(eras.into_iter().collect::<Vec<_>>());
					}
				}
			}

			Ok(processed_eras_by_nodes)
		}

		fn fetch_processed_ehd_era_from_collector(
			cluster_id: &ClusterId,
			era: EhdEra,
			g_collector: &(NodePubKey, StorageNodeParams),
		) -> Result<aggregator_client::json::EHDEra, OCWError> {
			let ehd_eras = Self::fetch_processed_ehd_eras_from_collector(
				cluster_id,
				vec![g_collector.clone()].as_slice(),
			)?;

			let era = ehd_eras
				.iter()
				.flat_map(|eras| eras.iter())
				.find(|ehd| ehd.id == era)
				.ok_or(OCWError::FailedToFetchPaymentEra)?;

			Ok(era.clone())
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

	impl<T: Config> ValidatorVisitor<T> for Pallet<T> {
		fn is_ocw_validator(caller: T::AccountId) -> bool {
			if ValidatorToStashKey::<T>::contains_key(caller.clone()) {
				<ValidatorSet<T>>::get().contains(&caller)
			} else {
				false
			}
		}

		fn is_quorum_reached(quorum: Percent, members_count: usize) -> bool {
			let threshold = quorum * <ValidatorSet<T>>::get().len();
			threshold <= members_count
		}
	}

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

	impl From<aggregator_client::json::AggregationEraResponse> for EraActivity {
		fn from(era: aggregator_client::json::AggregationEraResponse) -> Self {
			Self { id: era.id, start: era.start, end: era.end }
		}
	}

	#[derive(Clone)]
	pub struct CustomerBatch {
		pub(crate) batch_index: BatchIndex,
		pub(crate) payers: Vec<CustomerCharge>,
		pub(crate) batch_proof: MMRProof,
	}

	#[derive(Clone)]
	pub struct ProviderBatch {
		pub(crate) batch_index: BatchIndex,
		pub(crate) payees: Vec<ProviderReward>,
		pub(crate) batch_proof: MMRProof,
	}

	impl Hashable for CustomerCharge {
		fn hash<T: Config>(&self) -> PayableUsageHash {
			let mut data = self.0.encode(); // customer_id
			data.extend_from_slice(&self.1.encode()); // amount
			T::Hasher::hash(&data)
		}
	}

	impl Hashable for ProviderReward {
		fn hash<T: Config>(&self) -> PayableUsageHash {
			let mut data = self.0.encode(); // provider_id
			data.extend_from_slice(&self.1.encode()); // amount
			T::Hasher::hash(&data)
		}
	}

	/// The `ConsolidatedAggregate` struct represents a merging result of multiple aggregates
	/// that have reached consensus on the usage criteria. This result should be taken into
	/// consideration when choosing the intensity of the challenge.
	#[allow(unused)]
	#[derive(Debug, Clone, PartialEq)]
	pub(crate) struct ConsolidatedAggregate<A: Aggregate> {
		/// The representative aggregate after consolidation
		pub(crate) aggregate: A,
		/// Number of aggregates that were consistent
		pub(crate) count: u16,
		/// Aggregators that provided consistent aggregates
		pub(crate) aggregators: Vec<AggregatorInfo>,
	}

	#[allow(unused)]
	impl<A: Aggregate> ConsolidatedAggregate<A> {
		pub(crate) fn new(aggregate: A, count: u16, aggregators: Vec<AggregatorInfo>) -> Self {
			ConsolidatedAggregate { aggregate, count, aggregators }
		}
	}

	#[allow(unused)]
	#[derive(Debug, Clone, PartialEq)]
	pub(crate) struct ConsistencyGroups<A: Aggregate> {
		pub(crate) consensus: Vec<ConsolidatedAggregate<A>>,
		pub(crate) quorum: Vec<ConsolidatedAggregate<A>>,
		pub(crate) others: Vec<ConsolidatedAggregate<A>>,
	}

	#[allow(unused)]
	#[derive(Debug, Serialize, Deserialize, Clone, Encode, Decode, Hash, TypeInfo, PartialEq)]
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
	#[allow(unused)]
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
			T::Hasher::hash(&data)
		}
	}

	/* ######## DAC v5 DDC endpoints ######## */
	impl<T: Config> Pallet<T> {
		/// Fetch processed EHD eras.
		///
		/// Parameters:
		/// - `node_params`: DAC node parameters
		#[allow(dead_code)]
		pub(crate) fn fetch_processed_ehd_eras(
			node_params: &StorageNodeParams,
		) -> Result<Vec<aggregator_client::json::EHDEra>, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				MAX_RETRIES_COUNT,
				T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
			);

			let response = client.payment_eras()?;

			Ok(response.into_iter().filter(|e| e.status == "PROCESSED").collect::<Vec<_>>())
		}

		/// Traverse EHD record.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `ehd_id`: EHDId is a concatenated representation of:
		///     1) A 32-byte node public key in hex
		///     2) Starting TCAA id
		///     3) Ending TCAA id
		/// - `tree_node_id` - merkle tree node identifier
		/// - `tree_levels_count` - merkle tree levels to request
		pub(crate) fn fetch_traversed_era_historical_document(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
			tree_node_id: u32,
			tree_levels_count: u32,
		) -> Result<Vec<aggregator_client::json::TraversedEHDResponse>, OCWError> {
			let collectors = Self::get_collectors_nodes(cluster_id).map_err(|_| {
				log::error!("❌ Error retrieving collectors for cluster {:?}", cluster_id);
				OCWError::FailedToFetchCollectors { cluster_id: *cluster_id }
			})?;

			for (collector_key, collector_params) in collectors {
				if collector_key != ehd_id.1 {
					continue;
				};

				if let Ok(host) = str::from_utf8(&collector_params.host) {
					let base_url = format!("http://{}:{}", host, collector_params.http_port);
					let client = aggregator_client::AggregatorClient::new(
						&base_url,
						Duration::from_millis(RESPONSE_TIMEOUT),
						MAX_RETRIES_COUNT,
						false, // no response signature verification for now
					);

					if let Ok(traversed_ehd) = client.traverse_era_historical_document(
						ehd_id.clone(),
						tree_node_id,
						tree_levels_count,
					) {
						// proceed with the first available EHD record for the prototype
						return Ok(traversed_ehd);
					} else {
						log::warn!(
							"⚠️  Collector from cluster {:?} is unavailable while fetching EHD record or responded with unexpected body. Key: {:?} Host: {:?}",
							cluster_id,
							collector_key,
							String::from_utf8(collector_params.host)
						);
					}
				}
			}

			Err(OCWError::FailedToFetchTraversedEHD)
		}

		/// Traverse PHD record.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `phd_id`: PHDId is a concatenated representation of:
		///     1) A 32-byte node public key in hex
		///     2) Starting TCAA id
		///     3) Ending TCAA id
		/// - `tree_node_id` - merkle tree node identifier
		/// - `tree_levels_count` - merkle tree levels to request
		pub(crate) fn fetch_traversed_partial_historical_document(
			cluster_id: &ClusterId,
			phd_id: PHDId,
			tree_node_id: u32,
			tree_levels_count: u32,
		) -> Result<Vec<aggregator_client::json::TraversedPHDResponse>, OCWError> {
			let collectors = Self::get_collectors_nodes(cluster_id).map_err(|_| {
				log::error!("❌ Error retrieving collectors for cluster {:?}", cluster_id);
				OCWError::FailedToFetchCollectors { cluster_id: *cluster_id }
			})?;

			for (collector_key, collector_params) in collectors {
				if collector_key != phd_id.0 {
					continue;
				};

				if let Ok(host) = str::from_utf8(&collector_params.host) {
					let base_url = format!("http://{}:{}", host, collector_params.http_port);
					let client = aggregator_client::AggregatorClient::new(
						&base_url,
						Duration::from_millis(RESPONSE_TIMEOUT),
						MAX_RETRIES_COUNT,
						false, // no response signature verification for now
					);

					if let Ok(traversed_phd) = client.traverse_partial_historical_document(
						phd_id.clone(),
						tree_node_id,
						tree_levels_count,
					) {
						// proceed with the first available EHD record for the prototype
						return Ok(traversed_phd);
					} else {
						log::warn!(
							"⚠️  Collector from cluster {:?} is unavailable while fetching PHD record or responded with unexpected body. Key: {:?} Host: {:?}",
							cluster_id,
							collector_key,
							String::from_utf8(collector_params.host)
						);
					}
				}
			}

			Err(OCWError::FailedToFetchTraversedPHD)
		}

		fn fetch_inspection_receipts(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
		) -> Result<BTreeMap<String, aggregator_client::json::GroupedInspectionReceipt>, OCWError> {
			// todo(yahortsaryk): infer the node deterministically
			let g_collector = Self::get_g_collectors_nodes(cluster_id)
				.map_err(|_| OCWError::FailedToFetchGCollectors { cluster_id: *cluster_id })?
				.first()
				.cloned()
				.ok_or(OCWError::FailedToFetchGCollectors { cluster_id: *cluster_id })?;

			if let Ok(host) = str::from_utf8(&g_collector.1.host) {
				let base_url = format!("http://{}:{}", host, g_collector.1.http_port);
				let client = aggregator_client::AggregatorClient::new(
					&base_url,
					Duration::from_millis(RESPONSE_TIMEOUT),
					MAX_RETRIES_COUNT,
					false, // no response signature verification for now
				);

				if let Ok(res) = client.fetch_grouped_inspection_receipts(ehd_id) {
					return Ok(res);
				}
			}

			Err(OCWError::FailedToFetchInspectionReceipt)
		}

		fn calculate_customers_usage_cutoff(
			cluster_id: &ClusterId,
			receipts_by_inspector: &BTreeMap<
				String,
				aggregator_client::json::GroupedInspectionReceipt,
			>,
		) -> Result<BTreeMap<T::AccountId, BucketUsage>, OCWError> {
			let mut buckets_usage_cutoff: BTreeMap<(DdcEra, BucketId, NodePubKey), BucketUsage> =
				BTreeMap::new();
			let mut buckets_to_customers: BTreeMap<BucketId, T::AccountId> = BTreeMap::new();
			let mut customers_usage_cutoff: BTreeMap<T::AccountId, BucketUsage> = BTreeMap::new();

			for inspection_receipt in receipts_by_inspector.values() {
				for unverified_branch in &inspection_receipt.buckets_inspection.unverified_branches
				{
					let collector_key = NodePubKey::try_from(unverified_branch.collector.clone())
						.map_err(|_| OCWError::FailedToParseNodeKey {
						node_key: unverified_branch.collector.clone(),
					})?;
					let (bucket_id, node_key) = match &unverified_branch.aggregate {
						AggregateKey::BucketSubAggregateKey(bucket_id, key) => {
							let node_key = NodePubKey::try_from(key.clone()).map_err(|_| {
								OCWError::FailedToParseNodeKey { node_key: key.clone() }
							})?;
							(bucket_id, node_key)
						},
						_ => continue, // can happen only in case of a malformed response or bug
					};

					for tcaa_id in unverified_branch.leafs.keys() {
						let tcaa_usage = Self::get_tcaa_bucket_usage(
							cluster_id,
							collector_key.clone(),
							*tcaa_id,
							node_key.clone(),
							*bucket_id,
						)?;

						#[allow(clippy::map_entry)]
						if !buckets_usage_cutoff.contains_key(&(
							*tcaa_id,
							*bucket_id,
							node_key.clone(),
						)) {
							buckets_usage_cutoff
								.insert((*tcaa_id, *bucket_id, node_key.clone()), tcaa_usage);
							if !buckets_to_customers.contains_key(bucket_id) {
								let customer_id = T::BucketManager::get_bucket_owner_id(*bucket_id)
									.map_err(|_| OCWError::FailedToFetchCustomerId {
										bucket_id: *bucket_id,
									})?;

								buckets_to_customers.insert(*bucket_id, customer_id);
							}
						}
					}
				}
			}

			for ((_, bucket_id, _), bucket_usage) in buckets_usage_cutoff {
				let customer_id = buckets_to_customers
					.get(&bucket_id)
					.ok_or(OCWError::FailedToFetchCustomerId { bucket_id })?;

				match customers_usage_cutoff.get_mut(customer_id) {
					Some(total_usage) => {
						total_usage.number_of_puts += bucket_usage.number_of_puts;
						total_usage.number_of_gets += bucket_usage.number_of_gets;
						total_usage.transferred_bytes += bucket_usage.transferred_bytes;
						total_usage.stored_bytes += bucket_usage.stored_bytes;
					},
					None => {
						customers_usage_cutoff.insert(customer_id.clone(), bucket_usage.clone());
					},
				}
			}

			Ok(customers_usage_cutoff)
		}

		fn calculate_providers_usage_cutoff(
			cluster_id: &ClusterId,
			receipts_by_inspector: &BTreeMap<
				String,
				aggregator_client::json::GroupedInspectionReceipt,
			>,
		) -> Result<BTreeMap<T::AccountId, NodeUsage>, OCWError> {
			let mut nodes_usage_cutoff: BTreeMap<(DdcEra, NodePubKey), NodeUsage> = BTreeMap::new();
			let mut nodes_to_providers: BTreeMap<NodePubKey, T::AccountId> = BTreeMap::new();
			let mut providers_usage_cutoff: BTreeMap<T::AccountId, NodeUsage> = BTreeMap::new();

			for inspection_receipt in receipts_by_inspector.values() {
				for unverified_branch in &inspection_receipt.nodes_inspection.unverified_branches {
					let collector_key = NodePubKey::try_from(unverified_branch.collector.clone())
						.map_err(|_| OCWError::FailedToParseNodeKey {
						node_key: unverified_branch.collector.clone(),
					})?;
					let node_key = match &unverified_branch.aggregate {
						AggregateKey::NodeAggregateKey(key) => NodePubKey::try_from(key.clone())
							.map_err(|_| OCWError::FailedToParseNodeKey {
								node_key: key.clone(),
							})?,
						_ => continue, // can happen only in case of a malformed response or bug
					};

					for tcaa_id in unverified_branch.leafs.keys() {
						let tcaa_usage = Self::get_tcaa_node_usage(
							cluster_id,
							collector_key.clone(),
							*tcaa_id,
							node_key.clone(),
						)?;

						#[allow(clippy::map_entry)]
						if !nodes_usage_cutoff.contains_key(&(*tcaa_id, node_key.clone())) {
							nodes_usage_cutoff.insert((*tcaa_id, node_key.clone()), tcaa_usage);
							if !nodes_to_providers.contains_key(&node_key) {
								let provider_id = T::NodeManager::get_node_provider_id(&node_key)
									.map_err(|_| {
									OCWError::FailedToFetchProviderId { node_key: node_key.clone() }
								})?;

								nodes_to_providers.insert(node_key.clone(), provider_id);
							}
						}
					}
				}
			}

			for ((_, node_key), node_usage) in nodes_usage_cutoff {
				let provider_id = nodes_to_providers
					.get(&node_key)
					.ok_or(OCWError::FailedToFetchProviderId { node_key: node_key.clone() })?;

				match providers_usage_cutoff.get_mut(provider_id) {
					Some(total_usage) => {
						total_usage.number_of_puts += node_usage.number_of_puts;
						total_usage.number_of_gets += node_usage.number_of_gets;
						total_usage.transferred_bytes += node_usage.transferred_bytes;
						total_usage.stored_bytes += node_usage.stored_bytes;
					},
					None => {
						providers_usage_cutoff.insert(provider_id.clone(), node_usage.clone());
					},
				}
			}

			Ok(providers_usage_cutoff)
		}

		/// Send Inspection Receipt.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `g_collector`: grouping collector node to save the receipt
		/// - `receipt`: inspection receipt
		pub(crate) fn send_inspection_receipt(
			cluster_id: &ClusterId,
			g_collector: &(NodePubKey, StorageNodeParams),
			receipt: aggregator_client::json::InspectionReceipt,
		) -> Result<(), OCWError> {
			if let Ok(host) = str::from_utf8(&g_collector.1.host) {
				let base_url = format!("http://{}:{}", host, g_collector.1.http_port);
				let client = aggregator_client::AggregatorClient::new(
					&base_url,
					Duration::from_millis(RESPONSE_TIMEOUT),
					MAX_RETRIES_COUNT,
					false, // no response signature verification for now
				);

				if client.send_inspection_receipt(receipt.clone()).is_ok() {
					// proceed with the first available EHD record for the prototype
					return Ok(());
				} else {
					log::warn!(
						"⚠️  Collector from cluster {:?} is unavailable while fetching EHD record or responded with unexpected body. Key: {:?} Host: {:?}",
						cluster_id,
						g_collector.0,
						String::from_utf8(g_collector.1.host.clone())
					);
				}
			}

			Err(OCWError::FailedToSaveInspectionReceipt)
		}

		pub(crate) fn fetch_node_challenge_response(
			cluster_id: &ClusterId,
			tcaa_id: DdcEra,
			collector_key: NodePubKey,
			node_key: NodePubKey,
			tree_node_ids: Vec<u64>,
		) -> Result<proto::ChallengeResponse, OCWError> {
			let collectors = Self::get_collectors_nodes(cluster_id)
				.map_err(|_| OCWError::FailedToFetchCollectors { cluster_id: *cluster_id })?;

			for (key, collector_params) in collectors {
				if key != collector_key {
					continue;
				};

				if let Ok(host) = str::from_utf8(&collector_params.host) {
					let base_url = format!("http://{}:{}", host, collector_params.http_port);
					let client = aggregator_client::AggregatorClient::new(
						&base_url,
						Duration::from_millis(RESPONSE_TIMEOUT),
						MAX_RETRIES_COUNT,
						false, // no response signature verification for now
					);

					if let Ok(node_challenge_res) = client.challenge_node_aggregate(
						tcaa_id,
						Into::<String>::into(node_key.clone()).as_str(),
						tree_node_ids.clone(),
					) {
						return Ok(node_challenge_res);
					} else {
						log::warn!(
							"Collector from cluster {:?} is unavailable while challenging node aggregate or responded with unexpected body. Key: {:?} Host: {:?}",
							cluster_id,
							collector_key,
							String::from_utf8(collector_params.host)
						);
					}
				}
			}

			Err(OCWError::FailedToFetchNodeChallenge)
		}

		pub(crate) fn fetch_bucket_challenge_response(
			cluster_id: &ClusterId,
			tcaa_id: DdcEra,
			collector_key: NodePubKey,
			node_key: NodePubKey,
			bucket_id: BucketId,
			tree_node_ids: Vec<u64>,
		) -> Result<proto::ChallengeResponse, OCWError> {
			let collectors = Self::get_collectors_nodes(cluster_id)
				.map_err(|_| OCWError::FailedToFetchCollectors { cluster_id: *cluster_id })?;

			for (key, collector_params) in collectors {
				if key != collector_key {
					continue;
				};

				if let Ok(host) = str::from_utf8(&collector_params.host) {
					let base_url = format!("http://{}:{}", host, collector_params.http_port);
					let client = aggregator_client::AggregatorClient::new(
						&base_url,
						Duration::from_millis(RESPONSE_TIMEOUT),
						MAX_RETRIES_COUNT,
						false, // no response signature verification for now
					);

					if let Ok(node_challenge_res) = client.challenge_bucket_sub_aggregate(
						tcaa_id,
						bucket_id,
						Into::<String>::into(node_key.clone()).as_str(),
						tree_node_ids.clone(),
					) {
						return Ok(node_challenge_res);
					} else {
						log::warn!(
							"Collector from cluster {:?} is unavailable while challenging bucket sub-aggregate or responded with unexpected body. Key: {:?} Host: {:?}",
							cluster_id,
							collector_key,
							String::from_utf8(collector_params.host)
						);
					}
				}
			}

			Err(OCWError::FailedToFetchBucketChallenge)
		}
	}

	/* ######## DAC v4 functions ######## */
	impl<T: Config> Pallet<T> {
		#[allow(clippy::type_complexity)]
		pub(crate) fn _v4_process_era(
			cluster_id: &ClusterId,
			era_activity: EraActivity,
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

			let dac_nodes = Self::get_collectors_nodes(cluster_id).map_err(|_| {
				log::error!("❌ Error retrieving dac nodes to validate cluster {:?}", cluster_id);
				vec![OCWError::FailedToFetchCollectors { cluster_id: *cluster_id }]
			})?;

			log::info!(
				"👁️‍🗨️  Start processing DAC for cluster_id: {:?} era_id; {:?}",
				cluster_id,
				era_activity.id
			);

			// todo: move to cluster protocol parameters
			let dac_redundancy_factor = T::DAC_REDUNDANCY_FACTOR;
			let aggregators_quorum = T::AggregatorsQuorum::get();

			let nodes_aggregates_by_aggregator =
				Self::_v4_fetch_nodes_aggregates_for_era(cluster_id, era_activity.id, &dac_nodes)
					.map_err(|err| vec![err])?;

			let buckets_aggregates_by_aggregator =
				Self::_v4_fetch_buckets_aggregates_for_era(cluster_id, era_activity.id, &dac_nodes)
					.map_err(|err| vec![err])?;

			let buckets_sub_aggregates_groups =
				Self::_v4_group_buckets_sub_aggregates_by_consistency(
					cluster_id,
					era_activity.id,
					buckets_aggregates_by_aggregator,
					dac_redundancy_factor,
					aggregators_quorum,
				);

			let total_buckets_usage = Self::_v4_get_total_usage(
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
				Self::split_to_batches(&total_buckets_usage, batch_size.into()),
				era_activity.id,
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

			let customers_activity_root = Self::create_merkle_root(
				cluster_id,
				&customers_activity_batch_roots,
				era_activity.id,
			)
			.map_err(|err| vec![err])?;

			log::info!(
				"👁️‍🗨️‍  Customer Activity batches tree for cluster_id: {:?} era_id: {:?} is: batch with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
				hex::encode(customers_activity_root),
					customer_batch_roots_string,
			);

			let nodes_aggregates_groups = Self::_v4_group_nodes_aggregates_by_consistency(
				cluster_id,
				era_activity.id,
				nodes_aggregates_by_aggregator,
				dac_redundancy_factor,
				aggregators_quorum,
			);

			let total_nodes_usage = Self::_v4_get_total_usage(
				cluster_id,
				era_activity.id,
				nodes_aggregates_groups,
				true,
			)?;

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
				Self::split_to_batches(&total_nodes_usage, batch_size.into()),
				era_activity.id,
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

			let nodes_activity_root =
				Self::create_merkle_root(cluster_id, &nodes_activity_batch_roots, era_activity.id)
					.map_err(|err| vec![err])?;

			log::info!(
				"👁️‍🗨️  Node Activity batches tree for cluster_id: {:?} era_id: {:?} are: batch with root {:?} for activities {:?}",
				cluster_id,
				era_activity.id,
				hex::encode(nodes_activity_root),
					nodes_activity_batch_roots_string,
			);

			Self::_v4_store_verified_usage(
				cluster_id,
				era_activity.id,
				&total_buckets_usage,
				customers_activity_root,
				&customers_activity_batch_roots,
				&total_nodes_usage,
				nodes_activity_root,
				&nodes_activity_batch_roots,
			);
			log::info!(
				"👁️‍🗨️‍  End processing DAC for cluster_id: {:?} era_id: {:?}",
				cluster_id,
				era_activity.id
			);
			Ok(Some((
				era_activity,
				customers_activity_root,
				nodes_activity_root,
				customers_activity_batch_roots,
				nodes_activity_batch_roots,
			)))
		}

		/// Fetch node usage of an era.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
		pub(crate) fn _v4_fetch_nodes_aggregates_for_era(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<
			Vec<(AggregatorInfo, Vec<aggregator_client::json::NodeAggregateResponse>)>,
			OCWError,
		> {
			let mut nodes_aggregates = Vec::new();

			for (node_key, node_params) in dac_nodes {
				let aggregates_res =
					Self::_v4_fetch_node_aggregates(cluster_id, era_id, node_params);
				if aggregates_res.is_err() {
					log::warn!(
						"Aggregator from cluster {:?} is unavailable while fetching nodes aggregates. Key: {:?} Host: {:?}",
						cluster_id,
						node_key,
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
		}

		/// Fetch customer usage for an era.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
		pub(crate) fn _v4_fetch_buckets_aggregates_for_era(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<
			Vec<(AggregatorInfo, Vec<aggregator_client::json::BucketAggregateResponse>)>,
			OCWError,
		> {
			let mut bucket_aggregates: Vec<(
				AggregatorInfo,
				Vec<aggregator_client::json::BucketAggregateResponse>,
			)> = Vec::new();

			for (node_key, node_params) in dac_nodes {
				let aggregates_res =
					Self::_v4_fetch_bucket_aggregates(cluster_id, era_id, node_params);
				if aggregates_res.is_err() {
					log::warn!(
						"Aggregator from cluster {:?} is unavailable while fetching buckets aggregates. Key: {:?} Host: {:?}",
						cluster_id,
						node_key,
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
		pub(crate) fn _v4_group_buckets_sub_aggregates_by_consistency(
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
				Self::_v4_group_by_consistency(buckets_sub_aggregates, redundancy_factor, quorum);

			log::info!("👁️‍🗨️‍🌕 Bucket Sub-Aggregates, which are in consensus for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, buckets_sub_aggregates_groups.consensus);
			log::info!("👁️‍🗨️‍🌗 Bucket Sub-Aggregates, which are in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, buckets_sub_aggregates_groups.quorum);
			log::info!("👁️‍🗨️‍🌘 Bucket Sub-Aggregates, which are neither in consensus nor in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, buckets_sub_aggregates_groups.others);

			buckets_sub_aggregates_groups
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
		/// - `nodes_aggregates_by_aggregator: &[(NodePubKey, Vec<A>)]`: A list of tuples, where
		///   each tuple contains a node's public key and a vector of activities reported by that
		///   node.
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
		pub(crate) fn _v4_group_nodes_aggregates_by_consistency(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			nodes_aggregates_by_aggregator: Vec<(
				AggregatorInfo,
				Vec<aggregator_client::json::NodeAggregateResponse>,
			)>,
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
				Self::_v4_group_by_consistency(nodes_aggregates, redundancy_factor, quorum);

			log::info!("👁️‍🗨️‍🌕 Node Aggregates, which are in consensus for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, nodes_aggregates_groups.consensus);
			log::info!("👁️‍🗨️‍🌗 Node Aggregates, which are in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, nodes_aggregates_groups.quorum);
			log::info!("👁️‍🗨️‍🌘 Node Aggregates, which are neither in consensus nor in quorum for cluster_id: {:?} for era_id: {:?}:::  {:?}", cluster_id, era_id, nodes_aggregates_groups.others);

			nodes_aggregates_groups
		}

		pub(crate) fn _v4_get_total_usage<A: Aggregate>(
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

			let verified_usage = Self::_v4_challenge_others(
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

		pub(crate) fn _v4_challenge_others<A: Aggregate>(
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
					// let is_passed = Self::_v4_challenge_aggregate(_cluster_id, _era_id,
					// &defective_aggregate)?;
					if should_challenge {
						is_passed = Self::_v4_challenge_aggregate_proto(
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

		pub(crate) fn _v4_group_by_consistency<A>(
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

		pub(crate) fn _v4_challenge_aggregate<A: Aggregate>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate: &A,
		) -> Result<bool, Vec<OCWError>> {
			let number_of_identifiers = T::MAX_MERKLE_NODE_IDENTIFIER;

			log::info!(
				"👁️‍🗨️  Challenge process starts when bucket sub aggregates are not in consensus!"
			);

			let aggregate_key = aggregate.get_key();
			let merkle_node_ids = Self::_v4_find_random_merkle_node_ids(
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

			let challenge_response = Self::_v4_fetch_challenge_responses(
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

			let calculated_merkle_root = Self::_v4_get_hash_from_merkle_path(
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

			let root_merkle_node = Self::_v4_fetch_traverse_response(
				era_id,
				aggregate_key.clone(),
				1,
				1,
				&aggregator.node_params,
			)
			.map_err(|_| {
				vec![OCWError::TraverseResponseError {
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

		pub(crate) fn _v4_challenge_aggregate_proto<A: Aggregate>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate: &A,
		) -> Result<bool, Vec<OCWError>> {
			let number_of_identifiers = T::MAX_MERKLE_NODE_IDENTIFIER;

			log::info!(
				"👁️‍🗨️  Challenge process starts when bucket sub aggregates are not in consensus!"
			);

			let aggregate_key = aggregate.get_key();
			let merkle_node_ids = Self::_v4_find_random_merkle_node_ids(
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

			let challenge_response = Self::_v4_fetch_challenge_responses_proto(
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

			let are_signatures_valid = signature::Verify::verify(&challenge_response);

			if are_signatures_valid {
				log::info!("👍 Valid challenge signatures for aggregate key: {:?}", aggregate_key,);
			} else {
				log::info!("👎 Invalid challenge signatures at aggregate key: {:?}", aggregate_key,);
			}

			Ok(are_signatures_valid)
		}

		/// Fetch Challenge node aggregate or bucket sub-aggregate.
		pub(crate) fn _v4_fetch_challenge_responses(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_node_identifiers: Vec<u64>,
			aggregator: AggregatorInfo,
		) -> Result<aggregator_client::json::ChallengeAggregateResponse, OCWError> {
			let response = Self::_v4_fetch_challenge_response(
				era_id,
				aggregate_key.clone(),
				merkle_node_identifiers.clone(),
				&aggregator.node_params,
			)
			.map_err(|_| OCWError::ChallengeResponseError {
				cluster_id: *cluster_id,
				era_id,
				aggregate_key,
				aggregator: aggregator.node_pub_key,
			})?;

			Ok(response)
		}

		/// Challenge node aggregate or bucket sub-aggregate.
		pub(crate) fn _v4_fetch_challenge_responses_proto(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_tree_node_id: Vec<u64>,
			aggregator: AggregatorInfo,
		) -> Result<proto::ChallengeResponse, OCWError> {
			let response = Self::_v4_fetch_challenge_response_proto(
				era_id,
				aggregate_key.clone(),
				merkle_tree_node_id.clone(),
				&aggregator.node_params,
			)
			.map_err(|_| OCWError::ChallengeResponseError {
				cluster_id: *cluster_id,
				era_id,
				aggregate_key,
				aggregator: aggregator.node_pub_key,
			})?;

			Ok(response)
		}

		/// Fetch node usage.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `era_id`: era id
		/// - `node_params`: DAC node parameters
		pub(crate) fn _v4_fetch_node_aggregates(
			_cluster_id: &ClusterId,
			era_id: DdcEra,
			node_params: &StorageNodeParams,
		) -> Result<Vec<aggregator_client::json::NodeAggregateResponse>, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				MAX_RETRIES_COUNT,
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
		}

		/// Fetch challenge response.
		///
		/// Parameters:
		/// - `era_id`: era id
		/// - `aggregate_key`: key of the aggregate to challenge
		/// - `merkle_node_identifiers`: set of merkle node identifiers to challenge
		/// - `node_params`: aggregator node parameters
		pub(crate) fn _v4_fetch_challenge_response(
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

		pub(crate) fn _v4_find_random_merkle_node_ids(
			number_of_identifiers: usize,
			number_of_leaves: u64,
			aggregate_key: AggregateKey,
		) -> Vec<u64> {
			let nonce_key = match aggregate_key {
				AggregateKey::NodeAggregateKey(node_id) => node_id,
				AggregateKey::BucketSubAggregateKey(.., node_id) => node_id,
			};

			let nonce = Self::store_and_fetch_nonce(nonce_key);
			let mut small_rng = SmallRng::seed_from_u64(nonce);

			let total_levels = number_of_leaves.ilog2() + 1;
			let int_list: Vec<u64> = (0..total_levels as u64).collect();

			let ids: Vec<u64> = int_list
				.choose_multiple(&mut small_rng, number_of_identifiers)
				.cloned()
				.collect::<Vec<u64>>();

			ids
		}

		/// Fetch processed era for across all nodes.
		///
		/// Parameters:
		/// - `cluster_id`: Cluster id
		/// - `node_params`: DAC node parameters
		fn _v4_fetch_processed_era_for_nodes(
			cluster_id: &ClusterId,
			dac_nodes: &[(NodePubKey, StorageNodeParams)],
		) -> Result<Vec<Vec<EraActivity>>, OCWError> {
			let mut processed_eras_by_nodes: Vec<Vec<EraActivity>> = Vec::new();

			for (node_key, node_params) in dac_nodes {
				let processed_eras_by_node = Self::_v4_fetch_processed_eras(node_params);
				if processed_eras_by_node.is_err() {
					log::warn!(
						"Aggregator from cluster {:?} is unavailable while fetching processed eras. Key: {:?} Host: {:?}",
						cluster_id,
						node_key,
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
		/// Fetch processed era.
		///
		/// Parameters:
		/// - `node_params`: DAC node parameters
		#[allow(dead_code)]
		pub(crate) fn _v4_fetch_processed_eras(
			node_params: &StorageNodeParams,
		) -> Result<Vec<aggregator_client::json::AggregationEraResponse>, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				MAX_RETRIES_COUNT,
				T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
			);

			let response = client.eras()?;

			Ok(response.into_iter().filter(|e| e.status == "PROCESSED").collect::<Vec<_>>())
		}

		pub(crate) fn _v4_get_hash_from_merkle_path(
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
					Self::create_merkle_root(cluster_id, &leaf_record_hashes, era_id)
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
						Self::create_merkle_root(cluster_id, &[resulting_hash, path_hash], era_id)
							.map_err(|err| vec![err])?;

					log::info!("👁️‍🗨️  Fetched leaf node root aggregate_key: {:?} for path:{:?} leaf_node_hash: {:?}",
						aggregate_key, path, hex::encode(node_root));

					resulting_hash = node_root;
				}
			}

			Ok(resulting_hash)
		}

		pub(crate) fn _v4_derive_usage_key(cluster_id: &ClusterId, era_id: DdcEra) -> Vec<u8> {
			format!("offchain::activities::{:?}::{:?}", cluster_id, era_id).into_bytes()
		}

		#[allow(clippy::too_many_arguments)]
		pub(crate) fn _v4_store_verified_usage<A: Encode, B: Encode>(
			cluster_id: &ClusterId,
			era_id: DdcEra,
			buckets_deltas: &[A],
			buckets_deltas_root: DeltaUsageHash,
			buckets_deltas_batch_roots: &[DeltaUsageHash],
			nodes_deltas: &[B],
			nodes_deltas_root: DeltaUsageHash,
			nodes_deltas_batch_roots: &[DeltaUsageHash],
		) {
			let key = Self::_v4_derive_usage_key(cluster_id, era_id);
			let encoded_tuple = (
				buckets_deltas,
				buckets_deltas_root,
				buckets_deltas_batch_roots,
				nodes_deltas,
				nodes_deltas_root,
				nodes_deltas_batch_roots,
			)
				.encode();

			// Store the serialized data in local offchain storage
			local_storage_set(StorageKind::PERSISTENT, &key, &encoded_tuple);
		}

		/// Fetch traverse response.
		///
		/// Parameters:
		/// - `era_id`: era id
		/// - `aggregate_key`: key of the aggregate to challenge
		/// - `merkle_node_identifiers`: set of merkle node identifiers to challenge
		/// - `levels`: a number of levels to raverse
		/// - `node_params`: aggregator node parameters
		pub(crate) fn _v4_fetch_traverse_response(
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
				MAX_RETRIES_COUNT,
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
			}?;

			Ok(response)
		}

		/// Fetch protobuf challenge response.
		pub(crate) fn _v4_fetch_challenge_response_proto(
			era_id: DdcEra,
			aggregate_key: AggregateKey,
			merkle_tree_node_id: Vec<u64>,
			node_params: &StorageNodeParams,
		) -> Result<proto::ChallengeResponse, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				MAX_RETRIES_COUNT,
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

		/// Fetch customer usage.
		///
		/// Parameters:
		/// - `cluster_id`: cluster id of a cluster
		/// - `tcaa_id`: tcaa id
		/// - `node_params`: DAC node parameters
		pub(crate) fn _v4_fetch_bucket_aggregates(
			_cluster_id: &ClusterId,
			tcaa_id: DdcEra,
			node_params: &StorageNodeParams,
		) -> Result<Vec<aggregator_client::json::BucketAggregateResponse>, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				MAX_RETRIES_COUNT,
				T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
			);

			let mut buckets_aggregates = Vec::new();
			let mut prev_token = None;

			loop {
				let response = client.buckets_aggregates(
					tcaa_id,
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
		}

		/// Fetch customer usage.
		///
		/// Parameters:
		/// - `node_params`: Requesting DDC node
		pub(crate) fn check_grouping_collector(
			node_params: &StorageNodeParams,
		) -> Result<bool, http::Error> {
			let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
			let base_url = format!("http://{}:{}", host, node_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				MAX_RETRIES_COUNT,
				T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
			);

			let response = client.check_grouping_collector()?;
			Ok(response.is_g_collector)
		}
	}

	impl<T: Config> sp_application_crypto::BoundToRuntimeAppPublic for Pallet<T> {
		type Public = T::AuthorityId;
	}

	impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
		type Key = T::AuthorityId;

		#[allow(clippy::multiple_bound_locations)]
		fn on_genesis_session<'a, I: 'a>(validators: I)
		where
			I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
		{
			log::info!("🙌Adding Validator from genesis session.");
			let validators = validators
				.map(|(_, k)| T::AccountId::decode(&mut &k.into().encode()[..]).unwrap())
				.collect::<Vec<_>>();

			// only active validators in session - this is NOT all the validators
			ValidatorSet::<T>::put(validators);
		}

		#[allow(clippy::multiple_bound_locations)]
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

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		// the first key is 'verification_key' that is used to sign inspection receipts
		// the second key is 'stash_key' that should be used in staking by this validator
		pub validators: Vec<(T::AccountId, T::AccountId)>,
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
			for (verification_key, stash_key) in &self.validators {
				// <ValidatorSet<T>> should be updated in 'on_genesis_session' handler
				if <ValidatorSet<T>>::get().contains(verification_key) {
					ValidatorToStashKey::<T>::insert(verification_key, stash_key);
				}
			}
		}
	}
}
