//! Tests for the module.

use ddc_primitives::{
	traits::cluster::ClusterManager, ClusterBondingParams, ClusterFeesParams, ClusterId,
	ClusterParams, ClusterPricingParams, NodeParams, NodePubKey, StorageNodeMode,
	StorageNodeParams,
};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use frame_system::Config;
use hex_literal::hex;
use sp_runtime::{traits::Hash, Perquintill};

use super::{mock::*, *};

#[test]
fn create_cluster_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = AccountId::from([1; 32]);
		let cluster_reserve_id = AccountId::from([2; 32]);
		let auth_contract = AccountId::from([3; 32]);

		let cluster_gov_params = ClusterGovParams {
			treasury_share: Perquintill::from_float(0.05),
			validators_share: Perquintill::from_float(0.01),
			cluster_reserve_share: Perquintill::from_float(0.02),
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
				cluster_id,
				cluster_manager_id.clone(),
				cluster_reserve_id.clone(),
				ClusterParams { node_provider_auth_contract: Some(auth_contract.clone()) },
				cluster_gov_params.clone()
			),
			BadOrigin
		);

		// Creating 1 cluster should work fine
		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::root(),
			cluster_id,
			cluster_manager_id.clone(),
			cluster_reserve_id.clone(),
			ClusterParams { node_provider_auth_contract: Some(auth_contract.clone()) },
			cluster_gov_params.clone()
		));

		let created_cluster = DdcClusters::clusters(cluster_id).unwrap();
		assert_eq!(created_cluster.cluster_id, cluster_id);
		assert_eq!(created_cluster.manager_id, cluster_manager_id);
		assert_eq!(created_cluster.reserve_id, cluster_reserve_id);
		assert_eq!(created_cluster.props.node_provider_auth_contract, Some(auth_contract.clone()));

		let created_cluster_gov_params = DdcClusters::clusters_gov_params(cluster_id).unwrap();
		assert_eq!(created_cluster_gov_params.treasury_share, cluster_gov_params.treasury_share);
		assert_eq!(
			created_cluster_gov_params.validators_share,
			cluster_gov_params.validators_share
		);
		assert_eq!(
			created_cluster_gov_params.cluster_reserve_share,
			cluster_gov_params.cluster_reserve_share
		);
		assert_eq!(
			created_cluster_gov_params.storage_bond_size,
			cluster_gov_params.storage_bond_size
		);
		assert_eq!(
			created_cluster_gov_params.storage_chill_delay,
			cluster_gov_params.storage_chill_delay
		);
		assert_eq!(
			created_cluster_gov_params.storage_unbonding_delay,
			cluster_gov_params.storage_unbonding_delay
		);
		assert_eq!(
			created_cluster_gov_params.unit_per_mb_stored,
			cluster_gov_params.unit_per_mb_stored
		);
		assert_eq!(
			created_cluster_gov_params.unit_per_mb_streamed,
			cluster_gov_params.unit_per_mb_streamed
		);
		assert_eq!(
			created_cluster_gov_params.unit_per_put_request,
			cluster_gov_params.unit_per_put_request
		);
		assert_eq!(
			created_cluster_gov_params.unit_per_get_request,
			cluster_gov_params.unit_per_get_request
		);

		// Creating cluster with same id should fail
		assert_noop!(
			DdcClusters::create_cluster(
				RuntimeOrigin::root(),
				cluster_id,
				cluster_manager_id,
				cluster_reserve_id,
				ClusterParams { node_provider_auth_contract: Some(auth_contract) },
				cluster_gov_params
			),
			Error::<Test>::ClusterAlreadyExists
		);

		// Checking storage
		assert!(Clusters::<Test>::contains_key(cluster_id));
		// Checking that event was emitted
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(Event::ClusterCreated { cluster_id }.into())
	})
}

#[test]
fn add_and_delete_node_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = AccountId::from([1; 32]);
		let cluster_reserve_id = AccountId::from([2; 32]);
		let node_pub_key = AccountId::from([3; 32]);

		let contract_id = deploy_contract();

		// Cluster doesn't exist
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
			),
			Error::<Test>::ClusterDoesNotExist
		);

		// Creating 1 cluster should work fine
		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::root(),
			cluster_id,
			cluster_manager_id.clone(),
			cluster_reserve_id.clone(),
			ClusterParams { node_provider_auth_contract: Some(cluster_manager_id.clone()) },
			ClusterGovParams {
				treasury_share: Perquintill::from_float(0.05),
				validators_share: Perquintill::from_float(0.01),
				cluster_reserve_share: Perquintill::from_float(0.02),
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
				RuntimeOrigin::signed(cluster_reserve_id),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
			),
			Error::<Test>::OnlyClusterManager
		);

		// Node doesn't exist
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
			),
			Error::<Test>::AttemptToAddNonExistentNode
		);

		let storage_node_params = StorageNodeParams {
			mode: StorageNodeMode::Storage,
			host: vec![1u8; 255],
			domain: vec![2u8; 255],
			ssl: true,
			http_port: 35000u16,
			grpc_port: 25000u16,
			p2p_port: 15000u16,
		};

		// Node created
		assert_ok!(DdcNodes::create_node(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			NodePubKey::StoragePubKey(node_pub_key.clone()),
			NodeParams::StorageParams(storage_node_params)
		));

		// Node doesn't exist
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
			),
			Error::<Test>::NodeAuthContractCallFailed
		);

		// Set the correct address for auth contract
		assert_ok!(DdcClusters::set_cluster_params(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			cluster_id,
			ClusterParams { node_provider_auth_contract: Some(contract_id) },
		));

		// Node added succesfully
		assert_ok!(DdcClusters::add_node(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			cluster_id,
			NodePubKey::StoragePubKey(node_pub_key.clone()),
		));

		assert!(<DdcClusters as ClusterManager<Test>>::contains_node(
			&cluster_id,
			&NodePubKey::StoragePubKey(node_pub_key.clone())
		));

		// Node already assigned
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
			),
			Error::<Test>::AttemptToAddAlreadyAssignedNode
		);

		// Checking that event was emitted
		System::assert_last_event(
			Event::ClusterNodeAdded {
				cluster_id,
				node_pub_key: NodePubKey::StoragePubKey(node_pub_key.clone()),
			}
			.into(),
		);

		// Remove node
		assert_ok!(DdcClusters::remove_node(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			cluster_id,
			NodePubKey::StoragePubKey(node_pub_key.clone()),
		));

		// Checking that event was emitted
		System::assert_last_event(
			Event::ClusterNodeRemoved {
				cluster_id,
				node_pub_key: NodePubKey::StoragePubKey(node_pub_key.clone()),
			}
			.into(),
		);

		// Remove node should fail
		assert_noop!(
			DdcClusters::remove_node(
				RuntimeOrigin::signed(cluster_manager_id),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key),
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
			let cluster_manager_id = AccountId::from([1; 32]);
			let node_pub_key = AccountId::from([3; 32]);
			// Admin account who deploys the contract.
			let alice = cluster_manager_id;
			let _ = Balances::deposit_creating(&alice, 1_000_000_000_000);

			// Load the contract code.
			let wasm = &include_bytes!("./test_data/node_provider_auth_white_list.wasm")[..];
			let wasm_hash = <Test as Config>::Hashing::hash(wasm);
			let contract_args = encode_constructor();

			// Deploy the contract.
			const GAS_LIMIT: frame_support::weights::Weight =
				Weight::from_parts(100_000_000_000, 0).set_proof_size(u64::MAX);
			const ENDOWMENT: Balance = 0;
			Contracts::instantiate_with_code(
				RuntimeOrigin::signed(alice.clone()),
				ENDOWMENT,
				GAS_LIMIT,
				None,
				wasm.to_vec(),
				contract_args.clone(),
				vec![],
			)
			.unwrap();

			// Configure worker with the contract address.
			let contract_id = Contracts::contract_address(&alice, &wasm_hash, &contract_args, &[]);

			pub const ADD_DDC_NODE_SELECTOR: [u8; 4] = hex!("7a04093d");
			let node_pub_key = NodePubKey::StoragePubKey(node_pub_key);

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
				GAS_LIMIT,
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

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = AccountId::from([1; 32]);
		let cluster_reserve_id = AccountId::from([2; 32]);
		let auth_contract_1 = AccountId::from([3; 32]);
		let auth_contract_2 = AccountId::from([4; 32]);

		// Cluster doesn't exist
		assert_noop!(
			DdcClusters::set_cluster_params(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				ClusterParams { node_provider_auth_contract: Some(auth_contract_1.clone()) },
			),
			Error::<Test>::ClusterDoesNotExist
		);

		// Creating 1 cluster should work fine
		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::root(),
			cluster_id,
			cluster_manager_id.clone(),
			cluster_reserve_id.clone(),
			ClusterParams { node_provider_auth_contract: Some(auth_contract_1) },
			ClusterGovParams {
				treasury_share: Perquintill::from_float(0.05),
				validators_share: Perquintill::from_float(0.01),
				cluster_reserve_share: Perquintill::from_float(0.02),
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
				RuntimeOrigin::signed(cluster_reserve_id),
				cluster_id,
				ClusterParams { node_provider_auth_contract: Some(auth_contract_2.clone()) },
			),
			Error::<Test>::OnlyClusterManager
		);

		assert_ok!(DdcClusters::set_cluster_params(
			RuntimeOrigin::signed(cluster_manager_id),
			cluster_id,
			ClusterParams { node_provider_auth_contract: Some(auth_contract_2.clone()) },
		));

		let updated_cluster = DdcClusters::clusters(cluster_id).unwrap();
		assert_eq!(updated_cluster.props.node_provider_auth_contract, Some(auth_contract_2));

		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(Event::ClusterParamsSet { cluster_id }.into())
	})
}

#[test]
fn set_cluster_gov_params_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = AccountId::from([1; 32]);
		let cluster_reserve_id = AccountId::from([2; 32]);
		let auth_contract = AccountId::from([3; 32]);

		let cluster_gov_params = ClusterGovParams {
			treasury_share: Perquintill::from_float(0.05),
			validators_share: Perquintill::from_float(0.01),
			cluster_reserve_share: Perquintill::from_float(0.02),
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
				cluster_id,
				cluster_gov_params.clone()
			),
			Error::<Test>::ClusterDoesNotExist
		);

		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::root(),
			cluster_id,
			cluster_manager_id.clone(),
			cluster_reserve_id,
			ClusterParams { node_provider_auth_contract: Some(auth_contract) },
			cluster_gov_params.clone()
		));

		assert_noop!(
			DdcClusters::set_cluster_gov_params(
				RuntimeOrigin::signed(cluster_manager_id),
				cluster_id,
				cluster_gov_params
			),
			BadOrigin
		);

		let updated_gov_params = ClusterGovParams {
			treasury_share: Perquintill::from_float(0.06),
			validators_share: Perquintill::from_float(0.02),
			cluster_reserve_share: Perquintill::from_float(0.03),
			storage_bond_size: 1000,
			storage_chill_delay: 500,
			storage_unbonding_delay: 500,
			unit_per_mb_stored: 100,
			unit_per_mb_streamed: 100,
			unit_per_put_request: 100,
			unit_per_get_request: 100,
		};

		assert_ok!(DdcClusters::set_cluster_gov_params(
			RuntimeOrigin::root(),
			cluster_id,
			updated_gov_params.clone()
		));

		let updated_cluster_gov_params = DdcClusters::clusters_gov_params(cluster_id).unwrap();
		assert_eq!(updated_cluster_gov_params.treasury_share, updated_gov_params.treasury_share);
		assert_eq!(
			updated_cluster_gov_params.validators_share,
			updated_gov_params.validators_share
		);
		assert_eq!(
			updated_cluster_gov_params.cluster_reserve_share,
			updated_gov_params.cluster_reserve_share
		);
		assert_eq!(
			updated_cluster_gov_params.storage_bond_size,
			updated_gov_params.storage_bond_size
		);
		assert_eq!(
			updated_cluster_gov_params.storage_chill_delay,
			updated_gov_params.storage_chill_delay
		);
		assert_eq!(
			updated_cluster_gov_params.storage_unbonding_delay,
			updated_gov_params.storage_unbonding_delay
		);
		assert_eq!(
			updated_cluster_gov_params.unit_per_mb_stored,
			updated_gov_params.unit_per_mb_stored
		);
		assert_eq!(
			updated_cluster_gov_params.unit_per_mb_streamed,
			updated_gov_params.unit_per_mb_streamed
		);
		assert_eq!(
			updated_cluster_gov_params.unit_per_put_request,
			updated_gov_params.unit_per_put_request
		);
		assert_eq!(
			updated_cluster_gov_params.unit_per_get_request,
			updated_gov_params.unit_per_get_request
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(Event::ClusterGovParamsSet { cluster_id }.into())
	})
}

#[test]
fn cluster_visitor_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = AccountId::from([1; 32]);
		let cluster_reserve_id = AccountId::from([2; 32]);
		let auth_contract = AccountId::from([3; 32]);

		let cluster_gov_params = ClusterGovParams {
			treasury_share: Perquintill::from_float(0.05),
			validators_share: Perquintill::from_float(0.01),
			cluster_reserve_share: Perquintill::from_float(0.02),
			storage_bond_size: 100,
			storage_chill_delay: 50,
			storage_unbonding_delay: 50,
			unit_per_mb_stored: 10,
			unit_per_mb_streamed: 10,
			unit_per_put_request: 10,
			unit_per_get_request: 10,
		};

		// Creating 1 cluster should work fine
		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::root(),
			cluster_id,
			cluster_manager_id,
			cluster_reserve_id.clone(),
			ClusterParams { node_provider_auth_contract: Some(auth_contract) },
			cluster_gov_params
		));

		assert_ok!(<DdcClusters as ClusterVisitor<Test>>::ensure_cluster(&cluster_id));

		assert_eq!(
			<DdcClusters as ClusterVisitor<Test>>::get_bond_size(&cluster_id, NodeType::Storage)
				.unwrap(),
			100u128
		);
		assert_eq!(
			<DdcClusters as ClusterVisitor<Test>>::get_bond_size(&cluster_id, NodeType::Storage)
				.unwrap(),
			100u128
		);

		assert_eq!(
			<DdcClusters as ClusterVisitor<Test>>::get_pricing_params(&cluster_id).unwrap(),
			ClusterPricingParams {
				unit_per_mb_stored: 10,
				unit_per_mb_streamed: 10,
				unit_per_put_request: 10,
				unit_per_get_request: 10,
			}
		);

		assert_eq!(
			<DdcClusters as ClusterVisitor<Test>>::get_fees_params(&cluster_id).unwrap(),
			ClusterFeesParams {
				treasury_share: Perquintill::from_float(0.05),
				validators_share: Perquintill::from_float(0.01),
				cluster_reserve_share: Perquintill::from_float(0.02)
			}
		);

		assert_eq!(
			<DdcClusters as ClusterVisitor<Test>>::get_reserve_account_id(&cluster_id).unwrap(),
			cluster_reserve_id
		);

		assert_eq!(
			<DdcClusters as ClusterVisitor<Test>>::get_chill_delay(&cluster_id, NodeType::Storage)
				.unwrap(),
			50
		);
		assert_eq!(
			<DdcClusters as ClusterVisitor<Test>>::get_chill_delay(&cluster_id, NodeType::Storage)
				.unwrap(),
			50
		);

		assert_eq!(
			<DdcClusters as ClusterVisitor<Test>>::get_unbonding_delay(
				&cluster_id,
				NodeType::Storage
			)
			.unwrap(),
			50
		);
		assert_eq!(
			<DdcClusters as ClusterVisitor<Test>>::get_unbonding_delay(
				&cluster_id,
				NodeType::Storage
			)
			.unwrap(),
			50
		);

		assert_eq!(
			<DdcClusters as ClusterVisitor<Test>>::get_bonding_params(&cluster_id).unwrap(),
			ClusterBondingParams::<BlockNumberFor<Test>> {
				storage_bond_size: 100,
				storage_chill_delay: 50,
				storage_unbonding_delay: 50,
			}
		);
	})
}

#[test]
fn cluster_creator_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = AccountId::from([1; 32]);
		let cluster_reserve_id = AccountId::from([2; 32]);
		let auth_contract = AccountId::from([3; 32]);

		let cluster_gov_params = ClusterGovParams {
			treasury_share: Perquintill::from_float(0.05),
			validators_share: Perquintill::from_float(0.01),
			cluster_reserve_share: Perquintill::from_float(0.02),
			storage_bond_size: 100,
			storage_chill_delay: 50,
			storage_unbonding_delay: 50,
			unit_per_mb_stored: 10,
			unit_per_mb_streamed: 10,
			unit_per_put_request: 10,
			unit_per_get_request: 10,
		};

		assert_ok!(<DdcClusters as ClusterCreator<Test, BalanceOf<Test>>>::create_new_cluster(
			cluster_id,
			cluster_manager_id,
			cluster_reserve_id,
			ClusterParams { node_provider_auth_contract: Some(auth_contract) },
			cluster_gov_params
		));

		assert!(Clusters::<Test>::contains_key(cluster_id));
		assert!(ClustersGovParams::<Test>::contains_key(cluster_id));
	})
}
