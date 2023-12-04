//! DdcStaking pallet benchmarking.

use ddc_primitives::CDNNodePubKey;
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;
use sp_std::prelude::*;
use testing_utils::*;

use super::*;
use crate::{cdn_node::CDNNodeProps, Pallet as DdcNodes};

const USER_SEED: u32 = 999666;

benchmarks! {
	create_node {
		let (user, node, cdn_node_params, _) = create_user_and_config::<T>("user", USER_SEED);

	  whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), node, cdn_node_params)
	verify {
		assert!(CDNNodes::<T>::contains_key(CDNNodePubKey::new([0; 32])));
	}

	delete_node {
		let (user, node, cdn_node_params, _) = create_user_and_config::<T>("user", USER_SEED);

	  DdcNodes::<T>::create_node(RawOrigin::Signed(user.clone()).into(),node.clone(), cdn_node_params)?;

		whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), node)
	verify {
	  assert!(!CDNNodes::<T>::contains_key(CDNNodePubKey::new([0; 32])));
	}

	set_node_params {
	  let (user, node, cdn_node_params, cdn_node_params_new) = create_user_and_config::<T>("user", USER_SEED);

	DdcNodes::<T>::create_node(RawOrigin::Signed(user.clone()).into(),node.clone(), cdn_node_params)?;

	whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), node, cdn_node_params_new)
	verify {
	  assert_eq!(CDNNodes::<T>::try_get(
	  CDNNodePubKey::new([0; 32])).unwrap().props,
		CDNNodeProps {
			host: vec![2u8, 255].try_into().unwrap(),
			http_port: 45000u16,
			grpc_port: 55000u16,
			p2p_port: 65000u16,
		});
	}

	impl_benchmark_test_suite!(
		DdcNodes,
		crate::mock::ExtBuilder.build(),
		crate::mock::Test,
	);
}
