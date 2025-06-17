//! # Fee Handler Pallet
//!
//! This pallet is responsible for handling fee distribution in a Substrate-based blockchain.
//! It provides functionality to configure fee distribution proportions and transfer fees to
//! designated accounts (e.g., treasury and fee pot accounts).
//!
//! ## Overview
//!
//! - Allows governance to configure fee distribution proportions.
//! - Handles fee transfers to the treasury and fee pot accounts based on the configured
//!   proportions.
//! - Emits events for key actions such as configuration updates and fee transfers.
//!
//! ## Dispatchable Functions
//!
//! - `manual_topup`: Allows a user to manually top up the fee pot account.
//! - `fee_distribution_config`: Allows governance to set the fee distribution proportions.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod weights;

#[cfg(test)]
mod mock;

use frame_support::{
	sp_runtime::SaturatedConversion,
	traits::{
		fungible::{Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
	},
};
use ddc_primitives::traits::FeeHandler;
pub use pallet::*;
use weights::WeightInfo;

#[allow(deprecated)]
#[allow(clippy::let_unit_value)]
#[allow(clippy::manual_inspect)]
#[frame_support::pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::AccountIdConversion;

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Native Currency Support.
		type Currency: Mutate<Self::AccountId> + Inspect<Self::AccountId>;
		/// Governance origin for privileged calls.
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Pallet ID for the fee pot account.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Pallet ID for the treasury account.
		#[pallet::constant]
		type TreasuryPalletId: Get<PalletId>;
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Manual top-up of the fee pot account.
		ManualFeeAccountTopUp { source: T::AccountId, amount: u128 },
		/// Native Token Burn event
		NativeTokenBurned(T::AccountId, u128),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Fee distribution configuration is not set.
		FeeDistributionConfigNotSet,
		/// Arithmetic overflow occurred.
		ArithmeticOverflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Allows a user to manually top up the fee pot account.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::manual_topup())]
		pub fn manual_topup(origin: OriginFor<T>, amount: u128) -> DispatchResult {
			let who = ensure_signed(origin)?;
			T::Currency::transfer(
				&who,
				&Self::fee_pot_account_id(),
				amount.saturated_into(),
				Preservation::Preserve,
			)?;
			Self::deposit_event(Event::ManualFeeAccountTopUp { source: who, amount });
			Ok(())
		}

		/// Burn Native tokens of an account
		///
		/// # Parameters
		///
		/// * `who`: AccountId
		/// * `amount`: Amount of native tokens to burn.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::burn_native_tokens())]
		pub fn burn_native_tokens(
			origin: OriginFor<T>,
			who: T::AccountId,
			amount: u128,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			let burned_amt = <T as Config>::Currency::burn_from(
				&who,
				amount.saturated_into(),
				Preservation::Preserve,
				Precision::BestEffort,
				Fortitude::Force,
			)?;
			Self::deposit_event(Event::<T>::NativeTokenBurned(who, burned_amt.saturated_into()));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns the account ID of the fee pot.
		pub fn fee_pot_account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		/// Returns the account ID of the treasury.
		pub fn treasury_account_id() -> T::AccountId {
			T::TreasuryPalletId::get().into_account_truncating()
		}
	}

	impl<T: Config> FeeHandler<T::AccountId> for Pallet<T> {
		fn handle_fee(source: T::AccountId, fee_amount: u128) -> DispatchResult {
			let fee_pot_account = Pallet::<T>::fee_pot_account_id();
			T::Currency::transfer(
				&source,
				&fee_pot_account,
				fee_amount.saturated_into(),
				Preservation::Expendable,
			)?;
			Ok(())
		}
	}
}
