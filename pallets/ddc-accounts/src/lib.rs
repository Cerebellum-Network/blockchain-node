#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use codec::{Decode, Encode, HasCompact};

use frame_support::{
	parameter_types,
	traits::{Currency, DefensiveSaturating, ExistenceRequirement, UnixTime},
	BoundedVec, PalletId,
};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, AtLeast32BitUnsigned, Saturating, Zero},
	RuntimeDebug, SaturatedConversion,
};

use sp_staking::EraIndex;
use sp_std::prelude::*;

pub use pallet::*;

pub const TIME_START_MS: u128 = 1_672_531_200_000;
pub const ERA_DURATION_MS: u128 = 120_000;
pub const ERA_IN_BLOCKS: u8 = 20;

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
pub struct Bucket<AccountId> {
	bucket_id: u128,
	owner_id: AccountId,
	public_availability: bool,
	resources_reserved: u128,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct BucketsDetails<Balance: HasCompact> {
	pub bucket_id: u128,
	pub amount: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ReceiverDetails<AccountId, Balance: HasCompact> {
	cdn_owner: AccountId,
	amount: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct AccountsLedger<AccountId, Balance: HasCompact> {
	/// The stash account whose balance is actually locked and can be used for CDN usage.
	pub stash: AccountId,
	/// The total amount of the stash's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	#[codec(compact)]
	pub total: Balance,
	/// The total amount of the stash's balance that will be accessible for CDN payments in any
	/// forthcoming rounds.
	#[codec(compact)]
	pub active: Balance,
	/// Any balance that is becoming free, which may eventually be transferred out of the stash
	/// (assuming that the content owner has to pay for network usage). It is assumed that this
	/// will be treated as a first in, first out queue where the new (higher value) eras get pushed
	/// on the back.
	pub unlocking: BoundedVec<UnlockChunk<Balance>, MaxUnlockingChunks>,
}

impl<AccountId, Balance: HasCompact + Copy + Saturating + AtLeast32BitUnsigned + Zero>
	AccountsLedger<AccountId, Balance>
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
				log::info!("Chunk era: {:?}", chunk.era);
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

	/// Charge funds that were scheduled for unlocking.
	///
	/// Returns the updated ledger, and the amount actually charged.
	fn charge_unlocking(mut self, value: Balance) -> (Self, Balance) {
		let mut unlocking_balance = Balance::zero();

		while let Some(last) = self.unlocking.last_mut() {
			if unlocking_balance + last.value <= value {
				unlocking_balance += last.value;
				self.active -= last.value;
				self.unlocking.pop();
			} else {
				let diff = value - unlocking_balance;

				unlocking_balance += diff;
				self.active -= diff;
				last.value -= diff;
			}

			if unlocking_balance >= value {
				break
			}
		}

		(self, unlocking_balance)
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

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The accounts's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Number of eras that staked funds must remain bonded for.
		#[pallet::constant]
		type BondingDuration: Get<EraIndex>;
		type TimeProvider: UnixTime;
	}

	/// Map from all locked "stash" accounts to the controller account.
	#[pallet::storage]
	#[pallet::getter(fn bonded)]
	pub type Bonded<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

	/// Map from all (unlocked) "controller" accounts to the info regarding the staking.
	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	pub type Ledger<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, AccountsLedger<T::AccountId, BalanceOf<T>>>;

	#[pallet::type_value]
	pub fn DefaultBucketCount<T: Config>() -> u128 {
		0_u128
	}
	#[pallet::storage]
	#[pallet::getter(fn buckets_count)]
	pub type BucketsCount<T: Config> =
		StorageValue<Value = u128, QueryKind = ValueQuery, OnEmpty = DefaultBucketCount<T>>;

	/// Map from all locked accounts and their buckets to the bucket structure
	#[pallet::storage]
	#[pallet::getter(fn buckets)]
	pub type Buckets<T: Config> =
		StorageMap<_, Twox64Concat, u128, Bucket<T::AccountId>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account has bonded this amount. \[stash, amount\]
		///
		/// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
		/// it will not be emitted for staking rewards when they are added to stake.
		Deposited(T::AccountId, BalanceOf<T>),
		/// An account has unbonded this amount. \[stash, amount\]
		Unbonded(T::AccountId, BalanceOf<T>),
		/// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
		/// from the unlocking queue. \[stash, amount\]
		Withdrawn(T::AccountId, BalanceOf<T>),
		/// Total amount charged from all accounts to pay CDN nodes
		Charged(BalanceOf<T>),
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
		/// Cannot deposit dust
		InsufficientDeposit,
		/// Can not schedule more unlock chunks.
		NoMoreChunks,
		/// Internal state has become somehow corrupted and the operation cannot continue.
		BadState,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig;

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			let account_id = <Pallet<T>>::account_id();
			let min = T::Currency::minimum_balance();
			if T::Currency::free_balance(&account_id) < min {
				let _ = T::Currency::make_free_balance_be(&account_id, min);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn charge_payments(
			origin: OriginFor<T>,
			paying_accounts: Vec<BucketsDetails<BalanceOf<T>>>,
		) -> DispatchResult {
			let validator = ensure_signed(origin)?;
			let mut total_charged = BalanceOf::<T>::zero();

			for bucket_details in paying_accounts.iter() {
				let bucket: Bucket<T::AccountId> = Self::buckets(bucket_details.bucket_id).unwrap();
				let content_owner = bucket.owner_id;
				let amount = bucket_details.amount;

				let mut ledger = Self::ledger(&content_owner).ok_or(Error::<T>::NotController)?;
				if ledger.active >= amount {
					ledger.total -= amount;
					ledger.active -= amount;
					total_charged += amount;
					Self::update_ledger(&content_owner, &ledger);
				} else {
					let diff = amount - ledger.active;
					total_charged += ledger.active;
					ledger.total -= ledger.active;
					ledger.active = BalanceOf::<T>::zero();
					let (ledger, charged) = ledger.charge_unlocking(diff);
					Self::update_ledger(&content_owner, &ledger);
					total_charged += charged;
				}
			}
			Self::deposit_event(Event::<T>::Charged(total_charged));

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn create_bucket(
			origin: OriginFor<T>,
			public_availability: bool,
			resources_reserved: u128,
		) -> DispatchResult {
			let bucket_owner = ensure_signed(origin)?;
			let cur_bucket_id = Self::buckets_count();

			let bucket = Bucket {
				bucket_id: cur_bucket_id + 1,
				owner_id: bucket_owner,
				public_availability,
				resources_reserved,
			};

			<BucketsCount<T>>::set(cur_bucket_id + 1);
			<Buckets<T>>::insert(cur_bucket_id + 1, bucket);
			Ok(())
		}

		/// Take the origin account as a stash and lock up `value` of its balance. `controller` will
		/// be the account that controls it.
		///
		/// `value` must be more than the `minimum_balance` specified by `T::Currency`.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash account.
		///
		/// Emits `Deposited`.
		#[pallet::weight(10_000)]
		pub fn deposit(
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

			// Reject a deposit which is considered to be _dust_.
			if value < T::Currency::minimum_balance() {
				Err(Error::<T>::InsufficientDeposit)?
			}

			frame_system::Pallet::<T>::inc_consumers(&stash).map_err(|_| Error::<T>::BadState)?;

			<Bonded<T>>::insert(&stash, &controller);

			let stash_balance = T::Currency::free_balance(&stash);
			let value = value.min(stash_balance);
			Self::deposit_event(Event::<T>::Deposited(stash.clone(), value));
			let item = AccountsLedger {
				stash: stash.clone(),
				total: value,
				active: value,
				unlocking: Default::default(),
			};
			Self::update_ledger_and_deposit(&stash, &controller, &item)?;
			Ok(())
		}

		/// Add some extra amount that have appeared in the stash `free_balance` into the balance up
		/// for CDN payments.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
		///
		/// Emits `Deposited`.
		#[pallet::weight(10_000)]
		pub fn deposit_extra(
			origin: OriginFor<T>,
			#[pallet::compact] max_additional: BalanceOf<T>,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;

			let controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
			let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

			let stash_balance = T::Currency::free_balance(&stash);
			let extra = stash_balance.min(max_additional);
			ledger.total += extra;
			ledger.active += extra;
			// Last check: the new active amount of ledger must be more than ED.
			ensure!(
				ledger.active >= T::Currency::minimum_balance(),
				Error::<T>::InsufficientDeposit
			);

			Self::update_ledger_and_deposit(&stash, &controller, &ledger)?;

			Self::deposit_event(Event::<T>::Deposited(stash.clone(), extra));

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
		/// Emits `Unbonded`.
		///
		/// See also [`Call::withdraw_unbonded`].
		#[pallet::weight(10_000)]
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

				// Avoid there being a dust balance left in the accounts system.
				if ledger.active < T::Currency::minimum_balance() {
					value += ledger.active;
					ledger.active = Zero::zero();
				}

				// Note: bonding for extra era to allow for accounting
				let era = Self::get_current_era() + T::BondingDuration::get();
				log::info!("Era for the unbond: {:?}", era);

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
		#[pallet::weight(10_000)]
		pub fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			let (stash, old_total) = (ledger.stash.clone(), ledger.total);
			let current_era = Self::get_current_era();
			ledger = ledger.consolidate_unlocked(current_era);
			log::info!("Current era: {:?}", current_era);

			if ledger.unlocking.is_empty() && ledger.active < T::Currency::minimum_balance() {
				log::info!("Killing stash");
				// This account must have called `unbond()` with some value that caused the active
				// portion to fall below existential deposit + will have no more unlocking chunks
				// left. We can now safely remove all accounts-related information.
				Self::kill_stash(&stash)?;
			} else {
				log::info!("Updating ledger");
				// This was the consequence of a partial unbond. just update the ledger and move on.
				Self::update_ledger(&controller, &ledger);
			};

			log::info!("Current total: {:?}", ledger.total);
			log::info!("Old total: {:?}", old_total);

			// `old_total` should never be less than the new total because
			// `consolidate_unlocked` strictly subtracts balance.
			if ledger.total < old_total {
				log::info!("Preparing for transfer");
				// Already checked that this won't overflow by entry condition.
				let value = old_total - ledger.total;

				let account_id = Self::account_id();

				T::Currency::transfer(&account_id, &stash, value, ExistenceRequirement::KeepAlive)?;
				Self::deposit_event(Event::<T>::Withdrawn(stash, value));
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
		/// Update the ledger for a controller.
		///
		/// This will also deposit the funds to pallet.
		fn update_ledger_and_deposit(
			stash: &T::AccountId,
			controller: &T::AccountId,
			ledger: &AccountsLedger<T::AccountId, BalanceOf<T>>,
		) -> DispatchResult {
			let account_id = Self::account_id();

			T::Currency::transfer(
				stash,
				&account_id,
				ledger.total,
				ExistenceRequirement::KeepAlive,
			)?;
			<Ledger<T>>::insert(controller, ledger);

			Ok(())
		}

		/// Update the ledger for a controller.
		fn update_ledger(
			controller: &T::AccountId,
			ledger: &AccountsLedger<T::AccountId, BalanceOf<T>>,
		) {
			<Ledger<T>>::insert(controller, ledger);
		}

		/// Remove all associated data of a stash account from the accounts system.
		///
		/// Assumes storage is upgraded before calling.
		///
		/// This is called:
		/// - after a `withdraw_unbonded()` call that frees all of a stash's bonded balance.
		fn kill_stash(stash: &T::AccountId) -> DispatchResult {
			let controller = <Bonded<T>>::get(stash).ok_or(Error::<T>::NotStash)?;

			<Bonded<T>>::remove(stash);
			<Ledger<T>>::remove(&controller);

			frame_system::Pallet::<T>::dec_consumers(stash);

			Ok(())
		}

		// Get the current era.
		fn get_current_era() -> EraIndex {
			((T::TimeProvider::now().as_millis() - TIME_START_MS) / ERA_DURATION_MS)
				.try_into()
				.unwrap()
		}

		// Charge payments from content owners
		pub fn charge_payments_new(
			paying_accounts: Vec<BucketsDetails<BalanceOf<T>>>,
			pricing: u128,
		) -> DispatchResult {
			let mut total_charged = BalanceOf::<T>::zero();

			for bucket_details in paying_accounts.iter() {
				let bucket: Bucket<T::AccountId> = Self::buckets(bucket_details.bucket_id).unwrap();
				let content_owner = bucket.owner_id;
				let amount = bucket_details.amount * pricing.saturated_into::<BalanceOf<T>>();

				let mut ledger = Self::ledger(&content_owner).ok_or(Error::<T>::NotController)?;
				if ledger.active >= amount {
					ledger.total -= amount;
					ledger.active -= amount;
					total_charged += amount;
					log::info!("Ledger updated state: {:?}", &ledger);
					Self::update_ledger(&content_owner, &ledger);
				} else {
					let diff = amount - ledger.active;
					total_charged += ledger.active;
					ledger.total -= ledger.active;
					ledger.active = BalanceOf::<T>::zero();
					let (ledger, charged) = ledger.charge_unlocking(diff);
					log::info!("Ledger updated state: {:?}", &ledger);
					Self::update_ledger(&content_owner, &ledger);
					total_charged += charged;
				}
			}
			log::info!("Total charged: {:?}", &total_charged);

			Self::deposit_event(Event::<T>::Charged(total_charged));
			log::info!("Deposit event executed");

			Ok(())
		}
	}
}
