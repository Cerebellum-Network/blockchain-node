#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_erc20::WeightInfo for WeightInfo<T> {
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
