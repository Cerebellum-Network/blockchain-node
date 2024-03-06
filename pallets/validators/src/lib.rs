//! # DDC Nodes Pallet
//!
//! The Validators pallet is a temporary pallet used to recover from the incident in Mainnet
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use frame_support::pallet_prelude::*;
pub use pallet::*;
use pallet_session::SessionManager;
use sp_std::prelude::*;
pub mod weights;

use crate::weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::dispatch::DispatchResult;
	use frame_system::{ensure_root, pallet_prelude::*};

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
		frame_support::traits::StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		ValidatorAdded { validator: T::AccountId },
		ValidatorRemoved { validator: T::AccountId },
		ValidatorsSet { validators: Vec<T::AccountId> },
		AllValidatorsCleared,
	}

	#[pallet::error]
	pub enum Error<T> {
		AlreadyExists,
		DoNotExists,
		NoValidatorsPassed,
	}

	#[pallet::storage]
	#[pallet::getter(fn validators)]
	pub(crate) type Validators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::add_one())]
		pub fn add_one(origin: OriginFor<T>, validator_to_add: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

			Validators::<T>::try_mutate(|validators| -> DispatchResult {
				ensure!(!validators.contains(&validator_to_add), Error::<T>::AlreadyExists);
				validators.push(validator_to_add.clone());
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::ValidatorAdded { validator: validator_to_add });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::remove_one())]
		pub fn remove_one(
			origin: OriginFor<T>,
			validator_to_remove: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			let mut validators = Validators::<T>::get();
			ensure!(validators.contains(&validator_to_remove), Error::<T>::DoNotExists);

			Validators::<T>::try_mutate(|validators| -> DispatchResult {
				let index = validators.iter().position(|v| *v == validator_to_remove).ok_or(Error::<T>::DoNotExists)?;
				validators.remove(index);
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::ValidatorRemoved { validator: validator_to_remove });

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::set_list())]
		pub fn set_list(origin: OriginFor<T>, validators: Vec<T::AccountId>) -> DispatchResult {
			ensure_root(origin)?;
			Validators::<T>::set(validators.clone());
			Self::deposit_event(Event::<T>::ValidatorsSet { validators });
			Ok(())
		}
	}

	impl<T: Config> SessionManager<T::AccountId> for Pallet<T> {
		fn new_session(_new_index: sp_staking::SessionIndex) -> Option<Vec<T::AccountId>> {
			let validators: Vec<T::AccountId> = Validators::<T>::get();
			if validators.is_empty() {
				None
			} else {
				Some(validators)
			}
		}

		fn end_session(_end_index: sp_staking::SessionIndex) {
			// Do nothing
		}

		fn start_session(_start_index: sp_staking::SessionIndex) {
			// Do nothing
		}
	}
}
