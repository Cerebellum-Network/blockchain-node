#![cfg_attr(not(feature = "std"), no_std)]

pub use frame_support::pallet_prelude::*;
pub use frame_system::pallet_prelude::*;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::storage]
	#[pallet::getter(fn global_era_counter)]
	pub type GlobalEraCounter<T: Config> = StorageValue<_, u32>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100_000)]
		pub fn inc_era(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(era) = <GlobalEraCounter<T>>::get() {
				let new_era = era.checked_add(1).unwrap_or_default();
				<GlobalEraCounter<T>>::put(new_era);
			} else {
				<GlobalEraCounter<T>>::put(1);
			}
			Ok(())
		}
	}
}
