use ddc_primitives::{
	ClusterId, MergeActivityHash, StorageNodeMode, StorageNodeParams, StorageNodePubKey, KEY_TYPE,
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
	ValidatorSet::<Test>::put(validators.clone());

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
		stored_bytes: -100,
		transferred_bytes: 50,
		number_of_puts: 10,
		number_of_gets: 20,
	};
	let node2 = NodeActivity {
		node_id: "1".to_string(),
		stored_bytes: -101,
		transferred_bytes: 51,
		number_of_puts: 11,
		number_of_gets: 21,
	};
	let node3 = NodeActivity {
		node_id: "2".to_string(),
		stored_bytes: 102,
		transferred_bytes: 52,
		number_of_puts: 12,
		number_of_gets: 22,
	};
	let node4 = NodeActivity {
		node_id: "3".to_string(),
		stored_bytes: 103,
		transferred_bytes: 53,
		number_of_puts: 13,
		number_of_gets: 23,
	};
	let node5 = NodeActivity {
		node_id: "4".to_string(),
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
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		};
		let node_activity2 = NodeActivity {
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
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		};
		let customer_activity2 = CustomerActivity {
			bucket_id: 222,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 1000,
				transferred_bytes: 500,
				number_of_puts: 100,
				number_of_gets: 200,
			}],
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
		assert_eq!(
			activities[0].sub_aggregates[0].number_of_gets,
			customer_activity1.sub_aggregates[0].number_of_gets
		);
		assert_eq!(
			activities[0].sub_aggregates[0].number_of_puts,
			customer_activity1.sub_aggregates[0].number_of_puts
		);
		assert_eq!(
			activities[0].sub_aggregates[0].transferred_bytes,
			customer_activity1.sub_aggregates[0].transferred_bytes
		);
		assert_eq!(
			activities[0].sub_aggregates[0].stored_bytes,
			customer_activity1.sub_aggregates[0].stored_bytes
		);

		assert_eq!(
			activities[1].sub_aggregates[0].number_of_gets,
			customer_activity2.sub_aggregates[0].number_of_gets
		);
		assert_eq!(
			activities[1].sub_aggregates[0].number_of_puts,
			customer_activity2.sub_aggregates[0].number_of_puts
		);
		assert_eq!(
			activities[1].sub_aggregates[0].transferred_bytes,
			customer_activity2.sub_aggregates[0].transferred_bytes
		);
		assert_eq!(
			activities[1].sub_aggregates[0].stored_bytes,
			customer_activity2.sub_aggregates[0].stored_bytes
		);
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
			bucket_id: 1,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		},
		CustomerActivity {
			bucket_id: 1,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		},
		CustomerActivity {
			bucket_id: 1,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		},
	];
	let result = DdcVerification::reach_consensus(&activities, 3);
	assert!(result.is_some());
	assert_eq!(result.unwrap().sub_aggregates[0].stored_bytes, 100);
}

#[test]
fn test_reach_consensus_failure() {
	let activities = vec![
		CustomerActivity {
			bucket_id: 1,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		},
		CustomerActivity {
			bucket_id: 1,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		},
		CustomerActivity {
			bucket_id: 1,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 300,
				transferred_bytes: 150,
				number_of_puts: 30,
				number_of_gets: 60,
			}],
		},
	];
	let result = DdcVerification::reach_consensus(&activities, 3);
	assert!(result.is_none());
}

#[test]
fn test_reach_consensus_threshold() {
	let activities = vec![
		CustomerActivity {
			bucket_id: 1,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		},
		CustomerActivity {
			bucket_id: 1,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		},
		CustomerActivity {
			bucket_id: 1,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		},
	];

	let mut result = DdcVerification::reach_consensus(&activities, 2);
	assert!(result.is_some());
	assert_eq!(result.unwrap().sub_aggregates[0].stored_bytes, 100);
	result = DdcVerification::reach_consensus(&activities, 3);
	assert!(result.is_none());
}

#[test]
fn test_reach_consensus_exact_threshold() {
	let activities = vec![
		CustomerActivity {
			bucket_id: 1,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		},
		CustomerActivity {
			bucket_id: 1,
			sub_aggregates: vec![BucketSubAggregate {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
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
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_2,
			vec![CustomerActivity {
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
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
	assert_eq!(consensus_activities[0].sub_aggregates[0].stored_bytes, 100);
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
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_1.clone(),
			vec![CustomerActivity {
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_2.clone(),
			vec![CustomerActivity {
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 110,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_0,
			vec![CustomerActivity {
				bucket_id: 2,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 110,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				bucket_id: 2,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 110,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_2,
			vec![CustomerActivity {
				bucket_id: 2,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 110,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
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
	assert_eq!(consensus_activities[1].sub_aggregates[0].stored_bytes, 110);
	assert_eq!(consensus_activities[1].bucket_id, 2);
	assert_eq!(consensus_activities[0].sub_aggregates[0].stored_bytes, 100);
	assert_eq!(consensus_activities[0].bucket_id, 1);
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
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
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
	match &errors[0] {
		OCWError::NotEnoughBucketsForConsensus { cluster_id, era_id, bucket_id } => {
			assert_eq!(*bucket_id, 1);
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
	assert_eq!(errors.len(), 2);
	match &errors[0] {
		OCWError::NotEnoughNodesForConsensus { cluster_id, era_id, node_id } => {
			assert_eq!(*node_id, "0".to_string());
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
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 200,
					transferred_bytes: 100,
					number_of_puts: 20,
					number_of_gets: 40,
				}],
			}],
		),
		(
			node_pubkey_2,
			vec![CustomerActivity {
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 300,
					transferred_bytes: 150,
					number_of_puts: 30,
					number_of_gets: 60,
				}],
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
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_1.clone(),
			vec![CustomerActivity {
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 200,
					transferred_bytes: 100,
					number_of_puts: 20,
					number_of_gets: 40,
				}],
			}],
		),
		(
			node_pubkey_2.clone(),
			vec![CustomerActivity {
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 300,
					transferred_bytes: 150,
					number_of_puts: 30,
					number_of_gets: 60,
				}],
			}],
		),
		(
			node_pubkey_0,
			vec![CustomerActivity {
				bucket_id: 2,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				bucket_id: 2,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 200,
					transferred_bytes: 100,
					number_of_puts: 20,
					number_of_gets: 40,
				}],
			}],
		),
		(
			node_pubkey_2,
			vec![CustomerActivity {
				bucket_id: 2,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 300,
					transferred_bytes: 150,
					number_of_puts: 30,
					number_of_gets: 60,
				}],
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
			assert_eq!(*id, customers_activity[3].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
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
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_1.clone(),
			vec![CustomerActivity {
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 200,
					transferred_bytes: 100,
					number_of_puts: 20,
					number_of_gets: 40,
				}],
			}],
		),
		(
			node_pubkey_2.clone(),
			vec![CustomerActivity {
				bucket_id: 1,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 300,
					transferred_bytes: 150,
					number_of_puts: 30,
					number_of_gets: 60,
				}],
			}],
		),
		(
			node_pubkey_0,
			vec![CustomerActivity {
				bucket_id: 2,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 100,
					transferred_bytes: 50,
					number_of_puts: 10,
					number_of_gets: 20,
				}],
			}],
		),
		(
			node_pubkey_1,
			vec![CustomerActivity {
				bucket_id: 2,
				sub_aggregates: vec![BucketSubAggregate {
					NodeID: "1".to_string(),
					stored_bytes: 200,
					transferred_bytes: 100,
					number_of_puts: 20,
					number_of_gets: 40,
				}],
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
	assert_eq!(errors.len(), 3);
	match &errors[0] {
		OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, customers_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
	match &errors[1] {
		OCWError::NotEnoughBucketsForConsensus { cluster_id, era_id, bucket_id } => {
			assert_eq!(*bucket_id, 2);
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
	assert_eq!(errors.len(), 3);
	match &errors[0] {
		OCWError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, nodes_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
	match &errors[1] {
		OCWError::NotEnoughNodesForConsensus { cluster_id, era_id, node_id } => {
			assert_eq!(*node_id, "1".to_string());
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
			response: Some(br#"[{"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host2, port),
			response: Some(br#"[{"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host3, port),
			response: Some(br#"[{"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host4, port),
			response: Some(br#"[{"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request5 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host5, port),
			response: Some(br#"[{"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request6 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host6, port),
			response: Some(br#"[{"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request7 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host7, port),
			response: Some(br#"[{"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request8 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host8, port),
			response: Some(br#"[{"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request9 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476814", host9, port),
			response: Some(br#"[{"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
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

#[test]
fn fetch_reward_activities_works() {
	let cluster_id = ClusterId::from([12; 20]);
	let a: ActivityHash = [0; 32];
	let b: ActivityHash = [1; 32];
	let c: ActivityHash = [2; 32];
	let d: ActivityHash = [3; 32];
	let e: ActivityHash = [4; 32];

	let leaves = [a, b, c, d, e];
	let era_id = 1;
	let total_usage: i64 = 56;

	let result = DdcVerification::fetch_reward_activities(
		&cluster_id,
		era_id,
		get_node_activities(),
		leaves.to_vec(),
		total_usage,
	);

	let usage = get_node_activities().iter().fold(
		NodeUsage { transferred_bytes: 0, stored_bytes: 0, number_of_puts: 0, number_of_gets: 0 },
		|mut acc, activity| {
			acc.transferred_bytes += activity.transferred_bytes;
			acc.stored_bytes += activity.stored_bytes;
			acc.number_of_puts += activity.number_of_puts;
			acc.number_of_gets += activity.number_of_gets;
			acc
		},
	);

	let ex_result = NodeUsage {
		stored_bytes: total_usage + usage.stored_bytes,
		number_of_puts: usage.number_of_puts,
		number_of_gets: usage.number_of_gets,
		transferred_bytes: usage.transferred_bytes,
	};

	assert_eq!(result.unwrap(), Some((era_id, (leaves.len() - 1) as u16, ex_result)));
}

#[test]
fn test_bucket_node_aggregates() {
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

		let port = 8080;

		let pending_request1 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476817", host1, port),
			response: Some(br#"[{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa318","stored_bytes":578,"transferred_bytes":578,"number_of_puts":2,"number_of_gets":0}]},{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319","stored_bytes":0,"transferred_bytes":505,"number_of_puts":0,"number_of_gets":1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476817", host2, port),
			response: Some(br#"[{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa318","stored_bytes":578,"transferred_bytes":578,"number_of_puts":2,"number_of_gets":0}]},{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319","stored_bytes":0,"transferred_bytes":506,"number_of_puts":0,"number_of_gets":1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476817", host3, port),
			response: Some(br#"[{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa318","stored_bytes":578,"transferred_bytes":578,"number_of_puts":2,"number_of_gets":0}]},{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319","stored_bytes":0,"transferred_bytes":505,"number_of_puts":0,"number_of_gets":1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476817", host4, port),
			response: Some(br#"[{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};


		offchain_state.expect_request(pending_request1);
		offchain_state.expect_request(pending_request2);
		offchain_state.expect_request(pending_request3);
		offchain_state.expect_request(pending_request4);
		drop(offchain_state);

		let cluster_id = ClusterId::from([1; 20]);
		let era_id = 476817;
		let min_nodes = 3;

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

		let customers_usage =
			DdcVerification::fetch_customers_usage_for_era(&cluster_id, era_id, &dac_nodes).unwrap();


		let result = DdcVerification::fetch_sub_trees(
			&cluster_id,
			era_id,
			customers_usage,
			min_nodes,
		);

		assert!(result.is_ok());
		// Sub_aggregates which are in consensus
		assert_eq!(
			result.clone().unwrap().0,
			[BucketNodeAggregatesActivity {
				bucket_id: 90235,
				node_id: "0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa318"
					.to_string(),
				stored_bytes: 578,
				transferred_bytes: 578,
				number_of_puts: 2,
				number_of_gets: 0
			}]
		);
		// Sub_aggregates which are not in consensus
		assert_eq!(
			result.unwrap().1,
			[
				BucketNodeAggregatesActivity {
					bucket_id: 90235,
					node_id: "0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319"
						.to_string(),
					stored_bytes: 0,
					transferred_bytes: 505,
					number_of_puts: 0,
					number_of_gets: 1
				},
				BucketNodeAggregatesActivity {
					bucket_id: 90235,
					node_id: "0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319"
						.to_string(),
					stored_bytes: 0,
					transferred_bytes: 505,
					number_of_puts: 0,
					number_of_gets: 1
				},
				BucketNodeAggregatesActivity {
					bucket_id: 90235,
					node_id: "0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319"
						.to_string(),
					stored_bytes: 0,
					transferred_bytes: 506,
					number_of_puts: 0,
					number_of_gets: 1
				}
			]
		);
	});
}

#[test]
fn test_find_random_merkle_node_ids() {
	let bucket_node_aggregates_not_in_consensus: Vec<BucketNodeAggregatesActivity> =
		vec![BucketNodeAggregatesActivity {
			bucket_id: 90235,
			node_id: "0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319"
				.to_string(),
			stored_bytes: 0,
			transferred_bytes: 505,
			number_of_puts: 4,
			number_of_gets: 5,
		}];

	for bucket_node_aggregate_activity in bucket_node_aggregates_not_in_consensus {
		let result =
			DdcVerification::find_random_merkle_node_ids(3, bucket_node_aggregate_activity);
		assert_eq!(result.len(), 3);
	}
}

#[test]
fn test_challenge_sub_aggregates_not_in_consensus() {
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

		let port = 8080;

		let pending_request1 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets/123229/challenge?eraId=5757773&nodeId=0x1f50f1455f60f5774564233d321a116ca45ae3188b2200999445706d04839d72&merkleTreeNodeId=0,1,2,3", host1, port),
			response: Some(br#"{"proofs":[{"merkle_tree_node_id":8,"usage":{"stored_bytes":7803124,"transferred_bytes":0,"number_of_puts":0,"number_of_gets":2},"path":["I6/PDqZEQjRN88uhAtxdbH9dyraCmYfrHMg1MdmZ7jw=","FyhHXKzOniadUAYKGCYHdruWmfQx5GPUNqi9PATUWa8=","EBSZRilEyPnRyWk7mD6wHkvLajaKw60om/IyXwCFhGU="],"leafs":[{"record":{"id":"BgZltA5RS1uuDZNB/epSRw==","upstream":{"request":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"I5r9I0Aq9n9+oJZYWvF25sUScdzW8jdvcRMSLiiPiG2iUlT1SNE9ZAgUbvf/pFfEEdgCE+sH3GDb2vD4hxMwDw=="}},"ack":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","bytesStoredOrDelivered":"4998498","timestamp":"1727332041464","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"nmXU5AAXDCyW7aNtaEFc9XCdAjnVq0Rb1KGWjGMpYKQIcrrEX573gfRRsvqriO0xBXFoSo5e2NZVyhg0RVP6AA=="}}},"downstream":[{"request":{"parentRequest":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"I5r9I0Aq9n9+oJZYWvF25sUScdzW8jdvcRMSLiiPiG2iUlT1SNE9ZAgUbvf/pFfEEdgCE+sH3GDb2vD4hxMwDw=="}},"requestId":"6da7c481-295d-4eb7-877d-91a15f3e0c2b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"Q0vIry+GuzvS+e/Q4N3fq9DmLMF68efTb1xw6WSFSzqbRXOzwHDHj0KTIc3IvVq/CPJtuJISSLBK950tRuwWBw=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"2db9f3eb-ec5b-41a4-8b76-55181ba85db7","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"1AfyEXV/Cu+2JZBeWkKsTwGms/vc7iGIOx5/KsQyusb0pIeH7lmRFcKA7HoRFbi/mV76KBVqfcXiMwEFCmAVCA=="}},"response":{"status":"OK","time":8,"peerID":"nvmK2cNia6cl5412z8/EtNB+hPA4hGW8frmS4+EXI0o="}},{"request":{"requestId":"3733e66d-b339-4087-825f-1ace13fb6b5b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"VQmTDmpmpdAu7kv4jvaLB2BFux3npqYRVkn5GVEWlJl1REUICGKGhv3RUnSA82ICGYVK3zQePc9F3jb1tsEJBg=="}},"response":{"status":"OK","time":28,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"6f5ac1bd-32bb-4ca1-9950-e1f948905cb2","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"iO0b3Uq5+T/mtvY6e++8cGMeRFn68BRpe071JD/ho+7ttRvpzVhFMJTYoedI+Ec2qRsBhToWnms6IoPc6h06Dg=="}},"response":{"status":"OK","time":171,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}}],"timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"7otAb/JLJ1Ml69fuQ+uTLDoANF9oltQNL/jh5nYCiCSfVYKfNIVjRIwVaN2Ny6SKWcWoOR4UkmKeBwCD5zfUBQ=="}},"transferred_bytes":4998498,"stored_bytes":0},{"record":{"id":"fhLxrAvlQLC4p/a4DWCppw==","upstream":{"request":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"FINISBtFPE3XtVlBvAtUQzFl7LcK2Er1sbbmJyWvEK51d+3y7b8HTyLS3MKNXigoAo4UGwPPTrs8kzOwRqq4Bg=="}},"ack":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","bytesStoredOrDelivered":"2804626","timestamp":"1727332036764","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hjS58a/xlXOlOVzMXQELCZDFvW7REKqzhjyVjQ+vffJUusv90MtCTibdXgZoHnZ5m2L0s8xttINixKjOecnvBA=="}}},"downstream":[{"request":{"requestId":"690a94d3-d0ff-47b5-ac36-117d1bec4d02","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_METADATA","pieceCid":"AAAAAAAB4V0BAh4g6668NKzMXM0daveO1vUPE4/zchaT7ris3SrlOP5x/do=","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"N9kKpwokMabB1d+jbQS7Zynt2n0efSSvQNNTeFxsbOeOiULriCRYxEfMvl1MQpVsYjuVoMh2XjjB7/jRmgAUBw=="}},"response":{"status":"OK","time":1,"peerID":"MC+TffOg7ExljoEiQ550jSJ0QuvUk871IaHhSUOEQ5U="}},{"request":{"requestId":"17c0b6c0-64b8-4535-85c1-c6d5897edd26","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"N0X1gZtM/1Cb9DjXxrzVE3PjYb4Kje6RYvqtOpWKG+wV4hD5vnFzK0sHBqlv66SI/0cPbeK5ezcheY/Rg9eyDw=="}},"response":{"status":"OK","time":6,"peerID":"3Lg/UeZVT7P8oEgH+YM20WBBm/DFT0eddgt23x4EvaI="}},{"request":{"requestId":"b3cc54b7-78ed-4bef-b0d6-f3d517a62091","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KzIRMhhrjuUGIxIzczQ2ClFmfxVLy5am6dTOrTy5HorB9OVPxc7oSxfvaBUFt7iWUfSqYJoYIG5LZTEao1JtCg=="}},"response":{"status":"OK","time":33,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"79ebf6d6-f4fc-4030-9a8d-ba24fb6d326a","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KEt4YZBFRYb4efOQvpazW3dCeu3WqCHymSWYdAUV1ft++bOA43v6PA20YYCteFMvRuMwgLDj1D/CPTC7+qa/Aw=="}},"response":{"status":"OK","time":46,"peerID":"abGJf196inde46TgDzLiC7nTDhzdQhSc4b1QqaogYEA="}},{"request":{"requestId":"5995d138-360a-447e-b34b-c355ab2802a0","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"AiSueFF4DlIgED+h9IXwvSfQmIHFUTMaMBL/4oLKZTIWWGexNeR/DwIF56ctou8hjx7jY64Zon3ljFoREFIqAA=="}},"response":{"status":"OK","time":171,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}},{"request":{"parentRequest":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"FINISBtFPE3XtVlBvAtUQzFl7LcK2Er1sbbmJyWvEK51d+3y7b8HTyLS3MKNXigoAo4UGwPPTrs8kzOwRqq4Bg=="}},"requestId":"3c900ccd-0277-47a1-9064-79c7573eb177","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"RYI0LjzSCYz8op7fu9g/+Bj4q4WvNv2KC1c/5Sm/ry3B3h9jfM/HZxTIYgGFZJeuJ7TvAPbcf9CN/8NhVbHMAQ=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"7db4e5fd-70b4-419b-9501-4e5bf098ef3b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KAWxT4tY0t/K1BZaZu1GOQxLvdsjr273q7jCro10Emh3CgV0AXCNdDJADzX3RZihMfEqyFPKH6sqH4flvcKXAA=="}},"response":{"status":"OK","time":6,"peerID":"3Lg/UeZVT7P8oEgH+YM20WBBm/DFT0eddgt23x4EvaI="}},{"request":{"requestId":"2fd878d9-5e3b-46d2-a466-47b9434e3bb6","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"OBlAH8GC0KFtlI0v8/um1xU6mH6uIc+mgqp+AxP7BL8G/wVwDVPtSsQ/zcsbrLsGhauVZDJ1K4prrdjpjsVRBg=="}},"response":{"status":"OK","time":15,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"baaa8d1d-e6ac-4c63-95a4-cdc6dd10f9c2","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"iBrqYGIfsKSvEtSyVhU6VSbfMBoB7Ca01xa7JH1Bbn6FfNDC0EEjx9JKmQNSAZFPf0YOAj5dPDxQsvUSgRpqDg=="}},"response":{"status":"OK","time":19,"peerID":"abGJf196inde46TgDzLiC7nTDhzdQhSc4b1QqaogYEA="}}],"timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"HvUlFfBmfny40+mr+KP86msvQwJ1+p6EIfyzYDgizCGeik/dYBDGIB91NreQY3/lm9cnlHBE5nWegDS/8cswCA=="}},"transferred_bytes":2804626,"stored_bytes":0}]},{"merkle_tree_node_id":9,"usage":{"stored_bytes":3549468,"transferred_bytes":0,"number_of_puts":0,"number_of_gets":2},"path":["I6/PDqZEQjRN88uhAtxdbH9dyraCmYfrHMg1MdmZ7jw=","FyhHXKzOniadUAYKGCYHdruWmfQx5GPUNqi9PATUWa8=","EBSZRilEyPnRyWk7mD6wHkvLajaKw60om/IyXwCFhGU="],"leafs":[{"record":{"id":"hkKGGV7FQ0m/Oyx7+kLLNA==","upstream":{"request":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"b9RfmuKUuZzJ19z9LemIv5OOm/twk3rvfod+fEmZupd6j/efuIv1bLkhguudJUzGFAqObBKPBJ3U3jv9xSnBDA=="}},"ack":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","bytesStoredOrDelivered":"2789096","timestamp":"1727332044907","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"Z4IFvxknUG+Itz6kPGeLnKnjE5h6PaoNDGeadEuaqY7XHlavIY14Wn+GlFltdCEIK8S2UHmdxuyenekbTrNyBQ=="}}},"downstream":[{"request":{"parentRequest":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"b9RfmuKUuZzJ19z9LemIv5OOm/twk3rvfod+fEmZupd6j/efuIv1bLkhguudJUzGFAqObBKPBJ3U3jv9xSnBDA=="}},"requestId":"a10bf6c8-9da9-429b-a5ac-485bde0081fd","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"MgOezVKgqjSmMCzYNiludAciy7lBcUKjP0sNKuj3jLBQKkF1sL92ExjW/bDnvFFuRCxpAlrLZJEtDXe8Jg/pCQ=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"fe35db07-5fb4-488b-9d86-ac7a6b754c86","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"mCu4MVaJE1adkEd2r6jD2fxHcGvxaQZ7OC1+8W88GdmnLkZwKSIxW4dhdC6Mynjt7VCYC8CTbLfVhfeqD3C7DA=="}},"response":{"status":"OK","time":6,"peerID":"MC+TffOg7ExljoEiQ550jSJ0QuvUk871IaHhSUOEQ5U="}},{"request":{"requestId":"71d7fa68-00bf-44bc-b16e-120f24970370","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hyXyAgiCe9Ny8lAY0tJO/qI82FQFfbYyQK/QZq385XDpOkHRhrPGp9+XOAmxTZE+wbJnPPd+YAy+baXVMHuGBQ=="}},"response":{"status":"OK","time":43,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"7ad4580b-1df5-4060-a81b-e020db9d20c1","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"gAgc21i2RzYxxjISv0g70ZJInZT3hr1zl+HkQX5LS/CJda3QEhYM8CbkaVEFMb+aghrNyMyFMnHDEb2bNOh2Bw=="}},"response":{"status":"OK","time":172,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}}],"timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hfofRwv5/6Mqgt+M/DbKMCqR4R0DLlvkKjV8n9/VCtB5eA852xKwZQePAZhCSvlf9lwM+9fVwYdU6JfxTP00DA=="}},"transferred_bytes":2789096,"stored_bytes":0},{"record":{"id":"ldpO+ZPnSLWvotPE4C3rnQ==","upstream":{"request":{"requestId":"16af68b2-5bf3-4df6-92c1-be284190c760","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332045892","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"EJtOzQQnxxGjHqippQ0UtLp5CDGCPFUOvDThiVZc6SGctgV0QBMcSGrwo513S0gy1dLH3TaPLt0FBesVj9AgCg=="}},"ack":{"requestId":"16af68b2-5bf3-4df6-92c1-be284190c760","bytesStoredOrDelivered":"760372","timestamp":"1727332045893","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"u/ZwcGta6DIPVpqGPZZrolFJljSrk0gjj03ULThHCczOgzTYnIP2Q5xxo5Pwcm3ZnRvRjrjGX+ByPpKSJvBXCA=="}}},"timestamp":"1727332045892","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"huMb7m3yz1PAdeIBF7yNzHfrab2KzqyC8A7o8eG/b7KaWq2QSEQZ5nZuNsz9PZS4lWu1KvOW3OfgesSsl2bEDA=="}},"transferred_bytes":760372,"stored_bytes":0}]}]}"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets/123229/challenge?eraId=5757773&nodeId=0x1f50f1455f60f5774564233d321a116ca45ae3188b2200999445706d04839d72&merkleTreeNodeId=0,1,2,3", host2, port),
			response: Some(br#"{"proofs":[{"merkle_tree_node_id":8,"usage":{"stored_bytes":7803124,"transferred_bytes":0,"number_of_puts":0,"number_of_gets":2},"path":["I6/PDqZEQjRN88uhAtxdbH9dyraCmYfrHMg1MdmZ7jw=","FyhHXKzOniadUAYKGCYHdruWmfQx5GPUNqi9PATUWa8=","EBSZRilEyPnRyWk7mD6wHkvLajaKw60om/IyXwCFhGU="],"leafs":[{"record":{"id":"BgZltA5RS1uuDZNB/epSRw==","upstream":{"request":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"I5r9I0Aq9n9+oJZYWvF25sUScdzW8jdvcRMSLiiPiG2iUlT1SNE9ZAgUbvf/pFfEEdgCE+sH3GDb2vD4hxMwDw=="}},"ack":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","bytesStoredOrDelivered":"4998498","timestamp":"1727332041464","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"nmXU5AAXDCyW7aNtaEFc9XCdAjnVq0Rb1KGWjGMpYKQIcrrEX573gfRRsvqriO0xBXFoSo5e2NZVyhg0RVP6AA=="}}},"downstream":[{"request":{"parentRequest":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"I5r9I0Aq9n9+oJZYWvF25sUScdzW8jdvcRMSLiiPiG2iUlT1SNE9ZAgUbvf/pFfEEdgCE+sH3GDb2vD4hxMwDw=="}},"requestId":"6da7c481-295d-4eb7-877d-91a15f3e0c2b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"Q0vIry+GuzvS+e/Q4N3fq9DmLMF68efTb1xw6WSFSzqbRXOzwHDHj0KTIc3IvVq/CPJtuJISSLBK950tRuwWBw=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"2db9f3eb-ec5b-41a4-8b76-55181ba85db7","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"1AfyEXV/Cu+2JZBeWkKsTwGms/vc7iGIOx5/KsQyusb0pIeH7lmRFcKA7HoRFbi/mV76KBVqfcXiMwEFCmAVCA=="}},"response":{"status":"OK","time":8,"peerID":"nvmK2cNia6cl5412z8/EtNB+hPA4hGW8frmS4+EXI0o="}},{"request":{"requestId":"3733e66d-b339-4087-825f-1ace13fb6b5b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"VQmTDmpmpdAu7kv4jvaLB2BFux3npqYRVkn5GVEWlJl1REUICGKGhv3RUnSA82ICGYVK3zQePc9F3jb1tsEJBg=="}},"response":{"status":"OK","time":28,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"6f5ac1bd-32bb-4ca1-9950-e1f948905cb2","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"iO0b3Uq5+T/mtvY6e++8cGMeRFn68BRpe071JD/ho+7ttRvpzVhFMJTYoedI+Ec2qRsBhToWnms6IoPc6h06Dg=="}},"response":{"status":"OK","time":171,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}}],"timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"7otAb/JLJ1Ml69fuQ+uTLDoANF9oltQNL/jh5nYCiCSfVYKfNIVjRIwVaN2Ny6SKWcWoOR4UkmKeBwCD5zfUBQ=="}},"transferred_bytes":4998498,"stored_bytes":0},{"record":{"id":"fhLxrAvlQLC4p/a4DWCppw==","upstream":{"request":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"FINISBtFPE3XtVlBvAtUQzFl7LcK2Er1sbbmJyWvEK51d+3y7b8HTyLS3MKNXigoAo4UGwPPTrs8kzOwRqq4Bg=="}},"ack":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","bytesStoredOrDelivered":"2804626","timestamp":"1727332036764","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hjS58a/xlXOlOVzMXQELCZDFvW7REKqzhjyVjQ+vffJUusv90MtCTibdXgZoHnZ5m2L0s8xttINixKjOecnvBA=="}}},"downstream":[{"request":{"requestId":"690a94d3-d0ff-47b5-ac36-117d1bec4d02","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_METADATA","pieceCid":"AAAAAAAB4V0BAh4g6668NKzMXM0daveO1vUPE4/zchaT7ris3SrlOP5x/do=","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"N9kKpwokMabB1d+jbQS7Zynt2n0efSSvQNNTeFxsbOeOiULriCRYxEfMvl1MQpVsYjuVoMh2XjjB7/jRmgAUBw=="}},"response":{"status":"OK","time":1,"peerID":"MC+TffOg7ExljoEiQ550jSJ0QuvUk871IaHhSUOEQ5U="}},{"request":{"requestId":"17c0b6c0-64b8-4535-85c1-c6d5897edd26","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"N0X1gZtM/1Cb9DjXxrzVE3PjYb4Kje6RYvqtOpWKG+wV4hD5vnFzK0sHBqlv66SI/0cPbeK5ezcheY/Rg9eyDw=="}},"response":{"status":"OK","time":6,"peerID":"3Lg/UeZVT7P8oEgH+YM20WBBm/DFT0eddgt23x4EvaI="}},{"request":{"requestId":"b3cc54b7-78ed-4bef-b0d6-f3d517a62091","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KzIRMhhrjuUGIxIzczQ2ClFmfxVLy5am6dTOrTy5HorB9OVPxc7oSxfvaBUFt7iWUfSqYJoYIG5LZTEao1JtCg=="}},"response":{"status":"OK","time":33,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"79ebf6d6-f4fc-4030-9a8d-ba24fb6d326a","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KEt4YZBFRYb4efOQvpazW3dCeu3WqCHymSWYdAUV1ft++bOA43v6PA20YYCteFMvRuMwgLDj1D/CPTC7+qa/Aw=="}},"response":{"status":"OK","time":46,"peerID":"abGJf196inde46TgDzLiC7nTDhzdQhSc4b1QqaogYEA="}},{"request":{"requestId":"5995d138-360a-447e-b34b-c355ab2802a0","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"AiSueFF4DlIgED+h9IXwvSfQmIHFUTMaMBL/4oLKZTIWWGexNeR/DwIF56ctou8hjx7jY64Zon3ljFoREFIqAA=="}},"response":{"status":"OK","time":171,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}},{"request":{"parentRequest":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"FINISBtFPE3XtVlBvAtUQzFl7LcK2Er1sbbmJyWvEK51d+3y7b8HTyLS3MKNXigoAo4UGwPPTrs8kzOwRqq4Bg=="}},"requestId":"3c900ccd-0277-47a1-9064-79c7573eb177","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"RYI0LjzSCYz8op7fu9g/+Bj4q4WvNv2KC1c/5Sm/ry3B3h9jfM/HZxTIYgGFZJeuJ7TvAPbcf9CN/8NhVbHMAQ=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"7db4e5fd-70b4-419b-9501-4e5bf098ef3b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KAWxT4tY0t/K1BZaZu1GOQxLvdsjr273q7jCro10Emh3CgV0AXCNdDJADzX3RZihMfEqyFPKH6sqH4flvcKXAA=="}},"response":{"status":"OK","time":6,"peerID":"3Lg/UeZVT7P8oEgH+YM20WBBm/DFT0eddgt23x4EvaI="}},{"request":{"requestId":"2fd878d9-5e3b-46d2-a466-47b9434e3bb6","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"OBlAH8GC0KFtlI0v8/um1xU6mH6uIc+mgqp+AxP7BL8G/wVwDVPtSsQ/zcsbrLsGhauVZDJ1K4prrdjpjsVRBg=="}},"response":{"status":"OK","time":15,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"baaa8d1d-e6ac-4c63-95a4-cdc6dd10f9c2","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"iBrqYGIfsKSvEtSyVhU6VSbfMBoB7Ca01xa7JH1Bbn6FfNDC0EEjx9JKmQNSAZFPf0YOAj5dPDxQsvUSgRpqDg=="}},"response":{"status":"OK","time":19,"peerID":"abGJf196inde46TgDzLiC7nTDhzdQhSc4b1QqaogYEA="}}],"timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"HvUlFfBmfny40+mr+KP86msvQwJ1+p6EIfyzYDgizCGeik/dYBDGIB91NreQY3/lm9cnlHBE5nWegDS/8cswCA=="}},"transferred_bytes":2804626,"stored_bytes":0}]},{"merkle_tree_node_id":9,"usage":{"stored_bytes":3549468,"transferred_bytes":0,"number_of_puts":0,"number_of_gets":2},"path":["I6/PDqZEQjRN88uhAtxdbH9dyraCmYfrHMg1MdmZ7jw=","FyhHXKzOniadUAYKGCYHdruWmfQx5GPUNqi9PATUWa8=","EBSZRilEyPnRyWk7mD6wHkvLajaKw60om/IyXwCFhGU="],"leafs":[{"record":{"id":"hkKGGV7FQ0m/Oyx7+kLLNA==","upstream":{"request":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"b9RfmuKUuZzJ19z9LemIv5OOm/twk3rvfod+fEmZupd6j/efuIv1bLkhguudJUzGFAqObBKPBJ3U3jv9xSnBDA=="}},"ack":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","bytesStoredOrDelivered":"2789096","timestamp":"1727332044907","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"Z4IFvxknUG+Itz6kPGeLnKnjE5h6PaoNDGeadEuaqY7XHlavIY14Wn+GlFltdCEIK8S2UHmdxuyenekbTrNyBQ=="}}},"downstream":[{"request":{"parentRequest":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"b9RfmuKUuZzJ19z9LemIv5OOm/twk3rvfod+fEmZupd6j/efuIv1bLkhguudJUzGFAqObBKPBJ3U3jv9xSnBDA=="}},"requestId":"a10bf6c8-9da9-429b-a5ac-485bde0081fd","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"MgOezVKgqjSmMCzYNiludAciy7lBcUKjP0sNKuj3jLBQKkF1sL92ExjW/bDnvFFuRCxpAlrLZJEtDXe8Jg/pCQ=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"fe35db07-5fb4-488b-9d86-ac7a6b754c86","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"mCu4MVaJE1adkEd2r6jD2fxHcGvxaQZ7OC1+8W88GdmnLkZwKSIxW4dhdC6Mynjt7VCYC8CTbLfVhfeqD3C7DA=="}},"response":{"status":"OK","time":6,"peerID":"MC+TffOg7ExljoEiQ550jSJ0QuvUk871IaHhSUOEQ5U="}},{"request":{"requestId":"71d7fa68-00bf-44bc-b16e-120f24970370","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hyXyAgiCe9Ny8lAY0tJO/qI82FQFfbYyQK/QZq385XDpOkHRhrPGp9+XOAmxTZE+wbJnPPd+YAy+baXVMHuGBQ=="}},"response":{"status":"OK","time":43,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"7ad4580b-1df5-4060-a81b-e020db9d20c1","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"gAgc21i2RzYxxjISv0g70ZJInZT3hr1zl+HkQX5LS/CJda3QEhYM8CbkaVEFMb+aghrNyMyFMnHDEb2bNOh2Bw=="}},"response":{"status":"OK","time":172,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}}],"timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hfofRwv5/6Mqgt+M/DbKMCqR4R0DLlvkKjV8n9/VCtB5eA852xKwZQePAZhCSvlf9lwM+9fVwYdU6JfxTP00DA=="}},"transferred_bytes":2789096,"stored_bytes":0},{"record":{"id":"ldpO+ZPnSLWvotPE4C3rnQ==","upstream":{"request":{"requestId":"16af68b2-5bf3-4df6-92c1-be284190c760","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332045892","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"EJtOzQQnxxGjHqippQ0UtLp5CDGCPFUOvDThiVZc6SGctgV0QBMcSGrwo513S0gy1dLH3TaPLt0FBesVj9AgCg=="}},"ack":{"requestId":"16af68b2-5bf3-4df6-92c1-be284190c760","bytesStoredOrDelivered":"760372","timestamp":"1727332045893","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"u/ZwcGta6DIPVpqGPZZrolFJljSrk0gjj03ULThHCczOgzTYnIP2Q5xxo5Pwcm3ZnRvRjrjGX+ByPpKSJvBXCA=="}}},"timestamp":"1727332045892","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"huMb7m3yz1PAdeIBF7yNzHfrab2KzqyC8A7o8eG/b7KaWq2QSEQZ5nZuNsz9PZS4lWu1KvOW3OfgesSsl2bEDA=="}},"transferred_bytes":760372,"stored_bytes":0}]}]}"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets/123229/challenge?eraId=5757773&nodeId=0x1f50f1455f60f5774564233d321a116ca45ae3188b2200999445706d04839d72&merkleTreeNodeId=0,1,2,3", host3, port),
			response: Some(br#"{"proofs":[{"merkle_tree_node_id":8,"usage":{"stored_bytes":7803124,"transferred_bytes":0,"number_of_puts":0,"number_of_gets":2},"path":["I6/PDqZEQjRN88uhAtxdbH9dyraCmYfrHMg1MdmZ7jw=","FyhHXKzOniadUAYKGCYHdruWmfQx5GPUNqi9PATUWa8=","EBSZRilEyPnRyWk7mD6wHkvLajaKw60om/IyXwCFhGU="],"leafs":[{"record":{"id":"BgZltA5RS1uuDZNB/epSRw==","upstream":{"request":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"I5r9I0Aq9n9+oJZYWvF25sUScdzW8jdvcRMSLiiPiG2iUlT1SNE9ZAgUbvf/pFfEEdgCE+sH3GDb2vD4hxMwDw=="}},"ack":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","bytesStoredOrDelivered":"4998498","timestamp":"1727332041464","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"nmXU5AAXDCyW7aNtaEFc9XCdAjnVq0Rb1KGWjGMpYKQIcrrEX573gfRRsvqriO0xBXFoSo5e2NZVyhg0RVP6AA=="}}},"downstream":[{"request":{"parentRequest":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"I5r9I0Aq9n9+oJZYWvF25sUScdzW8jdvcRMSLiiPiG2iUlT1SNE9ZAgUbvf/pFfEEdgCE+sH3GDb2vD4hxMwDw=="}},"requestId":"6da7c481-295d-4eb7-877d-91a15f3e0c2b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"Q0vIry+GuzvS+e/Q4N3fq9DmLMF68efTb1xw6WSFSzqbRXOzwHDHj0KTIc3IvVq/CPJtuJISSLBK950tRuwWBw=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"2db9f3eb-ec5b-41a4-8b76-55181ba85db7","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"1AfyEXV/Cu+2JZBeWkKsTwGms/vc7iGIOx5/KsQyusb0pIeH7lmRFcKA7HoRFbi/mV76KBVqfcXiMwEFCmAVCA=="}},"response":{"status":"OK","time":8,"peerID":"nvmK2cNia6cl5412z8/EtNB+hPA4hGW8frmS4+EXI0o="}},{"request":{"requestId":"3733e66d-b339-4087-825f-1ace13fb6b5b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"VQmTDmpmpdAu7kv4jvaLB2BFux3npqYRVkn5GVEWlJl1REUICGKGhv3RUnSA82ICGYVK3zQePc9F3jb1tsEJBg=="}},"response":{"status":"OK","time":28,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"6f5ac1bd-32bb-4ca1-9950-e1f948905cb2","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"iO0b3Uq5+T/mtvY6e++8cGMeRFn68BRpe071JD/ho+7ttRvpzVhFMJTYoedI+Ec2qRsBhToWnms6IoPc6h06Dg=="}},"response":{"status":"OK","time":171,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}}],"timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"7otAb/JLJ1Ml69fuQ+uTLDoANF9oltQNL/jh5nYCiCSfVYKfNIVjRIwVaN2Ny6SKWcWoOR4UkmKeBwCD5zfUBQ=="}},"transferred_bytes":4998498,"stored_bytes":0},{"record":{"id":"fhLxrAvlQLC4p/a4DWCppw==","upstream":{"request":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"FINISBtFPE3XtVlBvAtUQzFl7LcK2Er1sbbmJyWvEK51d+3y7b8HTyLS3MKNXigoAo4UGwPPTrs8kzOwRqq4Bg=="}},"ack":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","bytesStoredOrDelivered":"2804626","timestamp":"1727332036764","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hjS58a/xlXOlOVzMXQELCZDFvW7REKqzhjyVjQ+vffJUusv90MtCTibdXgZoHnZ5m2L0s8xttINixKjOecnvBA=="}}},"downstream":[{"request":{"requestId":"690a94d3-d0ff-47b5-ac36-117d1bec4d02","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_METADATA","pieceCid":"AAAAAAAB4V0BAh4g6668NKzMXM0daveO1vUPE4/zchaT7ris3SrlOP5x/do=","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"N9kKpwokMabB1d+jbQS7Zynt2n0efSSvQNNTeFxsbOeOiULriCRYxEfMvl1MQpVsYjuVoMh2XjjB7/jRmgAUBw=="}},"response":{"status":"OK","time":1,"peerID":"MC+TffOg7ExljoEiQ550jSJ0QuvUk871IaHhSUOEQ5U="}},{"request":{"requestId":"17c0b6c0-64b8-4535-85c1-c6d5897edd26","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"N0X1gZtM/1Cb9DjXxrzVE3PjYb4Kje6RYvqtOpWKG+wV4hD5vnFzK0sHBqlv66SI/0cPbeK5ezcheY/Rg9eyDw=="}},"response":{"status":"OK","time":6,"peerID":"3Lg/UeZVT7P8oEgH+YM20WBBm/DFT0eddgt23x4EvaI="}},{"request":{"requestId":"b3cc54b7-78ed-4bef-b0d6-f3d517a62091","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KzIRMhhrjuUGIxIzczQ2ClFmfxVLy5am6dTOrTy5HorB9OVPxc7oSxfvaBUFt7iWUfSqYJoYIG5LZTEao1JtCg=="}},"response":{"status":"OK","time":33,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"79ebf6d6-f4fc-4030-9a8d-ba24fb6d326a","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KEt4YZBFRYb4efOQvpazW3dCeu3WqCHymSWYdAUV1ft++bOA43v6PA20YYCteFMvRuMwgLDj1D/CPTC7+qa/Aw=="}},"response":{"status":"OK","time":46,"peerID":"abGJf196inde46TgDzLiC7nTDhzdQhSc4b1QqaogYEA="}},{"request":{"requestId":"5995d138-360a-447e-b34b-c355ab2802a0","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"AiSueFF4DlIgED+h9IXwvSfQmIHFUTMaMBL/4oLKZTIWWGexNeR/DwIF56ctou8hjx7jY64Zon3ljFoREFIqAA=="}},"response":{"status":"OK","time":171,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}},{"request":{"parentRequest":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"FINISBtFPE3XtVlBvAtUQzFl7LcK2Er1sbbmJyWvEK51d+3y7b8HTyLS3MKNXigoAo4UGwPPTrs8kzOwRqq4Bg=="}},"requestId":"3c900ccd-0277-47a1-9064-79c7573eb177","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"RYI0LjzSCYz8op7fu9g/+Bj4q4WvNv2KC1c/5Sm/ry3B3h9jfM/HZxTIYgGFZJeuJ7TvAPbcf9CN/8NhVbHMAQ=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"7db4e5fd-70b4-419b-9501-4e5bf098ef3b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KAWxT4tY0t/K1BZaZu1GOQxLvdsjr273q7jCro10Emh3CgV0AXCNdDJADzX3RZihMfEqyFPKH6sqH4flvcKXAA=="}},"response":{"status":"OK","time":6,"peerID":"3Lg/UeZVT7P8oEgH+YM20WBBm/DFT0eddgt23x4EvaI="}},{"request":{"requestId":"2fd878d9-5e3b-46d2-a466-47b9434e3bb6","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"OBlAH8GC0KFtlI0v8/um1xU6mH6uIc+mgqp+AxP7BL8G/wVwDVPtSsQ/zcsbrLsGhauVZDJ1K4prrdjpjsVRBg=="}},"response":{"status":"OK","time":15,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"baaa8d1d-e6ac-4c63-95a4-cdc6dd10f9c2","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"iBrqYGIfsKSvEtSyVhU6VSbfMBoB7Ca01xa7JH1Bbn6FfNDC0EEjx9JKmQNSAZFPf0YOAj5dPDxQsvUSgRpqDg=="}},"response":{"status":"OK","time":19,"peerID":"abGJf196inde46TgDzLiC7nTDhzdQhSc4b1QqaogYEA="}}],"timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"HvUlFfBmfny40+mr+KP86msvQwJ1+p6EIfyzYDgizCGeik/dYBDGIB91NreQY3/lm9cnlHBE5nWegDS/8cswCA=="}},"transferred_bytes":2804626,"stored_bytes":0}]},{"merkle_tree_node_id":9,"usage":{"stored_bytes":3549468,"transferred_bytes":0,"number_of_puts":0,"number_of_gets":2},"path":["I6/PDqZEQjRN88uhAtxdbH9dyraCmYfrHMg1MdmZ7jw=","FyhHXKzOniadUAYKGCYHdruWmfQx5GPUNqi9PATUWa8=","EBSZRilEyPnRyWk7mD6wHkvLajaKw60om/IyXwCFhGU="],"leafs":[{"record":{"id":"hkKGGV7FQ0m/Oyx7+kLLNA==","upstream":{"request":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"b9RfmuKUuZzJ19z9LemIv5OOm/twk3rvfod+fEmZupd6j/efuIv1bLkhguudJUzGFAqObBKPBJ3U3jv9xSnBDA=="}},"ack":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","bytesStoredOrDelivered":"2789096","timestamp":"1727332044907","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"Z4IFvxknUG+Itz6kPGeLnKnjE5h6PaoNDGeadEuaqY7XHlavIY14Wn+GlFltdCEIK8S2UHmdxuyenekbTrNyBQ=="}}},"downstream":[{"request":{"parentRequest":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"b9RfmuKUuZzJ19z9LemIv5OOm/twk3rvfod+fEmZupd6j/efuIv1bLkhguudJUzGFAqObBKPBJ3U3jv9xSnBDA=="}},"requestId":"a10bf6c8-9da9-429b-a5ac-485bde0081fd","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"MgOezVKgqjSmMCzYNiludAciy7lBcUKjP0sNKuj3jLBQKkF1sL92ExjW/bDnvFFuRCxpAlrLZJEtDXe8Jg/pCQ=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"fe35db07-5fb4-488b-9d86-ac7a6b754c86","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"mCu4MVaJE1adkEd2r6jD2fxHcGvxaQZ7OC1+8W88GdmnLkZwKSIxW4dhdC6Mynjt7VCYC8CTbLfVhfeqD3C7DA=="}},"response":{"status":"OK","time":6,"peerID":"MC+TffOg7ExljoEiQ550jSJ0QuvUk871IaHhSUOEQ5U="}},{"request":{"requestId":"71d7fa68-00bf-44bc-b16e-120f24970370","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hyXyAgiCe9Ny8lAY0tJO/qI82FQFfbYyQK/QZq385XDpOkHRhrPGp9+XOAmxTZE+wbJnPPd+YAy+baXVMHuGBQ=="}},"response":{"status":"OK","time":43,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"7ad4580b-1df5-4060-a81b-e020db9d20c1","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"gAgc21i2RzYxxjISv0g70ZJInZT3hr1zl+HkQX5LS/CJda3QEhYM8CbkaVEFMb+aghrNyMyFMnHDEb2bNOh2Bw=="}},"response":{"status":"OK","time":172,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}}],"timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hfofRwv5/6Mqgt+M/DbKMCqR4R0DLlvkKjV8n9/VCtB5eA852xKwZQePAZhCSvlf9lwM+9fVwYdU6JfxTP00DA=="}},"transferred_bytes":2789096,"stored_bytes":0},{"record":{"id":"ldpO+ZPnSLWvotPE4C3rnQ==","upstream":{"request":{"requestId":"16af68b2-5bf3-4df6-92c1-be284190c760","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332045892","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"EJtOzQQnxxGjHqippQ0UtLp5CDGCPFUOvDThiVZc6SGctgV0QBMcSGrwo513S0gy1dLH3TaPLt0FBesVj9AgCg=="}},"ack":{"requestId":"16af68b2-5bf3-4df6-92c1-be284190c760","bytesStoredOrDelivered":"760372","timestamp":"1727332045893","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"u/ZwcGta6DIPVpqGPZZrolFJljSrk0gjj03ULThHCczOgzTYnIP2Q5xxo5Pwcm3ZnRvRjrjGX+ByPpKSJvBXCA=="}}},"timestamp":"1727332045892","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"huMb7m3yz1PAdeIBF7yNzHfrab2KzqyC8A7o8eG/b7KaWq2QSEQZ5nZuNsz9PZS4lWu1KvOW3OfgesSsl2bEDA=="}},"transferred_bytes":760372,"stored_bytes":0}]}]}"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets/123229/challenge?eraId=5757773&nodeId=0x1f50f1455f60f5774564233d321a116ca45ae3188b2200999445706d04839d72&merkleTreeNodeId=0,1,2,3", host4, port),
			response: Some(br#"{"proofs":[{"merkle_tree_node_id":8,"usage":{"stored_bytes":7803124,"transferred_bytes":0,"number_of_puts":0,"number_of_gets":2},"path":["I6/PDqZEQjRN88uhAtxdbH9dyraCmYfrHMg1MdmZ7jw=","FyhHXKzOniadUAYKGCYHdruWmfQx5GPUNqi9PATUWa8=","EBSZRilEyPnRyWk7mD6wHkvLajaKw60om/IyXwCFhGU="],"leafs":[{"record":{"id":"BgZltA5RS1uuDZNB/epSRw==","upstream":{"request":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"I5r9I0Aq9n9+oJZYWvF25sUScdzW8jdvcRMSLiiPiG2iUlT1SNE9ZAgUbvf/pFfEEdgCE+sH3GDb2vD4hxMwDw=="}},"ack":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","bytesStoredOrDelivered":"4998498","timestamp":"1727332041464","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"nmXU5AAXDCyW7aNtaEFc9XCdAjnVq0Rb1KGWjGMpYKQIcrrEX573gfRRsvqriO0xBXFoSo5e2NZVyhg0RVP6AA=="}}},"downstream":[{"request":{"parentRequest":{"requestId":"898a1574-7698-4c0e-b15b-62b7b16d2d38","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"I5r9I0Aq9n9+oJZYWvF25sUScdzW8jdvcRMSLiiPiG2iUlT1SNE9ZAgUbvf/pFfEEdgCE+sH3GDb2vD4hxMwDw=="}},"requestId":"6da7c481-295d-4eb7-877d-91a15f3e0c2b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"Q0vIry+GuzvS+e/Q4N3fq9DmLMF68efTb1xw6WSFSzqbRXOzwHDHj0KTIc3IvVq/CPJtuJISSLBK950tRuwWBw=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"2db9f3eb-ec5b-41a4-8b76-55181ba85db7","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"1AfyEXV/Cu+2JZBeWkKsTwGms/vc7iGIOx5/KsQyusb0pIeH7lmRFcKA7HoRFbi/mV76KBVqfcXiMwEFCmAVCA=="}},"response":{"status":"OK","time":8,"peerID":"nvmK2cNia6cl5412z8/EtNB+hPA4hGW8frmS4+EXI0o="}},{"request":{"requestId":"3733e66d-b339-4087-825f-1ace13fb6b5b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"VQmTDmpmpdAu7kv4jvaLB2BFux3npqYRVkn5GVEWlJl1REUICGKGhv3RUnSA82ICGYVK3zQePc9F3jb1tsEJBg=="}},"response":{"status":"OK","time":28,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"6f5ac1bd-32bb-4ca1-9950-e1f948905cb2","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"2097152","size":"1048576","timestamp":"1727332041286","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"iO0b3Uq5+T/mtvY6e++8cGMeRFn68BRpe071JD/ho+7ttRvpzVhFMJTYoedI+Ec2qRsBhToWnms6IoPc6h06Dg=="}},"response":{"status":"OK","time":171,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}}],"timestamp":"1727332041285","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"7otAb/JLJ1Ml69fuQ+uTLDoANF9oltQNL/jh5nYCiCSfVYKfNIVjRIwVaN2Ny6SKWcWoOR4UkmKeBwCD5zfUBQ=="}},"transferred_bytes":4998498,"stored_bytes":0},{"record":{"id":"fhLxrAvlQLC4p/a4DWCppw==","upstream":{"request":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"FINISBtFPE3XtVlBvAtUQzFl7LcK2Er1sbbmJyWvEK51d+3y7b8HTyLS3MKNXigoAo4UGwPPTrs8kzOwRqq4Bg=="}},"ack":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","bytesStoredOrDelivered":"2804626","timestamp":"1727332036764","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hjS58a/xlXOlOVzMXQELCZDFvW7REKqzhjyVjQ+vffJUusv90MtCTibdXgZoHnZ5m2L0s8xttINixKjOecnvBA=="}}},"downstream":[{"request":{"requestId":"690a94d3-d0ff-47b5-ac36-117d1bec4d02","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_METADATA","pieceCid":"AAAAAAAB4V0BAh4g6668NKzMXM0daveO1vUPE4/zchaT7ris3SrlOP5x/do=","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"N9kKpwokMabB1d+jbQS7Zynt2n0efSSvQNNTeFxsbOeOiULriCRYxEfMvl1MQpVsYjuVoMh2XjjB7/jRmgAUBw=="}},"response":{"status":"OK","time":1,"peerID":"MC+TffOg7ExljoEiQ550jSJ0QuvUk871IaHhSUOEQ5U="}},{"request":{"requestId":"17c0b6c0-64b8-4535-85c1-c6d5897edd26","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"N0X1gZtM/1Cb9DjXxrzVE3PjYb4Kje6RYvqtOpWKG+wV4hD5vnFzK0sHBqlv66SI/0cPbeK5ezcheY/Rg9eyDw=="}},"response":{"status":"OK","time":6,"peerID":"3Lg/UeZVT7P8oEgH+YM20WBBm/DFT0eddgt23x4EvaI="}},{"request":{"requestId":"b3cc54b7-78ed-4bef-b0d6-f3d517a62091","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KzIRMhhrjuUGIxIzczQ2ClFmfxVLy5am6dTOrTy5HorB9OVPxc7oSxfvaBUFt7iWUfSqYJoYIG5LZTEao1JtCg=="}},"response":{"status":"OK","time":33,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"79ebf6d6-f4fc-4030-9a8d-ba24fb6d326a","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KEt4YZBFRYb4efOQvpazW3dCeu3WqCHymSWYdAUV1ft++bOA43v6PA20YYCteFMvRuMwgLDj1D/CPTC7+qa/Aw=="}},"response":{"status":"OK","time":46,"peerID":"abGJf196inde46TgDzLiC7nTDhzdQhSc4b1QqaogYEA="}},{"request":{"requestId":"5995d138-360a-447e-b34b-c355ab2802a0","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"2097152","size":"1048576","timestamp":"1727332036563","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"AiSueFF4DlIgED+h9IXwvSfQmIHFUTMaMBL/4oLKZTIWWGexNeR/DwIF56ctou8hjx7jY64Zon3ljFoREFIqAA=="}},"response":{"status":"OK","time":171,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}},{"request":{"parentRequest":{"requestId":"7ff491ab-e126-433b-aafd-b0f30ce02c6f","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"FINISBtFPE3XtVlBvAtUQzFl7LcK2Er1sbbmJyWvEK51d+3y7b8HTyLS3MKNXigoAo4UGwPPTrs8kzOwRqq4Bg=="}},"requestId":"3c900ccd-0277-47a1-9064-79c7573eb177","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"RYI0LjzSCYz8op7fu9g/+Bj4q4WvNv2KC1c/5Sm/ry3B3h9jfM/HZxTIYgGFZJeuJ7TvAPbcf9CN/8NhVbHMAQ=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"7db4e5fd-70b4-419b-9501-4e5bf098ef3b","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"KAWxT4tY0t/K1BZaZu1GOQxLvdsjr273q7jCro10Emh3CgV0AXCNdDJADzX3RZihMfEqyFPKH6sqH4flvcKXAA=="}},"response":{"status":"OK","time":6,"peerID":"3Lg/UeZVT7P8oEgH+YM20WBBm/DFT0eddgt23x4EvaI="}},{"request":{"requestId":"2fd878d9-5e3b-46d2-a466-47b9434e3bb6","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"OBlAH8GC0KFtlI0v8/um1xU6mH6uIc+mgqp+AxP7BL8G/wVwDVPtSsQ/zcsbrLsGhauVZDJ1K4prrdjpjsVRBg=="}},"response":{"status":"OK","time":15,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"baaa8d1d-e6ac-4c63-95a4-cdc6dd10f9c2","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeIOuuvDSszFzNHWr3jtb1DxOP83IWk+64rN0q5Tj+cf3a","offset":"3145728","size":"1048576","timestamp":"1727332036739","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"iBrqYGIfsKSvEtSyVhU6VSbfMBoB7Ca01xa7JH1Bbn6FfNDC0EEjx9JKmQNSAZFPf0YOAj5dPDxQsvUSgRpqDg=="}},"response":{"status":"OK","time":19,"peerID":"abGJf196inde46TgDzLiC7nTDhzdQhSc4b1QqaogYEA="}}],"timestamp":"1727332036562","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"HvUlFfBmfny40+mr+KP86msvQwJ1+p6EIfyzYDgizCGeik/dYBDGIB91NreQY3/lm9cnlHBE5nWegDS/8cswCA=="}},"transferred_bytes":2804626,"stored_bytes":0}]},{"merkle_tree_node_id":9,"usage":{"stored_bytes":3549468,"transferred_bytes":0,"number_of_puts":0,"number_of_gets":2},"path":["I6/PDqZEQjRN88uhAtxdbH9dyraCmYfrHMg1MdmZ7jw=","FyhHXKzOniadUAYKGCYHdruWmfQx5GPUNqi9PATUWa8=","EBSZRilEyPnRyWk7mD6wHkvLajaKw60om/IyXwCFhGU="],"leafs":[{"record":{"id":"hkKGGV7FQ0m/Oyx7+kLLNA==","upstream":{"request":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"b9RfmuKUuZzJ19z9LemIv5OOm/twk3rvfod+fEmZupd6j/efuIv1bLkhguudJUzGFAqObBKPBJ3U3jv9xSnBDA=="}},"ack":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","bytesStoredOrDelivered":"2789096","timestamp":"1727332044907","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"Z4IFvxknUG+Itz6kPGeLnKnjE5h6PaoNDGeadEuaqY7XHlavIY14Wn+GlFltdCEIK8S2UHmdxuyenekbTrNyBQ=="}}},"downstream":[{"request":{"parentRequest":{"requestId":"17699178-f98f-4f3b-b4a8-39f4d765936d","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"b9RfmuKUuZzJ19z9LemIv5OOm/twk3rvfod+fEmZupd6j/efuIv1bLkhguudJUzGFAqObBKPBJ3U3jv9xSnBDA=="}},"requestId":"a10bf6c8-9da9-429b-a5ac-485bde0081fd","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"MgOezVKgqjSmMCzYNiludAciy7lBcUKjP0sNKuj3jLBQKkF1sL92ExjW/bDnvFFuRCxpAlrLZJEtDXe8Jg/pCQ=="}},"response":{"status":"OK","peerID":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI="}},{"request":{"requestId":"fe35db07-5fb4-488b-9d86-ac7a6b754c86","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"mCu4MVaJE1adkEd2r6jD2fxHcGvxaQZ7OC1+8W88GdmnLkZwKSIxW4dhdC6Mynjt7VCYC8CTbLfVhfeqD3C7DA=="}},"response":{"status":"OK","time":6,"peerID":"MC+TffOg7ExljoEiQ550jSJ0QuvUk871IaHhSUOEQ5U="}},{"request":{"requestId":"71d7fa68-00bf-44bc-b16e-120f24970370","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hyXyAgiCe9Ny8lAY0tJO/qI82FQFfbYyQK/QZq385XDpOkHRhrPGp9+XOAmxTZE+wbJnPPd+YAy+baXVMHuGBQ=="}},"response":{"status":"OK","time":43,"peerID":"8vUhAU5Da0JuQneyMmdlWuBNGFjIR1bZ7ZcNFycdGeQ="}},{"request":{"requestId":"7ad4580b-1df5-4060-a81b-e020db9d20c1","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","pieceCid":"AQIeII33s/PGqGZ8rKLrGXCBfH6Y0ariwQz2R3pYlbZh0GtA","offset":"3145728","size":"1048576","timestamp":"1727332044730","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"gAgc21i2RzYxxjISv0g70ZJInZT3hr1zl+HkQX5LS/CJda3QEhYM8CbkaVEFMb+aghrNyMyFMnHDEb2bNOh2Bw=="}},"response":{"status":"OK","time":172,"peerID":"SNu4dd8/d4Fs0BtajObzKUSuSsO0RTuTRcMyBolEXog="}}],"timestamp":"1727332044728","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"hfofRwv5/6Mqgt+M/DbKMCqR4R0DLlvkKjV8n9/VCtB5eA852xKwZQePAZhCSvlf9lwM+9fVwYdU6JfxTP00DA=="}},"transferred_bytes":2789096,"stored_bytes":0},{"record":{"id":"ldpO+ZPnSLWvotPE4C3rnQ==","upstream":{"request":{"requestId":"16af68b2-5bf3-4df6-92c1-be284190c760","requestType":"REQUEST_TYPE_GET","bucketId":"123229","pieceCid":"ZGRjLXBsYXlncm91bmQvc29ueV80a182MF9mcHMubXA0","timestamp":"1727332045892","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"EJtOzQQnxxGjHqippQ0UtLp5CDGCPFUOvDThiVZc6SGctgV0QBMcSGrwo513S0gy1dLH3TaPLt0FBesVj9AgCg=="}},"ack":{"requestId":"16af68b2-5bf3-4df6-92c1-be284190c760","bytesStoredOrDelivered":"760372","timestamp":"1727332045893","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"u/ZwcGta6DIPVpqGPZZrolFJljSrk0gjj03ULThHCczOgzTYnIP2Q5xxo5Pwcm3ZnRvRjrjGX+ByPpKSJvBXCA=="}}},"timestamp":"1727332045892","signature":{"signer":"H1DxRV9g9XdFZCM9MhoRbKRa4xiLIgCZlEVwbQSDnXI=","value":"huMb7m3yz1PAdeIBF7yNzHfrab2KzqyC8A7o8eG/b7KaWq2QSEQZ5nZuNsz9PZS4lWu1KvOW3OfgesSsl2bEDA=="}},"transferred_bytes":760372,"stored_bytes":0}]}]}"#.to_vec()),
			sent: true,
			..Default::default()
		};

		offchain_state.expect_request(pending_request1);
		offchain_state.expect_request(pending_request2);
		offchain_state.expect_request(pending_request3);
		offchain_state.expect_request(pending_request4);
		drop(offchain_state);

		let cluster_id = ClusterId::from([1; 20]);
		let era_id = 5757773;

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

		let bucket_node_aggregates_not_in_consensus: Vec<BucketNodeAggregatesActivity> =
			vec![BucketNodeAggregatesActivity {
				bucket_id: 123229,
				node_id: "0x1f50f1455f60f5774564233d321a116ca45ae3188b2200999445706d04839d72"
					.to_string(),
				stored_bytes: 0,
				transferred_bytes: 25143977,
				number_of_puts: 0,
				number_of_gets: 10,
			}];

		let _result =
			DdcVerification::challenge_sub_aggregates_not_in_consensus(&cluster_id, era_id, &dac_nodes, bucket_node_aggregates_not_in_consensus);

		//assert!(result.is_ok());

	});
}
