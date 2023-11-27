//! Tests for the module.

use ddc_primitives::{CDNNodeParams, NodePubKey, StorageNodeParams};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::AccountId32;

use super::{mock::*, *};

#[test]
fn create_node_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let bytes = [0u8; 32];
		let node_pub_key = AccountId32::from(bytes);
		let cdn_node_params = CDNNodeParams {
			host: vec![1u8, 255],
			http_port: 35000u16,
			grpc_port: 25000u16,
			p2p_port: 15000u16,
		};

		// Node params are not valid
		assert_noop!(
			DdcNodes::create_node(
				RuntimeOrigin::signed(1),
				NodePubKey::CDNPubKey(node_pub_key.clone()),
				NodeParams::StorageParams(StorageNodeParams {
					host: vec![1u8, 255],
					http_port: 35000u16,
					grpc_port: 25000u16,
					p2p_port: 15000u16,
				})
			),
			Error::<Test>::InvalidNodeParams
		);

		// Node created
		assert_ok!(DdcNodes::create_node(
			RuntimeOrigin::signed(1),
			NodePubKey::CDNPubKey(node_pub_key.clone()),
			NodeParams::CDNParams(cdn_node_params.clone())
		));

		// Node already exists
		assert_noop!(
			DdcNodes::create_node(
				RuntimeOrigin::signed(1),
				NodePubKey::CDNPubKey(node_pub_key.clone()),
				NodeParams::CDNParams(cdn_node_params)
			),
			Error::<Test>::NodeAlreadyExists
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(
			Event::NodeCreated { node_pub_key: NodePubKey::CDNPubKey(node_pub_key) }.into(),
		)
	})
}

#[test]
fn set_node_params_works() {
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
		let cdn_node_params = CDNNodeParams {
			host: vec![1u8, 255],
			http_port: 35000u16,
			grpc_port: 25000u16,
			p2p_port: 15000u16,
		};

		// Node doesn't exist
		assert_noop!(
			DdcNodes::set_node_params(
				RuntimeOrigin::signed(1),
				NodePubKey::CDNPubKey(node_pub_key.clone()),
				NodeParams::CDNParams(cdn_node_params.clone())
			),
			Error::<Test>::NodeDoesNotExist
		);

		// Node created
		assert_ok!(DdcNodes::create_node(
			RuntimeOrigin::signed(1),
			NodePubKey::CDNPubKey(node_pub_key.clone()),
			NodeParams::CDNParams(cdn_node_params.clone())
		));

		// Set node params
		assert_ok!(DdcNodes::set_node_params(
			RuntimeOrigin::signed(1),
			NodePubKey::CDNPubKey(node_pub_key.clone()),
			NodeParams::CDNParams(cdn_node_params.clone())
		));

		// Node params are not valid
		assert_noop!(
			DdcNodes::set_node_params(
				RuntimeOrigin::signed(1),
				NodePubKey::CDNPubKey(node_pub_key.clone()),
				NodeParams::StorageParams(storage_node_params)
			),
			Error::<Test>::InvalidNodeParams
		);

		// Only node provider can set params
		assert_noop!(
			DdcNodes::set_node_params(
				RuntimeOrigin::signed(2),
				NodePubKey::CDNPubKey(node_pub_key.clone()),
				NodeParams::CDNParams(cdn_node_params)
			),
			Error::<Test>::OnlyNodeProvider
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(
			Event::NodeParamsChanged { node_pub_key: NodePubKey::CDNPubKey(node_pub_key) }.into(),
		)
	})
}

#[test]
fn set_delete_node_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let bytes = [0u8; 32];
		let node_pub_key = AccountId32::from(bytes);
		let cdn_node_params = CDNNodeParams {
			host: vec![1u8, 255],
			http_port: 35000u16,
			grpc_port: 25000u16,
			p2p_port: 15000u16,
		};

		// Node doesn't exist
		assert_noop!(
			DdcNodes::delete_node(
				RuntimeOrigin::signed(1),
				NodePubKey::CDNPubKey(node_pub_key.clone())
			),
			Error::<Test>::NodeDoesNotExist
		);

		// Create node
		assert_ok!(DdcNodes::create_node(
			RuntimeOrigin::signed(1),
			NodePubKey::CDNPubKey(node_pub_key.clone()),
			NodeParams::CDNParams(cdn_node_params)
		));

		// Only node provider can delete
		assert_noop!(
			DdcNodes::delete_node(
				RuntimeOrigin::signed(2),
				NodePubKey::CDNPubKey(node_pub_key.clone())
			),
			Error::<Test>::OnlyNodeProvider
		);

		// Delete node
		assert_ok!(DdcNodes::delete_node(
			RuntimeOrigin::signed(1),
			NodePubKey::CDNPubKey(node_pub_key.clone()),
		));

		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(
			Event::NodeDeleted { node_pub_key: NodePubKey::CDNPubKey(node_pub_key) }.into(),
		)
	})
}
