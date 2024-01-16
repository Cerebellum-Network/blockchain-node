//! # DDC Nodes Pallet
//!
//! The DDC Clusters pallet is used to manage DDC Clusters
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## GenesisConfig
//!
//! The DDC Clusters pallet depends on the [`GenesisConfig`]. The
//! `GenesisConfig` is optional and allow to set some initial nodes in DDC.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]
#![feature(is_some_and)] // ToDo: delete at rustc > 1.70

pub mod weights;
use ddc_traits::pallet::{GetDdcOrigin, PalletsOriginOf};
use frame_support::{
	pallet_prelude::*,
	traits::{EnsureOriginWithArg, LockableCurrency, OriginTrait, UnfilteredDispatchable},
};
use frame_system::pallet_prelude::*;
pub use frame_system::Config as SysConfig;
pub use pallet::*;
use sp_runtime::traits::AccountIdConversion;
use sp_std::prelude::*;

use crate::weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::PalletId;

	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type PalletId: Get<PalletId>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		type WeightInfo: WeightInfo;
		type SubmitOrigin: EnsureOriginWithArg<
			Self::RuntimeOrigin,
			PalletsOriginOf<Self>,
			Success = Self::AccountId,
		>;
		type ClusterGovCreatorOrigin: GetDdcOrigin<Self>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		GenericEvent,
	}

	#[pallet::error]
	pub enum Error<T> {
		GenericError,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn submit_public(
			origin: OriginFor<T>,
			proposal_origin: Box<PalletsOriginOf<T>>,
		) -> DispatchResult {
			let _caller_id = T::SubmitOrigin::ensure_origin(origin, &proposal_origin)?;
			Self::deposit_event(Event::<T>::GenericEvent);
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn submit_via_internal(
			origin: OriginFor<T>,
			proposal_origin: Box<PalletsOriginOf<T>>,
		) -> DispatchResult {
			let _caller_id = ensure_signed(origin)?;
			let call = Call::<T>::submit_public { proposal_origin };
			call.dispatch_bypass_filter(frame_system::RawOrigin::Signed(Self::account_id()).into())
				.map(|_| ())
				.map_err(|e| e.error)?;

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn submit_internal(origin: OriginFor<T>) -> DispatchResult {
			let _caller_id = ensure_signed(origin)?;

			// let origin: T::RuntimeOrigin = frame_system::RawOrigin::Signed(_caller_id).into();
			// let pallets_origin: <T::RuntimeOrigin as
			// frame_support::traits::OriginTrait>::PalletsOrigin = origin.caller().clone();
			// let call = Call::<T>::submit_public { proposal_origin: Box::new(pallets_origin) };
			// call.dispatch_bypass_filter(frame_system::RawOrigin::Signed(Self::account_id()).
			// into()) 	.map(|_| ())
			// 	.map_err(|e| e.error)?;

			let origin2 = T::ClusterGovCreatorOrigin::get();
			let pallets_origin2: <T::RuntimeOrigin as
			frame_support::traits::OriginTrait>::PalletsOrigin = origin2.caller().clone();
			let call2 = Call::<T>::submit_public { proposal_origin: Box::new(pallets_origin2) };
			call2
				.dispatch_bypass_filter(frame_system::RawOrigin::Signed(Self::account_id()).into())
				// .dispatch_bypass_filter(
				// 	frame_system::RawOrigin::Signed(Self::sub_account_id(5u32)).into(),
				// )
				.map(|_| ())
				.map_err(|e| e.error)?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}
}
