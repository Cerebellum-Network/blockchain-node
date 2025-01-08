//! Tests for the module.

use codec::Compact;
use ddc_primitives::{
	traits::cluster::ClusterManager, ClusterBondingParams, ClusterFeesParams, ClusterId,
	ClusterParams, ClusterPricingParams, NodeParams, NodePubKey, StorageNodeMode,
	StorageNodeParams,
};
use frame_support::{assert_noop, assert_ok};
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

		let cluster_protocol_params = ClusterProtocolParams {
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
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			cluster_id,
			cluster_reserve_id.clone(),
			ClusterParams {
				node_provider_auth_contract: Some(auth_contract.clone()),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3
			},
			cluster_protocol_params.clone()
		));

		let created_cluster = Clusters::<Test>::get(cluster_id).unwrap();
		assert_eq!(created_cluster.cluster_id, cluster_id);
		assert_eq!(created_cluster.manager_id, cluster_manager_id);
		assert_eq!(created_cluster.reserve_id, cluster_reserve_id);
		assert_eq!(created_cluster.props.node_provider_auth_contract, Some(auth_contract.clone()));

		let created_cluster_protocol_params = ClustersGovParams::<Test>::get(cluster_id).unwrap();
		assert_eq!(
			created_cluster_protocol_params.treasury_share,
			cluster_protocol_params.treasury_share
		);
		assert_eq!(
			created_cluster_protocol_params.validators_share,
			cluster_protocol_params.validators_share
		);
		assert_eq!(
			created_cluster_protocol_params.cluster_reserve_share,
			cluster_protocol_params.cluster_reserve_share
		);
		assert_eq!(
			created_cluster_protocol_params.storage_bond_size,
			cluster_protocol_params.storage_bond_size
		);
		assert_eq!(
			created_cluster_protocol_params.storage_chill_delay,
			cluster_protocol_params.storage_chill_delay
		);
		assert_eq!(
			created_cluster_protocol_params.storage_unbonding_delay,
			cluster_protocol_params.storage_unbonding_delay
		);
		assert_eq!(
			created_cluster_protocol_params.unit_per_mb_stored,
			cluster_protocol_params.unit_per_mb_stored
		);
		assert_eq!(
			created_cluster_protocol_params.unit_per_mb_streamed,
			cluster_protocol_params.unit_per_mb_streamed
		);
		assert_eq!(
			created_cluster_protocol_params.unit_per_put_request,
			cluster_protocol_params.unit_per_put_request
		);
		assert_eq!(
			created_cluster_protocol_params.unit_per_get_request,
			cluster_protocol_params.unit_per_get_request
		);

		// Creating cluster with same id should fail
		assert_noop!(
			DdcClusters::create_cluster(
				RuntimeOrigin::signed(cluster_manager_id),
				cluster_id,
				cluster_reserve_id,
				ClusterParams {
					node_provider_auth_contract: Some(auth_contract),
					erasure_coding_required: 4,
					erasure_coding_total: 6,
					replication_total: 3
				},
				cluster_protocol_params
			),
			Error::<Test>::ClusterAlreadyExists
		);

		// Checking storage
		assert!(Clusters::<Test>::contains_key(cluster_id));
		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_has_event(Event::ClusterCreated { cluster_id }.into());
		System::assert_last_event(Event::ClusterProtocolParamsSet { cluster_id }.into());
	})
}

#[test]
fn add_join_and_delete_node_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = AccountId::from([1; 32]);
		let cluster_reserve_id = AccountId::from([2; 32]);
		let node_pub_key = AccountId::from([3; 32]);
		let node_pub_key2 = AccountId::from([4; 32]);

		let contract_id = deploy_contract();

		// Cluster doesn't exist
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
				ClusterNodeKind::Genesis
			),
			Error::<Test>::ClusterDoesNotExist
		);
		assert_noop!(
			DdcClusters::join_cluster(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
			),
			Error::<Test>::ClusterDoesNotExist
		);

		// Creating 1 cluster should work fine
		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			cluster_id,
			cluster_reserve_id.clone(),
			ClusterParams {
				node_provider_auth_contract: None,
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3
			},
			ClusterProtocolParams {
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

		// Cluster has no auth smart contract
		assert_noop!(
			DdcClusters::join_cluster(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
			),
			Error::<Test>::NodeIsNotAuthorized
		);

		// Set an incorrect address for auth contract
		assert_ok!(DdcClusters::set_cluster_params(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			cluster_id,
			ClusterParams {
				node_provider_auth_contract: Some(cluster_manager_id.clone()),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3
			},
		));

		// Not Cluster Manager
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(cluster_reserve_id),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
				ClusterNodeKind::Genesis
			),
			Error::<Test>::OnlyClusterManager
		);

		// Node doesn't exist
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
				ClusterNodeKind::Genesis
			),
			Error::<Test>::AttemptToAddNonExistentNode
		);
		assert_noop!(
			DdcClusters::join_cluster(
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
			NodeParams::StorageParams(storage_node_params.clone()),
		));

		// Not node provider
		assert_noop!(
			DdcClusters::join_cluster(
				RuntimeOrigin::signed(node_pub_key.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
			),
			Error::<Test>::OnlyNodeProvider
		);

		// Set the correct address for auth contract
		assert_ok!(DdcClusters::set_cluster_params(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			cluster_id,
			ClusterParams {
				node_provider_auth_contract: Some(contract_id),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3
			},
		));

		assert_ok!(DdcClusters::bond_cluster(&cluster_id));

		// Node is not authorized to join
		assert_ok!(DdcNodes::create_node(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			NodePubKey::StoragePubKey(node_pub_key2.clone()),
			NodeParams::StorageParams(storage_node_params.clone()),
		));
		assert_noop!(
			DdcClusters::join_cluster(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key2.clone()),
			),
			Error::<Test>::NodeIsNotAuthorized
		);

		// Node added successfully
		assert_ok!(DdcClusters::add_node(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			cluster_id,
			NodePubKey::StoragePubKey(node_pub_key.clone()),
			ClusterNodeKind::Genesis
		));

		assert!(<DdcClusters as ClusterManager<Test>>::contains_node(
			&cluster_id,
			&NodePubKey::StoragePubKey(node_pub_key.clone()),
			None
		));

		// Node already assigned
		assert_noop!(
			DdcClusters::add_node(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
				ClusterNodeKind::Genesis
			),
			Error::<Test>::AttemptToAddAlreadyAssignedNode
		);
		assert_noop!(
			DdcClusters::join_cluster(
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
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				NodePubKey::StoragePubKey(node_pub_key.clone()),
			),
			Error::<Test>::AttemptToRemoveNotAssignedNode
		);

		// Node joined successfully
		assert_ok!(DdcClusters::join_cluster(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			cluster_id,
			NodePubKey::StoragePubKey(node_pub_key.clone()),
		));

		assert!(<DdcClusters as ClusterManager<Test>>::contains_node(
			&cluster_id,
			&NodePubKey::StoragePubKey(node_pub_key.clone()),
			None
		));

		// Checking that event was emitted
		System::assert_last_event(
			Event::ClusterNodeAdded {
				cluster_id,
				node_pub_key: NodePubKey::StoragePubKey(node_pub_key.clone()),
			}
			.into(),
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
				Some(Compact(100)),
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
				ClusterParams {
					node_provider_auth_contract: Some(auth_contract_1.clone()),
					erasure_coding_required: 4,
					erasure_coding_total: 6,
					replication_total: 3
				},
			),
			Error::<Test>::ClusterDoesNotExist
		);

		// Creating 1 cluster should work fine
		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			cluster_id,
			cluster_reserve_id.clone(),
			ClusterParams {
				node_provider_auth_contract: Some(auth_contract_1),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3
			},
			ClusterProtocolParams {
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
				ClusterParams {
					node_provider_auth_contract: Some(auth_contract_2.clone()),
					erasure_coding_required: 4,
					erasure_coding_total: 6,
					replication_total: 3
				},
			),
			Error::<Test>::OnlyClusterManager
		);

		assert_noop!(
			DdcClusters::set_cluster_params(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				ClusterParams {
					node_provider_auth_contract: Some(auth_contract_2.clone()),
					erasure_coding_required: 1,
					erasure_coding_total: 6,
					replication_total: 3
				},
			),
			Error::<Test>::ErasureCodingRequiredDidNotMeetMinimum
		);

		assert_noop!(
			DdcClusters::set_cluster_params(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				ClusterParams {
					node_provider_auth_contract: Some(auth_contract_2.clone()),
					erasure_coding_required: 4,
					erasure_coding_total: 1,
					replication_total: 3
				},
			),
			Error::<Test>::ErasureCodingTotalNotMeetMinimum
		);

		assert_noop!(
			DdcClusters::set_cluster_params(
				RuntimeOrigin::signed(cluster_manager_id.clone()),
				cluster_id,
				ClusterParams {
					node_provider_auth_contract: Some(auth_contract_2.clone()),
					erasure_coding_required: 4,
					erasure_coding_total: 6,
					replication_total: 1
				},
			),
			Error::<Test>::ReplicationTotalDidNotMeetMinimum
		);

		assert_ok!(DdcClusters::set_cluster_params(
			RuntimeOrigin::signed(cluster_manager_id),
			cluster_id,
			ClusterParams {
				node_provider_auth_contract: Some(auth_contract_2.clone()),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3
			},
		));

		let updated_cluster = Clusters::<Test>::get(cluster_id).unwrap();
		assert_eq!(updated_cluster.props.node_provider_auth_contract, Some(auth_contract_2));
		assert_eq!(updated_cluster.props.erasure_coding_required, 4);
		assert_eq!(updated_cluster.props.erasure_coding_total, 6);
		assert_eq!(updated_cluster.props.replication_total, 3);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 3);
		System::assert_last_event(Event::ClusterParamsSet { cluster_id }.into())
	})
}

#[test]
fn set_last_validated_era_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = AccountId::from([1; 32]);
		let cluster_reserve_id = AccountId::from([2; 32]);
		let auth_contract_1 = AccountId::from([3; 32]);
		let era_id: DdcEra = 22;

		// Cluster doesn't exist
		assert_noop!(
			<DdcClusters as ClusterValidator<Test>>::set_last_paid_era(&cluster_id, era_id),
			Error::<Test>::ClusterDoesNotExist
		);

		// Creating 1 cluster should work fine
		assert_ok!(DdcClusters::create_cluster(
			RuntimeOrigin::signed(cluster_manager_id.clone()),
			cluster_id,
			cluster_reserve_id.clone(),
			ClusterParams {
				node_provider_auth_contract: Some(auth_contract_1),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3
			},
			ClusterProtocolParams {
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

		assert_ok!(<DdcClusters as ClusterValidator<Test>>::set_last_paid_era(&cluster_id, era_id));

		let updated_cluster = Clusters::<Test>::get(cluster_id).unwrap();
		assert_eq!(updated_cluster.last_paid_era, era_id);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 3);
		System::assert_last_event(Event::ClusterEraPaid { cluster_id, era_id }.into())
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

		let cluster_protocol_params = ClusterProtocolParams {
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
			RuntimeOrigin::signed(cluster_manager_id),
			cluster_id,
			cluster_reserve_id.clone(),
			ClusterParams {
				node_provider_auth_contract: Some(auth_contract),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3
			},
			cluster_protocol_params
		));

		assert!(<DdcClusters as ClusterQuery<Test>>::cluster_exists(&cluster_id));

		assert_eq!(
			<DdcClusters as ClusterProtocol<Test, BalanceOf<Test>>>::get_bond_size(
				&cluster_id,
				NodeType::Storage
			)
			.unwrap(),
			100u128
		);
		assert_eq!(
			<DdcClusters as ClusterProtocol<Test, BalanceOf<Test>>>::get_bond_size(
				&cluster_id,
				NodeType::Storage
			)
			.unwrap(),
			100u128
		);

		assert_eq!(
			<DdcClusters as ClusterProtocol<Test, BalanceOf<Test>>>::get_pricing_params(
				&cluster_id
			)
			.unwrap(),
			ClusterPricingParams {
				unit_per_mb_stored: 10,
				unit_per_mb_streamed: 10,
				unit_per_put_request: 10,
				unit_per_get_request: 10,
			}
		);

		assert_eq!(
			<DdcClusters as ClusterProtocol<Test, BalanceOf<Test>>>::get_fees_params(&cluster_id)
				.unwrap(),
			ClusterFeesParams {
				treasury_share: Perquintill::from_float(0.05),
				validators_share: Perquintill::from_float(0.01),
				cluster_reserve_share: Perquintill::from_float(0.02)
			}
		);

		assert_eq!(
			<DdcClusters as ClusterProtocol<Test, BalanceOf<Test>>>::get_reserve_account_id(
				&cluster_id
			)
			.unwrap(),
			cluster_reserve_id
		);

		assert_eq!(
			<DdcClusters as ClusterProtocol<Test, BalanceOf<Test>>>::get_chill_delay(
				&cluster_id,
				NodeType::Storage
			)
			.unwrap(),
			50
		);
		assert_eq!(
			<DdcClusters as ClusterProtocol<Test, BalanceOf<Test>>>::get_chill_delay(
				&cluster_id,
				NodeType::Storage
			)
			.unwrap(),
			50
		);

		assert_eq!(
			<DdcClusters as ClusterProtocol<Test, BalanceOf<Test>>>::get_unbonding_delay(
				&cluster_id,
				NodeType::Storage
			)
			.unwrap(),
			50
		);
		assert_eq!(
			<DdcClusters as ClusterProtocol<Test, BalanceOf<Test>>>::get_unbonding_delay(
				&cluster_id,
				NodeType::Storage
			)
			.unwrap(),
			50
		);

		assert_eq!(
			<DdcClusters as ClusterProtocol<Test, BalanceOf<Test>>>::get_bonding_params(
				&cluster_id
			)
			.unwrap(),
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

		let cluster_protocol_params = ClusterProtocolParams {
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

		assert_ok!(<DdcClusters as ClusterCreator<Test, BalanceOf<Test>>>::create_cluster(
			cluster_id,
			cluster_manager_id,
			cluster_reserve_id,
			ClusterParams {
				node_provider_auth_contract: Some(auth_contract),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3
			},
			cluster_protocol_params,
		));

		assert!(Clusters::<Test>::contains_key(cluster_id));
		assert!(ClustersGovParams::<Test>::contains_key(cluster_id));
	})
}
