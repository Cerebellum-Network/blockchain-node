//! Tests for the module.

use ddc_primitives::{CDNNodeParams, ClusterId, ClusterParams, NodeParams, NodePubKey};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use frame_system::Config;
use hex_literal::hex;
use sp_runtime::{traits::Hash, AccountId32, Perbill};

use super::{mock::*, *};

#[test]
fn create_cluster_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_gov_params = ClusterGovParams {
			treasury_share: Perbill::from_float(0.05),
			validators_share: Perbill::from_float(0.01),
			cluster_reserve_share: Perbill::from_float(0.02),
			cdn_bond_size: 100,
			cdn_chill_delay: 50,
			cdn_unbonding_delay: 50,
			storage_bond_size: 100,
			storage_chill_delay: 50,
			storage_unbonding_delay: 50,
			unit_per_mb_stored: 10,
			unit_per_mb_streamed: 10,
			unit_per_put_request: 10,
			unit_per_get_request: 10,
		};

		// Creating cluster not with root signature should fail
		assert_noop!(
			DdcClusters::create_cluster(
				RuntimeOrigin::signed(AccountId::from([1; 32])),
				ClusterId::from([1; 20]),
				AccountId::from([1; 32]),
				AccountId::from([2; 32]),
				ClusterParams { node_provider_auth_contract: Some(AccountId::from([1; 32])) },
				cluster_gov_params.clone()
			),
			BadOrigin
		);

		// Creating 1 cluster should work fine
		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::root(),
			ClusterId::from([1; 20]),
			AccountId::from([1; 32]),
			AccountId::from([2; 32]),
			ClusterParams { node_provider_auth_contract: Some(AccountId::from([1; 32])) },
			cluster_gov_params.clone()
		));

		// Creating cluster with same id should fail
		assert_noop!(
			DdcClusters::create_cluster(
				RuntimeOrigin::root(),
				ClusterId::from([1; 20]),
				AccountId::from([1; 32]),
				AccountId::from([2; 32]),
				ClusterParams { node_provider_auth_contract: Some(AccountId::from([1; 32])) },
				cluster_gov_params
			),
			Error::<Test>::ClusterAlreadyExists
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(
			Event::ClusterCreated { cluster_id: ClusterId::from([1; 20]) }.into(),
		)
	})
}

#[test]
fn add_and_delete_node_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let contract_id = deploy_contract();

		// Cluster doesn't exist
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(AccountId::from([2; 32])),
				ClusterId::from([1; 20]),
				NodePubKey::CDNPubKey(AccountId::from([4; 32])),
			),
			Error::<Test>::ClusterDoesNotExist
		);

		// Creating 1 cluster should work fine
		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::root(),
			ClusterId::from([1; 20]),
			AccountId::from([1; 32]),
			AccountId::from([2; 32]),
			ClusterParams { node_provider_auth_contract: Some(AccountId::from([1; 32])) },
			ClusterGovParams {
				treasury_share: Perbill::from_float(0.05),
				validators_share: Perbill::from_float(0.01),
				cluster_reserve_share: Perbill::from_float(0.02),
				cdn_bond_size: 100,
				cdn_chill_delay: 50,
				cdn_unbonding_delay: 50,
				storage_bond_size: 100,
				storage_chill_delay: 50,
				storage_unbonding_delay: 50,
				unit_per_mb_stored: 10,
				unit_per_mb_streamed: 10,
				unit_per_put_request: 10,
				unit_per_get_request: 10,
			}
		));

		// Not Cluster Manager
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(AccountId::from([2; 32])),
				ClusterId::from([1; 20]),
				NodePubKey::CDNPubKey(AccountId::from([4; 32])),
			),
			Error::<Test>::OnlyClusterManager
		);

		// Node doesn't exist
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(AccountId::from([1; 32])),
				ClusterId::from([1; 20]),
				NodePubKey::CDNPubKey(AccountId::from([4; 32])),
			),
			Error::<Test>::AttemptToAddNonExistentNode
		);

		// Create node
		let bytes = [4u8; 32];
		let node_pub_key = AccountId32::from(bytes);

		let cdn_node_params = CDNNodeParams {
			host: vec![1u8, 255],
			http_port: 35000u16,
			grpc_port: 25000u16,
			p2p_port: 15000u16,
		};

		// Node created
		assert_ok!(DdcNodes::create_node(
			RuntimeOrigin::signed(AccountId::from([1; 32])),
			NodePubKey::CDNPubKey(node_pub_key),
			NodeParams::CDNParams(cdn_node_params)
		));

		// Node doesn't exist
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(AccountId::from([1; 32])),
				ClusterId::from([1; 20]),
				NodePubKey::CDNPubKey(AccountId::from([4; 32])),
			),
			Error::<Test>::NodeAuthContractCallFailed
		);

		// Set the correct address for auth contract
		assert_ok!(DdcClusters::set_cluster_params(
			RuntimeOrigin::signed(AccountId::from([1; 32])),
			ClusterId::from([1; 20]),
			ClusterParams { node_provider_auth_contract: Some(contract_id) },
		));

		// Node doesn't exist
		assert_ok!(DdcClusters::add_node(
			RuntimeOrigin::signed(AccountId::from([1; 32])),
			ClusterId::from([1; 20]),
			NodePubKey::CDNPubKey(AccountId::from([4; 32])),
		));

		// Node already assigned
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(AccountId::from([1; 32])),
				ClusterId::from([1; 20]),
				NodePubKey::CDNPubKey(AccountId::from([4; 32])),
			),
			Error::<Test>::AttemptToAddAlreadyAssignedNode
		);

		// Checking that event was emitted
		System::assert_last_event(
			Event::ClusterNodeAdded {
				cluster_id: ClusterId::from([1; 20]),
				node_pub_key: NodePubKey::CDNPubKey(AccountId::from([4; 32])),
			}
			.into(),
		);

		// Remove node
		assert_ok!(DdcClusters::remove_node(
			RuntimeOrigin::signed(AccountId::from([1; 32])),
			ClusterId::from([1; 20]),
			NodePubKey::CDNPubKey(AccountId::from([4; 32])),
		));

		// Checking that event was emitted
		System::assert_last_event(
			Event::ClusterNodeRemoved {
				cluster_id: ClusterId::from([1; 20]),
				node_pub_key: NodePubKey::CDNPubKey(AccountId::from([4; 32])),
			}
			.into(),
		);

		// Remove node should fail
		assert_noop!(
			DdcClusters::remove_node(
				RuntimeOrigin::signed(AccountId::from([1; 32])),
				ClusterId::from([1; 20]),
				NodePubKey::CDNPubKey(AccountId::from([4; 32])),
			),
			Error::<Test>::AttemptToRemoveNotAssignedNode
		);

		pub const CTOR_SELECTOR: [u8; 4] = hex!("9bae9d5e");

		fn encode_constructor() -> Vec<u8> {
			let mut call_data = CTOR_SELECTOR.to_vec();
			let x = 0_u128;
			for _ in 0..9 {
				x.encode_to(&mut call_data);
			}
			call_data
		}

		fn deploy_contract() -> AccountId {
			// Admin account who deploys the contract.
			let alice = AccountId::from([1; 32]);
			let _ = Balances::deposit_creating(&alice, 1_000_000_000_000);

			// Load the contract code.
			let wasm = &include_bytes!("./test_data/node_provider_auth_white_list.wasm")[..];
			let wasm_hash = <Test as Config>::Hashing::hash(wasm);
			let contract_args = encode_constructor();

			// Deploy the contract.
			const GAS_LIMIT: frame_support::weights::Weight =
				Weight::from_ref_time(100_000_000_000).set_proof_size(u64::MAX);
			const ENDOWMENT: Balance = 0;
			Contracts::instantiate_with_code(
				RuntimeOrigin::signed(alice.clone()),
				ENDOWMENT,
				GAS_LIMIT,
				None,
				wasm.to_vec(),
				contract_args,
				vec![],
			)
			.unwrap();

			// Configure worker with the contract address.
			let contract_id = Contracts::contract_address(&alice, &wasm_hash, &[]);

			pub const ADD_DDC_NODE_SELECTOR: [u8; 4] = hex!("7a04093d");
			let node_pub_key = NodePubKey::CDNPubKey(AccountId::from([4; 32]));

			let call_data = {
				// is_authorized(node_provider: AccountId, node: Vec<u8>, node_variant: u8) -> bool
				let args: ([u8; 4], Vec<u8>) =
					(ADD_DDC_NODE_SELECTOR, node_pub_key.encode()[1..].to_vec());
				args.encode()
			};

			let results = Contracts::call(
				RuntimeOrigin::signed(alice),
				contract_id.clone(),
				0,
				Weight::from_ref_time(1_000_000_000_000).set_proof_size(u64::MAX),
				None,
				call_data,
			);

			results.unwrap();

			contract_id
		}
	})
}

#[test]
fn set_cluster_params_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		// Cluster doesn't exist
		assert_noop!(
			DdcClusters::set_cluster_params(
				RuntimeOrigin::signed(AccountId::from([2; 32])),
				ClusterId::from([2; 20]),
				ClusterParams { node_provider_auth_contract: Some(AccountId::from([2; 32])) },
			),
			Error::<Test>::ClusterDoesNotExist
		);

		// Creating 1 cluster should work fine
		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::root(),
			ClusterId::from([1; 20]),
			AccountId::from([1; 32]),
			AccountId::from([2; 32]),
			ClusterParams { node_provider_auth_contract: Some(AccountId::from([1; 32])) },
			ClusterGovParams {
				treasury_share: Perbill::from_float(0.05),
				validators_share: Perbill::from_float(0.01),
				cluster_reserve_share: Perbill::from_float(0.02),
				cdn_bond_size: 100,
				cdn_chill_delay: 50,
				cdn_unbonding_delay: 50,
				storage_bond_size: 100,
				storage_chill_delay: 50,
				storage_unbonding_delay: 50,
				unit_per_mb_stored: 10,
				unit_per_mb_streamed: 10,
				unit_per_put_request: 10,
				unit_per_get_request: 10,
			}
		));

		assert_noop!(
			DdcClusters::set_cluster_params(
				RuntimeOrigin::signed(AccountId::from([2; 32])),
				ClusterId::from([1; 20]),
				ClusterParams { node_provider_auth_contract: Some(AccountId::from([2; 32])) },
			),
			Error::<Test>::OnlyClusterManager
		);

		assert_ok!(DdcClusters::set_cluster_params(
			RuntimeOrigin::signed(AccountId::from([1; 32])),
			ClusterId::from([1; 20]),
			ClusterParams { node_provider_auth_contract: Some(AccountId::from([2; 32])) },
		));

		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(
			Event::ClusterParamsSet { cluster_id: ClusterId::from([1; 20]) }.into(),
		)
	})
}

#[test]
fn set_cluster_gov_params_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_gov_params = ClusterGovParams {
			treasury_share: Perbill::from_float(0.05),
			validators_share: Perbill::from_float(0.01),
			cluster_reserve_share: Perbill::from_float(0.02),
			cdn_bond_size: 100,
			cdn_chill_delay: 50,
			cdn_unbonding_delay: 50,
			storage_bond_size: 100,
			storage_chill_delay: 50,
			storage_unbonding_delay: 50,
			unit_per_mb_stored: 10,
			unit_per_mb_streamed: 10,
			unit_per_put_request: 10,
			unit_per_get_request: 10,
		};

		// Cluster doesn't exist
		assert_noop!(
			DdcClusters::set_cluster_gov_params(
				RuntimeOrigin::root(),
				ClusterId::from([2; 20]),
				cluster_gov_params.clone()
			),
			Error::<Test>::ClusterDoesNotExist
		);

		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::root(),
			ClusterId::from([1; 20]),
			AccountId::from([1; 32]),
			AccountId::from([2; 32]),
			ClusterParams { node_provider_auth_contract: Some(AccountId::from([1; 32])) },
			cluster_gov_params.clone()
		));

		assert_noop!(
			DdcClusters::set_cluster_gov_params(
				RuntimeOrigin::signed(AccountId::from([1; 32])),
				ClusterId::from([1; 20]),
				cluster_gov_params.clone()
			),
			BadOrigin
		);

		assert_ok!(DdcClusters::set_cluster_gov_params(
			RuntimeOrigin::root(),
			ClusterId::from([1; 20]),
			cluster_gov_params
		));

		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(
			Event::ClusterGovParamsSet { cluster_id: ClusterId::from([1; 20]) }.into(),
		)
	})
}
