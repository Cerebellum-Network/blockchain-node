#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod testing_utils;

pub mod weights;
use crate::weights::WeightInfo;

use codec::{Decode, Encode, HasCompact};

use frame_support::{
	dispatch::Codec,
	parameter_types,
	traits::{Currency, DefensiveSaturating, LockIdentifier, UnixTime, WithdrawReasons},
	BoundedVec,
};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, CheckedSub, Saturating, Zero},
	RuntimeDebug,
};

use sp_staking::EraIndex;
use sp_std::prelude::*;

pub use pallet::*;

const DDC_STAKING_ID: LockIdentifier = *b"ddcstake"; // DDC maintainer's stake

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*, sp_runtime::traits::StaticLookup, traits::LockableCurrency,
		Blake2_128Concat,
	};
	use frame_system::pallet_prelude::*;

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

	/// The bond size required to become and maintain the role of a CDN or storage network
	/// participant.
	#[pallet::storage]
	pub type BondSize<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Map from all (unlocked) "controller" accounts to the info regarding the staking.
	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	pub type Ledger<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, StakingLedger<T::AccountId, BalanceOf<T>>>;

	/// The list of (wannabe) storage network participants stash keys.
	#[pallet::storage]
	#[pallet::getter(fn storages)]
	pub type Storages<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	/// The list of (wannabe) CDN participants stash keys.
	#[pallet::storage]
	#[pallet::getter(fn edges)]
	pub type Edges<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

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
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;

			if <Bonded<T>>::contains_key(&stash) {
				Err(Error::<T>::AlreadyBonded)?
			}

			let controller = T::Lookup::lookup(controller)?;

			if <Ledger<T>>::contains_key(&controller) {
				Err(Error::<T>::AlreadyPaired)?
			}

			let stash_free = T::Currency::free_balance(&stash);

			// Reject a bond which is considered to be _dust_.
			if stash_free < T::Currency::minimum_balance() {
				Err(Error::<T>::InsufficientBond)?
			}

			let bond_size = BondSize::<T>::get();

			// Reject a bond which is lower then required.
			if stash_free < bond_size {
				Err(Error::<T>::InsufficientBond)?
			}

			frame_system::Pallet::<T>::inc_consumers(&stash).map_err(|_| Error::<T>::BadState)?;

			// You're auto-bonded forever, here. We might improve this by only bonding when
			// you actually store/serve and remove once you unbond.
			<Bonded<T>>::insert(&stash, &controller);

			Self::deposit_event(Event::<T>::Bonded(stash.clone(), bond_size));
			let item = StakingLedger {
				stash,
				total: bond_size,
				active: bond_size,
				unlocking: Default::default(),
			};
			Self::update_ledger(&controller, &item);
			Ok(())
		}

		/// Schedule a bond of the stash to be unlocked ready for transfer out after the bond
		/// period ends.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		///
		/// Once the unlock period is done, you can call `withdraw_unbonded` to actually move
		/// the funds out of management ready for transfer.
		///
		/// No more than a limited number of unlocking chunks (see `MaxUnlockingChunks`)
		/// can co-exists at the same time. In that case, [`Call::withdraw_unbonded`] need
		/// to be called first to remove some of the chunks (if possible). This feature is actually
		/// not required because we unlock the whole bond at once, means it is impossible to have
		/// more then one unlocking at time. But this is inherited from the `pallet-staking` and we
		/// may remove in some future version.
		///
		/// If a user encounters the `InsufficientBond` error when calling this extrinsic,
		/// they should call `chill` first in order to free up their bonded funds.
		///
		/// Emits `Unbonded`.
		///
		/// See also [`Call::withdraw_unbonded`].
		#[pallet::weight(T::WeightInfo::unbond())]
		pub fn unbond(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

			if ledger.active.is_zero() {
				// Nothing to unbond.
				return Ok(())
			}

			ensure!(
				ledger.unlocking.len() < MaxUnlockingChunks::get() as usize,
				Error::<T>::NoMoreChunks,
			);

			// Make sure that the user maintains enough active bond for their role.
			// If a user runs into this error, they should chill first.
			ensure!(!Storages::<T>::get().contains(&ledger.stash), Error::<T>::InsufficientBond);
			ensure!(!Edges::<T>::get().contains(&ledger.stash), Error::<T>::InsufficientBond);

			let era = Self::current_era().unwrap_or(0) + T::BondingDuration::get();

			// Unbond actual active stake instead of the current `BondSize` to allow users bond and
			// unbond the same amount regardless of changes of the `BondSize`.
			let unbond_value = ledger.active.clone();
			ledger.active = Zero::zero();

			if let Some(mut chunk) = ledger.unlocking.last_mut().filter(|chunk| chunk.era == era) {
				// To keep the chunk count down, we only keep one chunk per era. Since
				// `unlocking` is a FiFo queue, if a chunk exists for `era` we know that it will
				// be the last one.
				chunk.value = chunk.value.defensive_saturating_add(unbond_value)
			} else {
				ledger
					.unlocking
					.try_push(UnlockChunk { value: unbond_value, era })
					.map_err(|_| Error::<T>::NoMoreChunks)?;
			};
			// NOTE: ledger must be updated prior to calling `Self::weight_of`.
			Self::update_ledger(&controller, &ledger);

			Self::deposit_event(Event::<T>::Unbonded(ledger.stash, unbond_value));

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

		/// Declare the desire to participate in storage network for the origin controller.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		#[pallet::weight(T::WeightInfo::store())]
		pub fn store(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

			ensure!(ledger.active >= BondSize::<T>::get(), Error::<T>::InsufficientBond);
			let stash = &ledger.stash;

			// Can't participate in storage network if already participating in CDN.
			ensure!(!Edges::<T>::get().contains(&stash), Error::<T>::AlreadyInRole);

			Self::do_add_storage(stash);
			Ok(())
		}

		/// Declare the desire to participate in CDN for the origin controller.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		#[pallet::weight(T::WeightInfo::serve())]
		pub fn serve(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

			ensure!(ledger.active >= BondSize::<T>::get(), Error::<T>::InsufficientBond);
			let stash = &ledger.stash;

			// Can't participate in CDN if already participating in storage network.
			ensure!(!Storages::<T>::get().contains(&stash), Error::<T>::AlreadyInRole);

			Self::do_add_edge(stash);
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
		/// * `bond_size`: The active bond needed to be a Storage or Edge node.
		///
		/// RuntimeOrigin must be Root to call this function.
		///
		/// NOTE: Existing nominators and validators will not be affected by this update.
		#[pallet::weight(10_000)]
		pub fn set_staking_configs(
			origin: OriginFor<T>,
			bond_size: ConfigOp<BalanceOf<T>>,
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

			config_op_exp!(BondSize<T>, bond_size);

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

		/// This function will add a storage network participant to the `Storages` storage map.
		///
		/// If the storage network participant already exists, their preferences will be updated.
		///
		/// NOTE: you must ALWAYS use this function to add a storage network participant to the
		/// system. Any access to `Storages` outside of this function is almost certainly
		/// wrong.
		pub fn do_add_storage(who: &T::AccountId) {
			Storages::<T>::append(who);
		}

		/// This function will remove a storage network participant from the `Storages` list.
		///
		/// Returns true if `who` was removed from `Storages`, otherwise false.
		///
		/// NOTE: you must ALWAYS use this function to remove a storage network participant from the
		/// system. Any access to `Storages` outside of this function is almost certainly
		/// wrong.
		pub fn do_remove_storage(who: &T::AccountId) -> bool {
			Storages::<T>::mutate(|storages| {
				let maybe_position = storages.iter().position(|s| s == who);
				if let Some(index) = maybe_position {
					storages.swap_remove(index);
					return true
				};
				false
			})
		}

		/// This function will add a CDN participant to the `Edges` storage map.
		///
		/// If the CDN participant already exists, their preferences will be updated.
		///
		/// NOTE: you must ALWAYS use this function to add a CDN participant to the system. Any
		/// access to `Edges` outside of this function is almost certainly
		/// wrong.
		pub fn do_add_edge(who: &T::AccountId) {
			Edges::<T>::append(who);
		}

		/// This function will remove a CDN participant from the `Edges` list.
		///
		/// Returns true if `who` was removed from `Edges`, otherwise false.
		///
		/// NOTE: you must ALWAYS use this function to remove a storage network participant from the
		/// system. Any access to `Edges` outside of this function is almost certainly
		/// wrong.
		pub fn do_remove_edge(who: &T::AccountId) -> bool {
			Edges::<T>::mutate(|storages| {
				let maybe_position = storages.iter().position(|s| s == who);
				if let Some(index) = maybe_position {
					storages.swap_remove(index);
					return true
				};
				false
			})
		}
	}
}
