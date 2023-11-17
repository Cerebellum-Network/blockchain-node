//! DdcStaking pallet benchmarking.

use super::*;
use crate::Pallet as DdcCustomers;
use ddc_primitives::{BucketId, ClusterId};
// use testing_utils::*;

use sp_std::prelude::*;

pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;

const USER_SEED: u32 = 999666;

benchmarks! {
	create_bucket {
    let cluster_id = ClusterId::from([1; 20]);
    let user = account("user", USER_SEED, 0u32);

	  whitelist_account!(user);
	}: _(RawOrigin::Signed(user), cluster_id)
	verify {
		assert_eq!(DdcCustomers::<T>::buckets_count(), 1);
	}

	deposit {
    let user = account::<T::AccountId>("user", USER_SEED, 0u32);
    let balance = T::Currency::minimum_balance() * 100u32.into();
    let _ = T::Currency::make_free_balance_be(&user, balance);
    let amount = T::Currency::minimum_balance() * 50u32.into();

	  whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), amount)
	verify {
		assert!(Ledger::<T>::contains_key(user));
	}


	// set_node_params {
	//   let (user, node, cdn_node_params, cdn_node_params_new) = create_user_and_config::<T>("user", USER_SEED);

	// DdcNodes::<T>::create_node(RawOrigin::Signed(user.clone()).into(),node.clone(), cdn_node_params)?;

	// whitelist_account!(user);
	// }: _(RawOrigin::Signed(user.clone()), node, cdn_node_params_new)
	// verify {
	//   assert_eq!(CDNNodes::<T>::try_get(
	//   CDNNodePubKey::new([0; 32])).unwrap().props,
	// 	CDNNodeProps {
	// 		host: vec![2u8, 255].try_into().unwrap(),
	// 		http_port: 45000u16,
	// 		grpc_port: 55000u16,
	// 		p2p_port: 65000u16,
	// 	});
	// }

  impl_benchmark_test_suite!(
    DdcCustomers,
    crate::mock::ExtBuilder::default().build(),
    crate::mock::Test,
  );
}
