//! # Pool Withdrawal Fix Pallet
//!
//! A pallet for handling pool withdrawal operations through delegation pallet connector.
//! This pallet provides withdrawal functionality that can be called by root or governance.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::Weight;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, EnsureOrigin, LockableCurrency},
	};
	use frame_system::pallet_prelude::*;
	use sp_staking::OnStakingUpdate;

	use crate::WeightInfo;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
		type Currency: LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>;
		/// Integrate Delegated Pallet
		type DelegationPalletConnector: OnStakingUpdate<Self::AccountId, BalanceOf<Self>>;
		/// Governance origin for withdrawal calls
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	/// Pallets use events to inform users when important changes are made.
	/// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Withdrawal was called for a stash account. [stash_account, amount]
		WithdrawCalled { stash_account: T::AccountId, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Withdrawal failed due to insufficient balance or other staking constraints.
		WithdrawalFailed,
	}

	/// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	/// These functions materialize as "extrinsics", which are often compared to transactions.
	/// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Call withdrawal for a stash account through the delegation pallet connector.
		/// This function can be called by root or governance.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::call_withdraw())]
		pub fn call_withdraw(
			origin: OriginFor<T>,
			stash_account: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Call the delegation pallet connector to handle withdrawal
			T::DelegationPalletConnector::on_withdraw(&stash_account, amount);

			// Emit an event.
			Self::deposit_event(Event::WithdrawCalled { stash_account, amount });

			Ok(())
		}
	}
}

/// Weight functions needed for pallet_pool_withdrawal_fix.
pub trait WeightInfo {
	fn call_withdraw() -> Weight;
}

/// Default weight implementation for the pallet.
impl WeightInfo for () {
	fn call_withdraw() -> Weight {
		Weight::from_parts(10_000, 0)
	}
}
