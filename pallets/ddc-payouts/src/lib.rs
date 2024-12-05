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

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

pub mod migrations;

use ddc_primitives::{
	traits::{
		bucket::BucketManager, cluster::ClusterProtocol as ClusterProtocolType,
		customer::CustomerCharger as CustomerChargerType, node::NodeManager,
		pallet::PalletVisitor as PalletVisitorType, payout::PayoutProcessor, ClusterValidator,
	},
	BatchIndex, BillingFingerprintParams, BillingReportParams, BucketId, BucketUsage, ClusterId,
	CustomerCharge, DdcEra, Fingerprint, MMRProof, MergeMMRHash, NodePubKey, NodeUsage,
	PayableUsageHash, PayoutError, PayoutState, ProviderReward, AVG_SECONDS_MONTH,
	MAX_PAYOUT_BATCH_COUNT, MAX_PAYOUT_BATCH_SIZE, MILLICENTS,
};
use frame_election_provider_support::SortedListProvider;
use frame_support::{
	pallet_prelude::*,
	parameter_types,
	sp_runtime::SaturatedConversion,
	traits::{Currency, ExistenceRequirement, Get, LockableCurrency},
	BoundedBTreeSet,
};
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
	AccountId32, PerThing, Percent, Perquintill,
};
use sp_std::{collections::btree_set::BTreeSet, prelude::*};
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct BillingReportDebt {
	pub cluster_id: ClusterId,
	pub era: DdcEra,
	pub batch_index: BatchIndex,
	pub amount: u128,
}

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
	use ddc_primitives::{traits::ValidatorVisitor, ClusterPricingParams};
	use frame_support::PalletId;
	use sp_io::hashing::blake2_128;
	use sp_runtime::traits::{AccountIdConversion, Zero};

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
		frame_support::traits::StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type Currency: LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>;
		type CustomerCharger: CustomerChargerType<Self>;
		type BucketManager: BucketManager<Self>;
		type NodeManager: NodeManager<Self>;
		type TreasuryVisitor: PalletVisitorType<Self>;
		type ClusterProtocol: ClusterProtocolType<Self, BalanceOf<Self>>;
		type NominatorsAndValidatorsList: SortedListProvider<Self::AccountId>;
		type VoteScoreToU64: Convert<VoteScoreOf<Self>, u64>;
		type ValidatorVisitor: ValidatorVisitor<Self>;
		type AccountIdConverter: From<Self::AccountId> + Into<AccountId32>;
		type Hasher: Hash<Output = H256>;
		type ClusterValidator: ClusterValidator<Self>;
		#[pallet::constant]
		type ValidatorsQuorum: Get<Percent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		BillingReportInitialized {
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
			bucket_id: BucketId,
			amount: u128,
		},
		ChargeFailed {
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			customer_id: T::AccountId,
			bucket_id: BucketId,
			charged: u128,
			expected_to_charge: u128,
		},
		Indebted {
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			customer_id: T::AccountId,
			bucket_id: BucketId,
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
			total_distributed_reward: u128,
		},
		RewardingFinished {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		BillingReportFinalized {
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
		BillingFingerprintCommited {
			validator_id: T::AccountId,
			cluster_id: ClusterId,
			era_id: DdcEra,
			start_era: i64,
			end_era: i64,
			payers_merkle_root: PayableUsageHash,
			payees_merkle_root: PayableUsageHash,
		},
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		BillingReportDoesNotExist,
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
		BillingFingerprintIsCommitted,
		BillingFingerprintDoesNotExist,
		NoQuorumOnBillingFingerprint,
		FailedToCreateMerkleRoot,
		FailedToVerifyMerkleProof,
	}

	#[pallet::storage]
	#[pallet::getter(fn active_billing_reports)]
	pub type ActiveBillingReports<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		DdcEra,
		BillingReport<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn debtor_customers)]
	pub type DebtorCustomers<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ClusterId, Blake2_128Concat, T::AccountId, u128>;

	#[pallet::storage]
	#[pallet::getter(fn owing_providers)]
	pub type OwingProviders<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ClusterId, Blake2_128Concat, T::AccountId, u128>;

	/// The Billing report is used as a synchronization object during the multi-step payout process
	/// and contains overall information about the payout for a cluster in an era.
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct BillingReport<T: Config> {
		pub state: PayoutState,
		pub vault: T::AccountId,
		pub fingerprint: Fingerprint,
		pub total_customer_charge: CustomerCharge,
		pub total_distributed_reward: u128,
		pub charging_max_batch_index: BatchIndex,
		pub charging_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
		pub rewarding_max_batch_index: BatchIndex,
		pub rewarding_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
	}

	impl<T: pallet::Config> Default for BillingReport<T> {
		fn default() -> Self {
			Self {
				state: PayoutState::default(),
				vault: T::PalletId::get().into_account_truncating(),
				fingerprint: Default::default(),
				total_customer_charge: CustomerCharge::default(),
				total_distributed_reward: Zero::zero(),
				charging_max_batch_index: Zero::zero(),
				charging_processed_batches: BoundedBTreeSet::default(),
				rewarding_max_batch_index: Zero::zero(),
				rewarding_processed_batches: BoundedBTreeSet::default(),
			}
		}
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

	/// Billing fingerprint includes payment-sensitive data used to validate the payouts for a
	/// cluster in an era. The required quorum of validators must agree on the same payout
	/// fingerprint and commit it to let the payout process begin. The `payers_merkle_root` and
	/// `payees_merkle_root` hashes are being used to verify batches of customers and providers
	/// during the payout.
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct BillingFingerprint<AccountId> {
		pub cluster_id: ClusterId,
		pub era_id: DdcEra,
		pub start_era: i64,
		pub end_era: i64,
		pub payers_merkle_root: PayableUsageHash,
		pub payees_merkle_root: PayableUsageHash,
		pub cluster_usage: NodeUsage,
		pub validators: BTreeSet<AccountId>,
	}

	impl<AccountId> BillingFingerprint<AccountId> {
		pub fn selective_hash<T: Config>(&self) -> Fingerprint {
			let mut data = self.cluster_id.encode();
			data.extend_from_slice(&self.era_id.encode());
			data.extend_from_slice(&self.start_era.encode());
			data.extend_from_slice(&self.end_era.encode());
			data.extend_from_slice(&self.payers_merkle_root.encode());
			data.extend_from_slice(&self.payees_merkle_root.encode());
			data.extend_from_slice(&self.cluster_usage.encode());
			// we truncate the `validators` field on purpose as it's appendable collection that is
			// used for reaching the quorum on the billing fingerprint
			T::Hasher::hash(&data)
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn billing_fingerprints)]
	pub type BillingFingerprints<T: Config> =
		StorageMap<_, Blake2_128Concat, Fingerprint, BillingFingerprint<T::AccountId>>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

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

	fn get_provider_reward(
		payable_usage: &NodeUsage,
		cluster_usage: &NodeUsage,
		total_customer_charge: &CustomerCharge,
	) -> Option<ProviderReward> {
		let mut total_reward = ProviderReward::default();

		let mut ratio = Perquintill::from_rational(
			payable_usage.transferred_bytes as u128,
			cluster_usage.transferred_bytes as u128,
		);

		// ratio multiplied by X will be > 0, < X no overflow
		total_reward.transfer = ratio * total_customer_charge.transfer;

		ratio = Perquintill::from_rational(
			payable_usage.stored_bytes as u128,
			cluster_usage.stored_bytes as u128,
		);
		total_reward.storage = ratio * total_customer_charge.storage;

		ratio =
			Perquintill::from_rational(payable_usage.number_of_puts, cluster_usage.number_of_puts);
		total_reward.puts = ratio * total_customer_charge.puts;

		ratio =
			Perquintill::from_rational(payable_usage.number_of_gets, cluster_usage.number_of_gets);
		total_reward.gets = ratio * total_customer_charge.gets;

		Some(total_reward)
	}

	#[allow(clippy::field_reassign_with_default)]
	fn get_customer_charge<T: Config>(
		pricing: &ClusterPricingParams,
		payable_usage: &BucketUsage,
		start_era: i64,
		end_era: i64,
	) -> Result<CustomerCharge, DispatchError> {
		let mut total_charge = CustomerCharge::default();

		total_charge.transfer = (|| -> Option<u128> {
			(payable_usage.transferred_bytes as u128)
				.checked_mul(pricing.unit_per_mb_streamed)?
				.checked_div(byte_unit::MEBIBYTE)
		})()
		.ok_or(Error::<T>::ArithmeticOverflow)?;

		// Calculate the duration of the period in seconds
		let duration_seconds = end_era - start_era;
		let fraction_of_month =
			Perquintill::from_rational(duration_seconds as u64, AVG_SECONDS_MONTH as u64);

		total_charge.storage = fraction_of_month
			* (|| -> Option<u128> {
				(payable_usage.stored_bytes as u128)
					.checked_mul(pricing.unit_per_mb_stored)?
					.checked_div(byte_unit::MEBIBYTE)
			})()
			.ok_or(Error::<T>::ArithmeticOverflow)?;

		total_charge.gets = (payable_usage.number_of_gets as u128)
			.checked_mul(pricing.unit_per_get_request)
			.ok_or(Error::<T>::ArithmeticOverflow)?;

		total_charge.puts = (payable_usage.number_of_puts as u128)
			.checked_mul(pricing.unit_per_put_request)
			.ok_or(Error::<T>::ArithmeticOverflow)?;

		Ok(total_charge)
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
			payers: &[(BucketId, BucketUsage)],
			batch_proof: &MMRProof,
		) -> Result<bool, DispatchError> {
			let payers_batch = payers
				.iter()
				.map(|(bucket_id, usage)| {
					let mut data = bucket_id.encode();
					data.extend_from_slice(&usage.stored_bytes.encode());
					data.extend_from_slice(&usage.transferred_bytes.encode());
					data.extend_from_slice(&usage.number_of_puts.encode());
					data.extend_from_slice(&usage.number_of_gets.encode());
					T::Hasher::hash(&data)
				})
				.collect::<Vec<_>>();

			let batch_hash = Self::create_merkle_root(payers_batch.as_slice())?;

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
			payees: &[(NodePubKey, NodeUsage)],
			batch_proof: &MMRProof,
		) -> Result<bool, DispatchError> {
			let payees_batch = payees
				.iter()
				.map(|(node_key, usage)| {
					let mut data = node_key.encode();
					data.extend_from_slice(&usage.stored_bytes.encode());
					data.extend_from_slice(&usage.transferred_bytes.encode());
					data.extend_from_slice(&usage.number_of_puts.encode());
					data.extend_from_slice(&usage.number_of_gets.encode());
					T::Hasher::hash(&data)
				})
				.collect::<Vec<_>>();

			let batch_hash = Self::create_merkle_root(payees_batch.as_slice())?;

			let is_verified = Self::proof_merkle_leaf(
				payees_merkle_root,
				batch_hash,
				batch_index,
				max_batch_index,
				batch_proof,
			)?;

			Ok(is_verified)
		}

		pub(crate) fn create_merkle_root(
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
	}

	impl<T: Config> PayoutProcessor<T> for Pallet<T> {
		fn commit_billing_fingerprint(
			validator: T::AccountId,
			cluster_id: ClusterId,
			era_id: DdcEra,
			start_era: i64,
			end_era: i64,
			payers_merkle_root: PayableUsageHash,
			payees_merkle_root: PayableUsageHash,
			cluster_usage: NodeUsage,
		) -> DispatchResult {
			ensure!(end_era > start_era, Error::<T>::BadRequest);
			ensure!(payers_merkle_root != Default::default(), Error::<T>::BadRequest);
			ensure!(payees_merkle_root != Default::default(), Error::<T>::BadRequest);

			let last_paid_era = T::ClusterValidator::get_last_paid_era(&cluster_id)?;
			ensure!(era_id > last_paid_era, Error::<T>::BadRequest);

			let inited_billing_fingerprint = BillingFingerprint::<T::AccountId> {
				cluster_id,
				era_id,
				start_era,
				end_era,
				payers_merkle_root,
				payees_merkle_root,
				cluster_usage,
				validators: Default::default(),
			};
			let fingerprint = inited_billing_fingerprint.selective_hash::<T>();

			let mut billing_fingerprint = if let Some(commited_billing_fingerprint) =
				BillingFingerprints::<T>::get(fingerprint)
			{
				commited_billing_fingerprint
			} else {
				inited_billing_fingerprint
			};

			ensure!(
				billing_fingerprint.validators.insert(validator.clone()),
				Error::<T>::BillingFingerprintIsCommitted
			);

			BillingFingerprints::<T>::insert(fingerprint, billing_fingerprint);
			Self::deposit_event(Event::<T>::BillingFingerprintCommited {
				validator_id: validator,
				cluster_id,
				era_id,
				start_era,
				end_era,
				payers_merkle_root,
				payees_merkle_root,
			});

			Ok(())
		}

		fn begin_billing_report(
			cluster_id: ClusterId,
			era: DdcEra,
			fingerprint: Fingerprint,
		) -> DispatchResult {
			ensure!(
				ActiveBillingReports::<T>::try_get(cluster_id, era).is_err(),
				Error::<T>::NotExpectedState
			);

			let billing_fingerprint = BillingFingerprints::<T>::try_get(fingerprint)
				.map_err(|_| Error::<T>::BillingFingerprintDoesNotExist)?;

			ensure!(
				T::ValidatorVisitor::is_quorum_reached(
					T::ValidatorsQuorum::get(),
					billing_fingerprint.validators.len(),
				),
				Error::<T>::NoQuorumOnBillingFingerprint
			);

			let billing_report = BillingReport::<T> {
				vault: Self::account_id(),
				fingerprint,
				state: PayoutState::Initialized,
				..Default::default()
			};
			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);

			Self::deposit_event(Event::<T>::BillingReportInitialized { cluster_id, era });

			Ok(())
		}

		fn begin_charging_customers(
			cluster_id: ClusterId,
			era: DdcEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			ensure!(max_batch_index < MaxBatchesCount::get(), Error::<T>::BatchIndexOverflow);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == PayoutState::Initialized, Error::<T>::NotExpectedState);

			billing_report.charging_max_batch_index = max_batch_index;
			billing_report.state = PayoutState::ChargingCustomers;
			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);

			Self::deposit_event(Event::<T>::ChargingStarted { cluster_id, era });

			Ok(())
		}

		fn send_charging_customers_batch(
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			payers: &[(BucketId, BucketUsage)],
			batch_proof: MMRProof,
		) -> DispatchResult {
			ensure!(
				!payers.is_empty() && payers.len() <= MaxBatchSize::get() as usize,
				Error::<T>::BatchSizeIsOutOfBounds
			);

			let billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(
				billing_report.state == PayoutState::ChargingCustomers,
				Error::<T>::NotExpectedState
			);

			ensure!(
				billing_report.charging_max_batch_index >= batch_index,
				Error::<T>::BatchIndexIsOutOfRange
			);

			ensure!(
				!billing_report.charging_processed_batches.contains(&batch_index),
				Error::<T>::BatchIndexAlreadyProcessed
			);

			let billing_fingerprint = BillingFingerprints::<T>::try_get(billing_report.fingerprint)
				.map_err(|_| Error::<T>::BillingFingerprintDoesNotExist)?;

			let is_batch_verifed = Self::is_customers_batch_valid(
				billing_fingerprint.payers_merkle_root,
				batch_index,
				billing_report.charging_max_batch_index,
				payers,
				&batch_proof,
			)?;

			ensure!(is_batch_verifed, Error::<T>::BatchValidationFailed);

			let pricing = T::ClusterProtocol::get_pricing_params(&cluster_id)
				.map_err(|_| Error::<T>::NotExpectedClusterState)?;

			let mut updated_billing_report = billing_report;
			for (bucket_ref, payable_usage) in payers {
				let bucket_id = *bucket_ref;
				let customer_id = T::BucketManager::get_bucket_owner_id(bucket_id)?;

				let mut customer_charge = get_customer_charge::<T>(
					&pricing,
					payable_usage,
					billing_fingerprint.start_era,
					billing_fingerprint.end_era,
				)?;
				let total_customer_charge = (|| -> Option<u128> {
					customer_charge
						.transfer
						.checked_add(customer_charge.storage)?
						.checked_add(customer_charge.puts)?
						.checked_add(customer_charge.gets)
				})()
				.ok_or(Error::<T>::ArithmeticOverflow)?;

				let amount_actually_charged = match T::CustomerCharger::charge_bucket_owner(
					customer_id.clone(),
					updated_billing_report.vault.clone(),
					total_customer_charge,
				) {
					Ok(actually_charged) => actually_charged,
					Err(e) => {
						Self::deposit_event(Event::<T>::ChargeError {
							cluster_id,
							era,
							batch_index,
							customer_id: customer_id.clone(),
							amount: total_customer_charge,
							error: e,
						});
						0
					},
				};

				if amount_actually_charged < total_customer_charge {
					// debt
					let mut customer_debt =
						DebtorCustomers::<T>::try_get(cluster_id, customer_id.clone())
							.unwrap_or_else(|_| Zero::zero());

					let debt = total_customer_charge
						.checked_sub(amount_actually_charged)
						.ok_or(Error::<T>::ArithmeticOverflow)?;

					customer_debt =
						customer_debt.checked_add(debt).ok_or(Error::<T>::ArithmeticOverflow)?;

					DebtorCustomers::<T>::insert(cluster_id, customer_id.clone(), customer_debt);

					Self::deposit_event(Event::<T>::Indebted {
						cluster_id,
						era,
						batch_index,
						customer_id: customer_id.clone(),
						bucket_id,
						amount: debt,
					});

					Self::deposit_event(Event::<T>::ChargeFailed {
						cluster_id,
						era,
						batch_index,
						customer_id: customer_id.clone(),
						bucket_id,
						charged: amount_actually_charged,
						expected_to_charge: total_customer_charge,
					});

					// something was charged and should be added
					// calculate ratio
					let ratio =
						Perquintill::from_rational(amount_actually_charged, total_customer_charge);

					customer_charge.storage = ratio * customer_charge.storage;
					customer_charge.transfer = ratio * customer_charge.transfer;
					customer_charge.gets = ratio * customer_charge.gets;
					customer_charge.puts = ratio * customer_charge.puts;
				} else {
					Self::deposit_event(Event::<T>::Charged {
						cluster_id,
						era,
						batch_index,
						customer_id: customer_id.clone(),
						bucket_id,
						amount: total_customer_charge,
					});
				}

				updated_billing_report.total_customer_charge.storage = updated_billing_report
					.total_customer_charge
					.storage
					.checked_add(customer_charge.storage)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				updated_billing_report.total_customer_charge.transfer = updated_billing_report
					.total_customer_charge
					.transfer
					.checked_add(customer_charge.transfer)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				updated_billing_report.total_customer_charge.puts = updated_billing_report
					.total_customer_charge
					.puts
					.checked_add(customer_charge.puts)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				updated_billing_report.total_customer_charge.gets = updated_billing_report
					.total_customer_charge
					.gets
					.checked_add(customer_charge.gets)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				T::BucketManager::update_total_bucket_usage(
					&cluster_id,
					bucket_id,
					customer_id,
					payable_usage,
				)?;
			}

			updated_billing_report
				.charging_processed_batches
				.try_insert(batch_index)
				.map_err(|_| Error::<T>::BoundedVecOverflow)?;

			ActiveBillingReports::<T>::insert(cluster_id, era, updated_billing_report);

			Ok(())
		}

		fn end_charging_customers(cluster_id: ClusterId, era: DdcEra) -> DispatchResult {
			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(
				billing_report.state == PayoutState::ChargingCustomers,
				Error::<T>::NotExpectedState
			);
			Self::validate_batches(
				&billing_report.charging_processed_batches,
				&billing_report.charging_max_batch_index,
			)?;

			Self::deposit_event(Event::<T>::ChargingFinished { cluster_id, era });

			// deduct fees
			let fees = T::ClusterProtocol::get_fees_params(&cluster_id)
				.map_err(|_| Error::<T>::ClusterProtocolParamsNotSet)?;

			let total_customer_charge = (|| -> Option<u128> {
				billing_report
					.total_customer_charge
					.transfer
					.checked_add(billing_report.total_customer_charge.storage)?
					.checked_add(billing_report.total_customer_charge.puts)?
					.checked_add(billing_report.total_customer_charge.gets)
			})()
			.ok_or(Error::<T>::ArithmeticOverflow)?;

			let treasury_fee = fees.treasury_share * total_customer_charge;
			let validators_fee = fees.validators_share * total_customer_charge;
			let cluster_reserve_fee = fees.cluster_reserve_share * total_customer_charge;

			if treasury_fee > 0 {
				charge_treasury_fees::<T>(
					treasury_fee,
					&billing_report.vault,
					&T::TreasuryVisitor::get_account_id(),
				)?;

				Self::deposit_event(Event::<T>::TreasuryFeesCollected {
					cluster_id,
					era,
					amount: treasury_fee,
				});
			}

			if cluster_reserve_fee > 0 {
				charge_cluster_reserve_fees::<T>(
					cluster_reserve_fee,
					&billing_report.vault,
					&T::ClusterProtocol::get_reserve_account_id(&cluster_id)
						.map_err(|_| Error::<T>::NotExpectedClusterState)?,
				)?;
				Self::deposit_event(Event::<T>::ClusterReserveFeesCollected {
					cluster_id,
					era,
					amount: cluster_reserve_fee,
				});
			}

			if validators_fee > 0 {
				charge_validator_fees::<T>(validators_fee, &billing_report.vault, cluster_id, era)?;
				Self::deposit_event(Event::<T>::ValidatorFeesCollected {
					cluster_id,
					era,
					amount: validators_fee,
				});
			}

			// 1 - (X + Y + Z) > 0, 0 < X + Y + Z < 1
			let total_left_from_one =
				(fees.treasury_share + fees.validators_share + fees.cluster_reserve_share)
					.left_from_one();

			if !total_left_from_one.is_zero() {
				// X * Z < X, 0 < Z < 1
				billing_report.total_customer_charge.transfer =
					total_left_from_one * billing_report.total_customer_charge.transfer;
				billing_report.total_customer_charge.storage =
					total_left_from_one * billing_report.total_customer_charge.storage;
				billing_report.total_customer_charge.puts =
					total_left_from_one * billing_report.total_customer_charge.puts;
				billing_report.total_customer_charge.gets =
					total_left_from_one * billing_report.total_customer_charge.gets;
			}

			billing_report.state = PayoutState::CustomersChargedWithFees;
			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);

			Ok(())
		}

		fn begin_rewarding_providers(
			cluster_id: ClusterId,
			era: DdcEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			ensure!(max_batch_index < MaxBatchesCount::get(), Error::<T>::BatchIndexOverflow);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(
				billing_report.state == PayoutState::CustomersChargedWithFees,
				Error::<T>::NotExpectedState
			);

			billing_report.rewarding_max_batch_index = max_batch_index;
			billing_report.state = PayoutState::RewardingProviders;
			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);

			Self::deposit_event(Event::<T>::RewardingStarted { cluster_id, era });

			Ok(())
		}

		fn send_rewarding_providers_batch(
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			payees: &[(NodePubKey, NodeUsage)],
			batch_proof: MMRProof,
		) -> DispatchResult {
			ensure!(
				!payees.is_empty() && payees.len() <= MaxBatchSize::get() as usize,
				Error::<T>::BatchSizeIsOutOfBounds
			);

			let billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(
				billing_report.state == PayoutState::RewardingProviders,
				Error::<T>::NotExpectedState
			);
			ensure!(
				billing_report.rewarding_max_batch_index >= batch_index,
				Error::<T>::BatchIndexIsOutOfRange
			);
			ensure!(
				!billing_report.rewarding_processed_batches.contains(&batch_index),
				Error::<T>::BatchIndexAlreadyProcessed
			);

			let billing_fingerprint = BillingFingerprints::<T>::try_get(billing_report.fingerprint)
				.map_err(|_| Error::<T>::BillingFingerprintDoesNotExist)?;

			let is_batch_verified = Self::is_providers_batch_valid(
				billing_fingerprint.payees_merkle_root,
				batch_index,
				billing_report.rewarding_max_batch_index,
				payees,
				&batch_proof,
			)?;

			ensure!(is_batch_verified, Error::<T>::BatchValidationFailed);

			let max_dust = MaxDust::get().saturated_into::<BalanceOf<T>>();
			let mut updated_billing_report = billing_report.clone();
			for (node_key, payable_usage) in payees {
				let provider_id = T::NodeManager::get_node_provider_id(node_key)?;

				let provider_reward = get_provider_reward(
					payable_usage,
					&billing_fingerprint.cluster_usage,
					&billing_report.total_customer_charge,
				)
				.ok_or(Error::<T>::ArithmeticOverflow)?;

				let total_provider_reward = (|| -> Option<u128> {
					provider_reward
						.transfer
						.checked_add(provider_reward.storage)?
						.checked_add(provider_reward.puts)?
						.checked_add(provider_reward.gets)
				})()
				.ok_or(Error::<T>::ArithmeticOverflow)?;

				let mut reward_ = total_provider_reward;
				let mut reward: BalanceOf<T> =
					total_provider_reward.saturated_into::<BalanceOf<T>>();

				if total_provider_reward > 0 {
					let vault_balance = <T as pallet::Config>::Currency::free_balance(
						&updated_billing_report.vault,
					) - <T as pallet::Config>::Currency::minimum_balance();

					// 10000000000001 > 10000000000000 but is still ok
					if reward > vault_balance {
						if reward - vault_balance > max_dust {
							Self::deposit_event(Event::<T>::NotDistributedReward {
								cluster_id,
								era,
								batch_index,
								node_provider_id: provider_id.clone(),
								expected_reward: total_provider_reward,
								distributed_reward: vault_balance,
							});
						}

						reward = vault_balance;
					}

					<T as pallet::Config>::Currency::transfer(
						&updated_billing_report.vault,
						&provider_id,
						reward,
						ExistenceRequirement::AllowDeath,
					)?;

					reward_ = reward.saturated_into::<u128>();

					updated_billing_report.total_distributed_reward = updated_billing_report
						.total_distributed_reward
						.checked_add(reward_)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
				}

				Self::deposit_event(Event::<T>::Rewarded {
					cluster_id,
					era,
					batch_index,
					node_provider_id: provider_id,
					rewarded: reward_,
					expected_to_reward: total_provider_reward,
				});

				T::NodeManager::update_total_node_usage(node_key, payable_usage)?;
			}

			updated_billing_report
				.rewarding_processed_batches
				.try_insert(batch_index)
				.map_err(|_| Error::<T>::BoundedVecOverflow)?;

			ActiveBillingReports::<T>::insert(cluster_id, era, updated_billing_report);

			Ok(())
		}

		fn end_rewarding_providers(cluster_id: ClusterId, era: DdcEra) -> DispatchResult {
			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(
				billing_report.state == PayoutState::RewardingProviders,
				Error::<T>::NotExpectedState
			);

			Self::validate_batches(
				&billing_report.rewarding_processed_batches,
				&billing_report.rewarding_max_batch_index,
			)?;

			let expected_amount_to_reward = (|| -> Option<u128> {
				billing_report
					.total_customer_charge
					.transfer
					.checked_add(billing_report.total_customer_charge.storage)?
					.checked_add(billing_report.total_customer_charge.puts)?
					.checked_add(billing_report.total_customer_charge.gets)
			})()
			.ok_or(Error::<T>::ArithmeticOverflow)?;

			if expected_amount_to_reward - billing_report.total_distributed_reward > MaxDust::get()
			{
				Self::deposit_event(Event::<T>::NotDistributedOverallReward {
					cluster_id,
					era,
					expected_reward: expected_amount_to_reward,
					total_distributed_reward: billing_report.total_distributed_reward,
				});
			}

			billing_report.state = PayoutState::ProvidersRewarded;
			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);

			Self::deposit_event(Event::<T>::RewardingFinished { cluster_id, era });

			Ok(())
		}

		fn end_billing_report(cluster_id: ClusterId, era: DdcEra) -> DispatchResult {
			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(
				billing_report.state == PayoutState::ProvidersRewarded,
				Error::<T>::NotExpectedState
			);

			billing_report.charging_processed_batches.clear();
			billing_report.rewarding_processed_batches.clear();
			billing_report.state = PayoutState::Finalized;

			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);
			Self::deposit_event(Event::<T>::BillingReportFinalized { cluster_id, era });

			Ok(())
		}

		fn get_billing_report_status(cluster_id: &ClusterId, era: DdcEra) -> PayoutState {
			let billing_report = ActiveBillingReports::<T>::get(cluster_id, era);
			match billing_report {
				Some(report) => report.state,
				None => PayoutState::NotInitialized,
			}
		}

		fn all_customer_batches_processed(cluster_id: &ClusterId, era_id: DdcEra) -> bool {
			let billing_report = match ActiveBillingReports::<T>::try_get(cluster_id, era_id) {
				Ok(report) => report,
				Err(_) => return false,
			};

			Self::validate_batches(
				&billing_report.charging_processed_batches,
				&billing_report.charging_max_batch_index,
			)
			.is_ok()
		}

		fn all_provider_batches_processed(cluster_id: &ClusterId, era_id: DdcEra) -> bool {
			let billing_report = match ActiveBillingReports::<T>::try_get(cluster_id, era_id) {
				Ok(report) => report,
				Err(_) => return false,
			};

			Self::validate_batches(
				&billing_report.rewarding_processed_batches,
				&billing_report.rewarding_max_batch_index,
			)
			.is_ok()
		}

		fn get_next_customer_batch_for_payment(
			cluster_id: &ClusterId,
			era_id: DdcEra,
		) -> Result<Option<BatchIndex>, PayoutError> {
			let billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era_id)
				.map_err(|_| PayoutError::BillingReportDoesNotExist)?;

			for batch_index in 0..=billing_report.charging_max_batch_index {
				if !billing_report.charging_processed_batches.contains(&batch_index) {
					return Ok(Some(batch_index));
				}
			}

			Ok(None)
		}

		fn get_next_provider_batch_for_payment(
			cluster_id: &ClusterId,
			era_id: DdcEra,
		) -> Result<Option<BatchIndex>, PayoutError> {
			let billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era_id)
				.map_err(|_| PayoutError::BillingReportDoesNotExist)?;

			for batch_index in 0..=billing_report.rewarding_max_batch_index {
				if !billing_report.rewarding_processed_batches.contains(&batch_index) {
					return Ok(Some(batch_index));
				}
			}

			Ok(None)
		}

		fn create_billing_report(vault: T::AccountId, params: BillingReportParams) {
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

			let billing_report = BillingReport::<T> {
				vault,
				state: params.state,
				fingerprint: params.fingerprint,
				total_customer_charge: params.total_customer_charge,
				total_distributed_reward: params.total_distributed_reward,
				charging_max_batch_index: params.charging_max_batch_index,
				charging_processed_batches,
				rewarding_max_batch_index: params.rewarding_max_batch_index,
				rewarding_processed_batches,
			};

			ActiveBillingReports::<T>::insert(params.cluster_id, params.era, billing_report);
		}

		fn create_billing_fingerprint(
			params: BillingFingerprintParams<T::AccountId>,
		) -> Fingerprint {
			let billing_fingerprint = BillingFingerprint::<T::AccountId> {
				cluster_id: params.cluster_id,
				era_id: params.era,
				start_era: params.start_era,
				end_era: params.end_era,
				payers_merkle_root: params.payers_merkle_root,
				payees_merkle_root: params.payees_merkle_root,
				cluster_usage: params.cluster_usage,
				validators: params.validators,
			};

			let fingerprint = billing_fingerprint.selective_hash::<T>();
			BillingFingerprints::<T>::insert(fingerprint, billing_fingerprint);

			fingerprint
		}
	}
}
