#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_erc20.
pub trait WeightInfo {
    fn add_validators() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn add_validators() -> Weight {
        Weight::from_parts(195_000_000u64, 0)
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn add_validators() -> Weight {
        Weight::from_parts(195_000_000u64, 0)
    }
}
