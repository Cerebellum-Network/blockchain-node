#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]
#![allow(clippy::manual_inspect)]
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod weights;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;
use codec::{Decode, Encode};
use ddc_primitives::{
	traits::{
		bucket::BucketManager,
		cluster::{ClusterCreator, ClusterProtocol, ClusterQuery},
		customer::CustomerVisitor,
	},
	BucketId, BucketParams, ClusterId,
};
pub use pallet::*;
use polkadot_sdk::frame_support::{
	parameter_types,
	traits::{fungible::Inspect, Currency, ExistenceRequirement, UnfilteredDispatchable},
	BoundedVec, Deserialize, PalletId, Serialize,
};
use polkadot_sdk::frame_system::pallet_prelude::*;
use polkadot_sdk::sp_io::hashing::blake2_128;
use polkadot_sdk::sp_runtime::{
	traits::{AccountIdConversion, StaticLookup, Zero},
	RuntimeDebug, SaturatedConversion,
};
use polkadot_sdk::sp_std::fmt::Debug;
use polkadot_sdk::sp_std::prelude::*;
use scale_info::TypeInfo;

use crate::weights::WeightInfo;

pub mod migrations;

/// The balance type of this pallet.
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<
	<T as polkadot_sdk::frame_system::Config>::AccountId,
>>::Balance;

/// The balance type of contracts pallet.
pub type ContractsBalanceOf<T> =
	<<T as polkadot_sdk::pallet_contracts::Config>::Currency as Inspect<
		<T as polkadot_sdk::frame_system::Config>::AccountId,
	>>::Balance;

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
#[scale_info(skip_type_params(T))]
pub struct CustomerLedger<T: Config> {
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

impl<T: Config> CustomerLedger<T> {
	/// Initializes the default object using the given owner.
	pub fn default_from(owner: T::AccountId) -> Self {
		Self { owner, total: Zero::zero(), active: Zero::zero(), unlocking: Default::default() }
	}
}

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use polkadot_sdk::frame_support::{pallet_prelude::*, traits::LockableCurrency};
	use polkadot_sdk::frame_system::pallet_prelude::*;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: polkadot_sdk::frame_support::traits::StorageVersion =
		polkadot_sdk::frame_support::traits::StorageVersion::new(5);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config + polkadot_sdk::pallet_contracts::Config
	{
		/// The accounts's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type Currency: LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>;
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		type ClusterProtocol: ClusterProtocol<
			Self::AccountId,
			BlockNumberFor<Self>,
			BalanceOf<Self>,
		>;
		type ClusterCreator: ClusterCreator<Self::AccountId, BlockNumberFor<Self>, BalanceOf<Self>>;
		type WeightInfo: WeightInfo;
		type ContractMigrator: ContractMigrator<Self::AccountId, ContractsBalanceOf<Self>>;
	}

	#[pallet::storage]
	pub type ClusterLedger<T: Config> =
		StorageDoubleMap<_, Blake2_256, ClusterId, Blake2_256, T::AccountId, CustomerLedger<T>>;

	#[pallet::type_value]
	pub fn DefaultBucketCount<T: Config>() -> BucketId {
		0
	}

	#[pallet::storage]
	pub type BucketsCount<T: Config> =
		StorageValue<Value = BucketId, QueryKind = ValueQuery, OnEmpty = DefaultBucketCount<T>>;

	/// Map from bucket ID to the bucket structure
	#[pallet::storage]
	pub type Buckets<T: Config> = StorageMap<_, Twox64Concat, BucketId, Bucket<T>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Bucket with specific id created
		#[codec(index = 4)]
		BucketCreated { cluster_id: ClusterId, bucket_id: BucketId },
		/// Bucket with specific id updated
		#[codec(index = 5)]
		BucketUpdated { cluster_id: ClusterId, bucket_id: BucketId },
		/// Bucket with specific id marked as removed
		#[codec(index = 6)]
		BucketRemoved { cluster_id: ClusterId, bucket_id: BucketId },
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
		/// Not enough balance to deposit
		NotEnoughBalance,
		/// Bucket with speicifed id doesn't exist.
		NoBucketWithId,
		/// Internal state has become somehow corrupted and the operation cannot continue.
		BadState,
		/// Bucket with specified id doesn't exist
		BucketDoesNotExist,
		/// DDC Cluster with provided id doesn't exist
		ClusterDoesNotExist,
		// unauthorized operation
		Unauthorized,
		// Arithmetic overflow
		ArithmeticOverflow,
		// Arithmetic underflow
		ArithmeticUnderflow,
		// Transferring balance to pallet's vault has failed
		TransferFailed,
		/// Bucket is already removed
		AlreadyRemoved,
		/// Bucket belongs to another cluster
		ClusterMismatch,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub buckets: Vec<Bucket<T>>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { buckets: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for bucket in &self.buckets {
				let cur_bucket_id = <BucketsCount<T>>::get()
					.checked_add(1)
					.ok_or(Error::<T>::ArithmeticOverflow)
					.unwrap();
				<BucketsCount<T>>::set(cur_bucket_id);

				<Buckets<T>>::insert(cur_bucket_id, bucket);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create new bucket with specified cluster id
		///
		/// Anyone can create a bucket
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_bucket())]
		pub fn create_bucket(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			bucket_params: BucketParams,
		) -> DispatchResult {
			let bucket_owner = ensure_signed(origin)?;
			let bucket_id =
				BucketsCount::<T>::get().checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;

			Self::do_create_bucket(cluster_id, bucket_id, bucket_owner, bucket_params)?;

			Ok(())
		}

		/// Sets bucket parameters.
		///
		/// The dispatch origin for this call must be _Signed_ by the bucket owner.
		///
		/// Emits `BucketUpdated`.
		#[pallet::call_index(6)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_bucket_params())]
		pub fn set_bucket_params(
			origin: OriginFor<T>,
			bucket_id: BucketId,
			bucket_params: BucketParams,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let mut bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::NoBucketWithId)?;
			ensure!(bucket.owner_id == owner, Error::<T>::NotBucketOwner);

			bucket.is_public = bucket_params.is_public;
			let cluster_id = bucket.cluster_id;
			<Buckets<T>>::insert(bucket_id, bucket);
			Self::deposit_event(Event::<T>::BucketUpdated { cluster_id, bucket_id });

			Ok(())
		}

		/// Mark existing bucket with specified bucket id as removed
		///
		/// Only an owner can remove a bucket
		#[pallet::call_index(7)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::remove_bucket())]
		pub fn remove_bucket(origin: OriginFor<T>, bucket_id: BucketId) -> DispatchResult {
			// todo! can we set total_usage to None and save bytes
			let owner = ensure_signed(origin)?;
			let mut cluster_id = ClusterId::default();

			<Buckets<T>>::try_mutate(bucket_id, |maybe_bucket| -> DispatchResult {
				let bucket = maybe_bucket.as_mut().ok_or(Error::<T>::NoBucketWithId)?;
				ensure!(bucket.owner_id == owner, Error::<T>::NotBucketOwner);
				ensure!(!bucket.is_removed, Error::<T>::AlreadyRemoved);
				cluster_id = bucket.cluster_id;
				// Mark the bucket as removed
				bucket.is_removed = true;

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::BucketRemoved { cluster_id, bucket_id });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn pallet_account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		pub fn cluster_vault_id(cluster_id: &ClusterId) -> T::AccountId {
			let hash = blake2_128(&cluster_id.encode());

			// hash is 28 bytes
			T::PalletId::get().into_sub_account_truncating(hash)
		}

		fn do_create_bucket(
			cluster_id: ClusterId,
			bucket_id: BucketId,
			owner_id: T::AccountId,
			bucket_params: BucketParams,
		) -> DispatchResult {
			ensure!(
				<T::ClusterProtocol as ClusterQuery<T::AccountId>>::cluster_exists(&cluster_id),
				Error::<T>::ClusterDoesNotExist
			);

			let bucket = Bucket {
				bucket_id,
				owner_id,
				cluster_id,
				is_public: bucket_params.is_public,
				is_removed: false,
			};

			<BucketsCount<T>>::set(bucket_id);
			<Buckets<T>>::insert(bucket_id, bucket);

			Self::deposit_event(Event::<T>::BucketCreated { cluster_id, bucket_id });

			Ok(())
		}
	}

	impl<T: Config> BucketManager<T> for Pallet<T> {
		fn get_bucket_owner_id(bucket_id: BucketId) -> Result<T::AccountId, DispatchError> {
			let bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::BucketDoesNotExist)?;
			Ok(bucket.owner_id)
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn create_bucket(
			cluster_id: &ClusterId,
			bucket_id: BucketId,
			owner_id: T::AccountId,
			bucket_params: BucketParams,
		) -> Result<(), DispatchError> {
			Self::do_create_bucket(*cluster_id, bucket_id, owner_id, bucket_params)?;
			Ok(())
		}
	}

	impl<T: Config> CustomerVisitor<T> for Pallet<T> {
		fn get_bucket_owner(bucket_id: &BucketId) -> Result<T::AccountId, DispatchError> {
			let bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::NoBucketWithId)?;
			Ok(bucket.owner_id)
		}
	}

	pub trait ContractMigrator<AccountId, Balance> {
		fn deploy_contract(
			deployer: AccountId,
			value: Balance,
			gas_limit: Weight,
			storage_deposit_limit: Option<Balance>,
			code: Vec<u8>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> DispatchResult;

		fn call_contract(
			caller: AccountId,
			dest: AccountId,
			value: Balance,
			gas_limit: Weight,
			storage_deposit_limit: Option<Balance>,
			data: Vec<u8>,
		) -> DispatchResult;
	}

	impl<T: Config> ContractMigrator<T::AccountId, ContractsBalanceOf<T>> for Pallet<T>
	where
		ContractsBalanceOf<T>: codec::HasCompact,
		<ContractsBalanceOf<T> as codec::HasCompact>::Type:
			Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
		<<T as polkadot_sdk::frame_system::Config>::Lookup as StaticLookup>::Source:
			From<<T as polkadot_sdk::frame_system::Config>::AccountId>,
	{
		fn deploy_contract(
			deployer: T::AccountId,
			value: ContractsBalanceOf<T>,
			gas_limit: Weight,
			storage_deposit_limit: Option<ContractsBalanceOf<T>>,
			code: Vec<u8>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> DispatchResult {
			let instantiate_call: polkadot_sdk::pallet_contracts::Call<T> =
				polkadot_sdk::pallet_contracts::Call::instantiate_with_code {
					value,
					gas_limit,
					storage_deposit_limit: storage_deposit_limit.map(Into::into),
					code,
					data,
					salt,
				};

			let result = instantiate_call.dispatch_bypass_filter(
				polkadot_sdk::frame_system::RawOrigin::Signed(deployer).into(),
			);

			match result {
				Ok(_) => Ok(()),
				Err(e) => {
					log::error!("❌ Error deploying contract: {:?}", e.error);
					Err(e.error)
				},
			}
		}

		fn call_contract(
			caller: T::AccountId,
			dest: T::AccountId,
			value: ContractsBalanceOf<T>,
			gas_limit: Weight,
			storage_deposit_limit: Option<ContractsBalanceOf<T>>,
			data: Vec<u8>,
		) -> DispatchResult {
			let call_call: polkadot_sdk::pallet_contracts::Call<T> =
				polkadot_sdk::pallet_contracts::Call::call {
					dest: dest.into(),
					value,
					gas_limit,
					storage_deposit_limit: storage_deposit_limit.map(Into::into),
					data,
				};

			let result = call_call.dispatch_bypass_filter(
				polkadot_sdk::frame_system::RawOrigin::Signed(caller).into(),
			);

			match result {
				Ok(_) => Ok(()),
				Err(e) => {
					log::error!("❌ Error calling contract: {:?}", e.error);
					Err(e.error)
				},
			}
		}
	}
}
