//TODO: @jaxter write benchmarks and generate weights
//! Weight functions for pallet_dac_registry
//!
//! Simple default weight implementations for the DAC Registry pallet.

use sp_weights::Weight;

/// Weight functions needed for pallet_dac_registry.
pub trait WeightInfo {
	fn register_code(code_len: u32) -> Weight;
	fn update_meta() -> Weight;
	fn deregister_code() -> Weight;
}

/// Default weight implementation for the pallet.
impl WeightInfo for () {
	fn register_code(code_len: u32) -> Weight {
		Weight::from_parts(10_000 + code_len as u64 * 100, 0)
	}

	fn update_meta() -> Weight {
		Weight::from_parts(10_000, 0)
	}

	fn deregister_code() -> Weight {
		Weight::from_parts(10_000, 0)
	}
}

/// Weight functions for `pallet_dac_registry`.
pub struct SubstrateWeight<T>(sp_std::marker::PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn register_code(code_len: u32) -> Weight {
		Weight::from_parts(10_000 + code_len as u64 * 100, 0)
	}

	fn update_meta() -> Weight {
		Weight::from_parts(10_000, 0)
	}

	fn deregister_code() -> Weight {
		Weight::from_parts(10_000, 0)
	}
}
