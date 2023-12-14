//! Tests for the module.

use ddc_primitives::{NodePubKey, StorageNodeParams};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::AccountId32;

use super::{mock::*, *};

#[test]
fn create_storage_node_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let bytes = [0u8; 32];
		let node_pub_key = AccountId32::from(bytes);
		let storage_node_params = StorageNodeParams {
			host: vec![1u8, 255],
			http_port: 35000u16,
			grpc_port: 25000u16,
			p2p_port: 15000u16,
		};

		// Host length exceeds limit
		assert_noop!(
			DdcNodes::create_node(
				RuntimeOrigin::signed(1),
				NodePubKey::StoragePubKey(node_pub_key.clone()),
				NodeParams::StorageParams(StorageNodeParams {
					host: vec![1u8; 256],
					http_port: 35000u16,
					grpc_port: 25000u16,
					p2p_port: 15000u16,
				})
			),
			Error::<Test>::HostLenExceedsLimit
		);

		// Node created
		assert_ok!(DdcNodes::create_node(
			RuntimeOrigin::signed(1),
			NodePubKey::StoragePubKey(node_pub_key.clone()),
			NodeParams::StorageParams(storage_node_params.clone())
		));

		// Check storage
		assert!(StorageNodes::<Test>::contains_key(node_pub_key.clone()));
		assert!(DdcNodes::exists(&NodePubKey::StoragePubKey(node_pub_key.clone())));
		if let Ok(cluster_id) =
			DdcNodes::get_cluster_id(&NodePubKey::StoragePubKey(node_pub_key.clone()))
		{
			assert_eq!(cluster_id, None);
		}
		let storage_node = DdcNodes::storage_nodes(&node_pub_key).unwrap();
		assert_eq!(storage_node.pub_key, node_pub_key);

		// Node already exists
		assert_noop!(
			DdcNodes::create_node(
				RuntimeOrigin::signed(1),
				NodePubKey::StoragePubKey(node_pub_key.clone()),
				NodeParams::StorageParams(storage_node_params)
			),
			Error::<Test>::NodeAlreadyExists
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(
			Event::NodeCreated { node_pub_key: NodePubKey::StoragePubKey(node_pub_key) }.into(),
		)
	})
}

#[test]
fn create_storage_node_with_node_creator() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let bytes = [0u8; 32];
		let node_pub_key = AccountId32::from(bytes);
		let storage_node_params = StorageNodeParams {
			host: vec![1u8, 255],
			http_port: 35000u16,
			grpc_port: 25000u16,
			p2p_port: 15000u16,
		};

		// Node created
		assert_ok!(<DdcNodes as NodeCreator<Test>>::create_node(
			NodePubKey::StoragePubKey(node_pub_key.clone()),
			1u64,
			NodeParams::StorageParams(storage_node_params)
		));

		// Check storage
		assert!(StorageNodes::<Test>::contains_key(node_pub_key.clone()));
		assert!(DdcNodes::exists(&NodePubKey::StoragePubKey(node_pub_key.clone())));
		if let Ok(cluster_id) =
			DdcNodes::get_cluster_id(&NodePubKey::StoragePubKey(node_pub_key.clone()))
		{
			assert_eq!(cluster_id, None);
		}
		let storage_node = DdcNodes::storage_nodes(&node_pub_key).unwrap();
		assert_eq!(storage_node.pub_key, node_pub_key);
	})
}

#[test]
fn set_storage_node_params_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let bytes = [0u8; 32];
		let node_pub_key = AccountId32::from(bytes);
		let storage_node_params = StorageNodeParams {
			host: vec![1u8, 255],
			http_port: 35000u16,
			grpc_port: 25000u16,
			p2p_port: 15000u16,
		};

		// Node doesn't exist
		assert_noop!(
			DdcNodes::set_node_params(
				RuntimeOrigin::signed(1),
				NodePubKey::StoragePubKey(node_pub_key.clone()),
				NodeParams::StorageParams(storage_node_params.clone())
			),
			Error::<Test>::NodeDoesNotExist
		);

		// Node created
		assert_ok!(DdcNodes::create_node(
			RuntimeOrigin::signed(1),
			NodePubKey::StoragePubKey(node_pub_key.clone()),
			NodeParams::StorageParams(storage_node_params.clone())
		));

		// Set node params
		assert_ok!(DdcNodes::set_node_params(
			RuntimeOrigin::signed(1),
			NodePubKey::StoragePubKey(node_pub_key.clone()),
			NodeParams::StorageParams(storage_node_params.clone())
		));

		// Only node provider can set params
		assert_noop!(
			DdcNodes::set_node_params(
				RuntimeOrigin::signed(2),
				NodePubKey::StoragePubKey(node_pub_key.clone()),
				NodeParams::StorageParams(storage_node_params.clone())
			),
			Error::<Test>::OnlyNodeProvider
		);

		let bytes_2 = [1u8; 32];
		let node_pub_key_2 = AccountId32::from(bytes_2);
		let node = Node::<Test>::new(
			NodePubKey::StoragePubKey(node_pub_key_2),
			2u64,
			NodeParams::StorageParams(storage_node_params),
		)
		.unwrap();

		// Update should fail if node doesn't exist
		assert_noop!(
			<DdcNodes as NodeRepository<Test>>::update(node),
			NodeRepositoryError::StorageNodeDoesNotExist
		);

		// Storage host length exceeds limit
		assert_noop!(
			DdcNodes::set_node_params(
				RuntimeOrigin::signed(1),
				NodePubKey::StoragePubKey(node_pub_key.clone()),
				NodeParams::StorageParams(StorageNodeParams {
					host: vec![1u8; 256],
					http_port: 35000u16,
					grpc_port: 25000u16,
					p2p_port: 15000u16,
				})
			),
			Error::<Test>::HostLenExceedsLimit
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(
			Event::NodeParamsChanged { node_pub_key: NodePubKey::StoragePubKey(node_pub_key) }
				.into(),
		)
	})
}

#[test]
fn delete_storage_node_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let bytes = [0u8; 32];
		let node_pub_key = AccountId32::from(bytes);
		let storage_node_params = StorageNodeParams {
			host: vec![1u8, 255],
			http_port: 35000u16,
			grpc_port: 25000u16,
			p2p_port: 15000u16,
		};

		// Node doesn't exist
		assert_noop!(
			DdcNodes::delete_node(
				RuntimeOrigin::signed(1),
				NodePubKey::StoragePubKey(node_pub_key.clone())
			),
			Error::<Test>::NodeDoesNotExist
		);

		// Create node
		assert_ok!(DdcNodes::create_node(
			RuntimeOrigin::signed(1),
			NodePubKey::StoragePubKey(node_pub_key.clone()),
			NodeParams::StorageParams(storage_node_params)
		));

		// Only node provider can delete
		assert_noop!(
			DdcNodes::delete_node(
				RuntimeOrigin::signed(2),
				NodePubKey::StoragePubKey(node_pub_key.clone())
			),
			Error::<Test>::OnlyNodeProvider
		);

		// Delete node
		assert_ok!(DdcNodes::delete_node(
			RuntimeOrigin::signed(1),
			NodePubKey::StoragePubKey(node_pub_key.clone()),
		));

		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(
			Event::NodeDeleted { node_pub_key: NodePubKey::StoragePubKey(node_pub_key) }.into(),
		)
	})
}
