#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod testing_utils;

pub mod weights;
use crate::weights::WeightInfo;

use codec::{Decode, Encode, HasCompact};
use frame_system::pallet_prelude::*;
use frame_support::{
	BoundedVec,
	dispatch::Codec,
	pallet_prelude::*,
	parameter_types,
	traits::{Currency, DefensiveSaturating, LockableCurrency, LockIdentifier, UnixTime, WithdrawReasons},
};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Saturating, Zero, StaticLookup},
	RuntimeDebug,
};
use sp_staking::EraIndex;
use sp_std::prelude::*;

pub use pallet::*;

/// Two minutes.
///
/// If you are changing this, check `on_finalize` hook to ensure `CurrentEra` is capable to hold the
/// value with the new era duration.
const DDC_ERA_DURATION_MS: u128 = 120_000;

/// 2023-01-01 00:00:00 UTC
const DDC_ERA_START_MS: u128 = 1_672_531_200_000;
const DDC_STAKING_ID: LockIdentifier = *b"ddcstake"; // DDC maintainer's stake

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type ClusterId = u32;

parameter_types! {
	/// A limit to the number of pending unlocks an account may have in parallel.
	pub MaxUnlockingChunks: u32 = 32;
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
		Self { stash, total: Zero::zero(), active: Zero::zero(), unlocking: Default::default() }
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

		Self { stash: self.stash, total, active: self.active, unlocking }
	}
}

/// Cluster staking parameters.
#[derive(Clone, Decode, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ClusterSettings<T: Config> {
	/// The bond size required to become and maintain the role of a CDN participant.
	#[codec(compact)]
	pub edge_bond_size: BalanceOf<T>,
	/// The bond size required to become and maintain the role of a storage network participant.
	#[codec(compact)]
	pub storage_bond_size: BalanceOf<T>,
}

impl<T: pallet::Config> Default for ClusterSettings<T> {
	/// Default to the values specified in the runtime config.
	fn default() -> Self {
		Self {
			edge_bond_size: T::DefaultEdgeBondSize::get(),
			storage_bond_size: T::DefaultStorageBondSize::get(),
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

	/// Possible operations on the configuration values of this pallet.
	#[derive(TypeInfo, Debug, Clone, Encode, Decode, PartialEq)]
	pub enum ConfigOp<T: Default + Codec> {
		/// Don't change.
		Noop,
		/// Set the given value.
		Set(T),
		/// Remove from storage.
		Remove,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		/// Default bond size for a CDN participant.
		#[pallet::constant]
		type DefaultEdgeBondSize: Get<BalanceOf<Self>>;

		/// Default bond size for a storage network participant.
		#[pallet::constant]
		type DefaultStorageBondSize: Get<BalanceOf<Self>>;

		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Number of eras that staked funds must remain bonded for.
		#[pallet::constant]
		type BondingDuration: Get<EraIndex>;
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// Time used for computing era index. It is guaranteed to start being called from the first
		/// `on_finalize`.
		type UnixTime: UnixTime;
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

	/// The bond size required to become and maintain the role of a CDN participant.
	#[pallet::storage]
	pub type EdgeBondSize<T: Config> =
		StorageValue<_, BalanceOf<T>, ValueQuery, T::DefaultEdgeBondSize>;

	/// The bond size required to become and maintain the role of a storage network participant.
	#[pallet::storage]
	pub type StorageBondSize<T: Config> =
		StorageValue<_, BalanceOf<T>, ValueQuery, T::DefaultStorageBondSize>;

	/// Map from all (unlocked) "controller" accounts to the info regarding the staking.
	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	pub type Ledger<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, StakingLedger<T::AccountId, BalanceOf<T>>>;

	/// The map of (wannabe) CDN participants stash keys to the DDC cluster ID they wish to
	/// participate into.
	#[pallet::storage]
	#[pallet::getter(fn edges)]
	pub type Edges<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ClusterId>;

	/// The map of (wannabe) storage network participants stash keys to the DDC cluster ID they wish
	/// to participate into..
	#[pallet::storage]
	#[pallet::getter(fn storages)]
	pub type Storages<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ClusterId>;

	/// The current era index.
	///
	/// This is the latest planned era, depending on how the Session pallet queues the validator
	/// set, it might be active or not.
	#[pallet::storage]
	#[pallet::getter(fn current_era)]
	pub type CurrentEra<T> = StorageValue<_, EraIndex>;

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
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not a controller account.
		NotController,
		/// Not a stash account.
		NotStash,
		/// Stash is already bonded.
		AlreadyBonded,
		/// Controller is already paired.
		AlreadyPaired,
		/// Cannot have a storage network or CDN participant, with the size less than defined by
		/// governance (see `BondSize`). If unbonding is the intention, `chill` first to remove
		/// one's role as storage/edge.
		InsufficientBond,
		/// Can not schedule more unlock chunks.
		NoMoreChunks,
		/// Internal state has become somehow corrupted and the operation cannot continue.
		BadState,
		// An account already declared a desire to participate in the network with a certain role
		// and to take another role it should call `chill` first.
		AlreadyInRole,
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

			frame_system::Pallet::<T>::inc_consumers(&stash).map_err(|_| Error::<T>::BadState)?;

			// You're auto-bonded forever, here. We might improve this by only bonding when
			// you actually store/serve and remove once you unbond __everything__.
			<Bonded<T>>::insert(&stash, &controller);

			let stash_balance = T::Currency::free_balance(&stash);
			let value = value.min(stash_balance);
			Self::deposit_event(Event::<T>::Bonded(stash.clone(), value));
			let item =
				StakingLedger { stash, total: value, active: value, unlocking: Default::default() };
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

				let min_active_bond = if Edges::<T>::contains_key(&ledger.stash) {
					EdgeBondSize::<T>::get()
				} else if Storages::<T>::contains_key(&ledger.stash) {
					StorageBondSize::<T>::get()
				} else {
					Zero::zero()
				};

				// Make sure that the user maintains enough active bond for their role.
				// If a user runs into this error, they should chill first.
				ensure!(ledger.active >= min_active_bond, Error::<T>::InsufficientBond);

				// Note: in case there is no current era it is fine to bond one era more.
				let era = Self::current_era().unwrap_or(0) + T::BondingDuration::get();
				if let Some(mut chunk) =
					ledger.unlocking.last_mut().filter(|chunk| chunk.era == era)
				{
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

		/// Declare the desire to participate in CDN for the origin controller.
		///
		/// `cluster` is the ID of the DDC cluster the participant wishes to join.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash. The
		/// bond size must be greater than or equal to the `EdgeBondSize`.
		#[pallet::weight(T::WeightInfo::serve())]
		pub fn serve(origin: OriginFor<T>, cluster: ClusterId) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

			ensure!(ledger.active >= EdgeBondSize::<T>::get(), Error::<T>::InsufficientBond);
			let stash = &ledger.stash;

			// Can't participate in CDN if already participating in storage network.
			ensure!(!Storages::<T>::contains_key(&stash), Error::<T>::AlreadyInRole);

			Self::do_add_edge(stash, cluster);
			Ok(())
		}

		/// Declare the desire to participate in storage network for the origin controller.
		///
		/// `cluster` is the ID of the DDC cluster the participant wishes to join.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash. The
		/// bond size must be greater than or equal to the `StorageBondSize`.
		#[pallet::weight(T::WeightInfo::store())]
		pub fn store(origin: OriginFor<T>, cluster: ClusterId) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

			ensure!(ledger.active >= StorageBondSize::<T>::get(), Error::<T>::InsufficientBond);
			let stash = &ledger.stash;

			// Can't participate in storage network if already participating in CDN.
			ensure!(!Edges::<T>::contains_key(&stash), Error::<T>::AlreadyInRole);

			Self::do_add_storage(stash, cluster);
			Ok(())
		}

		/// Declare no desire to either participate in storage network or CDN.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		#[pallet::weight(T::WeightInfo::chill())]
		pub fn chill(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			Self::chill_stash(&ledger.stash);
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

		/// Update the DDC staking configurations .
		///
		/// * `storage_bond_size`: The active bond needed to be a Storage node.
		/// * `edge_bond_size`: The active bond needed to be an Edge node.
		///
		/// RuntimeOrigin must be Root to call this function.
		///
		/// NOTE: Existing nominators and validators will not be affected by this update.
		#[pallet::weight(10_000)]
		pub fn set_staking_configs(
			origin: OriginFor<T>,
			storage_bond_size: ConfigOp<BalanceOf<T>>,
			edge_bond_size: ConfigOp<BalanceOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			macro_rules! config_op_exp {
				($storage:ty, $op:ident) => {
					match $op {
						ConfigOp::Noop => (),
						ConfigOp::Set(v) => <$storage>::put(v),
						ConfigOp::Remove => <$storage>::kill(),
					}
				};
			}

			config_op_exp!(StorageBondSize<T>, storage_bond_size);
			config_op_exp!(EdgeBondSize<T>, edge_bond_size);

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
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
			let chilled_as_edge = Self::do_remove_edge(stash);
			if chilled_as_storage || chilled_as_edge {
				Self::deposit_event(Event::<T>::Chilled(stash.clone()));
			}
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

			Self::do_remove_storage(stash);
			Self::do_remove_edge(stash);

			frame_system::Pallet::<T>::dec_consumers(stash);

			Ok(())
		}

		/// This function will add a CDN participant to the `Edges` storage map.
		///
		/// If the CDN participant already exists, their cluster will be updated.
		///
		/// NOTE: you must ALWAYS use this function to add a CDN participant to the system. Any
		/// access to `Edges` outside of this function is almost certainly
		/// wrong.
		pub fn do_add_edge(who: &T::AccountId, cluster: ClusterId) {
			Edges::<T>::insert(who, cluster);
		}

		/// This function will remove a CDN participant from the `Edges` map.
		///
		/// Returns true if `who` was removed from `Edges`, otherwise false.
		///
		/// NOTE: you must ALWAYS use this function to remove a storage network participant from the
		/// system. Any access to `Edges` outside of this function is almost certainly
		/// wrong.
		pub fn do_remove_edge(who: &T::AccountId) -> bool {
			Edges::<T>::take(who).is_some()
		}

		/// This function will add a storage network participant to the `Storages` storage map.
		///
		/// If the storage network participant already exists, their cluster will be updated.
		///
		/// NOTE: you must ALWAYS use this function to add a storage network participant to the
		/// system. Any access to `Storages` outside of this function is almost certainly
		/// wrong.
		pub fn do_add_storage(who: &T::AccountId, cluster: ClusterId) {
			Storages::<T>::insert(who, cluster);
		}

		/// This function will remove a storage network participant from the `Storages` map.
		///
		/// Returns true if `who` was removed from `Storages`, otherwise false.
		///
		/// NOTE: you must ALWAYS use this function to remove a storage network participant from the
		/// system. Any access to `Storages` outside of this function is almost certainly
		/// wrong.
		pub fn do_remove_storage(who: &T::AccountId) -> bool {
			Storages::<T>::take(who).is_some()
		}
	}
}
