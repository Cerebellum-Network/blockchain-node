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
#![allow(clippy::manual_inspect)]
// todo(yahortsaryk) tests for DAC v4 payments should be completely revised
// #[cfg(test)]
// pub(crate) mod mock;
// #[cfg(test)]
// mod tests;

pub mod migrations;

use ddc_primitives::{
	traits::{
		bucket::BucketManager, cluster::ClusterProtocol as ClusterProtocolType,
		customer::CustomerCharger as CustomerChargerType, node::NodeManager,
		pallet::PalletVisitor as PalletVisitorType, payout::PayoutProcessor, ClusterValidator,
	},
	BatchIndex, ClusterId, CustomerCharge, DdcEra, EHDId, EhdEra, Fingerprint, MMRProof,
	MergeMMRHash, NodePubKey, NodeUsage, PayableUsageHash, PaymentEra, PayoutError,
	PayoutFingerprintParams, PayoutReceiptParams, PayoutState, MAX_PAYOUT_BATCH_COUNT,
	MAX_PAYOUT_BATCH_SIZE, MILLICENTS,
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
	AccountId32, Percent, Perquintill, Saturating,
};
use sp_std::{collections::btree_set::BTreeSet, prelude::*};

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

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
		frame_support::traits::StorageVersion::new(3);

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
}
