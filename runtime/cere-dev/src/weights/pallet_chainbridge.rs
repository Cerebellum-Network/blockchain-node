#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;


pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_chainbridge::WeightInfo for WeightInfo<T> {
	fn set_threshold() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn set_resource() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn remove_resource() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn whitelist_chain() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn add_relayer() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn remove_relayer() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn acknowledge_proposal() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn reject_proposal() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
	fn eval_vote_state() -> Weight {
		Weight::from_parts(195_000_000u64, 0)
	}
}
