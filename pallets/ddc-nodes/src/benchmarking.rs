//! DdcNodes pallet benchmarking.

use ddc_primitives::{StorageNodeMode, StorageNodePubKey};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_std::prelude::*;
use testing_utils::*;

use super::*;
use crate::{storage_node::StorageNodeProps, Pallet as DdcNodes};

const USER_SEED: u32 = 999666;

#[benchmarks]
mod benchmarks {

	use super::*;

	#[benchmark]
	fn create_node() {
		let (user, node, storage_node_params, _) = create_user_and_config::<T>("user", USER_SEED);
		whitelist_account!(user);

		#[extrinsic_call]
		create_node::<T>(RawOrigin::Signed(user.clone()), node, storage_node_params);

		assert!(StorageNodes::<T>::contains_key(StorageNodePubKey::new([0; 32])));
	}

	#[benchmark]
	fn delete_node() {
		let (user, node, storage_node_params, _) = create_user_and_config::<T>("user", USER_SEED);

		DdcNodes::<T>::create_node(
			RawOrigin::Signed(user.clone()).into(),
			node.clone(),
			storage_node_params,
		)
		.expect("create_node failed");

		whitelist_account!(user);

		#[extrinsic_call]
		delete_node::<T>(RawOrigin::Signed(user.clone()), node);

		assert!(!StorageNodes::<T>::contains_key(StorageNodePubKey::new([0; 32])));
	}

	#[benchmark]
	fn set_node_params() {
		let (user, node, storage_node_params, new_storage_node_params) =
			create_user_and_config::<T>("user", USER_SEED);

		DdcNodes::<T>::create_node(
			RawOrigin::Signed(user.clone()).into(),
			node.clone(),
			storage_node_params,
		)
		.expect("create_node failed");

		whitelist_account!(user);

		#[extrinsic_call]
		set_node_params::<T>(RawOrigin::Signed(user.clone()), node, new_storage_node_params);

		assert_eq!(
			StorageNodes::<T>::try_get(StorageNodePubKey::new([0; 32])).unwrap().props,
			StorageNodeProps {
				mode: StorageNodeMode::Storage,
				host: vec![3u8; 255].try_into().unwrap(),
				domain: vec![4u8; 255].try_into().unwrap(),
				ssl: true,
				http_port: 45000u16,
				grpc_port: 55000u16,
				p2p_port: 65000u16,
			}
		);
	}

	#[benchmark]
	fn migration_v2_nodes_step() -> Result<(), BenchmarkError> {
		use crate::migrations::{
			v1::StorageNodes as V1StorageNodes, v2::StorageNodes as V2StorageNodes,
			v2_mbm::LazyMigrationV1ToV2,
		};

		let setup = LazyMigrationV1ToV2::<T>::setup_benchmark_env_for_migration();
		assert_eq!(V1StorageNodes::<T>::iter().count(), 1);

		#[block]
		{
			LazyMigrationV1ToV2::<T>::nodes_step(None);
		}

		assert_eq!(V2StorageNodes::<T>::iter().count(), 1);
		let node = V2StorageNodes::<T>::get(&setup.node_key);
		assert!(node.is_some());

		Ok(())
	}
}
