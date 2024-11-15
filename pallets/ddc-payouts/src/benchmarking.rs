//! DdcPayouts pallet benchmarking.

pub use frame_benchmarking::{account, benchmarks, whitelist_account};
use frame_system::RawOrigin;
use sp_std::prelude::*;

use super::*;
use crate::Pallet as DdcPayouts;

benchmarks! {
	dummy_call {
		let user = account::<T::AccountId>("name", 0, 0);
		whitelist_account!(user);
	}: _(RawOrigin::Signed(user), 100u128)
	verify {
	}
}
