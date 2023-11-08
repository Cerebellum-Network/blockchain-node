//! # DDC Staking Pallet
//!
//! The DDC Staking pallet is used to manage funds at stake by CDN and storage network maintainers.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## GenesisConfig
//!
//! The DDC Staking pallet depends on the [`GenesisConfig`]. The
//! `GenesisConfig` is optional and allow to set some initial stakers in DDC.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]
#![feature(is_some_and)] // ToDo: delete at rustc > 1.70

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod testing_utils;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
use crate::weights::WeightInfo;

use codec::{Decode, Encode, HasCompact};
pub use ddc_primitives::{ClusterId, NodePubKey, NodeType};
use ddc_traits::{
	cluster::{ClusterVisitor, ClusterVisitorError},
	staking::{StakingVisitor, StakingVisitorError},
};

use frame_support::{
	assert_ok,
	pallet_prelude::*,
	parameter_types,
	traits::{
		Currency, DefensiveSaturating, ExistenceRequirement, LockIdentifier, LockableCurrency,
		UnixTime, WithdrawReasons,
	},
	BoundedVec, PalletId,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, AtLeast32BitUnsigned, Saturating, StaticLookup, Zero},
	RuntimeDebug, SaturatedConversion,
};
use sp_staking::EraIndex;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

pub use pallet::*;

/// Two minutes.
///
/// If you are changing this, check `on_finalize` hook to ensure `CurrentEra` is capable to hold the
/// value with the new era duration.
pub const DDC_ERA_DURATION_MS: u128 = 120_000;

/// 2023-01-01 00:00:00 UTC
pub const DDC_ERA_START_MS: u128 = 1_672_531_200_000;
const DDC_STAKING_ID: LockIdentifier = *b"ddcstake"; // DDC maintainer's stake

/// Counter for the number of "reward" points earned by a given staker.
pub type RewardPoint = u64;

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

parameter_types! {
	/// A limit to the number of pending unlocks an account may have in parallel.
	pub MaxUnlockingChunks: u32 = 32;
}

/// Reward points of an era. Used to split era total payout between stakers.
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Clone)]
pub struct EraRewardPoints<AccountId: Ord> {
	/// Total number of points. Equals the sum of reward points for each staker.
	pub total: RewardPoint,
	/// The reward points earned by a given staker.
	pub individual: BTreeMap<AccountId, RewardPoint>,
}

/// Reward points for particular era. To be used in a mapping.
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Clone)]
pub struct EraRewardPointsPerNode {
	/// Era points accrued
	pub era: EraIndex,
	/// Total number of points for node
	pub points: RewardPoint,
}

/// Reward paid for some era.
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Clone)]
pub struct EraRewardsPaid<Balance: HasCompact> {
	/// Era number
	pub era: EraIndex,
	/// Cere tokens paid
	pub reward: Balance,
}

impl<AccountId: Ord> Default for EraRewardPoints<AccountId> {
	fn default() -> Self {
		EraRewardPoints { total: Default::default(), individual: BTreeMap::new() }
	}
}

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct UnlockChunk<Balance: HasCompact> {
	/// Amount of funds to be unlocked.
	#[codec(compact)]
	value: Balance,
	/// Era number at which point it'll be unlocked.
	#[codec(compact)]
	era: EraIndex,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StakingLedger<AccountId, Balance: HasCompact> {
	/// The stash account whose balance is actually locked and at stake.
	pub stash: AccountId,
	/// The total amount of the stash's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	#[codec(compact)]
	pub total: Balance,
	/// The total amount of the stash's balance that will be at stake in any forthcoming
	/// rounds.
	#[codec(compact)]
	pub active: Balance,
	/// Era number at which chilling will be allowed.
	pub chilling: Option<EraIndex>,
	/// Any balance that is becoming free, which may eventually be transferred out of the stash
	/// (assuming it doesn't get slashed first). It is assumed that this will be treated as a first
	/// in, first out queue where the new (higher value) eras get pushed on the back.
	pub unlocking: BoundedVec<UnlockChunk<Balance>, MaxUnlockingChunks>,
}

impl<AccountId, Balance: HasCompact + Copy + Saturating + AtLeast32BitUnsigned + Zero>
	StakingLedger<AccountId, Balance>
{
	/// Initializes the default object using the given stash.
	pub fn default_from(stash: AccountId) -> Self {
		Self {
			stash,
			total: Zero::zero(),
			active: Zero::zero(),
			chilling: Default::default(),
			unlocking: Default::default(),
		}
	}

	/// Remove entries from `unlocking` that are sufficiently old and reduce the
	/// total by the sum of their balances.
	fn consolidate_unlocked(self, current_era: EraIndex) -> Self {
		let mut total = self.total;
		let unlocking: BoundedVec<_, _> = self
			.unlocking
			.into_iter()
			.filter(|chunk| {
				if chunk.era > current_era {
					true
				} else {
					total = total.saturating_sub(chunk.value);
					false
				}
			})
			.collect::<Vec<_>>()
			.try_into()
			.expect(
				"filtering items from a bounded vec always leaves length less than bounds. qed",
			);

		Self { stash: self.stash, total, active: self.active, chilling: self.chilling, unlocking }
	}
}

/// Cluster staking parameters.
#[derive(Clone, Decode, Encode, Eq, PartialEq, RuntimeDebugNoBound, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ClusterSettings<T: Config> {
	/// The bond size required to become and maintain the role of a CDN participant.
	#[codec(compact)]
	pub cdn_bond_size: BalanceOf<T>,
	/// Number of eras should pass before a CDN participant can chill.
	pub cdn_chill_delay: EraIndex,
	/// The bond size required to become and maintain the role of a storage network participant.
	#[codec(compact)]
	pub storage_bond_size: BalanceOf<T>,
	/// Number of eras should pass before a storage network participant can chill.
	pub storage_chill_delay: EraIndex,
}

impl<T: pallet::Config> Default for ClusterSettings<T> {
	/// Default to the values specified in the runtime config.
	fn default() -> Self {
		Self {
			cdn_bond_size: T::DefaultCDNBondSize::get(),
			cdn_chill_delay: T::DefaultCDNChillDelay::get(),
			storage_bond_size: T::DefaultStorageBondSize::get(),
			storage_chill_delay: T::DefaultStorageChillDelay::get(),
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		/// Default bond size for a CDN participant.
		#[pallet::constant]
		type DefaultCDNBondSize: Get<BalanceOf<Self>>;

		/// Default number or DDC eras required to pass before a CDN participant can chill.
		#[pallet::constant]
		type DefaultCDNChillDelay: Get<EraIndex>;

		/// Default bond size for a storage network participant.
		#[pallet::constant]
		type DefaultStorageBondSize: Get<BalanceOf<Self>>;

		/// Default number or DDC eras required to pass before a storage participant can chill.
		#[pallet::constant]
		type DefaultStorageChillDelay: Get<EraIndex>;

		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Number of eras that staked funds must remain bonded for.
		#[pallet::constant]
		type BondingDuration: Get<EraIndex>;
		/// To derive an account for withdrawing CDN rewards.
		type StakersPayoutSource: Get<PalletId>;
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// Time used for computing era index. It is guaranteed to start being called from the first
		/// `on_finalize`.
		type UnixTime: UnixTime;

		type ClusterVisitor: ClusterVisitor<Self>;
	}

	/// Map from all locked "stash" accounts to the controller account.
	#[pallet::storage]
	#[pallet::getter(fn bonded)]
	pub type Bonded<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

	/// DDC clusters staking settings.
	#[pallet::storage]
	#[pallet::getter(fn settings)]
	pub type Settings<T: Config> =
		StorageMap<_, Identity, ClusterId, ClusterSettings<T>, ValueQuery>;

	/// Map from all (unlocked) "controller" accounts to the info regarding the staking.
	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	pub type Ledger<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, StakingLedger<T::AccountId, BalanceOf<T>>>;

	/// The map of (wannabe) CDN participants stash keys to the DDC cluster ID they wish to
	/// participate into.
	#[pallet::storage]
	#[pallet::getter(fn cdns)]
	pub type CDNs<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ClusterId>;

	/// The map of (wannabe) storage network participants stash keys to the DDC cluster ID they wish
	/// to participate into.
	#[pallet::storage]
	#[pallet::getter(fn storages)]
	pub type Storages<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ClusterId>;

	/// Map from all "stash" accounts to the total paid out rewards
	///
	/// P.S. Not part of Mainnet
	#[pallet::storage]
	#[pallet::getter(fn rewards)]
	pub type Rewards<T: Config> = StorageMap<_, Identity, T::AccountId, BalanceOf<T>, ValueQuery>;

	/// Map from all "stash" accounts to the paid out rewards per era
	///
	/// P.S. Not part of Mainnet
	#[pallet::storage]
	#[pallet::getter(fn paid_eras_per_node)]
	pub type PaidErasPerNode<T: Config> =
		StorageMap<_, Identity, T::AccountId, Vec<EraRewardsPaid<BalanceOf<T>>>, ValueQuery>;

	/// Map to check if CDN participants received payments for specific era
	///
	/// Used to avoid double-spend in method [payout_stakers]
	#[pallet::storage]
	#[pallet::getter(fn paid_eras)]
	pub(super) type PaidEras<T: Config> = StorageMap<_, Twox64Concat, EraIndex, bool, ValueQuery>;

	/// The current era index.
	///
	/// This is the latest planned era, depending on how the Session pallet queues the validator
	/// set, it might be active or not.
	#[pallet::storage]
	#[pallet::getter(fn current_era)]
	pub type CurrentEra<T> = StorageValue<_, EraIndex>;

	/// The reward each CDN participant earned in the era.
	/// Mapping from Era to vector of CDN participants and respective rewards
	///
	/// See also [`pallet_staking::ErasRewardPoints`].
	#[pallet::storage]
	#[pallet::getter(fn eras_cdns_reward_points)]
	pub type ErasCDNsRewardPoints<T: Config> =
		StorageMap<_, Twox64Concat, EraIndex, EraRewardPoints<T::AccountId>, ValueQuery>;

	/// The reward each CDN participant earned in the era.
	/// Mapping from each CDN participant to vector of eras and rewards
	///
	/// P.S. Not part of Mainnet
	#[pallet::storage]
	#[pallet::getter(fn eras_cdns_reward_points_per_node)]
	pub type ErasCDNsRewardPointsPerNode<T: Config> =
		StorageMap<_, Identity, T::AccountId, Vec<EraRewardPointsPerNode>, ValueQuery>;

	/// Price per byte of the bucket traffic in smallest units of the currency.
	#[pallet::storage]
	#[pallet::getter(fn pricing)]
	pub type Pricing<T: Config> = StorageValue<_, u128>;

	/// A list of accounts allowed to become cluster managers.
	#[pallet::storage]
	#[pallet::getter(fn cluster_managers)]
	pub type ClusterManagers<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	/// Map from DDC node ID to the node operator stash account.
	#[pallet::storage]
	#[pallet::getter(fn nodes)]
	pub type Nodes<T: Config> = StorageMap<_, Twox64Concat, NodePubKey, T::AccountId>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub cdns: Vec<(T::AccountId, T::AccountId, NodePubKey, BalanceOf<T>, ClusterId)>,
		pub storages: Vec<(T::AccountId, T::AccountId, NodePubKey, BalanceOf<T>, ClusterId)>,
		pub settings: Vec<(ClusterId, BalanceOf<T>, EraIndex, BalanceOf<T>, EraIndex)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				cdns: Default::default(),
				storages: Default::default(),
				settings: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// clusters' settings
			for &(
				cluster,
				cdn_bond_size,
				cdn_chill_delay,
				storage_bond_size,
				storage_chill_delay,
			) in &self.settings
			{
				Settings::<T>::insert(
					cluster,
					ClusterSettings::<T> {
						cdn_bond_size,
						cdn_chill_delay,
						storage_bond_size,
						storage_chill_delay,
					},
				);
			}

			// Add initial CDN participants
			for &(ref stash, ref controller, ref node, balance, cluster) in &self.cdns {
				assert!(
					T::Currency::free_balance(&stash) >= balance,
					"Stash do not have enough balance to participate in CDN."
				);
				assert_ok!(Pallet::<T>::bond(
					T::RuntimeOrigin::from(Some(stash.clone()).into()),
					T::Lookup::unlookup(controller.clone()),
					node.clone(),
					balance,
				));
				assert_ok!(Pallet::<T>::serve(
					T::RuntimeOrigin::from(Some(controller.clone()).into()),
					cluster,
				));
			}

			// Add initial storage network participants
			for &(ref stash, ref controller, ref node, balance, cluster) in &self.storages {
				assert!(
					T::Currency::free_balance(&stash) >= balance,
					"Stash do not have enough balance to participate in storage network."
				);
				assert_ok!(Pallet::<T>::bond(
					T::RuntimeOrigin::from(Some(stash.clone()).into()),
					T::Lookup::unlookup(controller.clone()),
					node.clone(),
					balance,
				));
				assert_ok!(Pallet::<T>::store(
					T::RuntimeOrigin::from(Some(controller.clone()).into()),
					cluster,
				));
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account has bonded this amount. \[stash, amount\]
		///
		/// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
		/// it will not be emitted for staking rewards when they are added to stake.
		Bonded(T::AccountId, BalanceOf<T>),
		/// An account has unbonded this amount. \[stash, amount\]
		Unbonded(T::AccountId, BalanceOf<T>),
		/// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
		/// from the unlocking queue. \[stash, amount\]
		Withdrawn(T::AccountId, BalanceOf<T>),
		/// An account has stopped participating as either a storage network or CDN participant.
		/// \[stash\]
		Chilled(T::AccountId),
		/// An account has declared desire to stop participating in CDN or storage network soon.
		/// \[stash, cluster, era\]
		ChillSoon(T::AccountId, ClusterId, EraIndex),
		// Payout CDN nodes' stash accounts
		PayoutNodes(EraIndex, EraRewardPoints<T::AccountId>, u128),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not a controller account.
		NotController,
		/// Not a stash account.
		NotStash,
		/// Stash is already bonded.
		AlreadyBonded,
		/// Controller or node is already paired.
		AlreadyPaired,
		/// Cannot have a storage network or CDN participant, with the size less than defined by
		/// governance (see `BondSize`). If unbonding is the intention, `chill` first to remove
		/// one's role as storage/cdn node.
		InsufficientBond,
		/// Can not schedule more unlock chunks.
		NoMoreChunks,
		/// Internal state has become somehow corrupted and the operation cannot continue.
		BadState,
		/// An account already declared a desire to participate in the network with a certain role
		/// and to take another role it should call `chill` first.
		AlreadyInRole,
		/// Action is allowed at some point of time in future not reached yet.
		TooEarly,
		/// We are not yet sure that era has been valdiated by this time
		EraNotValidated,
		/// Attempt to assign reward point for some era more than once
		DuplicateRewardPoints,
		/// Attempt to double spend the assigned rewards per era
		DoubleSpendRewards,
		/// Pricing has not been set by sudo
		PricingNotSet,
		/// Payout amount overflows
		BudgetOverflow,
		/// Current era not set during runtime
		DDCEraNotSet,
		/// Origin of the call is not a controller of the stake associated with the provided node.
		NotNodeController,
		/// No stake found associated with the provided node.
		NodeHasNoStake,
		/// No cluster governance params found for cluster
		NoClusterGovParams,
		/// Conditions for fast chill are not met, try the regular `chill` from
		FastChillProhibited,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_n: BlockNumberFor<T>) {
			// Check if we have a new era and if so bump the current era index.
			let now_as_millis = T::UnixTime::now().as_millis();
			let computed_era: EraIndex =
				((now_as_millis - DDC_ERA_START_MS) / DDC_ERA_DURATION_MS) as u32; // saturated
			if Self::current_era() >= Some(computed_era) {
				return
			}
			CurrentEra::<T>::put(computed_era);
			// ToDo: add `on_initialize` hook to track `on_finalize` weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Take the origin account as a stash and lock up `value` of its balance. `controller` will
		/// be the account that controls it.
		///
		/// `value` must be more than the `minimum_balance` specified by `T::Currency`.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash account.
		///
		/// Emits `Bonded`.
		#[pallet::weight(T::WeightInfo::bond())]
		pub fn bond(
			origin: OriginFor<T>,
			controller: <T::Lookup as StaticLookup>::Source,
			node: NodePubKey,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;

			if <Bonded<T>>::contains_key(&stash) {
				Err(Error::<T>::AlreadyBonded)?
			}

			let controller = T::Lookup::lookup(controller)?;

			if <Ledger<T>>::contains_key(&controller) {
				Err(Error::<T>::AlreadyPaired)?
			}

			// Reject a bond which is considered to be _dust_.
			if value < T::Currency::minimum_balance() {
				Err(Error::<T>::InsufficientBond)?
			}

			// Reject a bond with a known DDC node.
			if Nodes::<T>::contains_key(&node) {
				Err(Error::<T>::AlreadyPaired)?
			}

			frame_system::Pallet::<T>::inc_consumers(&stash).map_err(|_| Error::<T>::BadState)?;

			Nodes::<T>::insert(&node, &stash);

			// You're auto-bonded forever, here. We might improve this by only bonding when
			// you actually store/serve and remove once you unbond __everything__.
			<Bonded<T>>::insert(&stash, &controller);

			let stash_balance = T::Currency::free_balance(&stash);
			let value = value.min(stash_balance);
			Self::deposit_event(Event::<T>::Bonded(stash.clone(), value));
			let item = StakingLedger {
				stash,
				total: value,
				active: value,
				chilling: Default::default(),
				unlocking: Default::default(),
			};
			Self::update_ledger(&controller, &item);
			Ok(())
		}

		/// Schedule a portion of the stash to be unlocked ready for transfer out after the bond
		/// period ends. If this leaves an amount actively bonded less than
		/// T::Currency::minimum_balance(), then it is increased to the full amount.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		///
		/// Once the unlock period is done, you can call `withdraw_unbonded` to actually move
		/// the funds out of management ready for transfer.
		///
		/// No more than a limited number of unlocking chunks (see `MaxUnlockingChunks`)
		/// can co-exists at the same time. In that case, [`Call::withdraw_unbonded`] need
		/// to be called first to remove some of the chunks (if possible).
		///
		/// If a user encounters the `InsufficientBond` error when calling this extrinsic,
		/// they should call `chill` first in order to free up their bonded funds.
		///
		/// Emits `Unbonded`.
		///
		/// See also [`Call::withdraw_unbonded`].
		#[pallet::weight(T::WeightInfo::unbond())]
		pub fn unbond(
			origin: OriginFor<T>,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			ensure!(
				ledger.unlocking.len() < MaxUnlockingChunks::get() as usize,
				Error::<T>::NoMoreChunks,
			);

			let mut value = value.min(ledger.active);

			if !value.is_zero() {
				ledger.active -= value;

				// Avoid there being a dust balance left in the staking system.
				if ledger.active < T::Currency::minimum_balance() {
					value += ledger.active;
					ledger.active = Zero::zero();
				}

				let min_active_bond = if let Some(cluster_id) = Self::cdns(&ledger.stash) {
					let bond_size = T::ClusterVisitor::get_bond_size(&cluster_id, NodeType::CDN)
						.map_err(|e| Into::<Error<T>>::into(ClusterVisitorError::from(e)))?;
					bond_size.saturated_into::<BalanceOf<T>>()
				} else if let Some(cluster_id) = Self::storages(&ledger.stash) {
					let bond_size =
						T::ClusterVisitor::get_bond_size(&cluster_id, NodeType::Storage)
							.map_err(|e| Into::<Error<T>>::into(ClusterVisitorError::from(e)))?;
					bond_size.saturated_into::<BalanceOf<T>>()
				} else {
					Zero::zero()
				};

				// Make sure that the user maintains enough active bond for their role in the
				// cluster. If a user runs into this error, they should chill first.
				ensure!(ledger.active >= min_active_bond, Error::<T>::InsufficientBond);

				// Note: in case there is no current era it is fine to bond one era more.
				let era = Self::current_era().unwrap_or(0) + T::BondingDuration::get();
				if let Some(chunk) = ledger.unlocking.last_mut().filter(|chunk| chunk.era == era) {
					// To keep the chunk count down, we only keep one chunk per era. Since
					// `unlocking` is a FiFo queue, if a chunk exists for `era` we know that it will
					// be the last one.
					chunk.value = chunk.value.defensive_saturating_add(value)
				} else {
					ledger
						.unlocking
						.try_push(UnlockChunk { value, era })
						.map_err(|_| Error::<T>::NoMoreChunks)?;
				};

				Self::update_ledger(&controller, &ledger);

				Self::deposit_event(Event::<T>::Unbonded(ledger.stash, value));
			}
			Ok(())
		}

		/// Remove any unlocked chunks from the `unlocking` queue from our management.
		///
		/// This essentially frees up that balance to be used by the stash account to do
		/// whatever it wants.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller.
		///
		/// Emits `Withdrawn`.
		///
		/// See also [`Call::unbond`].
		#[pallet::weight(T::WeightInfo::withdraw_unbonded())]
		pub fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			let (stash, old_total) = (ledger.stash.clone(), ledger.total);
			if let Some(current_era) = Self::current_era() {
				ledger = ledger.consolidate_unlocked(current_era)
			}

			if ledger.unlocking.is_empty() && ledger.active < T::Currency::minimum_balance() {
				// This account must have called `unbond()` with some value that caused the active
				// portion to fall below existential deposit + will have no more unlocking chunks
				// left. We can now safely remove all staking-related information.
				Self::kill_stash(&stash)?;
				// Remove the lock.
				T::Currency::remove_lock(DDC_STAKING_ID, &stash);
			} else {
				// This was the consequence of a partial unbond. just update the ledger and move on.
				Self::update_ledger(&controller, &ledger);
			};

			// `old_total` should never be less than the new total because
			// `consolidate_unlocked` strictly subtracts balance.
			if ledger.total < old_total {
				// Already checked that this won't overflow by entry condition.
				let value = old_total - ledger.total;
				Self::deposit_event(Event::<T>::Withdrawn(stash, value));
			}

			Ok(())
		}

		/// Declare the desire to participate in CDN for the origin controller. Also works to cancel
		/// a previous "chill".
		///
		/// `cluster` is the ID of the DDC cluster the participant wishes to join.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash. The
		/// bond size must be greater than or equal to the `CDNBondSize`.
		#[pallet::weight(T::WeightInfo::serve())]
		pub fn serve(origin: OriginFor<T>, cluster_id: ClusterId) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			T::ClusterVisitor::ensure_cluster(&cluster_id)
				.map_err(|e| Into::<Error<T>>::into(ClusterVisitorError::from(e)))?;

			let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			// Retrieve the respective bond size from Cluster Visitor
			let bond_size = T::ClusterVisitor::get_bond_size(&cluster_id, NodeType::CDN)
				.map_err(|e| Into::<Error<T>>::into(ClusterVisitorError::from(e)))?;

			ensure!(
				ledger.active >= bond_size.saturated_into::<BalanceOf<T>>(),
				Error::<T>::InsufficientBond
			);
			let stash = &ledger.stash;

			// Can't participate in CDN if already participating in storage network.
			ensure!(!Storages::<T>::contains_key(&stash), Error::<T>::AlreadyInRole);

			// Is it an attempt to cancel a previous "chill"?
			if let Some(current_cluster) = Self::cdns(&stash) {
				// Switching the cluster is prohibited. The user should chill first.
				ensure!(current_cluster == cluster_id, Error::<T>::AlreadyInRole);
				// Cancel previous "chill" attempts
				Self::reset_chilling(&controller);
				return Ok(())
			}

			Self::do_add_cdn(stash, cluster_id);
			Ok(())
		}

		/// Declare the desire to participate in storage network for the origin controller. Also
		/// works to cancel a previous "chill".
		///
		/// `cluster` is the ID of the DDC cluster the participant wishes to join.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash. The
		/// bond size must be greater than or equal to the `StorageBondSize`.
		#[pallet::weight(T::WeightInfo::store())]
		pub fn store(origin: OriginFor<T>, cluster_id: ClusterId) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			T::ClusterVisitor::ensure_cluster(&cluster_id)
				.map_err(|e| Into::<Error<T>>::into(ClusterVisitorError::from(e)))?;

			let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			// Retrieve the respective bond size from Cluster Visitor
			let bond_size = T::ClusterVisitor::get_bond_size(&cluster_id, NodeType::Storage)
				.map_err(|e| Into::<Error<T>>::into(ClusterVisitorError::from(e)))?;
			ensure!(
				ledger.active >= bond_size.saturated_into::<BalanceOf<T>>(),
				Error::<T>::InsufficientBond
			);
			let stash = &ledger.stash;

			// Can't participate in storage network if already participating in CDN.
			ensure!(!CDNs::<T>::contains_key(&stash), Error::<T>::AlreadyInRole);

			// Is it an attempt to cancel a previous "chill"?
			if let Some(current_cluster) = Self::storages(&stash) {
				// Switching the cluster is prohibited. The user should chill first.
				ensure!(current_cluster == cluster_id, Error::<T>::AlreadyInRole);
				// Cancel previous "chill" attempts
				Self::reset_chilling(&controller);
				return Ok(())
			}

			Self::do_add_storage(stash, cluster_id);

			Ok(())
		}

		/// Declare no desire to either participate in storage network or CDN.
		///
		/// Only in case the delay for the role _origin_ maintains in the cluster is set to zero in
		/// cluster settings, it removes the participant immediately. Otherwise, it requires at
		/// least two invocations to effectively remove the participant. The first invocation only
		/// updates the [`Ledger`] to note the DDC era at which the participant may "chill" (current
		/// era + the delay from the cluster settings). The second invocation made at the noted era
		/// (or any further era) will remove the participant from the list of CDN or storage network
		/// participants. If the cluster settings updated significantly decreasing the delay, one
		/// may invoke it again to decrease the era at with the participant may "chill". But it
		/// never increases the era at which the participant may "chill" even when the cluster
		/// settings updated increasing the delay.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		///
		/// Emits `ChillSoon`, `Chill`.
		#[pallet::weight(T::WeightInfo::chill())]
		pub fn chill(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			let current_era = match Self::current_era() {
				Some(era) => era,
				None => Err(Error::<T>::TooEarly)?, // can't chill before the first era
			};

			// Extract delay from the cluster settings.
			let (cluster, delay) = if let Some(cluster) = Self::cdns(&ledger.stash) {
				(cluster, Self::settings(cluster).cdn_chill_delay)
			} else if let Some(cluster) = Self::storages(&ledger.stash) {
				(cluster, Self::settings(cluster).storage_chill_delay)
			} else {
				return Ok(()) // already chilled
			};

			if delay == 0 {
				// No delay is set, so we can chill right away.
				Self::chill_stash(&ledger.stash);
				return Ok(())
			}

			let can_chill_from = current_era.defensive_saturating_add(delay);
			match ledger.chilling {
				None => {
					// No previous declarations of desire to chill. Note it to allow chilling soon.
					Self::chill_stash_soon(&ledger.stash, &controller, cluster, can_chill_from);
					return Ok(())
				},
				Some(chilling) if can_chill_from < chilling => {
					// Time to chill is not reached yet, but it is allowed to chill earlier. Update
					// to allow chilling sooner.
					Self::chill_stash_soon(&ledger.stash, &controller, cluster, can_chill_from);
					return Ok(())
				},
				Some(chilling) if chilling > current_era => Err(Error::<T>::TooEarly)?,
				Some(_) => (),
			}

			// It's time to chill.
			Self::chill_stash(&ledger.stash);
			Self::reset_chilling(&controller); // for future chilling

			Ok(())
		}

		/// (Re-)set the controller of a stash.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
		#[pallet::weight(T::WeightInfo::set_controller())]
		pub fn set_controller(
			origin: OriginFor<T>,
			controller: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			let old_controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
			let controller = T::Lookup::lookup(controller)?;
			if <Ledger<T>>::contains_key(&controller) {
				Err(Error::<T>::AlreadyPaired)?
			}
			if controller != old_controller {
				<Bonded<T>>::insert(&stash, &controller);
				if let Some(l) = <Ledger<T>>::take(&old_controller) {
					<Ledger<T>>::insert(&controller, l);
				}
			}
			Ok(())
		}

		/// Set custom DDC staking settings for a particular cluster.
		///
		/// * `settings` - The new settings for the cluster. If `None`, the settings will be removed
		///   from the storage and default settings will be used.
		///
		/// RuntimeOrigin must be Root to call this function.
		///
		/// NOTE: Existing CDN and storage network participants will not be affected by this
		/// settings update.
		#[pallet::weight(10_000)]
		pub fn set_settings(
			origin: OriginFor<T>,
			cluster: ClusterId,
			settings: Option<ClusterSettings<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			match settings {
				None => Settings::<T>::remove(cluster),
				Some(settings) => Settings::<T>::insert(cluster, settings),
			}

			Ok(())
		}

		/// Pay out all the stakers for a single era.
		#[pallet::weight(100_000)]
		pub fn payout_stakers(origin: OriginFor<T>, era: EraIndex) -> DispatchResult {
			ensure_signed(origin)?;
			let current_era = Self::current_era().ok_or(Error::<T>::DDCEraNotSet)?;

			// Makes sure this era hasn't been paid out yet
			ensure!(!Self::paid_eras(era), Error::<T>::DoubleSpendRewards);

			// This should be adjusted based on the finality of validation
			ensure!(current_era >= era + 2, Error::<T>::EraNotValidated);

			PaidEras::<T>::insert(era, true);
			Self::do_payout_stakers(era)
		}

		/// Set price per byte of the bucket traffic in smallest units of the currency.
		///
		/// The dispatch origin for this call must be _Root_.
		#[pallet::weight(10_000)]
		pub fn set_pricing(origin: OriginFor<T>, price_per_byte: u128) -> DispatchResult {
			ensure_root(origin)?;
			<Pricing<T>>::set(Some(price_per_byte));
			Ok(())
		}

		/// Add a new account to the list of cluster managers.
		///
		/// RuntimeOrigin must be Root to call this function.
		#[pallet::weight(T::WeightInfo::allow_cluster_manager())]
		pub fn allow_cluster_manager(
			origin: OriginFor<T>,
			grantee: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			ensure_root(origin)?;

			let grantee = T::Lookup::lookup(grantee)?;
			ClusterManagers::<T>::mutate(|grantees| {
				if !grantees.contains(&grantee) {
					grantees.push(grantee);
				}
			});

			Ok(())
		}

		/// Remove an account from the list of cluster managers.
		///
		/// RuntimeOrigin must be Root to call this function.
		#[pallet::weight(T::WeightInfo::disallow_cluster_manager())]
		pub fn disallow_cluster_manager(
			origin: OriginFor<T>,
			revokee: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			ensure_root(origin)?;

			let revokee = T::Lookup::lookup(revokee)?;
			ClusterManagers::<T>::mutate(|grantees| {
				if let Some(pos) = grantees.iter().position(|g| g == &revokee) {
					grantees.remove(pos);
				}
			});

			Ok(())
		}

		/// (Re-)set the DDC node of a node operator stash account. Requires to chill first.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
		#[pallet::weight(T::WeightInfo::set_node())]
		pub fn set_node(origin: OriginFor<T>, new_node: NodePubKey) -> DispatchResult {
			let stash = ensure_signed(origin)?;

			if let Some(existing_node_stash) = Nodes::<T>::get(&new_node) {
				if existing_node_stash != stash {
					Err(Error::<T>::AlreadyPaired)?
				}
			}

			// Ensure only one node per stash during the DDC era.
			ensure!(!<CDNs<T>>::contains_key(&stash), Error::<T>::AlreadyInRole);
			ensure!(!<Storages<T>>::contains_key(&stash), Error::<T>::AlreadyInRole);

			<Nodes<T>>::insert(new_node, stash);

			Ok(())
		}

		/// Allow cluster node candidate to chill in the next DDC era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller.
		#[pallet::weight(10_000)]
		pub fn fast_chill(origin: OriginFor<T>, node_pub_key: NodePubKey) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			let stash = <Ledger<T>>::get(&controller).ok_or(Error::<T>::NotController)?.stash;
			let node_stash = <Nodes<T>>::get(&node_pub_key).ok_or(Error::<T>::BadState)?;
			ensure!(stash == node_stash, Error::<T>::NotNodeController);

			let cluster_id = <CDNs<T>>::get(&stash)
				.or(<Storages<T>>::get(&stash))
				.ok_or(Error::<T>::NodeHasNoStake)?;

			let is_cluster_node = T::ClusterVisitor::cluster_has_node(&cluster_id, &node_pub_key);
			ensure!(!is_cluster_node, Error::<T>::FastChillProhibited);

			let can_chill_from = Self::current_era().unwrap_or(0) + 1;
			Self::chill_stash_soon(&stash, &controller, cluster_id, can_chill_from);

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn do_payout_stakers(era: EraIndex) -> DispatchResult {
			// ToDo: check that validation is finalised for era

			let era_reward_points: EraRewardPoints<T::AccountId> =
				<ErasCDNsRewardPoints<T>>::get(&era);

			let price_per_byte: u128 = match Self::pricing() {
				Some(pricing) => pricing,
				None => Err(Error::<T>::PricingNotSet)?,
			};

			// An account we withdraw the funds from and the amount of funds to withdraw.
			let payout_source_account: T::AccountId =
				T::StakersPayoutSource::get().into_account_truncating();

			// Transfer a part of the budget to each CDN participant rewarded this era.
			for (stash, points) in era_reward_points.clone().individual {
				let reward: BalanceOf<T> = match (points as u128 * price_per_byte).try_into() {
					Ok(value) => value,
					Err(_) => Err(Error::<T>::BudgetOverflow)?,
				};
				log::debug!(
					"Rewarding {:?} with {:?} points, reward size {:?}, balance \
					on payout source account {:?}",
					stash,
					points,
					reward,
					T::Currency::free_balance(&payout_source_account)
				);
				T::Currency::transfer(
					&payout_source_account,
					&stash,
					reward,
					ExistenceRequirement::AllowDeath,
				)?; // ToDo: all success or noop
				Rewards::<T>::mutate(&stash, |current_balance| {
					*current_balance += reward;
				});
				log::debug!("Total rewards to be inserted: {:?}", Self::rewards(&stash));
				PaidErasPerNode::<T>::mutate(&stash, |current_rewards| {
					let rewards = EraRewardsPaid { era, reward };
					current_rewards.push(rewards);
				});
			}
			Self::deposit_event(Event::<T>::PayoutNodes(
				era,
				era_reward_points.clone(),
				price_per_byte,
			));
			log::debug!("Payout event executed");

			log::debug!(
				"Balance left on payout source account {:?}",
				T::Currency::free_balance(&payout_source_account),
			);

			Ok(())
		}

		/// Update the ledger for a controller.
		///
		/// This will also update the stash lock.
		fn update_ledger(
			controller: &T::AccountId,
			ledger: &StakingLedger<T::AccountId, BalanceOf<T>>,
		) {
			T::Currency::set_lock(
				DDC_STAKING_ID,
				&ledger.stash,
				ledger.total,
				WithdrawReasons::all(),
			);
			<Ledger<T>>::insert(controller, ledger);
		}

		/// Chill a stash account.
		fn chill_stash(stash: &T::AccountId) {
			let chilled_as_storage = Self::do_remove_storage(stash);
			let chilled_as_cdn = Self::do_remove_cdn(stash);
			if chilled_as_storage || chilled_as_cdn {
				Self::deposit_event(Event::<T>::Chilled(stash.clone()));
			}
		}

		/// Note a desire of a stash account to chill soon.
		pub fn chill_stash_soon(
			stash: &T::AccountId,
			controller: &T::AccountId,
			cluster: ClusterId,
			can_chill_from: EraIndex,
		) {
			Ledger::<T>::mutate(&controller, |maybe_ledger| {
				if let Some(ref mut ledger) = maybe_ledger {
					ledger.chilling = Some(can_chill_from)
				}
			});
			Self::deposit_event(Event::<T>::ChillSoon(stash.clone(), cluster, can_chill_from));
		}

		/// Remove all associated data of a stash account from the staking system.
		///
		/// Assumes storage is upgraded before calling.
		///
		/// This is called:
		/// - after a `withdraw_unbonded()` call that frees all of a stash's bonded balance.
		/// - through `reap_stash()` if the balance has fallen to zero (through slashing).
		fn kill_stash(stash: &T::AccountId) -> DispatchResult {
			let controller = <Bonded<T>>::get(stash).ok_or(Error::<T>::NotStash)?;

			<Bonded<T>>::remove(stash);
			<Ledger<T>>::remove(&controller);

			if let Some((node, _)) = <Nodes<T>>::iter().find(|(_, v)| v == stash) {
				<Nodes<T>>::remove(node);
			}

			Self::do_remove_storage(stash);
			Self::do_remove_cdn(stash);

			frame_system::Pallet::<T>::dec_consumers(stash);

			Ok(())
		}

		/// This function will add a CDN participant to the `CDNs` storage map.
		///
		/// If the CDN participant already exists, their cluster will be updated.
		pub fn do_add_cdn(who: &T::AccountId, cluster: ClusterId) {
			CDNs::<T>::insert(who, cluster);
		}

		/// This function will remove a CDN participant from the `CDNs` map.
		///
		/// Returns true if `who` was removed from `CDNs`, otherwise false.
		pub fn do_remove_cdn(who: &T::AccountId) -> bool {
			CDNs::<T>::take(who).is_some()
		}

		/// This function will add a storage network participant to the `Storages` storage map.
		///
		/// If the storage network participant already exists, their cluster will be updated.
		pub fn do_add_storage(who: &T::AccountId, cluster: ClusterId) {
			Storages::<T>::insert(who, cluster);
		}

		/// This function will remove a storage network participant from the `Storages` map.
		///
		/// Returns true if `who` was removed from `Storages`, otherwise false.
		pub fn do_remove_storage(who: &T::AccountId) -> bool {
			Storages::<T>::take(who).is_some()
		}

		/// Reset the chilling era for a controller.
		pub fn reset_chilling(controller: &T::AccountId) {
			Ledger::<T>::mutate(&controller, |maybe_ledger| {
				if let Some(ref mut ledger) = maybe_ledger {
					ledger.chilling = None
				}
			});
		}
		/// Add reward points to CDN participants using their stash account ID.
		pub fn reward_by_ids(
			era: EraIndex,
			stakers_points: impl IntoIterator<Item = (T::AccountId, u64)>,
		) {
			<ErasCDNsRewardPoints<T>>::mutate(era, |era_rewards| {
				for (staker, points) in stakers_points.into_iter() {
					*era_rewards.individual.entry(staker).or_default() += points;
					era_rewards.total += points;
				}
			});
		}
	}

	impl<T: Config> StakingVisitor<T> for Pallet<T> {
		fn node_has_stake(
			node_pub_key: &NodePubKey,
			cluster_id: &ClusterId,
		) -> Result<bool, StakingVisitorError> {
			let stash =
				<Nodes<T>>::get(&node_pub_key).ok_or(StakingVisitorError::NodeStakeDoesNotExist)?;
			let maybe_cdn_in_cluster = CDNs::<T>::get(&stash);
			let maybe_storage_in_cluster = Storages::<T>::get(&stash);

			let has_stake: bool = maybe_cdn_in_cluster
				.or(maybe_storage_in_cluster)
				.is_some_and(|staking_cluster| staking_cluster == *cluster_id);

			Ok(has_stake)
		}

		fn node_is_chilling(node_pub_key: &NodePubKey) -> Result<bool, StakingVisitorError> {
			let stash =
				<Nodes<T>>::get(&node_pub_key).ok_or(StakingVisitorError::NodeStakeDoesNotExist)?;
			let controller =
				<Bonded<T>>::get(&stash).ok_or(StakingVisitorError::NodeStakeIsInBadState)?;

			let is_chilling = <Ledger<T>>::get(&controller)
				.ok_or(StakingVisitorError::NodeStakeIsInBadState)?
				.chilling
				.is_some();

			Ok(is_chilling)
		}
	}

	impl<T> From<ClusterVisitorError> for Error<T> {
		fn from(error: ClusterVisitorError) -> Self {
			match error {
				ClusterVisitorError::ClusterDoesNotExist => Error::<T>::NodeHasNoStake,
				ClusterVisitorError::ClusterGovParamsNotSet => Error::<T>::NoClusterGovParams,
			}
		}
	}
}
