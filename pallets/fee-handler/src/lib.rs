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

// todo! Add Unit tests and Benchmarking

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
	__private::RuntimeDebug,
	pallet_prelude::TypeInfo,
	sp_runtime::SaturatedConversion,
	traits::{
		fungible::Mutate,
		tokens::{Fortitude, Precision, Preservation},
	},
};
pub use pallet::*;
use sp_runtime::{Permill, Saturating};

#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct FeeDistributionProportion {
	treasury_proportion: Permill,
	fee_pot_proportion: Permill,
}

impl FeeDistributionProportion {
	/// Creates a new `FeeDistributionProportion` if the total proportions equal 100%.
	pub fn new(treasury_proportion: u32, fee_pot_proportion: u32) -> Option<Self> {
		let treasury_proportion = Permill::from_percent(treasury_proportion);
		let fee_pot_proportion = Permill::from_percent(fee_pot_proportion);
		let total = treasury_proportion.saturating_add(fee_pot_proportion);
		if total == Permill::one() {
			Some(Self { treasury_proportion, fee_pot_proportion })
		} else {
			None
		}
	}
}

pub trait FeeHandler<T: Config> {
	/// Handles the distribution of fees to the treasury and fee pot accounts.
	fn handle_fee(source: T::AccountId, fee_amount: u128) -> sp_runtime::DispatchResult;
}

// todo! Fixed clippy warnings
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
		type Currency: Mutate<Self::AccountId>;
		/// Governance origin for privileged calls.
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Pallet ID for the fee pot account.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Pallet ID for the treasury account.
		#[pallet::constant]
		type TreasuryPalletId: Get<PalletId>;
	}

	#[pallet::storage]
	pub type FeeDistributionProportionConfig<T> = StorageValue<_, FeeDistributionProportion>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Manual top-up of the fee pot account.
		ManualFeeAccountTopUp { source: T::AccountId, amount: u128 },
		/// Fee distribution configuration updated.
		FeeDistributionProportionConfigSet { config: FeeDistributionProportion },
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
		// todo! Add actual weights
		#[pallet::weight(10_000)]
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

		/// Allows governance to set the fee distribution proportions.
		#[pallet::call_index(1)]
		// todo! Add actual weights
		#[pallet::weight(10_000)]
		pub fn fee_distribution_config(
			origin: OriginFor<T>,
			treasury_fee_proportion: u32,
			fee_pot_proportion: u32,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			let fee_distribution_config =
				FeeDistributionProportion::new(treasury_fee_proportion, fee_pot_proportion)
					.ok_or(Error::<T>::ArithmeticOverflow)?;
			<FeeDistributionProportionConfig<T>>::put(fee_distribution_config.clone());
			Self::deposit_event(Event::FeeDistributionProportionConfigSet {
				config: fee_distribution_config,
			});
			Ok(())
		}

		/// Burn Native tokens of an account
		///
		/// # Parameters
		///
		/// * `who`: AccountId
		/// * `amount`: Amount of native tokens to burn.
		#[pallet::call_index(2)]
		// todo! Add actual weights
		#[pallet::weight(10_000)]
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

	impl<T: Config> FeeHandler<T> for Pallet<T> {
		fn handle_fee(source: T::AccountId, fee_amount: u128) -> DispatchResult {
			let fee_config: FeeDistributionProportion = <FeeDistributionProportionConfig<T>>::get()
				.ok_or(Error::<T>::FeeDistributionConfigNotSet)?;
			let fee_pot_amount = fee_config.fee_pot_proportion.mul_floor(fee_amount);
			let treasury_amount = fee_amount.saturating_sub(fee_pot_amount);

			let fee_pot_account = Pallet::<T>::fee_pot_account_id();
			let treasury_account = Pallet::<T>::treasury_account_id();

			T::Currency::transfer(
				&source,
				&fee_pot_account,
				fee_pot_amount.saturated_into(),
				Preservation::Preserve,
			)?;
			T::Currency::transfer(
				&source,
				&treasury_account,
				treasury_amount.saturated_into(),
				Preservation::Preserve,
			)?;
			Ok(())
		}
	}
}
