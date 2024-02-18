#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_erc20.
pub trait WeightInfo {
	fn transfer_hash() -> Weight;
	fn transfer_native() -> Weight;
	fn transfer_erc721() -> Weight;
	fn transfer() -> Weight;
	fn remark() -> Weight;
	fn mint_erc721() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn transfer_hash() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn transfer_native() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn transfer_erc721() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn transfer() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn remark() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn mint_erc721() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn transfer_hash() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn transfer_native() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn transfer_erc721() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn transfer() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn remark() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn mint_erc721() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
}
