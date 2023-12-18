//! DdcStaking pallet benchmarking.

use ddc_primitives::{StorageNodeMode, StorageNodePubKey};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;
use sp_std::prelude::*;
use testing_utils::*;

use super::*;
use crate::{storage_node::StorageNodeProps, Pallet as DdcNodes};

const USER_SEED: u32 = 999666;

benchmarks! {
	create_node {
		let (user, node, storage_node_params, _) = create_user_and_config::<T>("user", USER_SEED);

		whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), node, storage_node_params)
	verify {
		assert!(StorageNodes::<T>::contains_key(StorageNodePubKey::new([0; 32])));
	}

	delete_node {
		let (user, node, storage_node_params, _) = create_user_and_config::<T>("user", USER_SEED);

		DdcNodes::<T>::create_node(RawOrigin::Signed(user.clone()).into(),node.clone(), storage_node_params)?;

		whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), node)
	verify {
		assert!(!StorageNodes::<T>::contains_key(StorageNodePubKey::new([0; 32])));
	}

	set_node_params {
		let (user, node, storage_node_params, new_storage_node_params) = create_user_and_config::<T>("user", USER_SEED);

		DdcNodes::<T>::create_node(RawOrigin::Signed(user.clone()).into(),node.clone(), storage_node_params)?;

		whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), node, new_storage_node_params)
	verify {
		assert_eq!(StorageNodes::<T>::try_get(
			StorageNodePubKey::new([0; 32])).unwrap().props,
			StorageNodeProps {
				mode: StorageNodeMode::Storage,
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
