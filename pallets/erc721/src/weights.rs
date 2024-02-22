#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_erc721.
pub trait WeightInfo {
	fn mint() -> Weight;
	fn transfer() -> Weight;
	fn burn() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn mint() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn transfer() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn burn() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn mint() -> Weight {
		Weight::from_parts(195_000_000u64, 0)

	}
	fn transfer() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn burn() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
}
