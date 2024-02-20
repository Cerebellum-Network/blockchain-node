#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

pub mod weights;
use crate::weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

use codec::{Decode, Encode};
use ddc_primitives::{
	traits::{
		cluster::{ClusterCreator, ClusterVisitor},
		customer::{CustomerCharger, CustomerDepositor},
	},
	BucketId, ClusterId,
};
use frame_support::{
	parameter_types,
	traits::{Currency, DefensiveSaturating, ExistenceRequirement},
	BoundedVec, Deserialize, PalletId, Serialize,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use scale_info::TypeInfo;
use sp_io::hashing::blake2_128;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, CheckedSub, Saturating, Zero},
	RuntimeDebug, SaturatedConversion,
};
use sp_std::prelude::*;

pub mod migration;

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
pub struct UnlockChunk<T: Config> {
	/// Amount of funds to be unlocked.
	#[codec(compact)]
	value: BalanceOf<T>,
	/// Block number at which point it'll be unlocked.
	#[codec(compact)]
	block: BlockNumberFor<T>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
#[scale_info(skip_type_params(T))]
pub struct Bucket<T: Config> {
	bucket_id: BucketId,
	owner_id: T::AccountId,
	cluster_id: ClusterId,
	is_public: bool,
	is_removed: bool,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct BucketParams {
	is_public: bool,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct AccountsLedger<T: Config> {
	/// The owner account whose balance is actually locked and can be used to pay for DDC network
	/// usage.
	pub owner: T::AccountId,
	/// The total amount of the owner's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	#[codec(compact)]
	pub total: BalanceOf<T>,
	/// The total amount of the owner's balance that will be accessible for DDC network payouts in
	/// any forthcoming rounds.
	#[codec(compact)]
	pub active: BalanceOf<T>,
	/// Any balance that is becoming free, which may eventually be transferred out of the owner
	/// (assuming that the content owner has to pay for network usage). It is assumed that this
	/// will be treated as a first in, first out queue where the new (higher value) eras get pushed
	/// on the back.
	pub unlocking: BoundedVec<UnlockChunk<T>, MaxUnlockingChunks>,
}

impl<T: Config> AccountsLedger<T> {
	/// Initializes the default object using the given owner.
	pub fn default_from(owner: T::AccountId) -> Self {
		Self { owner, total: Zero::zero(), active: Zero::zero(), unlocking: Default::default() }
	}

	/// Remove entries from `unlocking` that are sufficiently old and reduce the
	/// total by the sum of their balances.
	fn consolidate_unlocked(self, current_block: BlockNumberFor<T>) -> Self {
		let mut total = self.total;
		let unlocking_result: Result<BoundedVec<_, _>, _> = self
			.unlocking
			.into_iter()
			.filter(|chunk| {
				if chunk.block > current_block {
					true
				} else {
					total = total.saturating_sub(chunk.value);
					false
				}
			})
			.collect::<Vec<_>>()
			.try_into();

		if let Ok(unlocking) = unlocking_result {
			Self { owner: self.owner, total, active: self.active, unlocking }
		} else {
			panic!("Failed to filter unlocking");
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, traits::LockableCurrency};
	use frame_system::pallet_prelude::*;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
		frame_support::traits::StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The accounts's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type Currency: LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Number of eras that staked funds must remain locked for.
		#[pallet::constant]
		type UnlockingDelay: Get<BlockNumberFor<Self>>;
		type ClusterVisitor: ClusterVisitor<Self>;
		type ClusterCreator: ClusterCreator<Self, BalanceOf<Self>>;
		type WeightInfo: WeightInfo;
	}

	/// Map from all (unlocked) "owner" accounts to the info regarding the staking.
	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	pub type Ledger<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, AccountsLedger<T>>;

	#[pallet::type_value]
	pub fn DefaultBucketCount<T: Config>() -> BucketId {
		0
	}

	#[pallet::storage]
	#[pallet::getter(fn buckets_count)]
	pub type BucketsCount<T: Config> =
		StorageValue<Value = BucketId, QueryKind = ValueQuery, OnEmpty = DefaultBucketCount<T>>;

	/// Map from bucket ID to the bucket structure
	#[pallet::storage]
	#[pallet::getter(fn buckets)]
	pub type Buckets<T: Config> = StorageMap<_, Twox64Concat, BucketId, Bucket<T>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		// todo! name events
		/// An account has deposited this amount. \[owner, amount\]
		///
		/// NOTE: This event is only emitted when funds are deposited via a dispatchable. Notably,
		/// it will not be emitted for staking rewards when they are added to stake.
		Deposited { owner_id: T::AccountId, amount: BalanceOf<T> },
		/// An account has initiated unlock for amount. \[owner, amount\]
		InitialDepositUnlock { owner_id: T::AccountId, amount: BalanceOf<T> },
		/// An account has called `withdraw_unlocked_deposit` and removed unlocking chunks worth
		/// `Balance` from the unlocking queue. \[owner, amount\]
		Withdrawn { owner_id: T::AccountId, amount: BalanceOf<T> },
		/// The account has been charged for the usage
		Charged { owner_id: T::AccountId, charged: BalanceOf<T>, expected_to_charge: BalanceOf<T> },
		/// Bucket with specific id created
		BucketCreated { bucket_id: BucketId },
		/// Bucket with specific id updated
		BucketUpdated { bucket_id: BucketId },
		/// Bucket with specific id marked as removed
		BucketRemoved { bucket_id: BucketId },
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
		// unauthorised operation
		Unauthorised,
		// Arithmetic overflow
		ArithmeticOverflow,
		// Arithmetic underflow
		ArithmeticUnderflow,
		// Transferring balance to pallet's vault has failed
		TransferFailed,
		/// Bucket is already removed
		AlreadyRemoved,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub feeder_account: Option<T::AccountId>,
		pub buckets: Vec<(Bucket<T>, BalanceOf<T>)>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { feeder_account: None, buckets: Default::default() }
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

			for (bucket, deposit) in &self.buckets {
				let cur_bucket_id = <BucketsCount<T>>::get()
					.checked_add(1)
					.ok_or(Error::<T>::ArithmeticOverflow)
					.unwrap();
				<BucketsCount<T>>::set(cur_bucket_id);

				<Buckets<T>>::insert(cur_bucket_id, bucket);

				let ledger = AccountsLedger::<T> {
					owner: bucket.owner_id.clone(),
					total: *deposit,
					active: *deposit,
					unlocking: Default::default(),
				};
				<Ledger<T>>::insert(&ledger.owner, &ledger);

				<T as pallet::Config>::Currency::deposit_into_existing(&account_id, *deposit)
					.unwrap();
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create new bucket with specified cluster id
		///
		/// Anyone can create a bucket
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_bucket())]
		pub fn create_bucket(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			bucket_params: BucketParams,
		) -> DispatchResult {
			let bucket_owner = ensure_signed(origin)?;
			let cur_bucket_id =
				Self::buckets_count().checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;

			<T as pallet::Config>::ClusterVisitor::ensure_cluster(&cluster_id)
				.map_err(|_| Error::<T>::ClusterDoesNotExist)?;

			let bucket = Bucket {
				bucket_id: cur_bucket_id,
				owner_id: bucket_owner,
				cluster_id,
				is_public: bucket_params.is_public,
				is_removed: false,
			};

			<BucketsCount<T>>::set(cur_bucket_id);
			<Buckets<T>>::insert(cur_bucket_id, bucket);

			Self::deposit_event(Event::<T>::BucketCreated { bucket_id: cur_bucket_id });

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
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::deposit())]
		pub fn deposit(
			origin: OriginFor<T>,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			<Self as CustomerDepositor<T>>::deposit(owner, value.saturated_into())?;
			Ok(())
		}

		/// Add some extra amount that have appeared in the owner `free_balance` into the balance up
		/// for DDC network payouts.
		///
		/// The dispatch origin for this call must be _Signed_ by the owner.
		///
		/// Emits `Deposited`.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::deposit_extra())]
		pub fn deposit_extra(
			origin: OriginFor<T>,
			#[pallet::compact] max_additional: BalanceOf<T>,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			<Self as CustomerDepositor<T>>::deposit_extra(owner, max_additional.saturated_into())?;
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
		/// Emits `InitialDepositUnlock`.
		///
		/// See also [`Call::withdraw_unlocked_deposit`].
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::unlock_deposit())]
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
				ledger.active =
					ledger.active.checked_sub(&value).ok_or(Error::<T>::ArithmeticUnderflow)?;

				// Avoid there being a dust balance left in the accounts system.
				if ledger.active < <T as pallet::Config>::Currency::minimum_balance() {
					value =
						value.checked_add(&ledger.active).ok_or(Error::<T>::ArithmeticOverflow)?;
					ledger.active = Zero::zero();
				}

				let current_block = <frame_system::Pallet<T>>::block_number();
				// Note: locking for extra block to allow for accounting
				// block + configurable value - shouldn't overflow
				let block = current_block + <T as pallet::Config>::UnlockingDelay::get();

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

				<Ledger<T>>::insert(&owner, &ledger);

				Self::deposit_event(Event::<T>::InitialDepositUnlock {
					owner_id: ledger.owner,
					amount: value,
				});
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
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::withdraw_unlocked_deposit_kill())]
		pub fn withdraw_unlocked_deposit(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let owner = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&owner).ok_or(Error::<T>::NotOwner)?;
			let (owner, old_total) = (ledger.owner.clone(), ledger.total);
			let current_block = <frame_system::Pallet<T>>::block_number();
			ledger = ledger.consolidate_unlocked(current_block);

			let post_info_weight = if ledger.unlocking.is_empty() &&
				ledger.active < <T as pallet::Config>::Currency::minimum_balance()
			{
				log::debug!("Killing owner");
				// This account must have called `unlock_deposit()` with some value that caused the
				// active portion to fall below existential deposit + will have no more unlocking
				// chunks left. We can now safely remove all accounts-related information.
				Self::kill_owner(&owner)?;
				// This is worst case scenario, so we use the full weight and return None
				None
			} else {
				log::debug!("Updating ledger");
				// This was the consequence of a partial deposit unlock. just update the ledger and
				// move on.
				<Ledger<T>>::insert(&owner, &ledger);
				// This is only an update, so we use less overall weight.
				Some(<T as pallet::Config>::WeightInfo::withdraw_unlocked_deposit_update())
			};

			log::debug!("Current total: {:?}", ledger.total);
			log::debug!("Old total: {:?}", old_total);

			// `old_total` should never be less than the new total because
			// `consolidate_unlocked` strictly subtracts balance.
			if ledger.total < old_total {
				log::debug!("Preparing for transfer");
				// Already checked that this won't overflow by entry condition.
				let value =
					old_total.checked_sub(&ledger.total).ok_or(Error::<T>::ArithmeticUnderflow)?;

				<T as pallet::Config>::Currency::transfer(
					&Self::account_id(),
					&owner,
					value,
					ExistenceRequirement::AllowDeath,
				)?;
				Self::deposit_event(Event::<T>::Withdrawn { owner_id: owner, amount: value });
			}

			Ok(post_info_weight.into())
		}

		/// Sets bucket parameters.
		///
		/// The dispatch origin for this call must be _Signed_ by the bucket owner.
		///
		/// Emits `BucketUpdated`.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::set_bucket_params())]
		pub fn set_bucket_params(
			origin: OriginFor<T>,
			bucket_id: BucketId,
			bucket_params: BucketParams,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let mut bucket = Self::buckets(bucket_id).ok_or(Error::<T>::NoBucketWithId)?;
			ensure!(bucket.owner_id == owner, Error::<T>::NotBucketOwner);

			bucket.is_public = bucket_params.is_public;
			<Buckets<T>>::insert(bucket_id, bucket);
			Self::deposit_event(Event::<T>::BucketUpdated { bucket_id });

			Ok(())
		}

		/// Mark existing bucket with specified bucket id as removed
		///
		/// Only an owner can remove a bucket
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::remove_bucket())]
		pub fn remove_bucket(origin: OriginFor<T>, bucket_id: BucketId) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			<Buckets<T>>::try_mutate(bucket_id, |maybe_bucket| -> DispatchResult {
				let bucket = maybe_bucket.as_mut().ok_or(Error::<T>::NoBucketWithId)?;
				ensure!(bucket.owner_id == owner, Error::<T>::NotBucketOwner);
				ensure!(!bucket.is_removed, Error::<T>::AlreadyRemoved);

				// Mark the bucket as removed
				bucket.is_removed = true;

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::BucketRemoved { bucket_id });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		pub fn sub_account_id(account_id: &T::AccountId) -> T::AccountId {
			let hash = blake2_128(&account_id.encode());

			// hash is 28 bytes
			T::PalletId::get().into_sub_account_truncating(hash)
		}

		/// Update the ledger for a owner.
		///
		/// This will also deposit the funds to pallet.
		fn update_ledger_and_deposit(
			owner: &T::AccountId,
			ledger: &AccountsLedger<T>,
		) -> DispatchResult {
			<T as pallet::Config>::Currency::transfer(
				owner,
				&Self::account_id(),
				ledger.total,
				ExistenceRequirement::AllowDeath,
			)?;
			<Ledger<T>>::insert(owner, ledger);

			Ok(())
		}

		/// Remove all associated data of a owner account from the accounts system.
		///
		/// Assumes storage is upgraded before calling.
		///
		/// This is called:
		/// - after a `withdraw_unlocked_deposit()` call that frees all of a owner's locked balance.
		fn kill_owner(owner: &T::AccountId) -> DispatchResult {
			<Ledger<T>>::remove(owner);

			frame_system::Pallet::<T>::dec_consumers(owner);

			Ok(())
		}

		/// Charge funds that were scheduled for unlocking.
		///
		/// Returns the updated ledger, and the amount actually charged.
		fn charge_unlocking(
			mut ledger: AccountsLedger<T>,
			value: BalanceOf<T>,
		) -> Result<(AccountsLedger<T>, BalanceOf<T>), Error<T>> {
			let mut unlocking_balance = BalanceOf::<T>::zero();

			while let Some(last) = ledger.unlocking.last_mut() {
				let temp = unlocking_balance
					.checked_add(&last.value)
					.ok_or(Error::<T>::ArithmeticOverflow)?;
				if temp <= value {
					unlocking_balance = temp;
					ledger.unlocking.pop();
				} else {
					let diff = value
						.checked_sub(&unlocking_balance)
						.ok_or(Error::<T>::ArithmeticUnderflow)?;

					unlocking_balance = unlocking_balance
						.checked_add(&diff)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					last.value =
						last.value.checked_sub(&diff).ok_or(Error::<T>::ArithmeticUnderflow)?;
				}

				if unlocking_balance >= value {
					break
				}
			}

			Ok((ledger, unlocking_balance))
		}
	}

	impl<T: Config> CustomerCharger<T> for Pallet<T> {
		fn charge_content_owner(
			content_owner: T::AccountId,
			billing_vault: T::AccountId,
			amount: u128,
		) -> Result<u128, DispatchError> {
			let actually_charged: BalanceOf<T>;
			let mut ledger = Self::ledger(&content_owner).ok_or(Error::<T>::NotOwner)?;
			let amount_to_deduct = amount.saturated_into::<BalanceOf<T>>();

			if ledger.active >= amount_to_deduct {
				actually_charged = amount_to_deduct;
				ledger.active = ledger
					.active
					.checked_sub(&amount_to_deduct)
					.ok_or(Error::<T>::ArithmeticUnderflow)?;
				ledger.total = ledger
					.total
					.checked_sub(&amount_to_deduct)
					.ok_or(Error::<T>::ArithmeticUnderflow)?;
			} else {
				let diff = amount_to_deduct
					.checked_sub(&ledger.active)
					.ok_or(Error::<T>::ArithmeticUnderflow)?;

				actually_charged = ledger.active;
				ledger.total = ledger
					.total
					.checked_sub(&ledger.active)
					.ok_or(Error::<T>::ArithmeticUnderflow)?;
				ledger.active = BalanceOf::<T>::zero();

				let (_ledger, charged) = Self::charge_unlocking(ledger, diff)?;
				ledger = _ledger;

				actually_charged.checked_add(&charged).ok_or(Error::<T>::ArithmeticUnderflow)?;
			}

			<T as pallet::Config>::Currency::transfer(
				&Self::account_id(),
				&billing_vault,
				actually_charged,
				ExistenceRequirement::AllowDeath,
			)?;

			<Ledger<T>>::insert(&content_owner, &ledger); // update state after successful transfer
			Self::deposit_event(Event::<T>::Charged {
				owner_id: content_owner,
				charged: actually_charged,
				expected_to_charge: amount_to_deduct,
			});

			Ok(actually_charged.saturated_into::<u128>())
		}
	}

	impl<T: Config> CustomerDepositor<T> for Pallet<T> {
		fn deposit(owner: T::AccountId, amount: u128) -> Result<(), DispatchError> {
			let value = amount.saturated_into::<BalanceOf<T>>();

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

			Self::update_ledger_and_deposit(&owner, &item)
				.map_err(|_| Error::<T>::TransferFailed)?;
			Self::deposit_event(Event::<T>::Deposited { owner_id: owner, amount: value });

			Ok(())
		}

		fn deposit_extra(owner: T::AccountId, amount: u128) -> Result<(), DispatchError> {
			let max_additional = amount.saturated_into::<BalanceOf<T>>();
			let mut ledger = Self::ledger(&owner).ok_or(Error::<T>::NotOwner)?;

			let owner_balance = <T as pallet::Config>::Currency::free_balance(&owner);
			let extra = owner_balance.min(max_additional);
			ledger.total =
				ledger.total.checked_add(&extra).ok_or(Error::<T>::ArithmeticOverflow)?;
			ledger.active =
				ledger.active.checked_add(&extra).ok_or(Error::<T>::ArithmeticOverflow)?;

			// Last check: the new active amount of ledger must be more than ED.
			ensure!(
				ledger.active >= <T as pallet::Config>::Currency::minimum_balance(),
				Error::<T>::InsufficientDeposit
			);

			Self::update_ledger_and_deposit(&owner, &ledger)
				.map_err(|_| Error::<T>::TransferFailed)?;
			Self::deposit_event(Event::<T>::Deposited { owner_id: owner, amount: extra });

			Ok(())
		}
	}
}
