use ddc_primitives::{ClusterId, MergeActivityHash, StorageNodeParams, StorageNodePubKey};
use frame_support::{assert_noop, assert_ok};
use sp_core::offchain::{
	testing::{PendingRequest, TestOffchainExt, TestTransactionPoolExt},
	OffchainDbExt, OffchainWorkerExt, Timestamp, TransactionPoolExt,
};
use sp_io::TestExternalities;
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
			provider_id: [1; 32],
			node_id: [1; 32],
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		};
		let node_activity2 = NodeActivity {
			provider_id: [2; 32],
			node_id: [2; 32],
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
			uri: format!("http://{}:{}/activity/node?eraId={}", host, port, era_id),
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
			customer_id: [1; 32],
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		};
		let customer_activity2 = CustomerActivity {
			bucket_id: 222,
			customer_id: [2; 32],
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
			customer_id: [0; 32],
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: [0; 32],
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: [0; 32],
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
			customer_id: [0; 32],
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: [0; 32],
			bucket_id: 1,
			stored_bytes: 200,
			transferred_bytes: 100,
			number_of_puts: 20,
			number_of_gets: 40,
		},
		CustomerActivity {
			customer_id: [0; 32],
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
			customer_id: [0; 32],
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: [0; 32],
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: [0; 32],
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
			customer_id: [0; 32],
			bucket_id: 1,
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		},
		CustomerActivity {
			customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				provider_id: [0; 32],
				node_id: [0; 32],
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![NodeActivity {
				provider_id: [0; 32],
				node_id: [0; 32],
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_2,
			vec![NodeActivity {
				provider_id: [0; 32],
				node_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				provider_id: [0; 32],
				node_id: [0; 32],
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![NodeActivity {
				provider_id: [0; 32],
				node_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				customer_id: [0; 32],
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
				node_id: [0; 32],
				provider_id: [0; 32],
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![NodeActivity {
				node_id: [0; 32],
				provider_id: [0; 32],
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2,
			vec![NodeActivity {
				node_id: [0; 32],
				provider_id: [0; 32],
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
fn test_empty_activities() {
	let activities: Vec<NodeActivity> = vec![];
	let result = DdcVerification::split_to_batches(&activities, 3);
	assert_eq!(result, Vec::<Vec<NodeActivity>>::new());
}

#[test]
fn test_single_batch() {
	let node1 = NodeActivity {
		node_id: [0; 32],
		provider_id: [0; 32],
		stored_bytes: 100,
		transferred_bytes: 50,
		number_of_puts: 10,
		number_of_gets: 20,
	};
	let node2 = NodeActivity {
		node_id: [1; 32],
		provider_id: [1; 32],
		stored_bytes: 101,
		transferred_bytes: 51,
		number_of_puts: 11,
		number_of_gets: 21,
	};
	let node3 = NodeActivity {
		node_id: [2; 32],
		provider_id: [2; 32],
		stored_bytes: 102,
		transferred_bytes: 52,
		number_of_puts: 12,
		number_of_gets: 22,
	};
	let activities = vec![node1.clone(), node2.clone(), node3.clone()];
	let mut sorted_activities = vec![node1.clone(), node2.clone(), node3.clone()];
	sorted_activities.sort();
	let result = DdcVerification::split_to_batches(&activities, 5);
	assert_eq!(result, vec![sorted_activities]);
}

#[test]
fn test_exact_batches() {
	let node1 = NodeActivity {
		node_id: [0; 32],
		provider_id: [0; 32],
		stored_bytes: 100,
		transferred_bytes: 50,
		number_of_puts: 10,
		number_of_gets: 20,
	};
	let node2 = NodeActivity {
		node_id: [1; 32],
		provider_id: [1; 32],
		stored_bytes: 101,
		transferred_bytes: 51,
		number_of_puts: 11,
		number_of_gets: 21,
	};
	let node3 = NodeActivity {
		node_id: [2; 32],
		provider_id: [2; 32],
		stored_bytes: 102,
		transferred_bytes: 52,
		number_of_puts: 12,
		number_of_gets: 22,
	};
	let node4 = NodeActivity {
		node_id: [3; 32],
		provider_id: [3; 32],
		stored_bytes: 103,
		transferred_bytes: 53,
		number_of_puts: 13,
		number_of_gets: 23,
	};
	let activities = vec![node1.clone(), node2.clone(), node3.clone(), node4.clone()];
	let mut sorted_activities = vec![node1.clone(), node2.clone(), node3.clone(), node4.clone()];
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
fn test_non_exact_batches() {
	let node1 = NodeActivity {
		node_id: [0; 32],
		provider_id: [0; 32],
		stored_bytes: 100,
		transferred_bytes: 50,
		number_of_puts: 10,
		number_of_gets: 20,
	};
	let node2 = NodeActivity {
		node_id: [1; 32],
		provider_id: [1; 32],
		stored_bytes: 101,
		transferred_bytes: 51,
		number_of_puts: 11,
		number_of_gets: 21,
	};
	let node3 = NodeActivity {
		node_id: [2; 32],
		provider_id: [2; 32],
		stored_bytes: 102,
		transferred_bytes: 52,
		number_of_puts: 12,
		number_of_gets: 22,
	};
	let node4 = NodeActivity {
		node_id: [3; 32],
		provider_id: [3; 32],
		stored_bytes: 103,
		transferred_bytes: 53,
		number_of_puts: 13,
		number_of_gets: 23,
	};
	let node5 = NodeActivity {
		node_id: [3; 32],
		provider_id: [3; 32],
		stored_bytes: 104,
		transferred_bytes: 54,
		number_of_puts: 14,
		number_of_gets: 24,
	};
	let activities =
		vec![node1.clone(), node2.clone(), node3.clone(), node4.clone(), node5.clone()];
	let mut sorted_activities =
		vec![node1.clone(), node2.clone(), node3.clone(), node4.clone(), node5.clone()];
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
				node_id: [0; 32],
				provider_id: [0; 32],
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1.clone(),
			vec![NodeActivity {
				node_id: [0; 32],
				provider_id: [0; 32],
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2.clone(),
			vec![NodeActivity {
				node_id: [0; 32],
				provider_id: [0; 32],
				stored_bytes: 300,
				transferred_bytes: 150,
				number_of_puts: 30,
				number_of_gets: 60,
			}],
		),
		(
			node_pubkey_0,
			vec![NodeActivity {
				node_id: [1; 32],
				provider_id: [0; 32],
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![NodeActivity {
				node_id: [1; 32],
				provider_id: [0; 32],
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2,
			vec![NodeActivity {
				node_id: [1; 32],
				provider_id: [0; 32],
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
				node_id: [0; 32],
				provider_id: [0; 32],
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1.clone(),
			vec![NodeActivity {
				node_id: [0; 32],
				provider_id: [0; 32],
				stored_bytes: 200,
				transferred_bytes: 100,
				number_of_puts: 20,
				number_of_gets: 40,
			}],
		),
		(
			node_pubkey_2.clone(),
			vec![NodeActivity {
				node_id: [0; 32],
				provider_id: [0; 32],
				stored_bytes: 300,
				transferred_bytes: 150,
				number_of_puts: 30,
				number_of_gets: 60,
			}],
		),
		(
			node_pubkey_0,
			vec![NodeActivity {
				node_id: [1; 32],
				provider_id: [0; 32],
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		),
		(
			node_pubkey_1,
			vec![NodeActivity {
				node_id: [1; 32],
				provider_id: [0; 32],
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
		let era_activity1 = EraActivity { id: 17 };
		let era_activity2 = EraActivity { id: 18 };
		let era_activity3 = EraActivity { id: 19 };
		let era_activity_json =
			serde_json::to_string(&vec![era_activity1, era_activity2, era_activity3]).unwrap();

		// Mock HTTP request and response
		let pending_request = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/era", host, port),
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
	ext.register_extension(OffchainDbExt::new(Box::new(offchain)));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		let mut offchain_state = offchain_state.write();
		offchain_state.timestamp = Timestamp::from_unix_millis(0);
		let host1 = "example1.com";
		let host2 = "example2.com";
		let host3 = "example3.com";
		let host4 = "example4.com";
		let port = 80;
		let era_activity1 = EraActivity { id: 16 };
		let era_activity2 = EraActivity { id: 17 };
		let era_activity3 = EraActivity { id: 18 };
		let era_activity4 = EraActivity { id: 19 };
		let era_activity_json1 = serde_json::to_string(&vec![
			era_activity1, //16
			era_activity2, //17
			era_activity3, //18
			era_activity4, //19
		])
		.unwrap();
		let era_activity_json2 = serde_json::to_string(&vec![
			era_activity1, //16
			era_activity2, //17
			era_activity3, //18
		])
		.unwrap();
		let era_activity_json3 = serde_json::to_string(&vec![
			era_activity1, //16
			era_activity2, //17
			era_activity3, //18
		])
		.unwrap();
		let era_activity_json4 = serde_json::to_string(&vec![
			era_activity1, //16
			era_activity2, //17
			era_activity3, //18
		])
		.unwrap();
		let pending_request1 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/era", host1, port),
			response: Some(era_activity_json1.as_bytes().to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/era", host2, port),
			response: Some(era_activity_json2.as_bytes().to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/era", host3, port),
			response: Some(era_activity_json3.as_bytes().to_vec()),
			sent: true,
			..Default::default()
		};
		let pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/era", host4, port),
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
		assert_eq!(result.unwrap().unwrap(), era_activity1.id); //16
	});
}

#[test]
fn test_get_last_validated_era() {
	let cluster_id = ClusterId::from([12; 20]);
	let era_1 = 1;
	let era_2 = 2;
	let payers_root: ActivityHash = [1; 32];
	let payees_root: ActivityHash = [2; 32];
	let validator: AccountId32 = [0; 32].into();

	new_test_ext().execute_with(|| {
		// Initially, there should be no validated eras
		assert_ok!(Pallet::<Test>::get_last_validated_era(&cluster_id, validator.clone()).map(
			|era| {
				assert_eq!(era, None);
			}
		));

		// Insert some validations
		let mut validators_map_1 = BTreeMap::new();
		validators_map_1.insert((payers_root, payees_root), vec![validator.clone()]);

		let validation_1 = EraValidation {
			validators: validators_map_1,
			payers_merkle_root_hash: payers_root,
			payees_merkle_root_hash: payees_root,
			status: EraValidationStatus::ValidatingData,
		};

		<EraValidations<Test>>::insert(cluster_id, era_1, validation_1);

		let mut validators_map_2 = BTreeMap::new();
		validators_map_2.insert((payers_root, payees_root), vec![validator.clone()]);

		let validation_2 = EraValidation {
			validators: validators_map_2,
			payers_merkle_root_hash: payers_root,
			payees_merkle_root_hash: payees_root,
			status: EraValidationStatus::ValidatingData,
		};

		<EraValidations<Test>>::insert(cluster_id, era_2, validation_2);

		// Now the last validated era should be ERA_2
		assert_ok!(Pallet::<Test>::get_last_validated_era(&cluster_id, validator).map(|era| {
			assert_eq!(era, Some(era_2));
		}));
	});
}

#[test]
fn merkle_tree_create_verify_works() {
	new_test_ext().execute_with(|| {
		let a: ActivityHash = [0; 32];
		let b: ActivityHash = [1; 32];
		let c: ActivityHash = [2; 32];
		let d: ActivityHash = [3; 32];
		let e: ActivityHash = [4; 32];
		let f: ActivityHash = [5; 32];

		let leaves = vec![a, b, c, d, e];

		let root = DdcVerification::create_merkle_root(&leaves).unwrap();

		assert_eq!(
			root,
			[
				205, 34, 92, 22, 66, 39, 53, 146, 126, 111, 191, 174, 107, 224, 161, 127, 150, 69,
				255, 15, 237, 252, 116, 39, 186, 26, 40, 154, 180, 110, 185, 7
			]
		);

		let store = MemStore::default();
		let mut mmr: MMR<ActivityHash, MergeActivityHash, &MemStore<ActivityHash>> =
			MemMMR::<_, MergeActivityHash>::new(0, &store);
		let leaf_position_map: Vec<(ActivityHash, u64)> = leaves
			.iter()
			.map(
				|a| (*a, mmr.push(*a).unwrap()), // todo! Need to remove unwrap
			)
			.collect();

		let leaf_position: Vec<(u64, ActivityHash)> = leaf_position_map
			.into_iter()
			.filter(|(l, _)| l == &c)
			.map(|(l, p)| (p, l))
			.collect();
		let position: Vec<u64> = leaf_position.clone().into_iter().map(|(p, _)| p).collect();

		assert!(DdcVerification::proof_merkle_leaf(
			root,
			mmr.gen_proof(position.clone()).unwrap(),
			leaf_position
		)
		.unwrap());

		assert_noop!(
			DdcVerification::proof_merkle_leaf(
				root,
				mmr.gen_proof(position).unwrap(),
				vec![(6, f)]
			),
			Error::<Test>::FailToVerifyMerkleProof
		);
	});
}
