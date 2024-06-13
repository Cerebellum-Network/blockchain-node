use ddc_primitives::{ClusterId, StorageNodeParams, StorageNodePubKey, KEY_TYPE};
use frame_support::{assert_noop, assert_ok};
use sp_core::{
	offchain::{
		testing::{PendingRequest, TestOffchainExt, TestTransactionPoolExt},
		OffchainDbExt, OffchainWorkerExt, Timestamp, TransactionPoolExt,
	},
	Pair, H256,
};
use sp_io::TestExternalities;
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
use sp_runtime::AccountId32;

use crate::{mock::*, ConsensusError, Error, Event, NodeActivity, *};

#[test]
fn create_billing_reports_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let dac_account = AccountId::from([1; 32]);
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let merkel_root_hash: H256 = array_bytes::hex_n_into_unchecked(
			"95803defe6ea9f41e7ec6afa497064f21bfded027d8812efacbdf984e630cbdc",
		);

		assert_ok!(DdcVerification::set_prepare_era_for_payout(
			RuntimeOrigin::signed(dac_account.clone()),
			cluster_id,
			era,
			ActivityHash::from(merkel_root_hash),
			ActivityHash::from(merkel_root_hash),
		));

		System::assert_last_event(Event::BillingReportCreated { cluster_id, era }.into());

		let report =
			DdcVerification::active_billing_reports(cluster_id, dac_account.clone()).unwrap();
		assert_eq!(report.payers_merkle_root_hash, ActivityHash::from(merkel_root_hash));

		assert_noop!(
			DdcVerification::set_prepare_era_for_payout(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				ActivityHash::from(merkel_root_hash),
				ActivityHash::from(merkel_root_hash)
			),
			Error::<Test>::BillingReportAlreadyExist
		);
	})
}

#[test]
fn set_validate_payout_batch_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let account_id1 = AccountId::from([0xa; 32]);
		let account_id2 = AccountId::from([0xb; 32]);
		let account_id3 = AccountId::from([0xc; 32]);
		let account_id4 = AccountId::from([0xd; 32]);
		let account_id6 = AccountId::from([0xe; 32]);
		let merkel_root_hash: H256 = array_bytes::hex_n_into_unchecked(
			"95803defe6ea9f41e7ec6afa497064f21bfded027d8812efacbdf984e630cbdc",
		);

		ValidatorSet::<Test>::put(vec![
			account_id1.clone(),
			account_id2.clone(),
			account_id3.clone(),
			account_id4.clone(),
			account_id6.clone(),
		]);
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let payout_data = PayoutData { hash: ActivityHash::default() };
		let payout_data1 = PayoutData { hash: ActivityHash::from(merkel_root_hash) };

		let validator_0 = AccountId::from([0xaa; 32]);
		assert_noop!(
			DdcVerification::set_validator_key(
				// register validator 1
				RuntimeOrigin::signed(validator_0.clone()),
				validator_0.clone(),
			),
			Error::<Test>::NotController
		);

		let validator_1 = AccountId::from([0xf; 32]);

		DdcVerification::set_validator_key(
			// register validator 1
			RuntimeOrigin::signed(validator_1.clone()),
			validator_1.clone(),
		)
		.unwrap();

		// 1. If user is not part of validator, he/she won't be able to sign extrinsic
		assert_noop!(
			DdcVerification::set_validate_payout_batch(
				RuntimeOrigin::signed(AccountId::from([0xff; 32])),
				cluster_id,
				era,
				payout_data.clone(),
			),
			Error::<Test>::DDCValidatorKeyNotRegistered
		);

		assert_noop!(
			DdcVerification::set_validate_payout_batch(
				RuntimeOrigin::signed(validator_1),
				cluster_id,
				era,
				payout_data.clone(),
			),
			Error::<Test>::Unauthorised
		);

		// 2. send signed transaction from valid validator
		DdcVerification::set_validator_key(
			// register validator 1
			RuntimeOrigin::signed(account_id1.clone()),
			account_id1.clone(),
		)
		.unwrap();

		assert_ok!(DdcVerification::set_validate_payout_batch(
			RuntimeOrigin::signed(account_id1.clone()),
			cluster_id,
			era,
			payout_data.clone(),
		));

		// 3. If validator already sent the data, he/she won't be able to submit same transaction
		assert_noop!(
			DdcVerification::set_validate_payout_batch(
				RuntimeOrigin::signed(AccountId::from(account_id1)),
				cluster_id,
				era,
				payout_data.clone(),
			),
			Error::<Test>::AlreadySignedPayoutBatch
		);

		// 4. send signed transaction from second valid validator
		DdcVerification::set_validator_key(
			// register validator 1
			RuntimeOrigin::signed(account_id2.clone()),
			account_id2.clone(),
		)
		.unwrap();

		assert_ok!(DdcVerification::set_validate_payout_batch(
			RuntimeOrigin::signed(account_id2.clone()),
			cluster_id,
			era,
			payout_data.clone(),
		));

		// 5. 2/3 rd validators have not signed yet the same data
		assert_eq!(DdcVerification::payout_batch(cluster_id, era), None);

		DdcVerification::set_validator_key(
			// register validator 1
			RuntimeOrigin::signed(account_id3.clone()),
			account_id3.clone(),
		)
		.unwrap();

		assert_ok!(DdcVerification::set_validate_payout_batch(
			RuntimeOrigin::signed(account_id3.clone()),
			cluster_id,
			era,
			payout_data.clone(),
		));

		// 6. send signed transaction from third valid validator but different hash

		DdcVerification::set_validator_key(
			// register validator 1
			RuntimeOrigin::signed(account_id6.clone()),
			account_id6.clone(),
		)
		.unwrap();

		assert_ok!(DdcVerification::set_validate_payout_batch(
			RuntimeOrigin::signed(account_id6.clone()),
			cluster_id,
			era,
			payout_data1.clone(),
		));

		// 7. 2/3 rd validators have not signed yet the same data
		assert_eq!(DdcVerification::payout_batch(cluster_id, era), None);

		// 8. send signed transaction from fourth valid validator
		DdcVerification::set_validator_key(
			// register validator 1
			RuntimeOrigin::signed(account_id4.clone()),
			account_id4.clone(),
		)
		.unwrap();
		assert_ok!(DdcVerification::set_validate_payout_batch(
			RuntimeOrigin::signed(account_id4.clone()),
			cluster_id,
			era,
			payout_data.clone(),
		));

		// 9. 2/3rd validators have sent the same data with same hash
		assert_eq!(DdcVerification::payout_batch(cluster_id, era).unwrap(), payout_data);
	})
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
		ConsensusError::NotEnoughNodesForConsensus { cluster_id, era_id, id } => {
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
		ConsensusError::NotEnoughNodesForConsensus { cluster_id, era_id, id } => {
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
		ConsensusError::ActivityNotInConsensus { cluster_id, era_id, id } => {
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
		ConsensusError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, customers_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
	match &errors[0] {
		ConsensusError::ActivityNotInConsensus { cluster_id, era_id, id } => {
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
		ConsensusError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, customers_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
	match &errors[0] {
		ConsensusError::NotEnoughNodesForConsensus { cluster_id, era_id, id } => {
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
		ConsensusError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, nodes_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
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
		ConsensusError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, nodes_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
	match &errors[1] {
		ConsensusError::ActivityNotInConsensus { cluster_id, era_id, id } => {
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
		ConsensusError::ActivityNotInConsensus { cluster_id, era_id, id } => {
			assert_eq!(*id, nodes_activity[0].1[0].get_consensus_id::<mock::Test>());
			assert_eq!(*cluster_id, cluster_id1);
			assert_eq!(*era_id, era_id1);
		},
		_ => panic!("Expected CustomerActivityNotInConsensus error"),
	}
	match &errors[1] {
		ConsensusError::NotEnoughNodesForConsensus { cluster_id, era_id, id } => {
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

		let result = Pallet::<Test>::fetch_processed_era(node_params);
		assert!(result.is_ok());
		let activities = result.unwrap();
		assert_eq!(activities[0].id, era_activity1.id);
		assert_eq!(activities[1].id, era_activity2.id);
	});
}

#[test]
fn get_era_to_prepare_for_payout_works() {
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

		let dac_account = AccountId::from([1; 32]);
		let cluster_id = ClusterId::from([12; 20]);
		let era = 16;
		let merkel_root_hash: H256 = array_bytes::hex_n_into_unchecked(
			"95803defe6ea9f41e7ec6afa497064f21bfded027d8812efacbdf984e630cbdc",
		);

		assert_noop!(
			Pallet::<Test>::get_era_to_prepare_for_payout(cluster_id, dac_nodes.clone()),
			Error::<Test>::EraToValidateRetrievalError
		);

		assert_ok!(DdcVerification::set_prepare_era_for_payout(
			RuntimeOrigin::signed(dac_account.clone()),
			cluster_id,
			era,
			ActivityHash::from(merkel_root_hash),
			ActivityHash::from(merkel_root_hash),
		));

		let result = Pallet::<Test>::get_era_to_prepare_for_payout(cluster_id, dac_nodes.clone());
		assert!(result.is_ok());
		assert_eq!(result.unwrap().unwrap(), era_activity2.id); //17
	});
}

#[test]
fn off_chain_worker_works() {
	let mut ext = TestExternalities::default();
	let (offchain, _offchain_state) = TestOffchainExt::new();
	let (pool, pool_state) = TestTransactionPoolExt::new();
	let cluster_id = ClusterId::from([12; 20]);
	let era = 16;
	let merkel_root_hash: H256 = array_bytes::hex_n_into_unchecked(
		"95803defe6ea9f41e7ec6afa497064f21bfded027d8812efacbdf984e630cbdc",
	);
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
		// Offchain worker should not be triggered if block number is not divided by 100
		let block = 102;
		System::set_block_number(block);

		DdcVerification::offchain_worker(block);
		assert_eq!(pool_state.write().transactions.pop(), None);

		// // Offchain worker should be triggered if block number is  divided by 100
		let block = 500;
		System::set_block_number(block);
		let dac_account = AccountId::from([1; 32]);

		ClusterToValidate::<Test>::put(cluster_id);
		let _ = DdcVerification::set_prepare_era_for_payout(
			RuntimeOrigin::signed(dac_account.clone()),
			cluster_id,
			era,
			ActivityHash::from(merkel_root_hash),
			ActivityHash::from(merkel_root_hash),
		);
		DdcVerification::offchain_worker(block);

		let tx = pool_state.write().transactions.pop().unwrap();

		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature.unwrap().0, 0);
		assert_eq!(
			tx.call,
			RuntimeCall::DdcVerification(Call::set_validate_payout_batch {
				cluster_id: Default::default(),
				era: DdcEra::default(),
				payout_data: PayoutData { hash: ActivityHash::default() },
			})
		);
	});
}
