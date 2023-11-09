#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use codec::{Decode, Encode, HasCompact};

use ddc_primitives::{BucketId, ClusterId};
use ddc_traits::cluster::ClusterVisitor;
use frame_support::{
	parameter_types,
	traits::{Currency, DefensiveSaturating, ExistenceRequirement},
	BoundedVec, PalletId,
};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, AtLeast32BitUnsigned, Saturating, Zero},
	RuntimeDebug, SaturatedConversion,
};
use sp_std::prelude::*;

pub use pallet::*;

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

parameter_types! {
	/// A limit to the number of pending unlocks an account may have in parallel.
	pub MaxUnlockingChunks: u32 = 32;
}

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct UnlockChunk<Balance: HasCompact, T: Config> {
	/// Amount of funds to be unlocked.
	#[codec(compact)]
	value: Balance,
	/// Block number at which point it'll be unlocked.
	#[codec(compact)]
	block: T::BlockNumber,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Bucket<AccountId> {
	bucket_id: BucketId,
	owner_id: AccountId,
	cluster_id: ClusterId,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct BucketsDetails<Balance: HasCompact> {
	pub bucket_id: BucketId,
	pub amount: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct AccountsLedger<AccountId, Balance: HasCompact, T: Config> {
	/// The owner account whose balance is actually locked and can be used for CDN usage.
	pub owner: AccountId,
	/// The total amount of the owner's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	#[codec(compact)]
	pub total: Balance,
	/// The total amount of the owner's balance that will be accessible for CDN payments in any
	/// forthcoming rounds.
	#[codec(compact)]
	pub active: Balance,
	/// Any balance that is becoming free, which may eventually be transferred out of the owner
	/// (assuming that the content owner has to pay for network usage). It is assumed that this
	/// will be treated as a first in, first out queue where the new (higher value) eras get pushed
	/// on the back.
	pub unlocking: BoundedVec<UnlockChunk<Balance, T>, MaxUnlockingChunks>,
}

impl<
		AccountId,
		Balance: HasCompact + Copy + Saturating + AtLeast32BitUnsigned + Zero,
		T: Config,
	> AccountsLedger<AccountId, Balance, T>
{
	/// Initializes the default object using the given owner.
	pub fn default_from(owner: AccountId) -> Self {
		Self { owner, total: Zero::zero(), active: Zero::zero(), unlocking: Default::default() }
	}

	/// Remove entries from `unlocking` that are sufficiently old and reduce the
	/// total by the sum of their balances.
	fn consolidate_unlocked(self, current_block: T::BlockNumber) -> Self {
		let mut total = self.total;
		let unlocking: BoundedVec<_, _> = self
			.unlocking
			.into_iter()
			.filter(|chunk| {
				log::debug!("Chunk era: {:?}", chunk.block);
				if chunk.block > current_block {
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

		Self { owner: self.owner, total, active: self.active, unlocking }
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
	use frame_support::{pallet_prelude::*, traits::LockableCurrency};
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
		/// Number of eras that staked funds must remain locked for.
		#[pallet::constant]
		type UnlockingDelay: Get<<Self as frame_system::Config>::BlockNumber>;
		type ClusterVisitor: ClusterVisitor<Self>;
	}

	/// Map from all (unlocked) "owner" accounts to the info regarding the staking.
	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	pub type Ledger<T: Config> =
		StorageMap<_, Identity, T::AccountId, AccountsLedger<T::AccountId, BalanceOf<T>, T>>;

	#[pallet::type_value]
	pub fn DefaultBucketCount<T: Config>() -> BucketId {
		0
	}
	#[pallet::storage]
	#[pallet::getter(fn buckets_count)]
	pub type BucketsCount<T: Config> =
		StorageValue<Value = BucketId, QueryKind = ValueQuery, OnEmpty = DefaultBucketCount<T>>;

	/// Map from bucket ID to to the bucket structure
	#[pallet::storage]
	#[pallet::getter(fn buckets)]
	pub type Buckets<T: Config> =
		StorageMap<_, Twox64Concat, BucketId, Bucket<T::AccountId>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account has deposited this amount. \[owner, amount\]
		///
		/// NOTE: This event is only emitted when funds are deposited via a dispatchable. Notably,
		/// it will not be emitted for staking rewards when they are added to stake.
		Deposited(T::AccountId, BalanceOf<T>),
		/// An account has initiated unlock for amount. \[owner, amount\]
		InitiatDepositUnlock(T::AccountId, BalanceOf<T>),
		/// An account has called `withdraw_unlocked_deposit` and removed unlocking chunks worth
		/// `Balance` from the unlocking queue. \[owner, amount\]
		Withdrawn(T::AccountId, BalanceOf<T>),
		/// Total amount charged from all accounts to pay CDN nodes
		Charged(BalanceOf<T>),
		/// Bucket with specific id created
		BucketCreated(BucketId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not a owner account.
		NotOwner,
		/// Not an owner of bucket
		NotBucketOwner,
		/// Owner is already paired with structure representing account.
		AlreadyPaired,
		/// Cannot deposit dust
		InsufficientDeposit,
		/// Can not schedule more unlock chunks.
		NoMoreChunks,
		/// Bucket with speicifed id doesn't exist.
		NoBucketWithId,
		/// Internal state has become somehow corrupted and the operation cannot continue.
		BadState,
		/// Bucket with specified id doesn't exist
		BucketDoesNotExist,
		/// DDC Cluster with provided id doesn't exist
		ClusterDoesNotExist,
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
			let min = <T as pallet::Config>::Currency::minimum_balance();
			if <T as pallet::Config>::Currency::free_balance(&account_id) < min {
				let _ = <T as pallet::Config>::Currency::make_free_balance_be(&account_id, min);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create new bucket with specified cluster id
		///
		/// Anyone can create a bucket
		#[pallet::weight(10_000)]
		pub fn create_bucket(origin: OriginFor<T>, cluster_id: ClusterId) -> DispatchResult {
			let bucket_owner = ensure_signed(origin)?;
			let cur_bucket_id = Self::buckets_count() + 1;

			<T as pallet::Config>::ClusterVisitor::ensure_cluster(&cluster_id)
				.map_err(|_| Error::<T>::ClusterDoesNotExist)?;

			let bucket = Bucket { bucket_id: cur_bucket_id, owner_id: bucket_owner, cluster_id };

			<BucketsCount<T>>::set(cur_bucket_id);
			<Buckets<T>>::insert(cur_bucket_id, bucket);

			Self::deposit_event(Event::<T>::BucketCreated(cur_bucket_id));

			Ok(())
		}

		/// Take the origin account as a owner and lock up `value` of its balance. `Owner` will
		/// be the account that controls it.
		///
		/// `value` must be more than the `minimum_balance` specified by `T::Currency`.
		///
		/// The dispatch origin for this call must be _Signed_ by the owner account.
		///
		/// Emits `Deposited`.
		#[pallet::weight(10_000)]
		pub fn deposit(
			origin: OriginFor<T>,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			if <Ledger<T>>::contains_key(&owner) {
				Err(Error::<T>::AlreadyPaired)?
			}

			// Reject a deposit which is considered to be _dust_.
			if value < <T as pallet::Config>::Currency::minimum_balance() {
				Err(Error::<T>::InsufficientDeposit)?
			}

			frame_system::Pallet::<T>::inc_consumers(&owner).map_err(|_| Error::<T>::BadState)?;

			let owner_balance = <T as pallet::Config>::Currency::free_balance(&owner);
			let value = value.min(owner_balance);
			let item = AccountsLedger {
				owner: owner.clone(),
				total: value,
				active: value,
				unlocking: Default::default(),
			};
			Self::update_ledger_and_deposit(&owner, &item)?;
			Self::deposit_event(Event::<T>::Deposited(owner, value));
			Ok(())
		}

		/// Add some extra amount that have appeared in the owner `free_balance` into the balance up
		/// for CDN payments.
		///
		/// The dispatch origin for this call must be _Signed_ by the owner.
		///
		/// Emits `Deposited`.
		#[pallet::weight(10_000)]
		pub fn deposit_extra(
			origin: OriginFor<T>,
			#[pallet::compact] max_additional: BalanceOf<T>,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			let mut ledger = Self::ledger(&owner).ok_or(Error::<T>::NotOwner)?;

			let owner_balance = <T as pallet::Config>::Currency::free_balance(&owner);
			let extra = owner_balance.min(max_additional);
			ledger.total += extra;
			ledger.active += extra;
			// Last check: the new active amount of ledger must be more than ED.
			ensure!(
				ledger.active >= <T as pallet::Config>::Currency::minimum_balance(),
				Error::<T>::InsufficientDeposit
			);

			Self::update_ledger_and_deposit(&owner, &ledger)?;

			Self::deposit_event(Event::<T>::Deposited(owner.clone(), extra));

			Ok(())
		}

		/// Schedule a portion of the owner deposited funds to be unlocked ready for transfer out
		/// after the lock period ends. If this leaves an amount actively locked less than
		/// T::Currency::minimum_balance(), then it is increased to the full amount.
		///
		/// The dispatch origin for this call must be _Signed_ by the owner.
		///
		/// Once the unlock period is done, you can call `withdraw_unlocked_deposit` to actually
		/// move the funds out of management ready for transfer.
		///
		/// No more than a limited number of unlocking chunks (see `MaxUnlockingChunks`)
		/// can co-exists at the same time. In that case, [`Call::withdraw_unlocked_deposit`] need
		/// to be called first to remove some of the chunks (if possible).
		///
		/// Emits `InitiatDepositUnlock`.
		///
		/// See also [`Call::withdraw_unlocked_deposit`].
		#[pallet::weight(10_000)]
		pub fn unlock_deposit(
			origin: OriginFor<T>,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&owner).ok_or(Error::<T>::NotOwner)?;
			ensure!(
				ledger.unlocking.len() < MaxUnlockingChunks::get() as usize,
				Error::<T>::NoMoreChunks,
			);

			let mut value = value.min(ledger.active);

			if !value.is_zero() {
				ledger.active -= value;

				// Avoid there being a dust balance left in the accounts system.
				if ledger.active < <T as pallet::Config>::Currency::minimum_balance() {
					value += ledger.active;
					ledger.active = Zero::zero();
				}

				let current_block = <frame_system::Pallet<T>>::block_number();
				// Note: locking for extra block to allow for accounting
				let block = current_block + <T as pallet::Config>::UnlockingDelay::get();
				log::debug!("Block for the unlock: {:?}", block);

				if let Some(chunk) =
					ledger.unlocking.last_mut().filter(|chunk| chunk.block == block)
				{
					// To keep the chunk count down, we only keep one chunk per era. Since
					// `unlocking` is a FiFo queue, if a chunk exists for `era` we know that it will
					// be the last one.
					chunk.value = chunk.value.defensive_saturating_add(value)
				} else {
					ledger
						.unlocking
						.try_push(UnlockChunk { value, block })
						.map_err(|_| Error::<T>::NoMoreChunks)?;
				};

				Self::update_ledger(&owner, &ledger);

				Self::deposit_event(Event::<T>::InitiatDepositUnlock(ledger.owner, value));
			}
			Ok(())
		}

		/// Remove any unlocked chunks from the `unlocking` queue from our management.
		///
		/// This essentially frees up that balance to be used by the owner account to do
		/// whatever it wants.
		///
		/// The dispatch origin for this call must be _Signed_ by the owner.
		///
		/// Emits `Withdrawn`.
		///
		/// See also [`Call::unlock_deposit`].
		#[pallet::weight(10_000)]
		pub fn withdraw_unlocked_deposit(origin: OriginFor<T>) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&owner).ok_or(Error::<T>::NotOwner)?;
			let (owner, old_total) = (ledger.owner.clone(), ledger.total);
			let current_block = <frame_system::Pallet<T>>::block_number();
			ledger = ledger.consolidate_unlocked(current_block);

			if ledger.unlocking.is_empty() &&
				ledger.active < <T as pallet::Config>::Currency::minimum_balance()
			{
				log::debug!("Killing owner");
				// This account must have called `unlock_deposit()` with some value that caused the
				// active portion to fall below existential deposit + will have no more unlocking
				// chunks left. We can now safely remove all accounts-related information.
				Self::kill_owner(&owner)?;
			} else {
				log::debug!("Updating ledger");
				// This was the consequence of a partial deposit unlock. just update the ledger and
				// move on.
				Self::update_ledger(&owner, &ledger);
			};

			log::debug!("Current total: {:?}", ledger.total);
			log::debug!("Old total: {:?}", old_total);

			// `old_total` should never be less than the new total because
			// `consolidate_unlocked` strictly subtracts balance.
			if ledger.total < old_total {
				log::debug!("Preparing for transfer");
				// Already checked that this won't overflow by entry condition.
				let value = old_total - ledger.total;

				let account_id = Self::account_id();

				<T as pallet::Config>::Currency::transfer(
					&account_id,
					&owner,
					value,
					ExistenceRequirement::KeepAlive,
				)?;
				Self::deposit_event(Event::<T>::Withdrawn(owner, value));
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
		/// Update the ledger for a owner.
		///
		/// This will also deposit the funds to pallet.
		fn update_ledger_and_deposit(
			owner: &T::AccountId,
			ledger: &AccountsLedger<T::AccountId, BalanceOf<T>, T>,
		) -> DispatchResult {
			let account_id = Self::account_id();

			<T as pallet::Config>::Currency::transfer(
				owner,
				&account_id,
				ledger.total,
				ExistenceRequirement::KeepAlive,
			)?;
			<Ledger<T>>::insert(owner, ledger);

			Ok(())
		}

		/// Update the ledger for a owner.
		fn update_ledger(
			owner: &T::AccountId,
			ledger: &AccountsLedger<T::AccountId, BalanceOf<T>, T>,
		) {
			<Ledger<T>>::insert(owner, ledger);
		}

		/// Remove all associated data of a owner account from the accounts system.
		///
		/// Assumes storage is upgraded before calling.
		///
		/// This is called:
		/// - after a `withdraw_unlocked_deposit()` call that frees all of a owner's locked balance.
		fn kill_owner(owner: &T::AccountId) -> DispatchResult {
			<Ledger<T>>::remove(&owner);

			frame_system::Pallet::<T>::dec_consumers(owner);

			Ok(())
		}

		// Charge payments from content owners
		pub fn charge_content_owners(
			paying_accounts: Vec<BucketsDetails<BalanceOf<T>>>,
			pricing: u128,
		) -> DispatchResult {
			let mut total_charged = BalanceOf::<T>::zero();

			for bucket_details in paying_accounts.iter() {
				let bucket: Bucket<T::AccountId> = Self::buckets(bucket_details.bucket_id)
					.ok_or(Error::<T>::BucketDoesNotExist)?;
				let content_owner = bucket.owner_id;
				let amount = bucket_details.amount * pricing.saturated_into::<BalanceOf<T>>();

				let mut ledger = Self::ledger(&content_owner).ok_or(Error::<T>::NotOwner)?;
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
			log::debug!("Total charged: {:?}", &total_charged);

			Self::deposit_event(Event::<T>::Charged(total_charged));
			log::debug!("Deposit event executed");

			Ok(())
		}
	}
}
