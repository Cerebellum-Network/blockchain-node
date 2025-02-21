//! # DDC Payouts Pallet
//!
//! The DDC Payouts pallet is used to distribute payouts based on DAC validation
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## GenesisConfig
//!
//! The DDC Payouts pallet depends on the [`GenesisConfig`]. The
//! `GenesisConfig` is optional and allow to set some initial nodes in DDC.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

// todo(yahortsaryk) tests for DAC v4 payments should be completely revised
#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

pub mod migrations;

use core::str;

use ddc_primitives::{
	traits::{
		cluster::ClusterProtocol as ClusterProtocolType,
		customer::CustomerCharger as CustomerChargerType,
		pallet::PalletVisitor as PalletVisitorType,
	},
	CustomerCharge as CustomerCosts, Fingerprint,
	MergeMMRHash, PayoutError,
	MAX_PAYOUT_BATCH_COUNT,
	MAX_PAYOUT_BATCH_SIZE, MILLICENTS,
	DAC_VERIFICATION_KEY_TYPE
};
use ddc_primitives::{proto, BucketId, AggregateKey};
use frame_election_provider_support::SortedListProvider;
use frame_support::{
	pallet_prelude::*,
	parameter_types,
	sp_runtime::SaturatedConversion,
	traits::{Currency, ExistenceRequirement, Get, LockableCurrency},
	BoundedBTreeSet,
};
use frame_system::{
	offchain::{Account, AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer}};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use polkadot_ckb_merkle_mountain_range::{
	helper::{leaf_index_to_mmr_size, leaf_index_to_pos},
	util::{MemMMR, MemStore},
	MerkleProof, MMR,
};
use scale_info::prelude::string::String;
use sp_core::H256;
use sp_runtime::{
	traits::{Convert, Hash},
	AccountId32, Percent, Perquintill, Saturating,
};
use sp_std::collections::btree_map::BTreeMap;
use sp_runtime::{
	offchain::{http, Duration, StorageKind},
	traits::IdentifyAccount,
	ArithmeticError,
};
use frame_support::traits::OneSessionHandler;
use itertools::Itertools;
pub use sp_io::{
	crypto::sr25519_public_keys,
	offchain::{
		local_storage_clear, local_storage_compare_and_set, local_storage_get, local_storage_set,
	},
};
use sp_application_crypto::RuntimeAppPublic;
use ddc_primitives::aggregator as aggregator_client;
use ddc_primitives::ProviderReward;
use ddc_primitives::DeltaUsageHash;
use ddc_primitives::{
	ocw_mutex::OcwMutex,
	traits::{
		BucketManager, ClusterManager, ClusterProtocol, ClusterValidator, CustomerVisitor,
		NodeManager, PayoutProcessor, StorageUsageProvider, ValidatorVisitor,
	},
	BatchIndex, BucketStorageUsage, BucketUsage, ClusterFeesParams, ClusterId,
	ClusterPricingParams, ClusterStatus, DdcEra, EHDId, EhdEra,
	MMRProof, NodeParams, NodePubKey, NodeStorageUsage, NodeUsage, PHDId, PayableUsageHash,
	PaymentEra, PayoutFingerprintParams, PayoutReceiptParams, PayoutState,
	ProviderReward as ProviderProfits, StorageNodeParams, StorageNodePubKey, AVG_SECONDS_MONTH,
};
//use frame_support::Hashable;

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type VoteScoreOf<T> =
	<<T as pallet::Config>::NominatorsAndValidatorsList as frame_election_provider_support::SortedListProvider<
		<T as frame_system::Config>::AccountId,
	>>::Score;

parameter_types! {
	pub MaxBatchesCount: u16 = MAX_PAYOUT_BATCH_COUNT;
	pub MaxDust: u128 = MILLICENTS;
	pub MaxBatchSize: u16 = MAX_PAYOUT_BATCH_SIZE;
}

#[frame_support::pallet]
pub mod pallet {
	use ddc_primitives::traits::ValidatorVisitor;
	use frame_support::PalletId;
	use sp_io::hashing::blake2_128;
	use sp_runtime::traits::{AccountIdConversion, Zero};
	use sp_io::hashing::blake2_256;
	use sp_std::collections::btree_set::BTreeSet;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
		frame_support::traits::StorageVersion::new(3);

	pub const MAX_RETRIES_COUNT: u32 = 3;
	const RESPONSE_TIMEOUT: u64 = 20000;

	pub const OCW_MUTEX_ID: &[u8] = b"payment_lock"; //TODO: Both offchain worker can run in parallel

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type Currency: LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>;
		type CustomerCharger: CustomerChargerType<Self>;
		type ClusterManager: ClusterManager<Self>;
		type BucketManager: BucketManager<Self>;
		type NodeManager: NodeManager<Self>;
		type TreasuryVisitor: PalletVisitorType<Self>;
		type ClusterProtocol: ClusterProtocolType<Self, BalanceOf<Self>>;
		type NominatorsAndValidatorsList: SortedListProvider<Self::AccountId>;
		type VoteScoreToU64: Convert<VoteScoreOf<Self>, u64>;
		type ValidatorVisitor: ValidatorVisitor<Self>;
		type AccountIdConverter: From<Self::AccountId> + Into<AccountId32>;
		type Hasher: Hash<Output = H256>;
		/// The identifier type for an authority.
		type AuthorityId: Member
		+ Parameter
		+ RuntimeAppPublic
		+ Ord
		+ MaybeSerializeDeserialize
		+ Into<sp_core::sr25519::Public>
		+ From<sp_core::sr25519::Public>;
		type ClusterValidator: ClusterValidator<Self>;
		#[pallet::constant]
		type ValidatorsQuorum: Get<Percent>;

		const MAX_PAYOUT_BATCH_SIZE: u16;

		const DISABLE_PAYOUTS_CUTOFF: bool;
		type OffchainIdentifierId: AppCrypto<Self::Public, Self::Signature>;

		const VERIFY_AGGREGATOR_RESPONSE_SIGNATURE: bool;
		const BLOCK_TO_START: u16;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		PayoutInitialized {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		ChargingStarted {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		Charged {
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			customer_id: T::AccountId,
			amount: u128,
		},
		ChargedPartially {
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			customer_id: T::AccountId,
			charged: u128,
			expected_to_charge: u128,
		},
		Indebted {
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			customer_id: T::AccountId,
			amount: u128,
		},
		ChargingFinished {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		TreasuryFeesCollected {
			cluster_id: ClusterId,
			era: DdcEra,
			amount: u128,
		},
		ClusterReserveFeesCollected {
			cluster_id: ClusterId,
			era: DdcEra,
			amount: u128,
		},
		ValidatorFeesCollected {
			cluster_id: ClusterId,
			era: DdcEra,
			amount: u128,
		},
		RewardingStarted {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		Rewarded {
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			node_provider_id: T::AccountId,
			rewarded: u128,
			expected_to_reward: u128,
		},
		ValidatorRewarded {
			cluster_id: ClusterId,
			era: DdcEra,
			validator_id: T::AccountId,
			amount: u128,
		},
		NotDistributedReward {
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			node_provider_id: T::AccountId,
			expected_reward: u128,
			distributed_reward: BalanceOf<T>,
		},
		NotDistributedOverallReward {
			cluster_id: ClusterId,
			era: DdcEra,
			expected_reward: u128,
			total_distributed_rewards: u128,
		},
		RewardingFinished {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		PayoutReceiptFinalized {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		ChargeError {
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			customer_id: T::AccountId,
			amount: u128,
			error: DispatchError,
		},
		PayoutFingerprintCommited {
			validator_id: T::AccountId,
			cluster_id: ClusterId,
			era_id: EhdEra,
			payers_merkle_root: PayableUsageHash,
			payees_merkle_root: PayableUsageHash,
		},
	}

	#[derive(Debug, Encode, Decode, Clone, TypeInfo, PartialEq)]
	pub enum OCWError {
		PaidEraRetrievalError {
			cluster_id: ClusterId,
		},
		/// Challenge Response Retrieval Error.
		ChallengeResponseError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: ddc_primitives::AggregateKey,
			aggregator: NodePubKey,
		},
		/// Traverse Response Retrieval Error.
		TraverseResponseError {
			cluster_id: ClusterId,
			era_id: DdcEra,
			aggregate_key: ddc_primitives::AggregateKey,
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
			bucket_id: ddc_primitives::BucketId,
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
		PayoutReceiptDoesNotExist,
		NotExpectedState,
		Unauthorized,
		BatchIndexAlreadyProcessed,
		BatchIndexIsOutOfRange,
		BatchesMissed,
		BatchIndexOverflow,
		BoundedVecOverflow,
		ArithmeticOverflow,
		NotExpectedClusterState,
		NotExpectedBucketState,
		BatchSizeIsOutOfBounds,
		ScoreRetrievalError,
		BadRequest,
		BatchValidationFailed,
		NoBucketWithId,
		NotBucketOwner,
		IncorrectClusterId,
		ClusterProtocolParamsNotSet,
		TotalStoredBytesLessThanZero,
		PayoutFingerprintCommitted,
		PayoutFingerprintDoesNotExist,
		NoQuorumOnPayoutFingerprint,
		FailedToCreateMerkleRoot,
		FailedToVerifyMerkleProof,
		//Added
		NodeRetrievalError,


	}

	#[pallet::storage]
	pub type PayoutReceipts<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		PaymentEra,
		PayoutReceipt<T>,
	>;

	#[pallet::storage]
	pub type DebtorCustomers<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ClusterId, Blake2_128Concat, T::AccountId, u128>;

	#[pallet::storage]
	pub type OwingProviders<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ClusterId, Blake2_128Concat, T::AccountId, u128>;

	/// List of validators.
	#[pallet::storage]
	pub type ValidatorSet<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	/// Validator stash key mapping
	#[pallet::storage]
	pub type ValidatorToStashKey<T: Config> = StorageMap<_, Identity, T::AccountId, T::AccountId>;

	pub(crate) trait Hashable {
		/// Hash of the entity
		fn hash<T: Config>(&self) -> H256;
	}

	#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Encode, Decode)]
	pub(crate) struct CustomerCharge(AccountId32, u128);

	#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Encode, Decode)]
	pub(crate) struct ProviderReward(AccountId32, u128);

	/// The Payout Receipt is used as a synchronization object during the multi-step payout process
	/// and contains overall information about the payout for a cluster in an era.
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct PayoutReceipt<T: Config> {
		pub state: PayoutState,
		pub vault: T::AccountId,
		pub fingerprint: Fingerprint,
		pub total_collected_charges: u128,
		pub total_distributed_rewards: u128,
		pub total_settled_fees: u128,
		pub charging_max_batch_index: BatchIndex,
		pub charging_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
		pub rewarding_max_batch_index: BatchIndex,
		pub rewarding_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
	}

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

	#[derive(Clone)]
	pub struct CustomerBatch {
		pub(crate) batch_index: BatchIndex,
		pub(crate) payers: Vec<CustomerCharge>,
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

	impl PayableEHDUsage {
		fn fingerprint(&self) -> Fingerprint {
			let mut data = self.cluster_id.encode();
			data.extend_from_slice(&self.ehd_id.encode());
			data.extend_from_slice(&self.payers_root.encode());
			data.extend_from_slice(&self.payees_root.encode());
			blake2_256(&data).into()
		}
	}

	impl<T: pallet::Config> Default for PayoutReceipt<T> {
		fn default() -> Self {
			Self {
				state: PayoutState::default(),
				vault: T::PalletId::get().into_account_truncating(),
				fingerprint: Default::default(),
				total_collected_charges: Zero::zero(),
				total_distributed_rewards: Zero::zero(),
				total_settled_fees: Zero::zero(),
				charging_max_batch_index: Zero::zero(),
				charging_processed_batches: BoundedBTreeSet::default(),
				rewarding_max_batch_index: Zero::zero(),
				rewarding_processed_batches: BoundedBTreeSet::default(),
			}
		}
	}

	#[derive(Clone)]
	pub struct ProviderBatch {
		pub(crate) batch_index: BatchIndex,
		pub(crate) payees: Vec<ProviderReward>,
		pub(crate) batch_proof: MMRProof,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Default)]
	// don't remove or change numbers, if needed add a new state to the end with new number
	// DAC uses the state value for integration!
	pub enum State {
		#[default]
		NotInitialized = 1,
		Initialized = 2,
		ChargingCustomers = 3,
		CustomersChargedWithFees = 4,
		RewardingProviders = 5,
		ProvidersRewarded = 6,
		Finalized = 7,
	}

	/// Payout Fingerprint includes payment-sensitive data used to validate the payouts for a
	/// cluster in an era. The required quorum of validators must agree on the same payout
	/// fingerprint and commit it to let the payout process begin. The `payers_merkle_root` and
	/// `payees_merkle_root` hashes are being used to verify batches of customers and providers
	/// during the payout.
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct PayoutFingerprint<AccountId> {
		pub ehd_id: String,
		pub cluster_id: ClusterId,
		pub payers_merkle_root: PayableUsageHash,
		pub payees_merkle_root: PayableUsageHash,
		pub validators: BTreeSet<AccountId>,
	}

	impl<AccountId> PayoutFingerprint<AccountId> {
		pub fn selective_hash<T: Config>(&self) -> Fingerprint {
			let mut data = self.cluster_id.encode();
			data.extend_from_slice(&self.ehd_id.encode());
			data.extend_from_slice(&self.payers_merkle_root.encode());
			data.extend_from_slice(&self.payees_merkle_root.encode());
			// we truncate the `validators` field on purpose as it's appendable collection that is
			// used for reaching the quorum on the payout fingerprint
			T::Hasher::hash(&data)
		}
	}

	#[pallet::storage]
	pub type PayoutFingerprints<T: Config> =
		StorageMap<_, Blake2_128Concat, Fingerprint, PayoutFingerprint<T::AccountId>>;

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
				log::debug!("Another payment OCW is already running, terminating...",);
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
			for cluster_id in clusters_ids {
				let payout_result =
					Self::start_payouts_phase(&cluster_id, &verification_account, &signer);
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


	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::call_index(0)]
		#[pallet::weight(100_000)] //FIXME: benchmark!
		#[allow(clippy::too_many_arguments)]
		pub fn commit_payout_fingerprint(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			ehd_id: String,
			payers_root: PayableUsageHash,
			payees_root: PayableUsageHash,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;
			//ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized); //TODO: Introduce it back
			<Self as PayoutProcessor<T>>::commit_payout_fingerprint(
				sender,
				cluster_id,
				ehd_id,
				payers_root,
				payees_root,
			)
		}

		#[pallet::call_index(1)]
		#[pallet::weight(100_000)] //FIXME: benchmark!
		pub fn begin_payout(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
			fingerprint: Fingerprint,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;
			//ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized); //TODO: Introduce it back
			<Self as PayoutProcessor<T>>::begin_payout(cluster_id, era_id, fingerprint)?;
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(100_000)] //FIXME: benchmark!
		pub fn begin_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized); //TODO: Introduce it back
			<Self as PayoutProcessor<T>>::begin_charging_customers(cluster_id, era_id, max_batch_index)
		}

		#[pallet::call_index(3)]
		#[pallet::weight(100_000)] //FIXME: benchmark!
		pub fn send_charging_customers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
			batch_index: BatchIndex,
			payers: Vec<(T::AccountId, u128)>,
			batch_proof: MMRProof,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized); //TODO: Introduce it back
			<Self as PayoutProcessor<T>>::send_charging_customers_batch(
				cluster_id,
				era_id,
				batch_index,
				&payers,
				batch_proof,
			)
		}

		#[pallet::call_index(4)]
		#[pallet::weight(100_000)] //FIXME: benchmark!
		pub fn end_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized); //TODO: Introduce it back
			<Self as PayoutProcessor<T>>::end_charging_customers(cluster_id, era_id)
		}

		#[pallet::call_index(5)]
		#[pallet::weight(100_000)] //FIXME: benchmark!
		pub fn begin_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized); //TODO: Introduce it back
			<Self as PayoutProcessor<T>>::begin_rewarding_providers(cluster_id, era_id, max_batch_index)
		}

		#[pallet::call_index(6)]
		#[pallet::weight(100_000)] //FIXME: benchmark!
		pub fn send_rewarding_providers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
			batch_index: BatchIndex,
			payees: Vec<(T::AccountId, u128)>,
			batch_proof: MMRProof,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized); //TODO: Introduce it back
			<Self as PayoutProcessor<T>>::send_rewarding_providers_batch(
				cluster_id,
				era_id,
				batch_index,
				&payees,
				batch_proof,
			)
		}

		#[pallet::call_index(7)]
		#[pallet::weight(100_000)] //FIXME: benchmark!
		pub fn end_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized); //TODO: Introduce it back
			<Self as PayoutProcessor<T>>::end_rewarding_providers(cluster_id, era_id)
		}

		#[pallet::call_index(8)]
		#[pallet::weight(100_000)] //FIXME: benchmark!
		pub fn end_payout(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era_id: PaymentEra,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//ensure!(Self::is_ocw_validator(sender.clone()), Error::<T>::Unauthorized); //TODO: Introduce it back
			<Self as PayoutProcessor<T>>::end_payout(cluster_id, era_id)?;
			T::ClusterValidator::set_last_paid_era(&cluster_id, era_id)
		}
	}

	fn charge_treasury_fees<T: Config>(
		treasury_fee: u128,
		vault: &T::AccountId,
		treasury_vault: &T::AccountId,
	) -> DispatchResult {
		let amount_to_deduct = treasury_fee.saturated_into::<BalanceOf<T>>();
		<T as pallet::Config>::Currency::transfer(
			vault,
			treasury_vault,
			amount_to_deduct,
			ExistenceRequirement::AllowDeath,
		)
	}

	fn charge_cluster_reserve_fees<T: Config>(
		cluster_reserve_fee: u128,
		vault: &T::AccountId,
		reserve_vault: &T::AccountId,
	) -> DispatchResult {
		let amount_to_deduct = cluster_reserve_fee.saturated_into::<BalanceOf<T>>();
		<T as pallet::Config>::Currency::transfer(
			vault,
			reserve_vault,
			amount_to_deduct,
			ExistenceRequirement::AllowDeath,
		)
	}

	fn get_current_exposure_ratios<T: Config>(
	) -> Result<Vec<(T::AccountId, Perquintill)>, DispatchError> {
		let mut total_score = 0;
		let mut individual_scores: Vec<(T::AccountId, u64)> = Vec::new();
		for staker_id in T::NominatorsAndValidatorsList::iter() {
			let s = T::NominatorsAndValidatorsList::get_score(&staker_id)
				.map_err(|_| Error::<T>::ScoreRetrievalError)?;
			let score = T::VoteScoreToU64::convert(s);
			total_score += score;

			individual_scores.push((staker_id, score));
		}

		let mut result = Vec::new();
		for (staker_id, score) in individual_scores {
			let ratio = Perquintill::from_rational(score, total_score);
			result.push((staker_id, ratio));
		}

		Ok(result)
	}

	fn charge_validator_fees<T: Config>(
		validators_fee: u128,
		vault: &T::AccountId,
		cluster_id: ClusterId,
		era: DdcEra,
	) -> DispatchResult {
		let stakers = get_current_exposure_ratios::<T>()?;

		for (staker_id, ratio) in stakers.iter() {
			let amount_to_deduct = *ratio * validators_fee;

			<T as pallet::Config>::Currency::transfer(
				vault,
				staker_id,
				amount_to_deduct.saturated_into::<BalanceOf<T>>(),
				ExistenceRequirement::AllowDeath,
			)?;

			pallet::Pallet::deposit_event(Event::<T>::ValidatorRewarded {
				cluster_id,
				era,
				validator_id: staker_id.clone(),
				amount: amount_to_deduct,
			});
		}

		Ok(())
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub feeder_account: Option<T::AccountId>,
		pub authorised_caller: Option<T::AccountId>,
		pub debtor_customers: Vec<(ClusterId, T::AccountId, u128)>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				feeder_account: None,
				authorised_caller: Default::default(),
				debtor_customers: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			let account_id = <Pallet<T>>::account_id();
			let min = <T as pallet::Config>::Currency::minimum_balance();
			let balance = <T as pallet::Config>::Currency::free_balance(&account_id);
			if balance < min {
				if let Some(vault) = &self.feeder_account {
					let _ = <T as pallet::Config>::Currency::transfer(
						vault,
						&account_id,
						min - balance,
						ExistenceRequirement::AllowDeath,
					);
				} else {
					let _ = <T as pallet::Config>::Currency::make_free_balance_be(&account_id, min);
				}
			}

			for (cluster_id, customer_id, debt) in &self.debtor_customers {
				DebtorCustomers::<T>::insert(cluster_id, customer_id, debt);
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		pub fn get_account_id_string(caller: T::AccountId) -> String {
			let account_id: T::AccountIdConverter = caller.into();
			let account_id_32: AccountId32 = account_id.into();
			let account_ref: &[u8; 32] = account_id_32.as_ref();
			hex::encode(account_ref)
		}
		pub fn sub_account_id(cluster_id: ClusterId, era: DdcEra) -> T::AccountId {
			let mut bytes = Vec::new();
			bytes.extend_from_slice(&cluster_id[..]);
			bytes.extend_from_slice(&era.encode());
			let hash = blake2_128(&bytes);

			// "modl" + "payouts_" + hash is 28 bytes, the T::AccountId is 32 bytes, so we should be
			// safe from the truncation and possible collisions caused by it. The rest 4 bytes will
			// be fulfilled with trailing zeros.
			T::PalletId::get().into_sub_account_truncating(hash)
		}

		pub(crate) fn validate_batches(
			batches: &BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
			max_batch_index: &BatchIndex,
		) -> DispatchResult {
			// Check if the Vec contains all integers between 1 and rewarding_max_batch_index
			ensure!(!batches.is_empty(), Error::<T>::BatchesMissed);

			ensure!((*max_batch_index + 1) as usize == batches.len(), Error::<T>::BatchesMissed);

			for index in 0..*max_batch_index + 1 {
				ensure!(batches.contains(&index), Error::<T>::BatchesMissed);
			}

			Ok(())
		}

		fn is_customers_batch_valid(
			payers_merkle_root: PayableUsageHash,
			batch_index: BatchIndex,
			max_batch_index: BatchIndex,
			payers: &[(T::AccountId, u128)],
			batch_proof: &MMRProof,
		) -> Result<bool, DispatchError> {
			let payers_batch = payers
				.iter()
				.map(|(customer_id, amount)| {
					let mut data = customer_id.encode();
					data.extend_from_slice(&amount.encode());
					T::Hasher::hash(&data)
				})
				.collect::<Vec<_>>();

			let batch_hash = Self::create_merkle_root_from_leaves(payers_batch.as_slice()).unwrap();//FIXME: remove unwrap

			let is_verified = Self::proof_merkle_leaf(
				payers_merkle_root,
				batch_hash,
				batch_index,
				max_batch_index,
				batch_proof,
			)?;

			Ok(is_verified)
		}

		fn is_providers_batch_valid(
			payees_merkle_root: PayableUsageHash,
			batch_index: BatchIndex,
			max_batch_index: BatchIndex,
			payees: &[(T::AccountId, u128)],
			batch_proof: &MMRProof,
		) -> Result<bool, DispatchError> {
			let payees_batch = payees
				.iter()
				.map(|(provider_id, amount)| {
					let mut data = provider_id.encode();
					data.extend_from_slice(&amount.encode());
					T::Hasher::hash(&data)
				})
				.collect::<Vec<_>>();

			let batch_hash = Self::create_merkle_root_from_leaves(payees_batch.as_slice()).unwrap(); //TODO: remove unwrap

			let is_verified = Self::proof_merkle_leaf(
				payees_merkle_root,
				batch_hash,
				batch_index,
				max_batch_index,
				batch_proof,
			)?;

			Ok(is_verified)
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

		pub(crate) fn create_merkle_root_from_leaves(
			leaves: &[PayableUsageHash],
		) -> Result<PayableUsageHash, DispatchError> {
			if leaves.is_empty() {
				return Ok(PayableUsageHash::default());
			}

			let store = MemStore::default();
			let mut mmr: MMR<PayableUsageHash, MergeMMRHash, &MemStore<PayableUsageHash>> =
				MemMMR::<_, MergeMMRHash>::new(0, &store);

			let mut leaves_with_position: Vec<(u64, PayableUsageHash)> =
				Vec::with_capacity(leaves.len());

			for &leaf in leaves {
				match mmr.push(leaf) {
					Ok(pos) => leaves_with_position.push((pos, leaf)),
					Err(_) => {
						return Err(Error::<T>::FailedToCreateMerkleRoot.into());
					},
				}
			}

			Ok(mmr.get_root().map_err(|_| Error::<T>::FailedToCreateMerkleRoot)?)
		}

		/// Verify whether leaf is part of tree
		///
		/// Parameters:
		/// - `root_hash`: merkle root hash
		/// - `batch_hash`: hash of the batch
		/// - `batch_index`: index of the batch
		/// - `batch_proof`: MMR proofs
		pub(crate) fn proof_merkle_leaf(
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
				if Self::get_payout_state(cluster_id, ehd_id.2) ==
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

		pub(crate) fn prepare_send_charging_customers_batch(
			cluster_id: &ClusterId,
		) -> Result<Option<(EHDId, CustomerBatch)>, Vec<OCWError>> {
			let batch_size = T::MAX_PAYOUT_BATCH_SIZE;

			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				if Self::get_payout_state(cluster_id, ehd_id.2) ==
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

		pub(crate) fn prepare_end_charging_customers(
			cluster_id: &ClusterId,
		) -> Result<Option<EHDId>, Vec<OCWError>> {
			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				if Self::get_payout_state(cluster_id, ehd_id.2) ==
					PayoutState::ChargingCustomers &&
					Self::is_customers_charging_finished(cluster_id, ehd_id.2)
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
				if Self::get_payout_state(cluster_id, ehd_id.2) ==
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

		pub(crate) fn prepare_send_rewarding_providers_batch(
			cluster_id: &ClusterId,
		) -> Result<Option<(EHDId, ProviderBatch)>, Vec<OCWError>> {
			let batch_size = T::MAX_PAYOUT_BATCH_SIZE;

			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				if Self::get_payout_state(cluster_id, ehd_id.2) ==
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

		pub(crate) fn prepare_end_rewarding_providers(
			cluster_id: &ClusterId,
		) -> Result<Option<EHDId>, Vec<OCWError>> {
			if let Some(ehd_id) = Self::get_ehd_id_for_payout(cluster_id) {
				if Self::get_payout_state(cluster_id, ehd_id.2) ==
					PayoutState::RewardingProviders &&
					Self::is_providers_rewarding_finished(cluster_id, ehd_id.2)
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
				if Self::get_payout_state(cluster_id, ehd_id.2) ==
					PayoutState::ProvidersRewarded
				{
					return Ok(Some(ehd_id));
				}
			}
			Ok(None)
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

		fn fetch_inspection_receipts(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
		) -> Result<BTreeMap<String, ddc_primitives::aggregator::json::GroupedInspectionReceipt>, OCWError>
		{
			// todo(yahortsaryk): infer the node deterministically
			let g_collector = Self::get_g_collectors_nodes(cluster_id)
				.map_err(|_| OCWError::FailedToFetchGCollectors { cluster_id: *cluster_id })?
				.first()
				.cloned()
				.ok_or(OCWError::FailedToFetchGCollectors { cluster_id: *cluster_id })?;

			if let Ok(host) = str::from_utf8(&g_collector.1.host) {
				let base_url = format!("http://{}:{}", host, g_collector.1.http_port);
				let client = ddc_primitives::aggregator::AggregatorClient::new(
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

		/// Fetch grouping collectors nodes of a cluster.
		/// Parameters:
		/// - `cluster_id`: Cluster id of a cluster.
		fn get_g_collectors_nodes(
			cluster_id: &ClusterId,
		) -> Result<Vec<(NodePubKey, ddc_primitives::StorageNodeParams)>, Error<T>> {
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

		fn fetch_rewarding_providers_batch(
			cluster_id: &ClusterId,
			batch_size: usize,
			ehd_id: EHDId,
			payees_usage: Vec<ProviderReward>,
			payees_batch_roots: Vec<PayableUsageHash>,
		) -> Result<Option<(EHDId, ProviderBatch)>, Vec<OCWError>> {
			let batch_index = <Self as PayoutProcessor<T>>::get_next_providers_batch(cluster_id, ehd_id.2)
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

		pub(crate) fn derive_ehd_paybale_usage_key(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
		) -> Vec<u8> {
			format!("offchain::paybale_usage::{:?}::{:?}", cluster_id, Into::<String>::into(ehd_id))
				.into_bytes()
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

		pub(crate) fn get_ehd_root(
			cluster_id: &ClusterId,
			ehd_id: EHDId,
		) -> Result<aggregator_client::json::TraversedEHDResponse, OCWError> {
			Self::fetch_traversed_era_historical_document(cluster_id, ehd_id, 1, 1)?
				.first()
				.ok_or(OCWError::FailedToFetchTraversedEHD)
				.cloned()
		}

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

		pub(crate) fn derive_last_inspected_ehd_key(cluster_id: &ClusterId) -> Vec<u8> {
			format!("offchain::inspected_ehds::v1::{:?}", cluster_id).into_bytes()
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

		fn fetch_charging_customers_batch(
			cluster_id: &ClusterId,
			batch_size: usize,
			ehd_id: EHDId,
			payers_usage: Vec<CustomerCharge>,
			payers_batch_roots: Vec<PayableUsageHash>,
		) -> Result<Option<(EHDId, CustomerBatch)>, Vec<OCWError>> {
			let batch_index = <Self as PayoutProcessor<T>>::get_next_customers_batch(cluster_id, ehd_id.2)
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
	}

	impl<T: Config> PayoutProcessor<T> for Pallet<T> {
		fn commit_payout_fingerprint(
			validator: T::AccountId,
			cluster_id: ClusterId,
			ehd_id: String,
			payers_merkle_root: PayableUsageHash,
			payees_merkle_root: PayableUsageHash,
		) -> DispatchResult {
			ensure!(payers_merkle_root != Default::default(), Error::<T>::BadRequest);
			ensure!(payees_merkle_root != Default::default(), Error::<T>::BadRequest);
			let last_paid_era = T::ClusterValidator::get_last_paid_era(&cluster_id)?;
			let era_id = EHDId::try_from(ehd_id.clone()).map_err(|_| Error::<T>::BadRequest)?.2;
			ensure!(era_id > last_paid_era, Error::<T>::BadRequest);
			let inited_payout_fingerprint = PayoutFingerprint::<T::AccountId> {
				cluster_id,
				ehd_id,
				payers_merkle_root,
				payees_merkle_root,
				validators: Default::default(),
			};
			let fingerprint = inited_payout_fingerprint.selective_hash::<T>();

			let mut payout_fingerprint = if let Some(commited_payout_fingerprint) =
				PayoutFingerprints::<T>::get(fingerprint)
			{
				commited_payout_fingerprint
			} else {
				inited_payout_fingerprint
			};

			ensure!(
				payout_fingerprint.validators.insert(validator.clone()),
				Error::<T>::PayoutFingerprintCommitted
			);

			PayoutFingerprints::<T>::insert(fingerprint, payout_fingerprint);
			Self::deposit_event(Event::<T>::PayoutFingerprintCommited {
				validator_id: validator,
				cluster_id,
				era_id,
				payers_merkle_root,
				payees_merkle_root,
			});

			Ok(())
		}

		fn begin_payout(
			cluster_id: ClusterId,
			era: PaymentEra,
			fingerprint: Fingerprint,
		) -> DispatchResult {
			ensure!(
				PayoutReceipts::<T>::try_get(cluster_id, era).is_err(),
				Error::<T>::NotExpectedState
			);

			let payout_fingerprint = PayoutFingerprints::<T>::try_get(fingerprint)
				.map_err(|_| Error::<T>::PayoutFingerprintDoesNotExist)?;

			ensure!(
				T::ValidatorVisitor::is_quorum_reached(
					T::ValidatorsQuorum::get(),
					payout_fingerprint.validators.len(),
				),
				Error::<T>::NoQuorumOnPayoutFingerprint
			);

			let payout_receipt = PayoutReceipt::<T> {
				vault: Self::account_id(),
				fingerprint,
				state: PayoutState::Initialized,
				..Default::default()
			};
			PayoutReceipts::<T>::insert(cluster_id, era, payout_receipt);

			Self::deposit_event(Event::<T>::PayoutInitialized { cluster_id, era });

			Ok(())
		}

		fn begin_charging_customers(
			cluster_id: ClusterId,
			era: PaymentEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			ensure!(max_batch_index < MaxBatchesCount::get(), Error::<T>::BatchIndexOverflow);

			let mut payout_receipt = PayoutReceipts::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::PayoutReceiptDoesNotExist)?;

			ensure!(payout_receipt.state == PayoutState::Initialized, Error::<T>::NotExpectedState);

			payout_receipt.charging_max_batch_index = max_batch_index;
			payout_receipt.state = PayoutState::ChargingCustomers;
			PayoutReceipts::<T>::insert(cluster_id, era, payout_receipt);

			Self::deposit_event(Event::<T>::ChargingStarted { cluster_id, era });

			Ok(())
		}

		fn send_charging_customers_batch(
			cluster_id: ClusterId,
			era: PaymentEra,
			batch_index: BatchIndex,
			payers: &[(T::AccountId, u128)],
			batch_proof: MMRProof,
		) -> DispatchResult {
			ensure!(
				!payers.is_empty() && payers.len() <= MaxBatchSize::get() as usize,
				Error::<T>::BatchSizeIsOutOfBounds
			);

			let payout_receipt = PayoutReceipts::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::PayoutReceiptDoesNotExist)?;

			ensure!(
				payout_receipt.state == PayoutState::ChargingCustomers,
				Error::<T>::NotExpectedState
			);

			ensure!(
				payout_receipt.charging_max_batch_index >= batch_index,
				Error::<T>::BatchIndexIsOutOfRange
			);

			ensure!(
				!payout_receipt.charging_processed_batches.contains(&batch_index),
				Error::<T>::BatchIndexAlreadyProcessed
			);

			let payout_fingerprint = PayoutFingerprints::<T>::try_get(payout_receipt.fingerprint)
				.map_err(|_| Error::<T>::PayoutFingerprintDoesNotExist)?;

			let is_batch_verifed = Self::is_customers_batch_valid(
				payout_fingerprint.payers_merkle_root,
				batch_index,
				payout_receipt.charging_max_batch_index,
				payers,
				&batch_proof,
			)?;

			ensure!(is_batch_verifed, Error::<T>::BatchValidationFailed);

			let mut updated_payout_receipt = payout_receipt;
			for (customer_id, total_charge) in payers {
				let actual_charge = match T::CustomerCharger::charge_customer(
					customer_id.clone(),
					updated_payout_receipt.vault.clone(),
					*total_charge,
				) {
					Ok(actual_charge) => actual_charge,
					Err(e) => {
						Self::deposit_event(Event::<T>::ChargeError {
							cluster_id,
							era,
							batch_index,
							customer_id: customer_id.clone(),
							amount: *total_charge,
							error: e,
						});
						0
					},
				};

				if actual_charge < *total_charge {
					// debt
					let mut customer_debt =
						DebtorCustomers::<T>::try_get(cluster_id, customer_id.clone())
							.unwrap_or_else(|_| Zero::zero());

					let debt = total_charge
						.checked_sub(actual_charge)
						.ok_or(Error::<T>::ArithmeticOverflow)?;

					customer_debt =
						customer_debt.checked_add(debt).ok_or(Error::<T>::ArithmeticOverflow)?;

					DebtorCustomers::<T>::insert(cluster_id, customer_id.clone(), customer_debt);

					Self::deposit_event(Event::<T>::Indebted {
						cluster_id,
						era,
						batch_index,
						customer_id: customer_id.clone(),
						amount: debt,
					});

					if actual_charge > 0 {
						Self::deposit_event(Event::<T>::ChargedPartially {
							cluster_id,
							era,
							batch_index,
							customer_id: customer_id.clone(),
							charged: actual_charge,
							expected_to_charge: *total_charge,
						});
					}
				} else {
					Self::deposit_event(Event::<T>::Charged {
						cluster_id,
						era,
						batch_index,
						customer_id: customer_id.clone(),
						amount: *total_charge,
					});
				}

				updated_payout_receipt.total_collected_charges = updated_payout_receipt
					.total_collected_charges
					.checked_add(actual_charge.saturated_into::<u128>())
					.ok_or(Error::<T>::ArithmeticOverflow)?;
			}

			updated_payout_receipt
				.charging_processed_batches
				.try_insert(batch_index)
				.map_err(|_| Error::<T>::BoundedVecOverflow)?;

			PayoutReceipts::<T>::insert(cluster_id, era, updated_payout_receipt);

			Ok(())
		}

		fn end_charging_customers(cluster_id: ClusterId, era: PaymentEra) -> DispatchResult {
			let mut payout_receipt = PayoutReceipts::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::PayoutReceiptDoesNotExist)?;

			ensure!(
				payout_receipt.state == PayoutState::ChargingCustomers,
				Error::<T>::NotExpectedState
			);
			Self::validate_batches(
				&payout_receipt.charging_processed_batches,
				&payout_receipt.charging_max_batch_index,
			)?;

			Self::deposit_event(Event::<T>::ChargingFinished { cluster_id, era });

			let fees = T::ClusterProtocol::get_fees_params(&cluster_id)
				.map_err(|_| Error::<T>::ClusterProtocolParamsNotSet)?;

			let treasury_fee = fees.treasury_share * payout_receipt.total_collected_charges;
			let validators_fee = fees.validators_share * payout_receipt.total_collected_charges;
			let cluster_reserve_fee =
				fees.cluster_reserve_share * payout_receipt.total_collected_charges;

			if treasury_fee > 0 {
				charge_treasury_fees::<T>(
					treasury_fee,
					&payout_receipt.vault,
					&T::TreasuryVisitor::get_account_id(),
				)?;

				payout_receipt.total_settled_fees = payout_receipt
					.total_settled_fees
					.checked_add(treasury_fee.saturated_into::<u128>())
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				Self::deposit_event(Event::<T>::TreasuryFeesCollected {
					cluster_id,
					era,
					amount: treasury_fee,
				});
			}

			if cluster_reserve_fee > 0 {
				charge_cluster_reserve_fees::<T>(
					cluster_reserve_fee,
					&payout_receipt.vault,
					&T::ClusterProtocol::get_reserve_account_id(&cluster_id)
						.map_err(|_| Error::<T>::NotExpectedClusterState)?,
				)?;

				payout_receipt.total_settled_fees = payout_receipt
					.total_settled_fees
					.checked_add(cluster_reserve_fee.saturated_into::<u128>())
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				Self::deposit_event(Event::<T>::ClusterReserveFeesCollected {
					cluster_id,
					era,
					amount: cluster_reserve_fee,
				});
			}

			if validators_fee > 0 {
				charge_validator_fees::<T>(validators_fee, &payout_receipt.vault, cluster_id, era)?;

				payout_receipt.total_settled_fees = payout_receipt
					.total_settled_fees
					.checked_add(validators_fee.saturated_into::<u128>())
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				Self::deposit_event(Event::<T>::ValidatorFeesCollected {
					cluster_id,
					era,
					amount: validators_fee,
				});
			}

			payout_receipt.state = PayoutState::CustomersChargedWithFees;
			PayoutReceipts::<T>::insert(cluster_id, era, payout_receipt);

			Ok(())
		}

		fn begin_rewarding_providers(
			cluster_id: ClusterId,
			era: PaymentEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			ensure!(max_batch_index < MaxBatchesCount::get(), Error::<T>::BatchIndexOverflow);

			let mut payout_receipt = PayoutReceipts::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::PayoutReceiptDoesNotExist)?;

			ensure!(
				payout_receipt.state == PayoutState::CustomersChargedWithFees,
				Error::<T>::NotExpectedState
			);

			payout_receipt.rewarding_max_batch_index = max_batch_index;
			payout_receipt.state = PayoutState::RewardingProviders;
			PayoutReceipts::<T>::insert(cluster_id, era, payout_receipt);

			Self::deposit_event(Event::<T>::RewardingStarted { cluster_id, era });

			Ok(())
		}

		fn send_rewarding_providers_batch(
			cluster_id: ClusterId,
			era: PaymentEra,
			batch_index: BatchIndex,
			payees: &[(T::AccountId, u128)],
			batch_proof: MMRProof,
		) -> DispatchResult {
			ensure!(
				!payees.is_empty() && payees.len() <= MaxBatchSize::get() as usize,
				Error::<T>::BatchSizeIsOutOfBounds
			);

			let payout_receipt = PayoutReceipts::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::PayoutReceiptDoesNotExist)?;

			ensure!(
				payout_receipt.state == PayoutState::RewardingProviders,
				Error::<T>::NotExpectedState
			);
			ensure!(
				payout_receipt.rewarding_max_batch_index >= batch_index,
				Error::<T>::BatchIndexIsOutOfRange
			);
			ensure!(
				!payout_receipt.rewarding_processed_batches.contains(&batch_index),
				Error::<T>::BatchIndexAlreadyProcessed
			);

			let payout_fingerprint = PayoutFingerprints::<T>::try_get(payout_receipt.fingerprint)
				.map_err(|_| Error::<T>::PayoutFingerprintDoesNotExist)?;

			let is_batch_verified = Self::is_providers_batch_valid(
				payout_fingerprint.payees_merkle_root,
				batch_index,
				payout_receipt.rewarding_max_batch_index,
				payees,
				&batch_proof,
			)?;

			ensure!(is_batch_verified, Error::<T>::BatchValidationFailed);

			let max_dust = MaxDust::get().saturated_into::<BalanceOf<T>>();
			let mut updated_payout_receipt = payout_receipt.clone();
			for (provider_id, total_reward) in payees {
				let mut actual_reward: BalanceOf<T> =
					(*total_reward).saturated_into::<BalanceOf<T>>();

				if *total_reward > 0 {
					let vault_balance = T::Currency::free_balance(&updated_payout_receipt.vault)
						.saturating_sub(T::Currency::minimum_balance());

					// 10000000000001 > 10000000000000 but is still ok
					if actual_reward > vault_balance {
						if actual_reward - vault_balance > max_dust {
							Self::deposit_event(Event::<T>::NotDistributedReward {
								cluster_id,
								era,
								batch_index,
								node_provider_id: provider_id.clone(),
								expected_reward: *total_reward,
								distributed_reward: vault_balance,
							});
						}

						actual_reward = vault_balance;
					}

					T::Currency::transfer(
						&updated_payout_receipt.vault,
						provider_id,
						actual_reward,
						ExistenceRequirement::AllowDeath,
					)?;

					updated_payout_receipt.total_distributed_rewards = updated_payout_receipt
						.total_distributed_rewards
						.checked_add(actual_reward.saturated_into::<u128>())
						.ok_or(Error::<T>::ArithmeticOverflow)?;
				}

				Self::deposit_event(Event::<T>::Rewarded {
					cluster_id,
					era,
					batch_index,
					node_provider_id: provider_id.clone(),
					rewarded: actual_reward.saturated_into(),
					expected_to_reward: *total_reward,
				});
			}

			updated_payout_receipt
				.rewarding_processed_batches
				.try_insert(batch_index)
				.map_err(|_| Error::<T>::BoundedVecOverflow)?;

			PayoutReceipts::<T>::insert(cluster_id, era, updated_payout_receipt);

			Ok(())
		}

		fn end_rewarding_providers(cluster_id: ClusterId, era: PaymentEra) -> DispatchResult {
			let mut payout_receipt = PayoutReceipts::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::PayoutReceiptDoesNotExist)?;

			ensure!(
				payout_receipt.state == PayoutState::RewardingProviders,
				Error::<T>::NotExpectedState
			);

			Self::validate_batches(
				&payout_receipt.rewarding_processed_batches,
				&payout_receipt.rewarding_max_batch_index,
			)?;

			if payout_receipt
				.total_collected_charges
				.saturating_sub(payout_receipt.total_distributed_rewards)
				.saturating_sub(payout_receipt.total_settled_fees) >
				MaxDust::get()
			{
				Self::deposit_event(Event::<T>::NotDistributedOverallReward {
					cluster_id,
					era,
					expected_reward: payout_receipt
						.total_collected_charges
						.saturating_sub(payout_receipt.total_settled_fees),
					total_distributed_rewards: payout_receipt.total_distributed_rewards,
				});
			}

			payout_receipt.state = PayoutState::ProvidersRewarded;
			PayoutReceipts::<T>::insert(cluster_id, era, payout_receipt);

			Self::deposit_event(Event::<T>::RewardingFinished { cluster_id, era });

			Ok(())
		}

		fn end_payout(cluster_id: ClusterId, era: PaymentEra) -> DispatchResult {
			let mut payout_receipt = PayoutReceipts::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::PayoutReceiptDoesNotExist)?;

			ensure!(
				payout_receipt.state == PayoutState::ProvidersRewarded,
				Error::<T>::NotExpectedState
			);

			payout_receipt.charging_processed_batches.clear();
			payout_receipt.rewarding_processed_batches.clear();
			payout_receipt.state = PayoutState::Finalized;

			PayoutReceipts::<T>::insert(cluster_id, era, payout_receipt);
			Self::deposit_event(Event::<T>::PayoutReceiptFinalized { cluster_id, era });

			Ok(())
		}

		fn get_payout_state(cluster_id: &ClusterId, era: PaymentEra) -> PayoutState {
			let payout_receipt = PayoutReceipts::<T>::get(cluster_id, era);
			match payout_receipt {
				Some(report) => report.state,
				None => PayoutState::NotInitialized,
			}
		}

		fn is_customers_charging_finished(cluster_id: &ClusterId, era_id: PaymentEra) -> bool {
			let payout_receipt = match PayoutReceipts::<T>::try_get(cluster_id, era_id) {
				Ok(report) => report,
				Err(_) => return false,
			};

			Self::validate_batches(
				&payout_receipt.charging_processed_batches,
				&payout_receipt.charging_max_batch_index,
			)
			.is_ok()
		}

		fn is_providers_rewarding_finished(cluster_id: &ClusterId, era_id: PaymentEra) -> bool {
			let payout_receipt = match PayoutReceipts::<T>::try_get(cluster_id, era_id) {
				Ok(report) => report,
				Err(_) => return false,
			};

			Self::validate_batches(
				&payout_receipt.rewarding_processed_batches,
				&payout_receipt.rewarding_max_batch_index,
			)
			.is_ok()
		}

		fn get_next_customers_batch(
			cluster_id: &ClusterId,
			era_id: PaymentEra,
		) -> Result<Option<BatchIndex>, PayoutError> {
			let payout_receipt = PayoutReceipts::<T>::try_get(cluster_id, era_id)
				.map_err(|_| PayoutError::PayoutReceiptDoesNotExist)?;

			for batch_index in 0..=payout_receipt.charging_max_batch_index {
				if !payout_receipt.charging_processed_batches.contains(&batch_index) {
					return Ok(Some(batch_index));
				}
			}

			Ok(None)
		}

		fn get_next_providers_batch(
			cluster_id: &ClusterId,
			era_id: PaymentEra,
		) -> Result<Option<BatchIndex>, PayoutError> {
			let payout_receipt = PayoutReceipts::<T>::try_get(cluster_id, era_id)
				.map_err(|_| PayoutError::PayoutReceiptDoesNotExist)?;

			for batch_index in 0..=payout_receipt.rewarding_max_batch_index {
				if !payout_receipt.rewarding_processed_batches.contains(&batch_index) {
					return Ok(Some(batch_index));
				}
			}

			Ok(None)
		}

		fn create_payout_receipt(vault: T::AccountId, params: PayoutReceiptParams) {
			let mut charging_processed_batches =
				BoundedBTreeSet::<BatchIndex, MaxBatchesCount>::new();
			for batch in params.charging_processed_batches {
				charging_processed_batches
					.try_insert(batch)
					.expect("Charging batch to be inserted");
			}

			let mut rewarding_processed_batches =
				BoundedBTreeSet::<BatchIndex, MaxBatchesCount>::new();
			for batch in params.rewarding_processed_batches {
				rewarding_processed_batches
					.try_insert(batch)
					.expect("Rewarding batch to be inserted");
			}

			let payout_receipt = PayoutReceipt::<T> {
				vault,
				state: params.state,
				fingerprint: params.fingerprint,
				total_collected_charges: params.total_collected_charges,
				total_distributed_rewards: params.total_distributed_rewards,
				total_settled_fees: params.total_settled_fees,
				charging_max_batch_index: params.charging_max_batch_index,
				charging_processed_batches,
				rewarding_max_batch_index: params.rewarding_max_batch_index,
				rewarding_processed_batches,
			};

			PayoutReceipts::<T>::insert(params.cluster_id, params.era, payout_receipt);
		}

		fn create_payout_fingerprint(params: PayoutFingerprintParams<T::AccountId>) -> Fingerprint {
			let payout_fingerprint = PayoutFingerprint::<T::AccountId> {
				cluster_id: params.cluster_id,
				ehd_id: params.ehd_id,
				payers_merkle_root: params.payers_merkle_root,
				payees_merkle_root: params.payees_merkle_root,
				validators: params.validators,
			};

			let fingerprint = payout_fingerprint.selective_hash::<T>();
			PayoutFingerprints::<T>::insert(fingerprint, payout_fingerprint);

			fingerprint
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
}
