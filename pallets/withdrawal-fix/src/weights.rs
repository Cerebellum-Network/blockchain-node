//! Weights for the withdrawal-fix pallet

#![cfg_attr(feature = "std", allow(unused_imports))]

use frame_support::{traits::Get, weights::Weight};

/// Weight functions needed for pallet_withdrawal_fix.
pub trait WeightInfo {
	/// Weight for `fix_withdrawal`.
	fn fix_withdrawal() -> Weight;
	/// Weight for `update_withdrawal_state`.
	fn update_withdrawal_state() -> Weight;
}

/// Weights for pallet_withdrawal_fix using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(sp_std::marker::PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn fix_withdrawal() -> Weight {
		Weight::from_parts(50_000, 0)
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}

	fn update_withdrawal_state() -> Weight {
		Weight::from_parts(30_000, 0)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
