//! Weights for `pallet_ismp` migration operations.
//!
//! Placeholder values copied from the polytope-labs gargantua benchmark run;
//! must be regenerated against the master benchmark machine before this
//! migration ships.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use polkadot_sdk::frame_support::{traits::Get, weights::Weight};
use polkadot_sdk::sp_std::marker::PhantomData;

/// Weight functions for `pallet_ismp` migrations.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: polkadot_sdk::frame_system::Config> pallet_ismp::weights::MigrationWeightInfo for WeightInfo<T> {
	fn drain_state_commitments_step(n: u32) -> Weight {
		Weight::from_parts(5_731_000, 0)
			.saturating_add(Weight::from_parts(0, 1240))
			.saturating_add(Weight::from_parts(1_079_117, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2546).saturating_mul(n.into()))
	}

	fn drain_state_machine_update_time_step(n: u32) -> Weight {
		Weight::from_parts(5_811_000, 0)
			.saturating_add(Weight::from_parts(0, 1253))
			.saturating_add(Weight::from_parts(1_150_373, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2513).saturating_mul(n.into()))
	}

	fn drain_child_trie_state_commitments_step(n: u32) -> Weight {
		Weight::from_parts(8_496_000, 0)
			.saturating_add(Weight::from_parts(0, 19224))
			.saturating_add(Weight::from_parts(519_700_988, 0).saturating_mul(n.into()))
			.saturating_add(Weight::from_parts(0, 77).saturating_mul(n.into()))
	}
}
