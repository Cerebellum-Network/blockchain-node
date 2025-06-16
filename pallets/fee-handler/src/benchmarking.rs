//! Fee Handler pallet benchmarking.

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;

use super::*;

const SEED: u32 = 0;

benchmarks! {
	manual_topup {
		let amount = T::Currency::minimum_balance() * 100u32.into();
		let user = frame_benchmarking::account::<T::AccountId>("user", 0, SEED);
		let fee_pot_account = Pallet::<T>::fee_pot_account_id();
		T::Currency::set_balance(&user, amount * 2u32.into());
		assert_eq!(T::Currency::balance(&user), amount * 2u32.into());
	}: _(RawOrigin::Signed(user.clone()), amount.saturated_into())
	verify {
		assert_eq!(T::Currency::balance(&user), amount);
		assert_eq!(T::Currency::balance(&fee_pot_account), amount);
	}

	burn_native_tokens {
		let amount = T::Currency::minimum_balance() * 100u32.into();
		let user = frame_benchmarking::account::<T::AccountId>("user", 0, SEED);
		T::Currency::set_balance(&user, amount * 2u32.into());
		assert_eq!(T::Currency::balance(&user), amount * 2u32.into());
	}: _(RawOrigin::Root, user.clone(), amount.saturated_into())
	verify {
		assert_eq!(T::Currency::balance(&user), amount);
	}
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
