#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_erc20.
pub trait WeightInfo {
    fn add_one() -> Weight;
    fn remove_one() -> Weight;
    fn clear_all() -> Weight;
}

// For the sake of time, the values are to arbitrary
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn add_one() -> Weight {
        Weight::from_parts(200_000_000u64, 0)
    }
    fn remove_one() -> Weight {
        Weight::from_parts(200_000_000u64, 0)
    }
    fn clear_all() -> Weight {
        Weight::from_parts(200_000_000u64, 0)
    }
}

// For the sake of time, the values are to arbitrary
// For backwards compatibility and tests
impl WeightInfo for () {
    fn add_one() -> Weight {
        Weight::from_parts(200_000_000u64, 0)
    }
    fn remove_one() -> Weight {
        Weight::from_parts(200_000_000u64, 0)
    }
    fn clear_all() -> Weight {
        Weight::from_parts(200_000_000u64, 0)
    }
}
