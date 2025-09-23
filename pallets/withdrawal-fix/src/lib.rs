//! # Withdrawal Fix Pallet
//!
//! This pallet is responsible for handling withdrawal fixes in a Substrate-based blockchain.
//! It provides functionality to fix withdrawal issues and manage withdrawal states.
//!
//! ## Overview
//!
//! - Allows governance to fix withdrawal issues.
//! - Manages withdrawal states and transitions.
//! - Emits events for key actions such as withdrawal fixes and state changes.
//!
//! ## Dispatchable Functions
//!
//! - `fix_withdrawal`: Allows governance to fix a withdrawal issue.
//! - `update_withdrawal_state`: Allows governance to update withdrawal state.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{
	traits::{
		fungible::{Inspect, Mutate},
	},
};
use codec::{Encode, Decode, DecodeWithMemTracking, MaxEncodedLen};
use scale_info::TypeInfo;
pub use pallet::*;

#[allow(deprecated)]
#[allow(clippy::let_unit_value)]
#[allow(clippy::manual_inspect)]
#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use crate::weights::WeightInfo;

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
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: crate::weights::WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Withdrawal fix applied successfully.
		WithdrawalFixed { account: T::AccountId, amount: u128 },
		/// Withdrawal state updated.
		WithdrawalStateUpdated { account: T::AccountId, state: WithdrawalState },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Withdrawal not found.
		WithdrawalNotFound,
		/// Invalid withdrawal state transition.
		InvalidStateTransition,
		/// Insufficient balance for withdrawal fix.
		InsufficientBalance,
		/// Withdrawal already processed.
		WithdrawalAlreadyProcessed,
	}

	#[pallet::storage]
	#[pallet::getter(fn withdrawal_states)]
	pub type WithdrawalStates<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		WithdrawalState,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn pending_withdrawals)]
	pub type PendingWithdrawals<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		u128,
		OptionQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Fix a withdrawal issue for a specific account.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::fix_withdrawal())]
		pub fn fix_withdrawal(
			origin: OriginFor<T>,
			account: T::AccountId,
			amount: u128,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Check if withdrawal exists
			ensure!(
				PendingWithdrawals::<T>::contains_key(&account),
				Error::<T>::WithdrawalNotFound
			);

			// Check if withdrawal is already processed
			let current_state = WithdrawalStates::<T>::get(&account);
			ensure!(
				current_state != Some(WithdrawalState::Processed),
				Error::<T>::WithdrawalAlreadyProcessed
			);

			// Update withdrawal state to processed
			WithdrawalStates::<T>::insert(&account, WithdrawalState::Processed);

			// Remove from pending withdrawals
			PendingWithdrawals::<T>::remove(&account);

			Self::deposit_event(Event::WithdrawalFixed { account, amount });
			Ok(())
		}

		/// Update withdrawal state for a specific account.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::update_withdrawal_state())]
		pub fn update_withdrawal_state(
			origin: OriginFor<T>,
			account: T::AccountId,
			state: WithdrawalState,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Validate state transition
			let current_state = WithdrawalStates::<T>::get(&account);
			Self::validate_state_transition(current_state, &state)?;

			// Update state
			WithdrawalStates::<T>::insert(&account, state.clone());

			Self::deposit_event(Event::WithdrawalStateUpdated { account, state });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Validate state transition
		fn validate_state_transition(
			current_state: Option<WithdrawalState>,
			new_state: &WithdrawalState,
		) -> Result<(), Error<T>> {
			match (current_state, new_state) {
				(None, WithdrawalState::Pending) => Ok(()),
				(Some(WithdrawalState::Pending), WithdrawalState::Processing) => Ok(()),
				(Some(WithdrawalState::Processing), WithdrawalState::Processed) => Ok(()),
				(Some(WithdrawalState::Processing), WithdrawalState::Failed) => Ok(()),
				(Some(WithdrawalState::Failed), WithdrawalState::Pending) => Ok(()),
				_ => Err(Error::<T>::InvalidStateTransition),
			}
		}
	}
}

/// Withdrawal state enumeration
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub enum WithdrawalState {
	/// Withdrawal is pending
	Pending,
	/// Withdrawal is being processed
	Processing,
	/// Withdrawal has been processed successfully
	Processed,
	/// Withdrawal failed
	Failed,
}
