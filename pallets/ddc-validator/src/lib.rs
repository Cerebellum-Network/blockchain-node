//! # DDC Validator pallet
//!
//! The DDC Validator pallet defines storage item to store validation results and implements OCW
//! (off-chain worker) to produce these results using the data from Data Activity Capture (DAC).
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//! - [`Hooks`]
//!
//!	## Notes
//!
//! - Era definition in this pallet is different than in the `pallet-staking`. Check DAC
//!   documentation for `era` definition used in this pallet.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use ddc_primitives::{
	traits::{
		cluster::{ClusterCreator as ClusterCreatorType, ClusterVisitor as ClusterVisitorType},
		staking::ProtocolStakingVisitor as ProtocolStakingVisitorType,
		customer::{
			CustomerCharger as CustomerChargerType, CustomerDepositor as CustomerDepositorType,
		},
		pallet::PalletVisitor as PalletVisitorType,
	},
};
use frame_election_provider_support::SortedListProvider;
use frame_support::{
	pallet_prelude::*,
	sp_runtime::SaturatedConversion,
	traits::{Currency, LockableCurrency, Randomness},
};
use frame_system::{ensure_signed, pallet_prelude::*};
pub use pallet::*;
use sp_runtime::{traits::Convert, PerThing, DispatchError, DispatchResult};
use sp_std::prelude::*;
use sp_runtime::offchain::{
	http, storage::StorageValueRef, storage_lock, storage_lock::StorageLock, Duration, Timestamp,
};
use sp_staking::EraIndex;

/// Offchain local storage key that holds the last era in which the validator completed its
/// assignment.
const LAST_VALIDATED_ERA_KEY: &[u8; 40] = b"pallet-ddc-validator::last_validated_era";
/// Offchain local storage that holds the validation lock
const VALIDATION_LOCK: &[u8; 37] = b"pallet-ddc-validator::validation_lock";

/// Local storage key that holds the flag to enable DDC validation. Set it to true (0x01) to enable
/// DDC validation, set it to false (0x00) or delete the key to disable it.
const ENABLE_DDC_VALIDATION_KEY: &[u8; 21] = b"enable-ddc-validation";

pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use frame_election_provider_support::SortedListProvider;
	use frame_support::traits::LockableCurrency;

	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		/// Something that provides randomness in the runtime. Required by the tasks assignment
		/// procedure.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		/// A dispatchable call.
		type RuntimeCall: From<Call<Self>>;

		type ClusterVisitor: ClusterVisitorType<Self>;
		type ValidatorsList: SortedListProvider<Self::AccountId>;
		type ProtocolStakingVisitor: ProtocolStakingVisitorType<Self, BalanceOf<Self>>;
	}

	/// A signal to start a process on all the validators.
	#[pallet::storage]
	#[pallet::getter(fn signal)]
	pub(super) type Signal<T: Config> = StorageValue<_, bool>;

	/// The last era for which the tasks assignment produced.
	#[pallet::storage]
	#[pallet::getter(fn last_managed_era)]
	pub type LastManagedEra<T: Config> = StorageValue<_, EraIndex>;

	/// The mapping of ddc validator keys to validator stash keys
	///
	/// Keys registered by validators are mapped to validator stash accounts.
	/// The mapping is formed in the way that facilitates fast checking that storage contains key.
	/// Similarly the validator stash is checked if he is still in the list of validators.
	#[pallet::storage]
	#[pallet::getter(fn get_stash_for_ddc_validator)]
	pub type DDCValidatorToStashKeys<T: Config> =
		StorageMap<_, Identity, T::AccountId, T::AccountId>;

	#[pallet::error]
	pub enum Error<T> {
		/// Caller is not controller of validator node
		NotController,
		/// Checked stash is not an active validator
		NotValidatorStash,
		/// OCW key has not been registered by validator
		DDCValidatorKeyNotRegistered,
		/// Attempt to charge content owners twice
		ContentOwnersDoubleSpend,
		/// Validation decision has been already set for CDN node for some era
		ValidationDecisionAlreadySet,
		/// Node is not participating in the network
		NodeNotActive,
		/// Pricing has not been set by sudo
		PricingNotSet,
		/// Current era not set during runtime
		DDCEraNotSet,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// Validator submits decision for an era
		ValidationDecision(EraIndex, T::AccountId),
		// Set era reward points
		EraRewardPoints(EraIndex, Vec<(T::AccountId, u64)>),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			if block_number <= 1u32.into() {
				return Weight::from_ref_time(0)
			}

			Signal::<T>::set(Some(false));

			let current_ddc_era = match T::ProtocolStakingVisitor::current_era() {
				Some(era) => era,
				None => {
					log::debug!("DDC era not set.");
					return Weight::from_ref_time(0)
				},
			};
			log::debug!("Current DDC era: {:?}.", current_ddc_era);

			// Skip assignment if already exists for current era
			match Self::last_managed_era() {
				Some(last_managed_era) if current_ddc_era < last_managed_era =>
					return Weight::from_ref_time(0),
				_ => (),
			}

			match Self::assign(3usize, current_ddc_era + 1) {
				Ok(_) => <LastManagedEra<T>>::put(current_ddc_era + 1),
				Err(e) => log::debug!("DDC validation assignment error: {:?}.", e),
			}

			Weight::from_ref_time(0)
		}

		fn offchain_worker(_block_number: T::BlockNumber) {
			// Skip if not a validator.
			if !sp_io::offchain::is_validator() {
				return
			}

			// Skip if DDC validation is not enabled.
			match StorageValueRef::persistent(ENABLE_DDC_VALIDATION_KEY).get::<bool>() {
				Ok(Some(enabled)) if enabled == true => (),
				_ => return,
			}

			let mut should_validate_because_new_era = true;

			let mut validation_lock = StorageLock::<storage_lock::Time>::new(VALIDATION_LOCK);

			// Skip if the validation is already in progress.
			if validation_lock.try_lock().is_err() {
				should_validate_because_new_era = false;
			}

			let last_validated_era_storage = StorageValueRef::persistent(LAST_VALIDATED_ERA_KEY);
			let last_validated_era = match last_validated_era_storage.get::<EraIndex>() {
				Ok(Some(last_validated_era)) => last_validated_era,
				_ => 0, // let's consider an absent or undecodable data as we never did a validation
			};

			let current_ddc_era = match T::ProtocolStakingVisitor::current_era() {
				Some(era) => era,
				None => {
					return
				},
			};

			// Skip if the validation is already complete for the era.
			if current_ddc_era <= last_validated_era {
				should_validate_because_new_era = false;
			}

			// Validation start forced externally?
			let should_validate_because_signal = Signal::<T>::get().unwrap_or(false);

			if !should_validate_because_new_era && !should_validate_because_signal {
				return
			}

			/*if let Err(e) = Self::validate_edges() {
				log::warn!("🔎 DDC validation failed. {}", e);
				return
			} */

			last_validated_era_storage.set(&current_ddc_era);
			log::info!("🔎 DDC validation complete for {} era.", current_ddc_era);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/*
		/// Set validation decision for a given CDN node in an era.
		///
		/// Only registered validator keys can call this extrinsic.
		/// Validation decision can be set only once per era per CDN node.
		/// CDN node should be active.
		///
		#[pallet::weight(10_000)]
		pub fn set_validation_decision(
			origin: OriginFor<T>,
			era: EraIndex,
			cdn_node: T::AccountId,
			validation_decision: ValidationDecision,
		) -> DispatchResult {
			let ddc_valitor_key = ensure_signed(origin)?;

			ensure!(
				DDCValidatorToStashKeys::<T>::contains_key(&ddc_valitor_key),
				Error::<T>::DDCValidatorKeyNotRegistered
			);

			ensure!(
				staking::Validators::<T>::contains_key(
					Self::get_stash_for_ddc_validator(&ddc_valitor_key).unwrap()
				),
				Error::<T>::NotValidatorStash
			);

			ensure!(
				!Self::validation_decision_set_for_node(era, &cdn_node),
				Error::<T>::ValidationDecisionAlreadySet
			);

			ensure!(
				<ddc_staking::pallet::Edges<T>>::contains_key(&cdn_node),
				Error::<T>::NodeNotActive
			);

			ValidationDecisions::<T>::insert(era, cdn_node.clone(), validation_decision.clone());

			ValidationDecisionSetForNode::<T>::insert(era, cdn_node.clone(), true);

			Self::deposit_event(Event::<T>::ValidationDecision(era, cdn_node, validation_decision));

			Ok(())
		}

		/// Set reward points for CDN participants at the given era.
		///
		/// Only registered validator keys can call this exstrinsic.
		/// Reward points can be set only once per era per CDN node.
		///	CDN node should be active.
		///
		/// `stakers_points` is a vector of (stash account ID, reward points) pairs. The rewards
		/// distribution will be based on total reward points, with each CDN participant receiving a
		/// proportionate reward based on their individual reward points.
		#[pallet::weight(100_000)]
		pub fn set_era_reward_points(
			origin: OriginFor<T>,
			era: EraIndex,
			stakers_points: Vec<(T::AccountId, u64)>,
		) -> DispatchResult {
			let ddc_valitor_key = ensure_signed(origin)?;

			ensure!(
				DDCValidatorToStashKeys::<T>::contains_key(&ddc_valitor_key),
				Error::<T>::DDCValidatorKeyNotRegistered
			);

			ensure!(
				staking::Validators::<T>::contains_key(
					Self::get_stash_for_ddc_validator(&ddc_valitor_key).unwrap()
				),
				Error::<T>::NotValidatorStash
			);

			let mut rewards_counter = 0;

			<ddc_staking::pallet::ErasEdgesRewardPoints<T>>::mutate(era, |era_rewards| {
				for (staker, points) in stakers_points.clone().into_iter() {
					if !Self::reward_points_set_for_node(era, &staker) {
						// ToDo deal with edge case when node is chilling
						if <ddc_staking::pallet::Edges<T>>::contains_key(&staker) {
							*era_rewards.individual.entry(staker.clone()).or_default() += points;
							era_rewards.total += points;
							<ddc_staking::pallet::ErasEdgesRewardPointsPerNode<T>>::mutate(
								&staker,
								|current_reward_points| {
									let rewards: ddc_staking::EraRewardPointsPerNode =
										ddc_staking::EraRewardPointsPerNode { era, points };
									current_reward_points.push(rewards);
								},
							);
							RewardPointsSetForNode::<T>::insert(era, staker, true);
							rewards_counter += 1;
						}
					}
				}
			});

			if rewards_counter > 0 {
				Self::deposit_event(Event::<T>::EraRewardPoints(era, stakers_points));
			}

			Ok(())
		} */

		/// Extrinsic registers a ddc validator key for future use
		///
		/// Only controller of validator can call this extrinsic
		/// Validator has to be in the active set
		#[pallet::weight(10_000)]
		pub fn set_validator_key(
			origin: OriginFor<T>,
			ddc_validator_pub: T::AccountId,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = T::ProtocolStakingVisitor::get_ledger(&controller)
				.ok_or(Error::<T>::NotController)?;

			ensure!(T::ValidatorsList::contains(&ledger.stash), Error::<T>::NotValidatorStash);

			DDCValidatorToStashKeys::<T>::insert(ddc_validator_pub, &ledger.stash);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn assign(_quorum_size: usize, _era: EraIndex) -> Result<(), DispatchError> {
			/*
			let validators: Vec<T::AccountId> = DDCValidatorToStashKeys::<T>::iter_keys().collect();
			log::debug!("Current validators: {:?}.", validators);

			if validators.len() == 0 {
				return Err(AssignmentError::NoValidators)
			}

			if validators.len() < quorum_size {
				return Err(AssignmentError::NotEnoughValidators {
					requested_quorum: quorum_size,
					available_validators: validators.len(),
				})
			}

			let edges: Vec<T::AccountId> = <ddc_staking::pallet::Edges<T>>::iter_keys().collect();
			log::debug!("Current edges: {:?}.", edges);

			if edges.len() == 0 {
				return Ok(())
			}

			let shuffled_validators = Self::shuffle(validators);
			let shuffled_edges = Self::shuffle(edges);

			let validators_keys: Vec<String> = shuffled_validators
				.iter()
				.map(|v| utils::account_to_string::<T>(v.clone()))
				.collect();

			// Create several groups of validators `quorum_size` length each.
			let quorums = Self::split(validators_keys, quorum_size);

			// Write an assignment to each validator in each quorum. The difference between the
			// number of edges assigned to each validator is not higher then 1. If the number of
			// edges is less then the number of quorums, some quorums will not have any edges
			// assigned.
			let mut quorums_cycle = quorums.iter().cycle();
			for edge in shuffled_edges {
				let Some(quorum_validators) = quorums_cycle.next() else {
					return Err(AssignmentError::DefensiveEmptyQuorumsCycle);
				};
				quorum_validators.iter().for_each(|validator| {
					Assignments::<T>::append(
						era,
						utils::string_to_account::<T>(validator.clone()),
						edge.clone(),
					);
				});
			} */

			Ok(())
		}
	}
}
