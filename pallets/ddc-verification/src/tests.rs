use ddc_primitives::{
	ClusterId, MergeActivityHash, StorageNodeParams, StorageNodePubKey, KEY_TYPE,
};
use frame_support::{assert_noop, assert_ok};
use sp_core::{
	offchain::{
		testing::{PendingRequest, TestOffchainExt, TestTransactionPoolExt},
		OffchainDbExt, OffchainStorage, OffchainWorkerExt, Timestamp, TransactionPoolExt,
	},
	Pair,
};
use sp_io::TestExternalities;
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
use sp_runtime::AccountId32;

use crate::{mock::*, Error, NodeActivity, OCWError, *};

#[allow(dead_code)]
fn register_validators(validators: Vec<AccountId32>) {
	ValidatorAssignments::<Test>::put(validators.clone());

	for validator in validators {
		assert_noop!(
			DdcVerification::set_validator_key(
				RuntimeOrigin::signed(validator.clone()),
				validator.clone(),
			),
			Error::<Test>::NotController
		);
	}
}

fn get_validators() -> Vec<AccountId32> {
	let validator1: AccountId32 = [1; 32].into();
	let validator2: AccountId32 = [2; 32].into();
	let validator3: AccountId32 = [3; 32].into();
	let validator4: AccountId32 = [4; 32].into();
	let validator5: AccountId32 = [5; 32].into();

	vec![validator1, validator2, validator3, validator4, validator5]
}

fn get_node_activities() -> Vec<NodeActivity> {
	let node1 = NodeActivity {
		node_id: "0".to_string(),
		provider_id: "0".to_string(),
		stored_bytes: 100,
		transferred_bytes: 50,
		number_of_puts: 10,
		number_of_gets: 20,
	};
	let node2 = NodeActivity {
		node_id: "1".to_string(),
		provider_id: "1".to_string(),
		stored_bytes: 101,
		transferred_bytes: 51,
		number_of_puts: 11,
		number_of_gets: 21,
	};
	let node3 = NodeActivity {
		node_id: "2".to_string(),
		provider_id: "2".to_string(),
		stored_bytes: 102,
		transferred_bytes: 52,
		number_of_puts: 12,
		number_of_gets: 22,
	};
	let node4 = NodeActivity {
		node_id: "3".to_string(),
		provider_id: "3".to_string(),
		stored_bytes: 103,
		transferred_bytes: 53,
		number_of_puts: 13,
		number_of_gets: 23,
	};
	let node5 = NodeActivity {
		node_id: "4".to_string(),
		provider_id: "4".to_string(),
		stored_bytes: 104,
		transferred_bytes: 54,
		number_of_puts: 14,
		number_of_gets: 24,
	};
	vec![node1, node2, node3, node4, node5]
}

#[test]
fn fetch_node_usage_works() {
	let mut ext = TestExternalities::default();
	let (offchain, offchain_state) = TestOffchainExt::new();
	let (pool, _) = TestTransactionPoolExt::new();

	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(OffchainDbExt::new(Box::new(offchain)));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		let mut offchain_state = offchain_state.write();
		offchain_state.timestamp = Timestamp::from_unix_millis(0);
		let host = "example.com";
		let port = 80;
		let era_id = 1;

		// Create a sample NodeActivity instance
		let node_activity1 = NodeActivity {
			provider_id: "1".to_string(),
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		};
		let node_activity2 = NodeActivity {
			provider_id: "2".to_string(),
			node_id: "2".to_string(),
			stored_bytes: 110,
			transferred_bytes: 510,
			number_of_puts: 110,
			number_of_gets: 210,
		};
		let nodes_activity_json =
			serde_json::to_string(&vec![node_activity1.clone(), node_activity2.clone()]).unwrap();

		// Mock HTTP request and response
		let pending_request = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId={}", host, port, era_id),
			response: Some(nodes_activity_json.as_bytes().to_vec()),
			sent: true,
			..Default::default()
		};
		offchain_state.expect_request(pending_request);
		drop(offchain_state);

		let era_id = 1;
		let cluster_id = ClusterId::from([1; 20]);
		let node_params = StorageNodeParams {
			ssl: false,
			host: host.as_bytes().to_vec(),
			http_port: port,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		};

		let result = Pallet::<Test>::fetch_node_usage(&cluster_id, era_id, &node_params);
		assert!(result.is_ok());
		let activities = result.unwrap();
		assert_eq!(activities[0].number_of_gets, node_activity1.number_of_gets);
		assert_eq!(activities[0].number_of_puts, node_activity1.number_of_puts);
		assert_eq!(activities[0].transferred_bytes, node_activity1.transferred_bytes);
		assert_eq!(activities[0].stored_bytes, node_activity1.stored_bytes);

		assert_eq!(activities[1].number_of_gets, node_activity2.number_of_gets);
		assert_eq!(activities[1].number_of_puts, node_activity2.number_of_puts);
		assert_eq!(activities[1].transferred_bytes, node_activity2.transferred_bytes);
		assert_eq!(activities[1].stored_bytes, node_activity2.stored_bytes);
	});
}

#[test]
fn fetch_customers_usage_works() {
	let mut ext = TestExternalities::default();
	let (offchain, offchain_state) = TestOffchainExt::new();
	let (pool, _) = TestTransactionPoolExt::new();

	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(OffchainDbExt::new(Box::new(offchain)));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		let mut offchain_state = offchain_state.write();
		offchain_state.timestamp = Timestamp::from_unix_millis(0);
		let host = "example.com";
		let port = 80;
		let era_id = 1;

		// Create a sample NodeActivity instance
		let customer_activity1 = CustomerActivity {
			bucket_id: 111,
			customer_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		};
		let customer_activity2 = CustomerActivity {
			bucket_id: 222,
			customer_id: "2".to_string(),
			stored_bytes: 1000,
			transferred_bytes: 500,
			number_of_puts: 100,
			number_of_gets: 200,
		};
		let customers_activity_json =
			serde_json::to_string(&vec![customer_activity1.clone(), customer_activity2.clone()])
				.unwrap();

		// Mock HTTP request and response
		let pending_request = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId={}", host, port, era_id),
			response: Some(customers_activity_json.as_bytes().to_vec()),
			sent: true,
			..Default::default()
		};
		offchain_state.expect_request(pending_request);
		drop(offchain_state);

		let era_id = 1;
		let cluster_id = ClusterId::from([1; 20]);
		let node_params = StorageNodeParams {
			ssl: false,
			host: host.as_bytes().to_vec(),
			http_port: port,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		};

		let result = Pallet::<Test>::fetch_customers_usage(&cluster_id, era_id, &node_params);
		assert!(result.is_ok());
		let activities = result.unwrap();
		assert_eq!(activities[0].number_of_gets, customer_activity1.number_of_gets);
		assert_eq!(activities[0].number_of_puts, customer_activity1.number_of_puts);
		assert_eq!(activities[0].transferred_bytes, customer_activity1.transferred_bytes);
		assert_eq!(activities[0].stored_bytes, customer_activity1.stored_bytes);

		assert_eq!(activities[1].number_of_gets, customer_activity2.number_of_gets);
		assert_eq!(activities[1].number_of_puts, customer_activity2.number_of_puts);
		assert_eq!(activities[1].transferred_bytes, customer_activity2.transferred_bytes);
		assert_eq!(activities[1].stored_bytes, customer_activity2.stored_bytes);
	});
}

#[test]
fn test_reach_consensus_empty() {
	let activities: Vec<CustomerActivity> = Vec::new();
	let result = DdcVerification::reach_consensus(&activities, 3);
	assert!(result.is_none());
}

#[test]
fn test_reach_consensus_success() {
	let activities = vec![
		CustomerActivity {
			customer_id: "0".to_string(),
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: "0".to_string(),
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: "0".to_string(),
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
	];
	let result = DdcVerification::reach_consensus(&activities, 3);
	assert!(result.is_some());
	assert_eq!(result.unwrap().stored_bytes, 100);
}

#[test]
fn test_reach_consensus_failure() {
	let activities = vec![
		CustomerActivity {
			customer_id: "0".to_string(),
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: "0".to_string(),
			bucket_id: 1,
			stored_bytes: 200,
			transferred_bytes: 100,
			number_of_puts: 20,
			number_of_gets: 40,
		},
		CustomerActivity {
			customer_id: "0".to_string(),
			bucket_id: 1,
			stored_bytes: 300,
			transferred_bytes: 150,
			number_of_puts: 30,
			number_of_gets: 60,
		},
	];
	let result = DdcVerification::reach_consensus(&activities, 3);
	assert!(result.is_none());
}

#[test]
fn test_reach_consensus_threshold() {
	let activities = vec![
		CustomerActivity {
			customer_id: "0".to_string(),
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: "0".to_string(),
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: "0".to_string(),
			bucket_id: 1,
			stored_bytes: 200,
			transferred_bytes: 100,
			number_of_puts: 20,
			number_of_gets: 40,
		},
	];

	let mut result = DdcVerification::reach_consensus(&activities, 2);
	assert!(result.is_some());
	assert_eq!(result.unwrap().stored_bytes, 100);
	result = DdcVerification::reach_consensus(&activities, 3);
	assert!(result.is_none());
}

#[test]
fn test_reach_consensus_exact_threshold() {
	let activities = vec![
		CustomerActivity {
			customer_id: "0".to_string(),
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: "0".to_string(),
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
	];
	let result = DdcVerification::reach_consensus(&activities, 3);
	assert!(result.is_none());
}

#[test]
fn test_get_consensus_customers_activity_success() {
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);

	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));
	let node_pubkey_2 = NodePubKey::StoragePubKey(AccountId32::new([2; 32]));

	let customers_activity = vec![
		(
			node_pubkey_0,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_2,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id,
		era_id,
		&customers_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_ok());
	let consensus_activities = result.unwrap();
	assert_eq!(consensus_activities.len(), 1);
	assert_eq!(consensus_activities[0].stored_bytes, 100);
}

#[test]
fn test_get_consensus_customers_activity_success2() {
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);

	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));
	let node_pubkey_2 = NodePubKey::StoragePubKey(AccountId32::new([2; 32]));

	let customers_activity = vec![
		(
			node_pubkey_0.clone(),
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1.clone(),
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_2.clone(),
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_0,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 2,
				stored_bytes: 110,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 2,
				stored_bytes: 110,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_2,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 2,
				stored_bytes: 110,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id,
		era_id,
		&customers_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_ok());
	let consensus_activities = result.unwrap();
	assert_eq!(consensus_activities.len(), 2);
	assert_eq!(consensus_activities[1].stored_bytes, 100);
	assert_eq!(consensus_activities[1].bucket_id, 1);
	assert_eq!(consensus_activities[0].stored_bytes, 110);
	assert_eq!(consensus_activities[0].bucket_id, 2);
}

#[test]
fn test_get_consensus_nodes_activity_success() {
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);

	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));
	let node_pubkey_2 = NodePubKey::StoragePubKey(AccountId32::new([2; 32]));

	let customers_activity = vec![
		(
			node_pubkey_0,
			vec![NodeActivity {
				provider_id: "0".to_string(),
				node_id: "0".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![NodeActivity {
				provider_id: "0".to_string(),
				node_id: "0".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_2,
			vec![NodeActivity {
				provider_id: "0".to_string(),
				node_id: "0".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id,
		era_id,
		&customers_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_ok());
	let consensus_activities = result.unwrap();
	assert_eq!(consensus_activities.len(), 1);
	assert_eq!(consensus_activities[0].stored_bytes, 100);
}

#[test]
fn test_get_consensus_customers_activity_empty() {
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);

	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));
	let node_pubkey_2 = NodePubKey::StoragePubKey(AccountId32::new([2; 32]));

	let customers_activity = vec![
		(node_pubkey_0.clone(), Vec::<CustomerActivity>::new()),
		(node_pubkey_1.clone(), Vec::<CustomerActivity>::new()),
		(node_pubkey_2.clone(), Vec::<CustomerActivity>::new()),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id,
		era_id,
		&customers_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_ok());
	let consensus_activities = result.unwrap();
	assert_eq!(consensus_activities.len(), 0);
}

#[test]
fn test_get_consensus_customers_activity_not_enough_nodes() {
	let cluster_id1 = ClusterId::from([1; 20]);
	let era_id1 = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);
	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));

	let customers_activity = vec![
		(
			node_pubkey_0,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id1,
		era_id1,
		&customers_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_err());
	let errors = result.err().unwrap();
	assert_eq!(errors.len(), 1);
	match &errors[0] {
		OCWError::NotEnoughNodesForConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, customers_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected NotEnoughNodes error"),
	}
}

#[test]
fn test_get_consensus_nodes_activity_not_enough_nodes() {
	let cluster_id1 = ClusterId::from([1; 20]);
	let era_id1 = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);
	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));

	let nodes_activity = vec![
		(
			node_pubkey_0,
			vec![NodeActivity {
				provider_id: "0".to_string(),
				node_id: "0".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![NodeActivity {
				provider_id: "0".to_string(),
				node_id: "0".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id1,
		era_id1,
		&nodes_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_err());
	let errors = result.err().unwrap();
	assert_eq!(errors.len(), 1);
	match &errors[0] {
		OCWError::NotEnoughNodesForConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, nodes_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected NotEnoughNodes error"),
	}
}

#[test]
fn test_get_consensus_customers_activity_not_in_consensus() {
	let cluster_id1 = ClusterId::from([1; 20]);
	let era_id1 = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);

	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));
	let node_pubkey_2 = NodePubKey::StoragePubKey(AccountId32::new([2; 32]));

	let customers_activity = vec![
		(
			node_pubkey_0,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 300,
				transferred_bytes: 150,
				number_of_puts: 30,
				number_of_gets: 60,
			}],
		),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id1,
		era_id1,
		&customers_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_err());
	let errors = result.err().unwrap();
	assert_eq!(errors.len(), 1);
	match &errors[0] {
		OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, customers_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
}

#[test]
fn test_get_consensus_customers_activity_not_in_consensus_2() {
	let cluster_id1 = ClusterId::from([1; 20]);
	let era_id1 = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);

	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));
	let node_pubkey_2 = NodePubKey::StoragePubKey(AccountId32::new([2; 32]));

	let customers_activity = vec![
		(
			node_pubkey_0.clone(),
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1.clone(),
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2.clone(),
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 300,
				transferred_bytes: 150,
				number_of_puts: 30,
				number_of_gets: 60,
			}],
		),
		(
			node_pubkey_0,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 2,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 2,
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 2,
				stored_bytes: 300,
				transferred_bytes: 150,
				number_of_puts: 30,
				number_of_gets: 60,
			}],
		),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id1,
		era_id1,
		&customers_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_err());
	let errors = result.err().unwrap();
	assert_eq!(errors.len(), 2);
	match &errors[1] {
		OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, customers_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
	match &errors[0] {
		OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, customers_activity[3].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
}

#[test]
fn test_get_consensus_customers_activity_diff_errors() {
	let cluster_id1 = ClusterId::from([1; 20]);
	let era_id1 = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);

	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));
	let node_pubkey_2 = NodePubKey::StoragePubKey(AccountId32::new([2; 32]));

	let customers_activity = vec![
		(
			node_pubkey_0.clone(),
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1.clone(),
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2.clone(),
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 1,
				stored_bytes: 300,
				transferred_bytes: 150,
				number_of_puts: 30,
				number_of_gets: 60,
			}],
		),
		(
			node_pubkey_0,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 2,
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				customer_id: "0".to_string(),
				bucket_id: 2,
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id1,
		era_id1,
		&customers_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_err());
	let errors = result.err().unwrap();
	assert_eq!(errors.len(), 2);
	match &errors[1] {
		OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, customers_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
	match &errors[0] {
		OCWError::NotEnoughNodesForConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, customers_activity[3].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
}

#[test]
fn test_get_consensus_nodes_activity_not_in_consensus() {
	let cluster_id1 = ClusterId::from([1; 20]);
	let era_id1 = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);

	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));
	let node_pubkey_2 = NodePubKey::StoragePubKey(AccountId32::new([2; 32]));

	let nodes_activity = vec![
		(
			node_pubkey_0,
			vec![NodeActivity {
				node_id: "0".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![NodeActivity {
				node_id: "0".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2,
			vec![NodeActivity {
				node_id: "0".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 300,
				transferred_bytes: 150,
				number_of_puts: 30,
				number_of_gets: 60,
			}],
		),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id1,
		era_id1,
		&nodes_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_err());
	let errors = result.err().unwrap();
	assert_eq!(errors.len(), 1);
	match &errors[0] {
		OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, nodes_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
}

#[test]
fn test_convert_to_batch_merkle_roots() {
	let nodes = get_node_activities();
	let activities_batch_1 = vec![nodes[0].clone(), nodes[1].clone(), nodes[2].clone()];
	let activities_batch_2 = vec![nodes[3].clone(), nodes[4].clone()];
	let cluster_id = ClusterId::default();
	let era_id_1 = 1;

	let result_roots = DdcVerification::convert_to_batch_merkle_roots(
		&cluster_id,
		era_id_1,
		vec![activities_batch_1.clone(), activities_batch_2.clone()],
	)
	.unwrap();
	let expected_roots: Vec<ActivityHash> = vec![
		DdcVerification::create_merkle_root(
			&cluster_id,
			era_id_1,
			&activities_batch_1.iter().map(|a| a.hash::<mock::Test>()).collect::<Vec<_>>(),
		)
		.unwrap(),
		DdcVerification::create_merkle_root(
			&cluster_id,
			era_id_1,
			&activities_batch_2.iter().map(|a| a.hash::<mock::Test>()).collect::<Vec<_>>(),
		)
		.unwrap(),
	];

	assert_eq!(result_roots, expected_roots);
}

#[test]
fn test_convert_to_batch_merkle_roots_empty() {
	let cluster_id = ClusterId::default();
	let era_id_1 = 1;
	let result_roots = DdcVerification::convert_to_batch_merkle_roots(
		&cluster_id,
		era_id_1,
		Vec::<Vec<NodeActivity>>::new(),
	)
	.unwrap();
	let expected_roots: Vec<ActivityHash> = Vec::<ActivityHash>::new();

	assert_eq!(result_roots, expected_roots);
}

#[test]
fn test_split_to_batches_empty_activities() {
	let activities: Vec<NodeActivity> = vec![];
	let result = DdcVerification::split_to_batches(&activities, 3);
	assert_eq!(result, Vec::<Vec<NodeActivity>>::new());
}

#[test]
fn test_split_to_batches_single_batch() {
	let nodes = get_node_activities();
	let activities = vec![nodes[0].clone(), nodes[1].clone(), nodes[2].clone()];
	let mut sorted_activities = vec![nodes[0].clone(), nodes[1].clone(), nodes[2].clone()];

	sorted_activities.sort();
	let result = DdcVerification::split_to_batches(&activities, 5);
	assert_eq!(result, vec![sorted_activities]);
}

#[test]
fn test_split_to_batches_exact_batches() {
	let nodes = get_node_activities();
	let activities = vec![nodes[0].clone(), nodes[1].clone(), nodes[2].clone(), nodes[3].clone()];
	let mut sorted_activities =
		vec![nodes[0].clone(), nodes[1].clone(), nodes[2].clone(), nodes[3].clone()];
	sorted_activities.sort();
	let result = DdcVerification::split_to_batches(&activities, 2);
	assert_eq!(
		result,
		vec![
			[sorted_activities[0].clone(), sorted_activities[1].clone()],
			[sorted_activities[2].clone(), sorted_activities[3].clone()]
		]
	);
}
#[test]
#[allow(clippy::vec_init_then_push)]
fn test_split_to_batches_non_exact_batches() {
	let nodes = get_node_activities();
	let activities = vec![
		nodes[0].clone(),
		nodes[1].clone(),
		nodes[2].clone(),
		nodes[3].clone(),
		nodes[4].clone(),
	];
	let mut sorted_activities = vec![
		nodes[0].clone(),
		nodes[1].clone(),
		nodes[2].clone(),
		nodes[3].clone(),
		nodes[4].clone(),
	];
	sorted_activities.sort();
	let result = DdcVerification::split_to_batches(&activities, 2);
	let mut expected: Vec<Vec<NodeActivity>> = Vec::new();
	expected.push(vec![sorted_activities[0].clone(), sorted_activities[1].clone()]);
	expected.push(vec![sorted_activities[2].clone(), sorted_activities[3].clone()]);
	expected.push(vec![sorted_activities[4].clone()]);

	assert_eq!(result, expected);
}

#[test]
fn test_get_consensus_nodes_activity_not_in_consensus2() {
	let cluster_id1 = ClusterId::from([1; 20]);
	let era_id1 = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);

	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));
	let node_pubkey_2 = NodePubKey::StoragePubKey(AccountId32::new([2; 32]));

	let nodes_activity = vec![
		(
			node_pubkey_0.clone(),
			vec![NodeActivity {
				node_id: "0".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1.clone(),
			vec![NodeActivity {
				node_id: "0".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2.clone(),
			vec![NodeActivity {
				node_id: "0".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 300,
				transferred_bytes: 150,
				number_of_puts: 30,
				number_of_gets: 60,
			}],
		),
		(
			node_pubkey_0,
			vec![NodeActivity {
				node_id: "1".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![NodeActivity {
				node_id: "1".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2,
			vec![NodeActivity {
				node_id: "1".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 300,
				transferred_bytes: 150,
				number_of_puts: 30,
				number_of_gets: 60,
			}],
		),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id1,
		era_id1,
		&nodes_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_err());
	let errors = result.err().unwrap();
	assert_eq!(errors.len(), 2);
	match &errors[0] {
		OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, nodes_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
	match &errors[1] {
		OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, nodes_activity[3].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
}

#[test]
fn test_get_consensus_nodes_activity_diff_errors() {
	let cluster_id1 = ClusterId::from([1; 20]);
	let era_id1 = 1;
	let min_nodes = 3;
	let threshold = Percent::from_percent(67);

	let node_pubkey_0 = NodePubKey::StoragePubKey(AccountId32::new([0; 32]));
	let node_pubkey_1 = NodePubKey::StoragePubKey(AccountId32::new([1; 32]));
	let node_pubkey_2 = NodePubKey::StoragePubKey(AccountId32::new([2; 32]));

	let nodes_activity = vec![
		(
			node_pubkey_0.clone(),
			vec![NodeActivity {
				node_id: "0".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1.clone(),
			vec![NodeActivity {
				node_id: "0".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2.clone(),
			vec![NodeActivity {
				node_id: "0".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 300,
				transferred_bytes: 150,
				number_of_puts: 30,
				number_of_gets: 60,
			}],
		),
		(
			node_pubkey_0,
			vec![NodeActivity {
				node_id: "1".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![NodeActivity {
				node_id: "1".to_string(),
				provider_id: "0".to_string(),
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
	];

	let result = DdcVerification::get_consensus_for_activities(
		&cluster_id1,
		era_id1,
		&nodes_activity,
		min_nodes,
		threshold,
	);
	assert!(result.is_err());
	let errors = result.err().unwrap();
	assert_eq!(errors.len(), 2);
	match &errors[0] {
		OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, nodes_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
	match &errors[1] {
		OCWError::NotEnoughNodesForConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, nodes_activity[3].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
}

#[test]
fn fetch_processed_era_works() {
	let mut ext = TestExternalities::default();
	let (offchain, offchain_state) = TestOffchainExt::new();
	let (pool, _) = TestTransactionPoolExt::new();

	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(OffchainDbExt::new(Box::new(offchain)));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		let mut offchain_state = offchain_state.write();
		offchain_state.timestamp = Timestamp::from_unix_millis(0);
		let host = "example1.com";
		let port = 80;

		// Create a sample EraActivity instance
		let era_activity1 = EraActivity { id: 17, start: 1, end: 2 };
		let era_activity2 = EraActivity { id: 18, start: 1, end: 2 };
		let era_activity3 = EraActivity { id: 19, start: 1, end: 2 };
		let era_activity_json = serde_json::to_string(&vec![
			era_activity1.clone(),
			era_activity2.clone(),
			era_activity3,
		])
		.unwrap();

		// Mock HTTP request and response
		let pending_request = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host, port),
			response: Some(era_activity_json.as_bytes().to_vec()),
			sent: true,
			..Default::default()
		};
		offchain_state.expect_request(pending_request);
		drop(offchain_state);

		let node_params = StorageNodeParams {
			ssl: false,
			host: host.as_bytes().to_vec(),
			http_port: port,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		};

		let result = Pallet::<Test>::fetch_processed_era(&node_params);
		assert!(result.is_ok());
		let activities = result.unwrap();
		assert_eq!(activities[0].id, era_activity1.id);
		assert_eq!(activities[1].id, era_activity2.id);
	});
}

#[test]
fn get_era_for_validation_works() {
	let mut ext = TestExternalities::default();
	let (offchain, offchain_state) = TestOffchainExt::new();
	let (pool, _) = TestTransactionPoolExt::new();

	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(OffchainDbExt::new(Box::new(offchain.clone())));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		let key = format!("offchain::validator::{:?}", KEY_TYPE).into_bytes();

		let mut offchain_state = offchain_state.write();
		offchain_state.persistent_storage.set(
			b"",
			&key,
			b"9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a".as_ref(),
		);
		offchain_state.timestamp = Timestamp::from_unix_millis(0);
		let host1 = "example1.com";
		let host2 = "example2.com";
		let host3 = "example3.com";
		let host4 = "example4.com";
		let port = 80;
		let era_activity1 = EraActivity { id: 16, start: 1, end: 2 };
		let era_activity2 = EraActivity { id: 17, start: 1, end: 2 };
		let era_activity3 = EraActivity { id: 18, start: 1, end: 2 };
		let era_activity4 = EraActivity { id: 19, start: 1, end: 2 };
		let era_activity_json1 = serde_json::to_string(&vec![
			era_activity1.clone(), //16
			era_activity2.clone(), //17
			era_activity3.clone(), //18
			era_activity4.clone(), //19
		])
		.unwrap();
		let era_activity_json2 = serde_json::to_string(&vec![
			era_activity1.clone(), //16
			era_activity2.clone(), //17
			era_activity3.clone(), //18
		])
		.unwrap();
		let era_activity_json3 = serde_json::to_string(&vec![
			era_activity1.clone(), //16
			era_activity2.clone(), //17
			era_activity3.clone(), //18
		])
		.unwrap();
		let era_activity_json4 = serde_json::to_string(&vec![
			era_activity1.clone(), //16
			era_activity2.clone(), //17
			era_activity3.clone(), //18
		])
		.unwrap();
		let pending_request1 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host1, port),
			response: Some(era_activity_json1.as_bytes().to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host2, port),
			response: Some(era_activity_json2.as_bytes().to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host3, port),
			response: Some(era_activity_json3.as_bytes().to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host4, port),
			response: Some(era_activity_json4.as_bytes().to_vec()),
			sent: true,
			..Default::default()
		};
		offchain_state.expect_request(pending_request1);
		offchain_state.expect_request(pending_request2);
		offchain_state.expect_request(pending_request3);
		offchain_state.expect_request(pending_request4);

		drop(offchain_state);

		let node_params1 = StorageNodeParams {
			ssl: false,
			host: host1.as_bytes().to_vec(),
			http_port: port,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		};

		let node_params2 = StorageNodeParams {
			ssl: false,
			host: host2.as_bytes().to_vec(),
			http_port: port,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example3.com".to_vec(),
		};

		let node_params3 = StorageNodeParams {
			ssl: false,
			host: host3.as_bytes().to_vec(),
			http_port: port,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example4.com".to_vec(),
		};

		let node_params4 = StorageNodeParams {
			ssl: false,
			host: host4.as_bytes().to_vec(),
			http_port: port,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example5.com".to_vec(),
		};

		let dac_nodes: Vec<(NodePubKey, StorageNodeParams)> = vec![
			(NodePubKey::StoragePubKey(StorageNodePubKey::new([1; 32])), node_params1),
			(NodePubKey::StoragePubKey(StorageNodePubKey::new([2; 32])), node_params2),
			(NodePubKey::StoragePubKey(StorageNodePubKey::new([3; 32])), node_params3),
			(NodePubKey::StoragePubKey(StorageNodePubKey::new([4; 32])), node_params4),
		];

		let cluster_id = ClusterId::from([12; 20]);
		let result = Pallet::<Test>::get_era_for_validation(&cluster_id, &dac_nodes);
		assert_eq!(result.unwrap().unwrap(), era_activity1); //16
	});
}

#[test]
fn test_get_last_validated_era() {
	let cluster_id1 = ClusterId::from([12; 20]);
	let cluster_id2 = ClusterId::from([13; 20]);
	let era_1 = 1;
	let era_2 = 2;
	let payers_root: ActivityHash = [1; 32];
	let payees_root: ActivityHash = [2; 32];
	let validators = get_validators();

	new_test_ext().execute_with(|| {
		assert_ok!(Pallet::<Test>::get_last_validated_era(&cluster_id1, validators[0].clone())
			.map(|era| {
				assert_eq!(era, None);
			}));

		let mut validators_map_1 = BTreeMap::new();
		validators_map_1.insert(
			(payers_root, payees_root),
			vec![validators[1].clone(), validators[2].clone(), validators[3].clone()],
		);

		let validation_1 = EraValidation {
			validators: validators_map_1,
			start_era: 1,
			end_era: 2,
			payers_merkle_root_hash: payers_root,
			payees_merkle_root_hash: payees_root,
			status: EraValidationStatus::ValidatingData,
		};

		<EraValidations<Test>>::insert(cluster_id1, era_1, validation_1);

		// still no - different accountid
		assert_ok!(Pallet::<Test>::get_last_validated_era(&cluster_id1, validators[0].clone())
			.map(|era| {
				assert_eq!(era, None);
			}));

		// still no - different cluster id
		assert_ok!(Pallet::<Test>::get_last_validated_era(&cluster_id2, validators[1].clone())
			.map(|era| {
				assert_eq!(era, None);
			}));

		let mut validators_map_2 = BTreeMap::new();
		validators_map_2
			.insert((payers_root, payees_root), vec![validators[2].clone(), validators[3].clone()]);

		let validation_2 = EraValidation {
			validators: validators_map_2,
			start_era: 1,
			end_era: 2,
			payers_merkle_root_hash: payers_root,
			payees_merkle_root_hash: payees_root,
			status: EraValidationStatus::ValidatingData,
		};

		<EraValidations<Test>>::insert(cluster_id1, era_2, validation_2);

		// Now the last validated era should be ERA_2
		assert_ok!(Pallet::<Test>::get_last_validated_era(&cluster_id1, validators[2].clone())
			.map(|era| {
				assert_eq!(era, Some(era_2));
			}));

		assert_ok!(Pallet::<Test>::get_last_validated_era(&cluster_id1, validators[1].clone())
			.map(|era| {
				assert_eq!(era, Some(era_1));
			}));
	});
}

#[test]
fn test_get_era_for_payout() {
	// Initialize test data
	let cluster_id = ClusterId::default(); // Replace with actual initialization
	let status = EraValidationStatus::ReadyForPayout; // Test with different statuses

	// Insert some era validations into storage
	let era_id_1 = 1;
	let era_id_2 = 2;
	let era_validation_1 = EraValidation::<Test> {
		validators: Default::default(),
		start_era: 0,
		end_era: 0,
		payers_merkle_root_hash: Default::default(),
		payees_merkle_root_hash: Default::default(),
		status: EraValidationStatus::ReadyForPayout,
	};
	let era_validation_2 = EraValidation::<Test> {
		validators: Default::default(),
		start_era: 0,
		end_era: 0,
		payers_merkle_root_hash: Default::default(),
		payees_merkle_root_hash: Default::default(),
		status: EraValidationStatus::PayoutInProgress,
	};

	new_test_ext().execute_with(|| {
		EraValidations::<Test>::insert(cluster_id, era_id_1, &era_validation_1);
		EraValidations::<Test>::insert(cluster_id, era_id_2, &era_validation_2);

		let mut result = Pallet::<Test>::get_era_for_payout(&cluster_id, status);
		assert_eq!(result, Some((era_id_1, 0, 0)));

		result =
			Pallet::<Test>::get_era_for_payout(&cluster_id, EraValidationStatus::PayoutSuccess);
		assert_eq!(result, None);
	});
}

#[test]
fn create_merkle_root_works() {
	new_test_ext().execute_with(|| {
		let a: ActivityHash = [0; 32];
		let b: ActivityHash = [1; 32];
		let c: ActivityHash = [2; 32];
		let d: ActivityHash = [3; 32];
		let e: ActivityHash = [4; 32];
		let cluster_id = ClusterId::default();
		let era_id_1 = 1;

		let leaves = vec![a, b, c, d, e];

		let root = DdcVerification::create_merkle_root(&cluster_id, era_id_1, &leaves).unwrap();

		assert_eq!(
			root,
			[
				205, 34, 92, 22, 66, 39, 53, 146, 126, 111, 191, 174, 107, 224, 161, 127, 150, 69,
				255, 15, 237, 252, 116, 39, 186, 26, 40, 154, 180, 110, 185, 7
			]
		);
	});
}

#[test]
fn create_merkle_root_empty() {
	new_test_ext().execute_with(|| {
		let cluster_id = ClusterId::default();
		let era_id_1 = 1;
		let leaves = Vec::<ActivityHash>::new();
		let root = DdcVerification::create_merkle_root(&cluster_id, era_id_1, &leaves).unwrap();

		assert_eq!(root, ActivityHash::default());
	});
}

#[test]
fn proof_merkle_leaf_works() {
	new_test_ext().execute_with(|| {
		let a: ActivityHash = [0; 32];
		let b: ActivityHash = [1; 32];
		let c: ActivityHash = [2; 32];
		let d: ActivityHash = [3; 32];
		let e: ActivityHash = [4; 32];
		let f: ActivityHash = [5; 32];

		let leaves = [a, b, c, d, e];
		let store = MemStore::default();
		let mut mmr: MMR<ActivityHash, MergeActivityHash, &MemStore<ActivityHash>> =
			MemMMR::<_, MergeActivityHash>::new(0, &store);
		let leaf_position_map: Vec<(ActivityHash, u64)> =
			leaves.iter().map(|a| (*a, mmr.push(*a).unwrap())).collect();

		let leaf_position: Vec<(u64, ActivityHash)> = leaf_position_map
			.iter()
			.filter(|&(l, _)| l == &c)
			.map(|&(ref l, p)| (p, *l))
			.collect();
		let position: Vec<u64> = leaf_position.clone().into_iter().map(|(p, _)| p).collect();
		let root = mmr.get_root().unwrap();

		assert_eq!(leaf_position.len(), 1);
		assert_eq!(position.len(), 1);
		assert!(DdcVerification::proof_merkle_leaf(
			root,
			&MMRProof {
				mmr_size: mmr.mmr_size(),
				proof: mmr.gen_proof(position.clone()).unwrap().proof_items().to_vec(),
				leaf_with_position: leaf_position[0]
			}
		)
		.unwrap());

		assert_noop!(
			DdcVerification::proof_merkle_leaf(
				root,
				&MMRProof {
					mmr_size: 0,
					proof: mmr.gen_proof(position).unwrap().proof_items().to_vec(),
					leaf_with_position: (6, f)
				}
			),
			Error::<Test>::FailToVerifyMerkleProof
		);
	});
}

#[test]
fn test_single_ocw_pallet_integration() {
	let mut ext = new_test_ext();
	let (offchain, offchain_state) = TestOffchainExt::new();
	let (pool, _pool_state) = TestTransactionPoolExt::new();

	let (pair, _seed) = sp_core::sr25519::Pair::from_phrase(
		"spider sell nice animal border success square soda stem charge caution echo",
		None,
	)
	.unwrap();
	let keystore = MemoryKeystore::new();
	keystore
		.insert(
			KEY_TYPE,
			"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa318",
			pair.public().as_ref(),
		)
		.unwrap();

	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(OffchainDbExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));
	ext.register_extension(KeystoreExt::new(keystore));

	ext.execute_with(|| {
		let mut offchain_state = offchain_state.write();
		let key = format!("offchain::validator::{:?}", KEY_TYPE).into_bytes();
		offchain_state.persistent_storage.set(
			b"",
			&key,
			b"9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a".as_ref(),
		);
		offchain_state.timestamp = Timestamp::from_unix_millis(0);
		let host1 = "178.251.228.236";
		let host2 = "95.217.8.119";
		let host3 = "178.251.228.42";
		let host4 = "37.27.30.47";
		let host5 = "178.251.228.49";
		let host6 = "159.69.207.65";
		let host7 = "178.251.228.165";
		let host8 = "49.13.211.157";
		let host9 = "178.251.228.44";
		let port = 8080;

		let pending_request1 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host1, port),
			response: Some(br#"[{"id":476814,"start":0,"end":1716533999999,"processing_time_ms":0,"total_records":0,"total_buckets":0},{"id":476815,"start":1716534000000,"end":1716537599999,"processing_time_ms":2,"total_records":54,"total_buckets":2},{"id":476816,"start":1716537600000,"end":1716541199999,"processing_time_ms":10,"total_records":803,"total_buckets":29},{"id":476817,"start":1716541200000,"end":1716544799999,"processing_time_ms":11,"total_records":986,"total_buckets":28}]"#.to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host2, port),
			response: Some(br#"[{"id":476814,"start":0,"end":1716533999999,"processing_time_ms":0,"total_records":0,"total_buckets":0},{"id":476815,"start":1716534000000,"end":1716537599999,"processing_time_ms":2,"total_records":54,"total_buckets":2},{"id":476816,"start":1716537600000,"end":1716541199999,"processing_time_ms":10,"total_records":803,"total_buckets":29},{"id":476817,"start":1716541200000,"end":1716544799999,"processing_time_ms":11,"total_records":986,"total_buckets":28}]"#.to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host3, port),
			response: Some(br#"[{"id":476814,"start":0,"end":1716533999999,"processing_time_ms":0,"total_records":0,"total_buckets":0},{"id":476815,"start":1716534000000,"end":1716537599999,"processing_time_ms":2,"total_records":54,"total_buckets":2},{"id":476816,"start":1716537600000,"end":1716541199999,"processing_time_ms":10,"total_records":803,"total_buckets":29},{"id":476817,"start":1716541200000,"end":1716544799999,"processing_time_ms":11,"total_records":986,"total_buckets":28}]"#.to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host4, port),
			response: Some(br#"[{"id":476814,"start":0,"end":1716533999999,"processing_time_ms":0,"total_records":0,"total_buckets":0},{"id":476815,"start":1716534000000,"end":1716537599999,"processing_time_ms":2,"total_records":54,"total_buckets":2},{"id":476816,"start":1716537600000,"end":1716541199999,"processing_time_ms":10,"total_records":803,"total_buckets":29},{"id":476817,"start":1716541200000,"end":1716544799999,"processing_time_ms":11,"total_records":986,"total_buckets":28}]"#.to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request5 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host5, port),
			response: Some(br#"[{"id":476814,"start":0,"end":1716533999999,"processing_time_ms":0,"total_records":0,"total_buckets":0},{"id":476815,"start":1716534000000,"end":1716537599999,"processing_time_ms":2,"total_records":54,"total_buckets":2},{"id":476816,"start":1716537600000,"end":1716541199999,"processing_time_ms":10,"total_records":803,"total_buckets":29},{"id":476817,"start":1716541200000,"end":1716544799999,"processing_time_ms":11,"total_records":986,"total_buckets":28}]"#.to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request6 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host6, port),
			response: Some(br#"[{"id":476814,"start":0,"end":1716533999999,"processing_time_ms":0,"total_records":0,"total_buckets":0},{"id":476815,"start":1716534000000,"end":1716537599999,"processing_time_ms":2,"total_records":54,"total_buckets":2},{"id":476816,"start":1716537600000,"end":1716541199999,"processing_time_ms":10,"total_records":803,"total_buckets":29},{"id":476817,"start":1716541200000,"end":1716544799999,"processing_time_ms":11,"total_records":986,"total_buckets":28}]"#.to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request7 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host7, port),
			response: Some(br#"[{"id":476814,"start":0,"end":1716533999999,"processing_time_ms":0,"total_records":0,"total_buckets":0},{"id":476815,"start":1716534000000,"end":1716537599999,"processing_time_ms":2,"total_records":54,"total_buckets":2},{"id":476816,"start":1716537600000,"end":1716541199999,"processing_time_ms":10,"total_records":803,"total_buckets":29},{"id":476817,"start":1716541200000,"end":1716544799999,"processing_time_ms":11,"total_records":986,"total_buckets":28}]"#.to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request8 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host8, port),
			response: Some(br#"[{"id":476814,"start":0,"end":1716533999999,"processing_time_ms":0,"total_records":0,"total_buckets":0},{"id":476815,"start":1716534000000,"end":1716537599999,"processing_time_ms":2,"total_records":54,"total_buckets":2},{"id":476816,"start":1716537600000,"end":1716541199999,"processing_time_ms":10,"total_records":803,"total_buckets":29},{"id":476817,"start":1716541200000,"end":1716544799999,"processing_time_ms":11,"total_records":986,"total_buckets":28}]"#.to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request9 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/eras", host9, port),
			response: Some(br#"[{"id":476814,"start":0,"end":1716533999999,"processing_time_ms":0,"total_records":0,"total_buckets":0},{"id":476815,"start":1716534000000,"end":1716537599999,"processing_time_ms":2,"total_records":54,"total_buckets":2},{"id":476816,"start":1716537600000,"end":1716541199999,"processing_time_ms":10,"total_records":803,"total_buckets":29},{"id":476817,"start":1716541200000,"end":1716544799999,"processing_time_ms":11,"total_records":986,"total_buckets":28}]"#.to_vec()),
			sent: true,
			..Default::default()
		};


		let node_pending_request1 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476814", host1, port),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","provider_id": "0xf6a3e4c537ccee3dbac555ef6df371b7e48594f1fd4f05135914c42b03e63b61","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","provider_id": "0x8d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a9ef98ad9c3626ba725e7","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476814", host2, port),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","provider_id": "0xf6a3e4c537ccee3dbac555ef6df371b7e48594f1fd4f05135914c42b03e63b61","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","provider_id": "0x8d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a9ef98ad9c3626ba725e7","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476814", host3, port),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","provider_id": "0xf6a3e4c537ccee3dbac555ef6df371b7e48594f1fd4f05135914c42b03e63b61","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","provider_id": "0x8d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a9ef98ad9c3626ba725e7","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476814", host4, port),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","provider_id": "0xf6a3e4c537ccee3dbac555ef6df371b7e48594f1fd4f05135914c42b03e63b61","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","provider_id": "0x8d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a9ef98ad9c3626ba725e7","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request5 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476814", host5, port),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","provider_id": "0xf6a3e4c537ccee3dbac555ef6df371b7e48594f1fd4f05135914c42b03e63b61","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","provider_id": "0x8d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a9ef98ad9c3626ba725e7","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request6 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476814", host6, port),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","provider_id": "0xf6a3e4c537ccee3dbac555ef6df371b7e48594f1fd4f05135914c42b03e63b61","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","provider_id": "0x8d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a9ef98ad9c3626ba725e7","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request7 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476814", host7, port),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","provider_id": "0xf6a3e4c537ccee3dbac555ef6df371b7e48594f1fd4f05135914c42b03e63b61","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","provider_id": "0x8d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a9ef98ad9c3626ba725e7","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request8 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476814", host8, port),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","provider_id": "0xf6a3e4c537ccee3dbac555ef6df371b7e48594f1fd4f05135914c42b03e63b61","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","provider_id": "0x8d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a9ef98ad9c3626ba725e7","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request9 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476814", host9, port),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","provider_id": "0xf6a3e4c537ccee3dbac555ef6df371b7e48594f1fd4f05135914c42b03e63b61","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","provider_id": "0x8d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a9ef98ad9c3626ba725e7","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request1 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host1, port),
			response: Some(br#"[{"bucket_id": 90235,"customer_id": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1},{"bucket_id": 90236,"customer_id": "0x9cc588b1d749b6d727d665463641cfeb1c8c843e81faf468d21922d6296b6f45","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host2, port),
			response: Some(br#"[{"bucket_id": 90235,"customer_id": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1},{"bucket_id": 90236,"customer_id": "0x9cc588b1d749b6d727d665463641cfeb1c8c843e81faf468d21922d6296b6f45","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host3, port),
			response: Some(br#"[{"bucket_id": 90235,"customer_id": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1},{"bucket_id": 90236,"customer_id": "0x9cc588b1d749b6d727d665463641cfeb1c8c843e81faf468d21922d6296b6f45","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host4, port),
			response: Some(br#"[{"bucket_id": 90235,"customer_id": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1},{"bucket_id": 90236,"customer_id": "0x9cc588b1d749b6d727d665463641cfeb1c8c843e81faf468d21922d6296b6f45","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request5 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host5, port),
			response: Some(br#"[{"bucket_id": 90235,"customer_id": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1},{"bucket_id": 90236,"customer_id": "0x9cc588b1d749b6d727d665463641cfeb1c8c843e81faf468d21922d6296b6f45","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request6 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host6, port),
			response: Some(br#"[{"bucket_id": 90235,"customer_id": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1},{"bucket_id": 90236,"customer_id": "0x9cc588b1d749b6d727d665463641cfeb1c8c843e81faf468d21922d6296b6f45","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request7 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host7, port),
			response: Some(br#"[{"bucket_id": 90235,"customer_id": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1},{"bucket_id": 90236,"customer_id": "0x9cc588b1d749b6d727d665463641cfeb1c8c843e81faf468d21922d6296b6f45","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request8 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host8, port),
			response: Some(br#"[{"bucket_id": 90235,"customer_id": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1},{"bucket_id": 90236,"customer_id": "0x9cc588b1d749b6d727d665463641cfeb1c8c843e81faf468d21922d6296b6f45","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request9 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host9, port),
			response: Some(br#"[{"bucket_id": 90235,"customer_id": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1},{"bucket_id": 90236,"customer_id": "0x9cc588b1d749b6d727d665463641cfeb1c8c843e81faf468d21922d6296b6f45","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		offchain_state.expect_request(pending_request1);
		offchain_state.expect_request(pending_request2);
		offchain_state.expect_request(pending_request3);
		offchain_state.expect_request(pending_request4);
		offchain_state.expect_request(pending_request5);
		offchain_state.expect_request(pending_request6);
		offchain_state.expect_request(pending_request7);
		offchain_state.expect_request(pending_request8);
		offchain_state.expect_request(pending_request9);
		offchain_state.expect_request(node_pending_request1);
		offchain_state.expect_request(node_pending_request2);
		offchain_state.expect_request(node_pending_request3);
		offchain_state.expect_request(node_pending_request4);
		offchain_state.expect_request(node_pending_request5);
		offchain_state.expect_request(node_pending_request6);
		offchain_state.expect_request(node_pending_request7);
		offchain_state.expect_request(node_pending_request8);
		offchain_state.expect_request(node_pending_request9);
		offchain_state.expect_request(bucket_pending_request1);
		offchain_state.expect_request(bucket_pending_request2);
		offchain_state.expect_request(bucket_pending_request3);
		offchain_state.expect_request(bucket_pending_request4);
		offchain_state.expect_request(bucket_pending_request5);
		offchain_state.expect_request(bucket_pending_request6);
		offchain_state.expect_request(bucket_pending_request7);
		offchain_state.expect_request(bucket_pending_request8);
		offchain_state.expect_request(bucket_pending_request9);
		drop(offchain_state);

		// // Offchain worker should be triggered if block number is  divided by 100
		let block = 500;
		System::set_block_number(block);
		let cluster_id = ClusterId::from([12; 20]);

		ClusterToValidate::<Test>::put(cluster_id);
		DdcVerification::offchain_worker(block);
	});
}
