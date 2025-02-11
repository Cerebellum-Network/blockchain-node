use ddc_primitives::{
	AggregatorInfo, ClusterId, DeltaUsageHash, MergeMMRHash, StorageNodeMode, StorageNodeParams,
	StorageNodePubKey, DAC_VERIFICATION_KEY_TYPE,
};
use frame_support::assert_noop;
use prost::Message;
use sp_core::{
	offchain::{
		testing::{PendingRequest, TestOffchainExt, TestTransactionPoolExt},
		OffchainDbExt, OffchainStorage, OffchainWorkerExt, Timestamp, TransactionPoolExt,
	},
	Pair, H256,
};
use sp_io::TestExternalities;
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
use sp_runtime::{offchain::Duration, AccountId32};

use crate::{mock::*, Error, *};

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

fn _get_validators() -> Vec<AccountId32> {
	let validator1: AccountId32 = [1; 32].into();
	let validator2: AccountId32 = [2; 32].into();
	let validator3: AccountId32 = [3; 32].into();
	let validator4: AccountId32 = [4; 32].into();
	let validator5: AccountId32 = [5; 32].into();

	vec![validator1, validator2, validator3, validator4, validator5]
}

fn get_node_activities() -> Vec<aggregator_client::json::NodeAggregate> {
	let aggregator = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.236".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example.com".to_vec(),
		},
	};

	let node1 = aggregator_client::json::NodeAggregate {
		node_id: "0".to_string(),
		stored_bytes: -100,
		transferred_bytes: 50,
		number_of_puts: 10,
		number_of_gets: 20,
		aggregator: aggregator.clone(),
	};
	let node2 = aggregator_client::json::NodeAggregate {
		node_id: "1".to_string(),
		stored_bytes: -101,
		transferred_bytes: 51,
		number_of_puts: 11,
		number_of_gets: 21,
		aggregator: aggregator.clone(),
	};
	let node3 = aggregator_client::json::NodeAggregate {
		node_id: "2".to_string(),
		stored_bytes: 102,
		transferred_bytes: 52,
		number_of_puts: 12,
		number_of_gets: 22,
		aggregator: aggregator.clone(),
	};
	let node4 = aggregator_client::json::NodeAggregate {
		node_id: "3".to_string(),
		stored_bytes: 103,
		transferred_bytes: 53,
		number_of_puts: 13,
		number_of_gets: 23,
		aggregator: aggregator.clone(),
	};
	let node5 = aggregator_client::json::NodeAggregate {
		node_id: "4".to_string(),
		stored_bytes: 104,
		transferred_bytes: 54,
		number_of_puts: 14,
		number_of_gets: 24,
		aggregator: aggregator.clone(),
	};
	vec![node1, node2, node3, node4, node5]
}

#[test]
fn fetch_node_aggregates_works() {
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

		// Create a sample aggregator_client::json::NodeAggregateResponse instance
		let node_activity1 = aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		};
		let node_activity2 = aggregator_client::json::NodeAggregateResponse {
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
			uri: format!(
				"http://{}:{}/activity/nodes?eraId={}&limit={}",
				host,
				port,
				era_id,
				pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE
			),
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

		let result = Pallet::<Test>::_v4_fetch_node_aggregates(&cluster_id, era_id, &node_params);
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
fn fetch_bucket_aggregates_works() {
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

		// Create a sample aggregator_client::json::NodeAggregateResponse instance
		let bucket_aggregate1 = aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 111,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		};
		let bucket_aggregate2 = aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 1000,
			transferred_bytes: 500,
			number_of_puts: 100,
			number_of_gets: 200,
			bucket_id: 222,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 1000,
				transferred_bytes: 500,
				number_of_puts: 100,
				number_of_gets: 200,
			}],
		};
		let customers_activity_json =
			serde_json::to_string(&vec![bucket_aggregate1.clone(), bucket_aggregate2.clone()])
				.unwrap();

		// Mock HTTP request and response
		let pending_request = PendingRequest {
			method: "GET".to_string(),
			uri: format!(
				"http://{}:{}/activity/buckets?eraId={}&limit={}",
				host,
				port,
				era_id,
				pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE
			),
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

		let result = Pallet::<Test>::_v4_fetch_bucket_aggregates(&cluster_id, era_id, &node_params);
		assert!(result.is_ok());
		let activities = result.unwrap();
		assert_eq!(
			activities[0].sub_aggregates[0].number_of_gets,
			bucket_aggregate1.sub_aggregates[0].number_of_gets
		);
		assert_eq!(
			activities[0].sub_aggregates[0].number_of_puts,
			bucket_aggregate1.sub_aggregates[0].number_of_puts
		);
		assert_eq!(
			activities[0].sub_aggregates[0].transferred_bytes,
			bucket_aggregate1.sub_aggregates[0].transferred_bytes
		);
		assert_eq!(
			activities[0].sub_aggregates[0].stored_bytes,
			bucket_aggregate1.sub_aggregates[0].stored_bytes
		);

		assert_eq!(
			activities[1].sub_aggregates[0].number_of_gets,
			bucket_aggregate2.sub_aggregates[0].number_of_gets
		);
		assert_eq!(
			activities[1].sub_aggregates[0].number_of_puts,
			bucket_aggregate2.sub_aggregates[0].number_of_puts
		);
		assert_eq!(
			activities[1].sub_aggregates[0].transferred_bytes,
			bucket_aggregate2.sub_aggregates[0].transferred_bytes
		);
		assert_eq!(
			activities[1].sub_aggregates[0].stored_bytes,
			bucket_aggregate2.sub_aggregates[0].stored_bytes
		);
	});
}

#[test]
fn buckets_sub_aggregates_in_consensus_merged() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(67);
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 476817;

	let aggregator1 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.236".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example1.com".to_vec(),
		},
	};

	let aggregator2 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "95.217.8.119".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		},
	};

	let aggregator3 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([3; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.42".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example3.com".to_vec(),
		},
	};

	let resp1 = (
		aggregator1,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 1,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		}],
	);

	let resp2 = (
		aggregator2,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 1,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		}],
	);

	let resp3 = (
		aggregator3,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 1,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		}],
	);

	let groups = DdcVerification::_v4_group_buckets_sub_aggregates_by_consistency(
		&cluster_id,
		era_id,
		vec![resp1, resp2, resp3],
		redundancy_factor,
		quorum,
	);
	assert_eq!(groups.consensus.len(), 1);
	assert_eq!(groups.quorum.len(), 0);
	assert_eq!(groups.others.len(), 0);

	let result = DdcVerification::_v4_get_total_usage(&cluster_id, era_id, groups, false);

	assert!(result.is_ok());
	let usages = result.unwrap();
	assert_eq!(usages.len(), 1); // 1 consolidated aggregate per 1 aggregation key

	let usage = usages.first().unwrap();
	assert_eq!(usage.stored_bytes, 100);
	assert_eq!(usage.transferred_bytes, 50);
	assert_eq!(usage.number_of_puts, 10);
	assert_eq!(usage.number_of_gets, 20);
}

#[test]
fn buckets_sub_aggregates_in_quorum_merged() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(67);
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 476817;

	let aggregator1 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.236".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example1.com".to_vec(),
		},
	};

	let aggregator2 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "95.217.8.119".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		},
	};

	let aggregator3 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([3; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.42".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example3.com".to_vec(),
		},
	};

	let resp1 = (
		aggregator1,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 1,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		}],
	);

	let resp2 = (
		aggregator2,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 200,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 1,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 200,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		}],
	);

	let resp3 = (
		aggregator3,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 1,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		}],
	);

	let groups = DdcVerification::_v4_group_buckets_sub_aggregates_by_consistency(
		&cluster_id,
		era_id,
		vec![resp1, resp2, resp3],
		redundancy_factor,
		quorum,
	);
	assert_eq!(groups.consensus.len(), 0);
	assert_eq!(groups.quorum.len(), 1); // 2 consistent aggregates merged into 1 in 'quorum'
	assert_eq!(groups.others.len(), 1); // 1 inconsistent aggregate goes to 'others'

	let result = DdcVerification::_v4_get_total_usage(&cluster_id, era_id, groups, false);

	assert!(result.is_ok());
	let usages = result.unwrap();
	assert_eq!(usages.len(), 1); // 1 consolidated aggregate per 1 aggregation key

	let usage = usages.first().unwrap();
	assert_eq!(usage.stored_bytes, 100);
	assert_eq!(usage.transferred_bytes, 50);
	assert_eq!(usage.number_of_puts, 10);
	assert_eq!(usage.number_of_gets, 20);
}

#[test]
fn buckets_sub_aggregates_in_others_merged() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(100);
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 476817;

	let aggregator1 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.236".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example1.com".to_vec(),
		},
	};

	let aggregator2 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "95.217.8.119".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		},
	};

	let aggregator3 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([3; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.42".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example3.com".to_vec(),
		},
	};

	let resp1 = (
		aggregator1,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 1,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		}],
	);

	let resp2 = (
		aggregator2,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 200,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 1,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 200,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		}],
	);

	let resp3 = (
		aggregator3,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 1,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		}],
	);

	let groups = DdcVerification::_v4_group_buckets_sub_aggregates_by_consistency(
		&cluster_id,
		era_id,
		vec![resp1, resp2, resp3],
		redundancy_factor,
		quorum,
	);

	assert_eq!(groups.consensus.len(), 0);
	assert_eq!(groups.quorum.len(), 0);
	assert_eq!(groups.others.len(), 2);

	let result = DdcVerification::_v4_get_total_usage(&cluster_id, era_id, groups, false);

	assert!(result.is_ok());
	let usages = result.unwrap();
	assert_eq!(usages.len(), 1); // 1 consolidated aggregate per 1 aggregation key

	let usage = usages.first().unwrap();
	assert_eq!(usage.stored_bytes, 100);
	assert_eq!(usage.transferred_bytes, 50);
	assert_eq!(usage.number_of_puts, 10);
	assert_eq!(usage.number_of_gets, 20);
}

#[test]
fn buckets_sub_aggregates_in_others_merged_2() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(100);
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 476817;

	let aggregator1 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.236".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example1.com".to_vec(),
		},
	};

	let aggregator2 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "95.217.8.119".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		},
	};

	let aggregator3 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([3; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.42".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example3.com".to_vec(),
		},
	};

	let resp1 = (
		aggregator1,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 1,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		}],
	);

	let resp2 = (
		aggregator2,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 200,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 2,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 200,
				transferred_bytes: 500,
				number_of_puts: 30,
				number_of_gets: 40,
			}],
		}],
	);

	let resp3 = (
		aggregator3,
		vec![aggregator_client::json::BucketAggregateResponse {
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			bucket_id: 1,
			sub_aggregates: vec![aggregator_client::json::BucketSubAggregateResponse {
				NodeID: "1".to_string(),
				stored_bytes: 100,
				transferred_bytes: 50,
				number_of_puts: 10,
				number_of_gets: 20,
			}],
		}],
	);

	let groups = DdcVerification::_v4_group_buckets_sub_aggregates_by_consistency(
		&cluster_id,
		era_id,
		vec![resp1, resp2, resp3],
		redundancy_factor,
		quorum,
	);

	assert_eq!(groups.consensus.len(), 0);
	assert_eq!(groups.quorum.len(), 0);
	assert_eq!(groups.others.len(), 2); // 2 inconsistent aggregates

	let result = DdcVerification::_v4_get_total_usage(&cluster_id, era_id, groups, false);

	assert!(result.is_ok());
	let usages = result.unwrap();
	assert_eq!(usages.len(), 2);

	let usage1 = usages.first().unwrap();
	assert_eq!(usage1.stored_bytes, 100);
	assert_eq!(usage1.transferred_bytes, 50);
	assert_eq!(usage1.number_of_puts, 10);
	assert_eq!(usage1.number_of_gets, 20);

	let usage2 = usages.get(1).unwrap();
	assert_eq!(usage2.stored_bytes, 200);
	assert_eq!(usage2.transferred_bytes, 500);
	assert_eq!(usage2.number_of_puts, 30);
	assert_eq!(usage2.number_of_gets, 40);
}

#[test]
fn nodes_aggregates_in_consensus_merged() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(67);
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 476817;

	let aggregator1 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.236".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example1.com".to_vec(),
		},
	};

	let aggregator2 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "95.217.8.119".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		},
	};

	let aggregator3 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([3; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.42".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example3.com".to_vec(),
		},
	};

	let resp1 = (
		aggregator1,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		}],
	);

	let resp2 = (
		aggregator2,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		}],
	);

	let resp3 = (
		aggregator3,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		}],
	);

	let groups = DdcVerification::_v4_group_nodes_aggregates_by_consistency(
		&cluster_id,
		era_id,
		vec![resp1, resp2, resp3],
		redundancy_factor,
		quorum,
	);
	assert_eq!(groups.consensus.len(), 1);
	assert_eq!(groups.quorum.len(), 0);
	assert_eq!(groups.others.len(), 0);

	let result = DdcVerification::_v4_get_total_usage(&cluster_id, era_id, groups, false);

	assert!(result.is_ok());
	let usages = result.unwrap();
	assert_eq!(usages.len(), 1); // 1 consolidated aggregate per 1 aggregation key

	let usage = usages.first().unwrap();
	assert_eq!(usage.stored_bytes, 100);
	assert_eq!(usage.transferred_bytes, 50);
	assert_eq!(usage.number_of_puts, 10);
	assert_eq!(usage.number_of_gets, 20);
}

#[test]
fn nodes_aggregates_in_quorum_merged() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(67);
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 476817;

	let aggregator1 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.236".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example1.com".to_vec(),
		},
	};

	let aggregator2 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "95.217.8.119".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		},
	};

	let aggregator3 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([3; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.42".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example3.com".to_vec(),
		},
	};

	let resp1 = (
		aggregator1,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		}],
	);

	let resp2 = (
		aggregator2,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 200,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		}],
	);

	let resp3 = (
		aggregator3,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		}],
	);

	let groups = DdcVerification::_v4_group_nodes_aggregates_by_consistency(
		&cluster_id,
		era_id,
		vec![resp1, resp2, resp3],
		redundancy_factor,
		quorum,
	);
	assert_eq!(groups.consensus.len(), 0);
	assert_eq!(groups.quorum.len(), 1); // 2 consistent aggregates merged into 1 in 'quorum'
	assert_eq!(groups.others.len(), 1); // 1 inconsistent aggregate goes to 'others'

	let result = DdcVerification::_v4_get_total_usage(&cluster_id, era_id, groups, false);

	assert!(result.is_ok());
	let usages = result.unwrap();
	assert_eq!(usages.len(), 1); // 1 consolidated aggregate per 1 aggregation key

	let usage = usages.first().unwrap();
	assert_eq!(usage.stored_bytes, 100);
	assert_eq!(usage.transferred_bytes, 50);
	assert_eq!(usage.number_of_puts, 10);
	assert_eq!(usage.number_of_gets, 20);
}

#[test]
fn nodes_aggregates_in_others_merged() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(100);
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 476817;

	let aggregator1 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.236".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example1.com".to_vec(),
		},
	};

	let aggregator2 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "95.217.8.119".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		},
	};

	let aggregator3 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([3; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.42".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example3.com".to_vec(),
		},
	};

	let resp1 = (
		aggregator1,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		}],
	);

	let resp2 = (
		aggregator2,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 200,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		}],
	);

	let resp3 = (
		aggregator3,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		}],
	);

	let groups = DdcVerification::_v4_group_nodes_aggregates_by_consistency(
		&cluster_id,
		era_id,
		vec![resp1, resp2, resp3],
		redundancy_factor,
		quorum,
	);

	assert_eq!(groups.consensus.len(), 0);
	assert_eq!(groups.quorum.len(), 0);
	assert_eq!(groups.others.len(), 2);

	let result = DdcVerification::_v4_get_total_usage(&cluster_id, era_id, groups, false);

	assert!(result.is_ok());
	let usages = result.unwrap();
	assert_eq!(usages.len(), 1); // 1 consolidated aggregate per 1 aggregation key

	let usage = usages.first().unwrap();
	assert_eq!(usage.stored_bytes, 200);
	assert_eq!(usage.transferred_bytes, 50);
	assert_eq!(usage.number_of_puts, 10);
	assert_eq!(usage.number_of_gets, 20);
}

#[test]
fn nodes_aggregates_in_others_merged_2() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(100);
	let cluster_id = ClusterId::from([1; 20]);
	let era_id = 476817;

	let aggregator1 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.236".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example1.com".to_vec(),
		},
	};

	let aggregator2 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "95.217.8.119".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example2.com".to_vec(),
		},
	};

	let aggregator3 = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([3; 32])),
		node_params: StorageNodeParams {
			ssl: false,
			host: "178.251.228.42".as_bytes().to_vec(),
			http_port: 8080,
			mode: StorageNodeMode::DAC,
			p2p_port: 5555,
			grpc_port: 4444,
			domain: b"example3.com".to_vec(),
		},
	};

	let resp1 = (
		aggregator1,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "2".to_string(),
			stored_bytes: 1000,
			transferred_bytes: 500,
			number_of_puts: 15,
			number_of_gets: 30,
		}],
	);

	let resp2 = (
		aggregator2,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 200,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		}],
	);

	let resp3 = (
		aggregator3,
		vec![aggregator_client::json::NodeAggregateResponse {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
		}],
	);

	let groups = DdcVerification::_v4_group_nodes_aggregates_by_consistency(
		&cluster_id,
		era_id,
		vec![resp1, resp2, resp3],
		redundancy_factor,
		quorum,
	);

	assert_eq!(groups.consensus.len(), 0);
	assert_eq!(groups.quorum.len(), 0);
	assert_eq!(groups.others.len(), 3); // 3 inconsistent aggregates

	let result = DdcVerification::_v4_get_total_usage(&cluster_id, era_id, groups, false);

	assert!(result.is_ok());
	let usages = result.unwrap();
	assert_eq!(usages.len(), 2);

	let usage1 = usages.get(1).unwrap();
	assert_eq!(usage1.stored_bytes, 200);
	assert_eq!(usage1.transferred_bytes, 50);
	assert_eq!(usage1.number_of_puts, 10);
	assert_eq!(usage1.number_of_gets, 20);

	let usage2 = usages.first().unwrap();
	assert_eq!(usage2.stored_bytes, 1000);
	assert_eq!(usage2.transferred_bytes, 500);
	assert_eq!(usage2.number_of_puts, 15);
	assert_eq!(usage2.number_of_gets, 30);
}

#[test]
fn buckets_sub_aggregates_grouped_by_consistency() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(67);
	let host = "example1.com";
	let port = 80;
	let node_params = StorageNodeParams {
		ssl: false,
		host: host.as_bytes().to_vec(),
		http_port: port,
		mode: StorageNodeMode::DAC,
		p2p_port: 5555,
		grpc_port: 4444,
		domain: b"example2.com".to_vec(),
	};
	let aggregator = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([0; 32])),
		node_params: node_params.clone(),
	};

	let buckets_sub_aggregates = vec![
		aggregator_client::json::BucketSubAggregate {
			bucket_id: 1,
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::BucketSubAggregate {
			bucket_id: 1,
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::BucketSubAggregate {
			bucket_id: 1,
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
	];

	let groups = DdcVerification::_v4_group_by_consistency(
		buckets_sub_aggregates,
		redundancy_factor,
		quorum,
	);

	assert_eq!(groups.consensus.len(), 1);
	assert_eq!(groups.quorum.len(), 0);
	assert_eq!(groups.others.len(), 0);

	let consolidated_aggregate = groups.consensus[0].aggregate.clone();
	assert_eq!(consolidated_aggregate.stored_bytes, 100);
	assert_eq!(consolidated_aggregate.transferred_bytes, 50);
	assert_eq!(consolidated_aggregate.number_of_puts, 10);
	assert_eq!(consolidated_aggregate.number_of_gets, 20);
	assert_eq!(groups.consensus[0].count, 3);
	assert_eq!(groups.consensus[0].aggregators.len(), 3);
}

#[test]
fn buckets_sub_aggregates_grouped_by_consistency_2() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(67);

	let host = "example1.com";
	let port = 80;
	let node_params = StorageNodeParams {
		ssl: false,
		host: host.as_bytes().to_vec(),
		http_port: port,
		mode: StorageNodeMode::DAC,
		p2p_port: 5555,
		grpc_port: 4444,
		domain: b"example2.com".to_vec(),
	};
	let aggregator = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([0; 32])),
		node_params: node_params.clone(),
	};

	let buckets_sub_aggregates = vec![
		aggregator_client::json::BucketSubAggregate {
			bucket_id: 1,
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::BucketSubAggregate {
			bucket_id: 1,
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::BucketSubAggregate {
			bucket_id: 1,
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::BucketSubAggregate {
			bucket_id: 2,
			node_id: "2".to_string(),
			stored_bytes: 110,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::BucketSubAggregate {
			bucket_id: 2,
			node_id: "2".to_string(),
			stored_bytes: 110,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::BucketSubAggregate {
			bucket_id: 2,
			node_id: "2".to_string(),
			stored_bytes: 110,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
	];

	let groups = DdcVerification::_v4_group_by_consistency(
		buckets_sub_aggregates,
		redundancy_factor,
		quorum,
	);

	assert_eq!(groups.consensus.len(), 2);
	assert_eq!(groups.quorum.len(), 0);
	assert_eq!(groups.others.len(), 0);

	let consolidated_aggregate_in_consensus_1 = groups.consensus[0].aggregate.clone();
	assert_eq!(consolidated_aggregate_in_consensus_1.bucket_id, 1);
	assert_eq!(consolidated_aggregate_in_consensus_1.stored_bytes, 100);
	assert_eq!(consolidated_aggregate_in_consensus_1.transferred_bytes, 50);
	assert_eq!(consolidated_aggregate_in_consensus_1.number_of_puts, 10);
	assert_eq!(consolidated_aggregate_in_consensus_1.number_of_gets, 20);
	assert_eq!(groups.consensus[0].count, 3);
	assert_eq!(groups.consensus[0].aggregators.len(), 3);

	let consolidated_aggregate_in_consensus_2 = groups.consensus[1].aggregate.clone();
	assert_eq!(consolidated_aggregate_in_consensus_2.bucket_id, 2);
	assert_eq!(consolidated_aggregate_in_consensus_2.stored_bytes, 110);
	assert_eq!(consolidated_aggregate_in_consensus_2.transferred_bytes, 50);
	assert_eq!(consolidated_aggregate_in_consensus_2.number_of_puts, 10);
	assert_eq!(consolidated_aggregate_in_consensus_2.number_of_gets, 20);
	assert_eq!(groups.consensus[1].count, 3);
	assert_eq!(groups.consensus[1].aggregators.len(), 3);
}

#[test]
fn nodes_aggregates_grouped_by_consistency() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(67);
	let host = "example1.com";
	let port = 80;
	let node_params = StorageNodeParams {
		ssl: false,
		host: host.as_bytes().to_vec(),
		http_port: port,
		mode: StorageNodeMode::DAC,
		p2p_port: 5555,
		grpc_port: 4444,
		domain: b"example2.com".to_vec(),
	};

	let aggregator = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([0; 32])),
		node_params: node_params.clone(),
	};

	let nodes_aggregates = vec![
		aggregator_client::json::NodeAggregate {
			node_id: "0".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::NodeAggregate {
			node_id: "0".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::NodeAggregate {
			node_id: "0".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
	];

	let groups =
		DdcVerification::_v4_group_by_consistency(nodes_aggregates, redundancy_factor, quorum);

	assert_eq!(groups.consensus.len(), 1);
	assert_eq!(groups.quorum.len(), 0);
	assert_eq!(groups.others.len(), 0);

	let consolidated_aggregate_in_consensus = groups.consensus[0].aggregate.clone();
	assert_eq!(consolidated_aggregate_in_consensus.stored_bytes, 100);
	assert_eq!(consolidated_aggregate_in_consensus.transferred_bytes, 50);
	assert_eq!(consolidated_aggregate_in_consensus.number_of_puts, 10);
	assert_eq!(consolidated_aggregate_in_consensus.number_of_gets, 20);
	assert_eq!(groups.consensus[0].count, 3);
	assert_eq!(groups.consensus[0].aggregators.len(), 3);
}

#[test]
fn nodes_aggregates_grouped_by_consistency_2() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(67);

	let host = "example1.com";
	let port = 80;
	let node_params = StorageNodeParams {
		ssl: false,
		host: host.as_bytes().to_vec(),
		http_port: port,
		mode: StorageNodeMode::DAC,
		p2p_port: 5555,
		grpc_port: 4444,
		domain: b"example2.com".to_vec(),
	};
	let aggregator = AggregatorInfo {
		node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([0; 32])),
		node_params: node_params.clone(),
	};

	let nodes_aggregates = vec![
		aggregator_client::json::NodeAggregate {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::NodeAggregate {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::NodeAggregate {
			node_id: "1".to_string(),
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::NodeAggregate {
			node_id: "2".to_string(),
			stored_bytes: 110,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::NodeAggregate {
			node_id: "2".to_string(),
			stored_bytes: 110,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
		aggregator_client::json::NodeAggregate {
			node_id: "2".to_string(),
			stored_bytes: 110,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			aggregator: aggregator.clone(),
		},
	];

	let groups =
		DdcVerification::_v4_group_by_consistency(nodes_aggregates, redundancy_factor, quorum);

	assert_eq!(groups.consensus.len(), 2);
	assert_eq!(groups.quorum.len(), 0);
	assert_eq!(groups.others.len(), 0);

	let consolidated_aggregate_1 = groups.consensus[0].aggregate.clone();
	assert_eq!(consolidated_aggregate_1.node_id, "2".to_string());
	assert_eq!(consolidated_aggregate_1.stored_bytes, 110);
	assert_eq!(consolidated_aggregate_1.transferred_bytes, 50);
	assert_eq!(consolidated_aggregate_1.number_of_puts, 10);
	assert_eq!(consolidated_aggregate_1.number_of_gets, 20);
	assert_eq!(groups.consensus[0].count, 3);
	assert_eq!(groups.consensus[0].aggregators.len(), 3);

	let consolidated_aggregate_2 = groups.consensus[1].aggregate.clone();

	assert_eq!(consolidated_aggregate_2.node_id, "1".to_string());
	assert_eq!(consolidated_aggregate_2.stored_bytes, 100);
	assert_eq!(consolidated_aggregate_2.transferred_bytes, 50);
	assert_eq!(consolidated_aggregate_2.number_of_puts, 10);
	assert_eq!(consolidated_aggregate_2.number_of_gets, 20);
	assert_eq!(groups.consensus[1].count, 3);
	assert_eq!(groups.consensus[1].aggregators.len(), 3);
}

#[test]
fn empty_bucket_sub_aggregates() {
	let redundancy_factor = 3;
	let quorum = Percent::from_percent(67);

	let empty = Vec::<aggregator_client::json::BucketSubAggregate>::new();
	let groups = DdcVerification::_v4_group_by_consistency(empty, redundancy_factor, quorum);

	assert_eq!(groups.consensus.len(), 0);
	assert_eq!(groups.quorum.len(), 0);
	assert_eq!(groups.others.len(), 0);
}

#[test]
fn bucket_sub_aggregates_are_fetched_and_grouped() {
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
			DAC_VERIFICATION_KEY_TYPE,
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
		let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();
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
        let port = 8080;

		let pending_request1 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476817&limit={}", host1, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa318","stored_bytes":578,"transferred_bytes":578,"number_of_puts":2,"number_of_gets":0}]},{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319","stored_bytes":0,"transferred_bytes":505,"number_of_puts":0,"number_of_gets":1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476817&limit={}", host2, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa318","stored_bytes":578,"transferred_bytes":578,"number_of_puts":2,"number_of_gets":0}]},{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319","stored_bytes":0,"transferred_bytes":506,"number_of_puts":0,"number_of_gets":1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476817&limit={}", host3, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa318","stored_bytes":578,"transferred_bytes":578,"number_of_puts":2,"number_of_gets":0}]},{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319","stored_bytes":0,"transferred_bytes":505,"number_of_puts":0,"number_of_gets":1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476817&limit={}", host4, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request5 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=476817&limit={}", host5, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"bucket_id":90235,"stored_bytes":0,"transferred_bytes":38,"number_of_puts":0,"number_of_gets":1,"sub_aggregates":[{"NodeID":"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa320","stored_bytes":578,"transferred_bytes":578,"number_of_puts":2,"number_of_gets":0}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};


        offchain_state.expect_request(pending_request1);
        offchain_state.expect_request(pending_request2);
        offchain_state.expect_request(pending_request3);
        offchain_state.expect_request(pending_request4);
        offchain_state.expect_request(pending_request5);

        drop(offchain_state);

        let cluster_id = ClusterId::from([1; 20]);
        let era_id = 476817;
        let redundancy_factor = 3;
        let aggregators_quorum = Percent::from_percent(67);

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

        let node_params5 = StorageNodeParams {
            ssl: false,
            host: host5.as_bytes().to_vec(),
            http_port: port,
            mode: StorageNodeMode::DAC,
            p2p_port: 5555,
            grpc_port: 4444,
            domain: b"example6.com".to_vec(),
        };

        let dac_nodes: Vec<(NodePubKey, StorageNodeParams)> = vec![
            (NodePubKey::StoragePubKey(StorageNodePubKey::new([1; 32])), node_params1.clone()),
            (NodePubKey::StoragePubKey(StorageNodePubKey::new([2; 32])), node_params2.clone()),
            (NodePubKey::StoragePubKey(StorageNodePubKey::new([3; 32])), node_params3.clone()),
            (NodePubKey::StoragePubKey(StorageNodePubKey::new([4; 32])), node_params4.clone()),
            (NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32])), node_params5.clone()),
        ];

        let bucket_aggregates_by_aggregator =
            DdcVerification::_v4_fetch_buckets_aggregates_for_era(&cluster_id, era_id, &dac_nodes)
                .unwrap();

        let groups =
            DdcVerification::_v4_group_buckets_sub_aggregates_by_consistency(&cluster_id, era_id, bucket_aggregates_by_aggregator, redundancy_factor, aggregators_quorum);


        // Sub aggregates which are in consensus
        let bucket_sub_aggregate_in_consensus = aggregator_client::json::BucketSubAggregate {
            bucket_id: 90235,
            node_id: "0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa318"
                .to_string(),
            stored_bytes: 578,
            transferred_bytes: 578,
            number_of_puts: 2,
            number_of_gets: 0,
            aggregator: AggregatorInfo {
                node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
                node_params: node_params1.clone(),
            },
        };

        assert_eq!(
            groups.consensus,
            vec![
                ConsolidatedAggregate::new(bucket_sub_aggregate_in_consensus, 3, vec![AggregatorInfo {
                    node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
                    node_params: node_params1.clone(),
                }, AggregatorInfo {
                    node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
                    node_params: node_params2.clone(),
                }, AggregatorInfo {
                    node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([3; 32])),
                    node_params: node_params3.clone(),
                }])
            ]
        );

        // Sub aggregates which are in quorum
        let bucket_sub_aggregate_in_quorum = aggregator_client::json::BucketSubAggregate {
            bucket_id: 90235,
            node_id: "0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319"
                .to_string(),
            stored_bytes: 0,
            transferred_bytes: 505,
            number_of_puts: 0,
            number_of_gets: 1,
            aggregator: AggregatorInfo {
                node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
                node_params: node_params1.clone(),
            },
        };

        assert_eq!(
            groups.quorum,
            vec![
                ConsolidatedAggregate::new(bucket_sub_aggregate_in_quorum, 2, vec![AggregatorInfo {
                    node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
                    node_params: node_params1.clone(),
                }, AggregatorInfo {
                    node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([3; 32])),
                    node_params: node_params3.clone(),
                }])
            ]
        );

        // Others sub aggregates
        let bucket_sub_aggregate1_in_others = aggregator_client::json::BucketSubAggregate {
            bucket_id: 90235,
            node_id: "0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319"
                .to_string(),
            stored_bytes: 0,
            transferred_bytes: 506,
            number_of_puts: 0,
            number_of_gets: 1,
            aggregator: AggregatorInfo {
                node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
                node_params: node_params2.clone(),
            },
        };

        let bucket_sub_aggregate2_in_others = aggregator_client::json::BucketSubAggregate {
            bucket_id: 90235,
            node_id: "0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa320"
                .to_string(),
            stored_bytes: 578,
            transferred_bytes: 578,
            number_of_puts: 2,
            number_of_gets: 0,
            aggregator: AggregatorInfo {
                node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([5; 32])),
                node_params: node_params5.clone(),
            },
        };

        assert_eq!(
            groups.others,
            vec![
                ConsolidatedAggregate::new(bucket_sub_aggregate2_in_others, 1, vec![AggregatorInfo {
                    node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([5; 32])),
                    node_params: node_params5.clone(),
                }]),
                ConsolidatedAggregate::new(bucket_sub_aggregate1_in_others, 1, vec![AggregatorInfo {
                    node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
                    node_params: node_params2.clone(),
                }]),
            ]
        );
    });
}

#[test]
fn node_aggregates_are_fetched_and_grouped() {
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
			DAC_VERIFICATION_KEY_TYPE,
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
		let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();
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
        let port = 8080;

		let pending_request1 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476817&limit={}", host1, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476817&limit={}", host2, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 48,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476817&limit={}", host3, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476817&limit={}", host4, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let pending_request5 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=476817&limit={}", host5, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0xfc28d5f5bb10212077a8654f62c4f8f0b5ab985fc322a51f5a3c75943b29194b","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

        offchain_state.expect_request(pending_request1);
        offchain_state.expect_request(pending_request2);
        offchain_state.expect_request(pending_request3);
        offchain_state.expect_request(pending_request4);
        offchain_state.expect_request(pending_request5);

        drop(offchain_state);

        let cluster_id = ClusterId::from([1; 20]);
        let era_id = 476817;
        let redundancy_factor = 3;
        let aggregators_quorum = Percent::from_percent(67);

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

        let node_params5 = StorageNodeParams {
            ssl: false,
            host: host5.as_bytes().to_vec(),
            http_port: port,
            mode: StorageNodeMode::DAC,
            p2p_port: 5555,
            grpc_port: 4444,
            domain: b"example6.com".to_vec(),
        };

        let dac_nodes: Vec<(NodePubKey, StorageNodeParams)> = vec![
            (NodePubKey::StoragePubKey(StorageNodePubKey::new([1; 32])), node_params1.clone()),
            (NodePubKey::StoragePubKey(StorageNodePubKey::new([2; 32])), node_params2.clone()),
            (NodePubKey::StoragePubKey(StorageNodePubKey::new([3; 32])), node_params3.clone()),
            (NodePubKey::StoragePubKey(StorageNodePubKey::new([4; 32])), node_params4.clone()),
            (NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32])), node_params5.clone()),
        ];

        let aggregates_by_aggregator =
            DdcVerification::_v4_fetch_nodes_aggregates_for_era(&cluster_id, era_id, &dac_nodes)
                .unwrap();

        let groups =
            DdcVerification::_v4_group_nodes_aggregates_by_consistency(&cluster_id, era_id, aggregates_by_aggregator, redundancy_factor, aggregators_quorum);
        // Node aggregates which are in consensus
        let node_aggregate_in_consensus = aggregator_client::json::NodeAggregate {
            node_id: "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e"
                .to_string(),
            stored_bytes: 675613289,
            transferred_bytes: 1097091579,
            number_of_puts: 889,
            number_of_gets: 97,
            aggregator: AggregatorInfo {
                node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
                node_params: node_params1.clone(),
            },
        };

        assert_eq!(
            groups.consensus,
            vec![ConsolidatedAggregate::new(node_aggregate_in_consensus.clone(), 3, vec![
                AggregatorInfo { node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])), node_params: node_params1.clone() },
                AggregatorInfo { node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])), node_params: node_params2.clone() },
                AggregatorInfo { node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([3; 32])), node_params: node_params3.clone() }])]
        );

        // Node aggregates which are in quorum
        let node_aggregate_in_quorum = aggregator_client::json::NodeAggregate {
            node_id: "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a"
                .to_string(),
            stored_bytes: 0,
            transferred_bytes: 38,
            number_of_puts: 0,
            number_of_gets: 1,
            aggregator: AggregatorInfo {
                node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])),
                node_params: node_params1.clone(),
            },
        };

        assert_eq!(
            groups.quorum, vec![ConsolidatedAggregate::new(node_aggregate_in_quorum.clone(), 2, vec![
                AggregatorInfo { node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([1; 32])), node_params: node_params1.clone() },
                AggregatorInfo { node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([4; 32])), node_params: node_params4.clone() }
            ])]
        );

        // Others nodes aggregates
        let node_aggregate1_in_others = aggregator_client::json::NodeAggregate {
            node_id: "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a"
                .to_string(),
            stored_bytes: 0,
            transferred_bytes: 48,
            number_of_puts: 0,
            number_of_gets: 1,
            aggregator: AggregatorInfo {
                node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
                node_params: node_params2.clone(),
            },
        };

        let node_aggregate2_in_others = aggregator_client::json::NodeAggregate {
            node_id: "0xfc28d5f5bb10212077a8654f62c4f8f0b5ab985fc322a51f5a3c75943b29194b"
                .to_string(),
            stored_bytes: 675613289,
            transferred_bytes: 1097091579,
            number_of_puts: 889,
            number_of_gets: 97,
            aggregator: AggregatorInfo {
                node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([5; 32])),
                node_params: node_params5.clone(),
            },
        };

        assert_eq!(
            groups.others, vec![
                ConsolidatedAggregate::new(node_aggregate2_in_others.clone(), 1, vec![AggregatorInfo {
                    node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([5; 32])),
                    node_params: node_params5.clone(),
                }]), ConsolidatedAggregate::new(node_aggregate1_in_others.clone(), 1, vec![AggregatorInfo {
                    node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([2; 32])),
                    node_params: node_params2.clone(),
                }])]
        );
    });
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
		vec![activities_batch_1.clone(), activities_batch_2.clone()],
		era_id_1,
	)
	.unwrap();
	let expected_roots = vec![
		DdcVerification::create_merkle_root(
			&cluster_id,
			&activities_batch_1.iter().map(|a| a.hash::<mock::Test>()).collect::<Vec<_>>(),
			era_id_1,
		)
		.unwrap(),
		DdcVerification::create_merkle_root(
			&cluster_id,
			&activities_batch_2.iter().map(|a| a.hash::<mock::Test>()).collect::<Vec<_>>(),
			era_id_1,
		)
		.unwrap(),
	];

	assert_eq!(result_roots, expected_roots);
}

#[test]
fn test_convert_to_batch_merkle_roots_empty() {
	let cluster_id = ClusterId::default();
	let batches: Vec<Vec<CustomerCharge>> = vec![];
	let result_roots =
		DdcVerification::convert_to_batch_merkle_roots(&cluster_id, batches, 1).unwrap();
	let expected_roots = vec![];

	assert_eq!(result_roots, expected_roots);
}

#[test]
fn test_split_to_batches_empty_activities() {
	let activities: Vec<aggregator_client::json::NodeAggregate> = vec![];
	let result = DdcVerification::split_to_batches(&activities, 3);
	assert_eq!(result, Vec::<Vec<aggregator_client::json::NodeAggregate>>::new());
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
	let mut expected: Vec<Vec<aggregator_client::json::NodeAggregate>> = Vec::new();
	expected.push(vec![sorted_activities[0].clone(), sorted_activities[1].clone()]);
	expected.push(vec![sorted_activities[2].clone(), sorted_activities[3].clone()]);
	expected.push(vec![sorted_activities[4].clone()]);

	assert_eq!(result, expected);
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

        // Mock HTTP request and response
        let pending_request = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host, port),
            response: Some(br#"[{"id":17,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":18,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":19,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0}]"#.to_vec()),
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

        let result = Pallet::<Test>::_v4_fetch_processed_eras(&node_params);
        assert!(result.is_ok());
        let activities = result.unwrap();

        let era_activity1 = EraActivity { id: 17, start: 1, end: 2 };
        let era_activity2 = EraActivity { id: 18, start: 1, end: 2 };
        let era_activity3 = EraActivity { id: 19, start: 1, end: 2 };

        assert_eq!(era_activity1, activities[0].clone().into());
        assert_eq!(era_activity2, activities[1].clone().into());
        assert_eq!(era_activity3, activities[2].clone().into());
    });
}

#[ignore = "DAC v5 is in progress"]
#[test]
fn get_era_for_validation_works() {
	let mut ext = TestExternalities::default();
	let (offchain, offchain_state) = TestOffchainExt::new();
	let (pool, _) = TestTransactionPoolExt::new();

	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(OffchainDbExt::new(Box::new(offchain.clone())));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();
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

        let pending_request1 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host1, port),
            response: Some(br#"[{"id":16,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":17,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":18,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":19,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        let pending_request2 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host2, port),
            response: Some(br#"[{"id":16,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":17,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":18,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        let pending_request3 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host3, port),
            response: Some(br#"[{"id":16,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":17,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":18,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        let pending_request4 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host4, port),
            response: Some(br#"[{"id":16,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":17,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":18,"status":"PROCESSED","start":1,"end":2,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        offchain_state.expect_request(pending_request1);
        offchain_state.expect_request(pending_request2);
        offchain_state.expect_request(pending_request3);
        offchain_state.expect_request(pending_request4);

        drop(offchain_state);

        let _node_params1 = StorageNodeParams {
            ssl: false,
            host: host1.as_bytes().to_vec(),
            http_port: port,
            mode: StorageNodeMode::DAC,
            p2p_port: 5555,
            grpc_port: 4444,
            domain: b"example2.com".to_vec(),
        };

        let _node_params2 = StorageNodeParams {
            ssl: false,
            host: host2.as_bytes().to_vec(),
            http_port: port,
            mode: StorageNodeMode::DAC,
            p2p_port: 5555,
            grpc_port: 4444,
            domain: b"example3.com".to_vec(),
        };

        let _node_params3 = StorageNodeParams {
            ssl: false,
            host: host3.as_bytes().to_vec(),
            http_port: port,
            mode: StorageNodeMode::DAC,
            p2p_port: 5555,
            grpc_port: 4444,
            domain: b"example4.com".to_vec(),
        };

        let _node_params4 = StorageNodeParams {
            ssl: false,
            host: host4.as_bytes().to_vec(),
            http_port: port,
            mode: StorageNodeMode::DAC,
            p2p_port: 5555,
            grpc_port: 4444,
            domain: b"example5.com".to_vec(),
        };
    });
}

#[test]
fn create_merkle_root_works() {
	new_test_ext().execute_with(|| {
		let a: DeltaUsageHash = H256([0; 32]);
		let b: DeltaUsageHash = H256([1; 32]);
		let c: DeltaUsageHash = H256([2; 32]);
		let d: DeltaUsageHash = H256([3; 32]);
		let e: DeltaUsageHash = H256([4; 32]);
		let cluster_id = ClusterId::default();
		let era_id_1 = 1;

		let leaves = vec![a, b, c, d, e];

		let root = DdcVerification::create_merkle_root(&cluster_id, &leaves, era_id_1).unwrap();

		assert_eq!(
			root,
			H256([
				205, 34, 92, 22, 66, 39, 53, 146, 126, 111, 191, 174, 107, 224, 161, 127, 150, 69,
				255, 15, 237, 252, 116, 39, 186, 26, 40, 154, 180, 110, 185, 7
			])
		);
	});
}

#[test]
fn create_merkle_root_empty() {
	new_test_ext().execute_with(|| {
		let cluster_id = ClusterId::default();
		let era_id_1 = 1;
		let leaves = Vec::<DeltaUsageHash>::new();
		let root = DdcVerification::create_merkle_root(&cluster_id, &leaves, era_id_1).unwrap();

		assert_eq!(root, DeltaUsageHash::default());
	});
}

#[test]
fn proof_merkle_leaf_works() {
	new_test_ext().execute_with(|| {
		let a: DeltaUsageHash = H256([0; 32]);
		let b: DeltaUsageHash = H256([1; 32]);
		let c: DeltaUsageHash = H256([2; 32]);
		let d: DeltaUsageHash = H256([3; 32]);
		let e: DeltaUsageHash = H256([4; 32]);
		let f: DeltaUsageHash = H256([5; 32]);

		let leaves = [a, b, c, d, e];
		let store = MemStore::default();
		let mut mmr: MMR<DeltaUsageHash, MergeMMRHash, &MemStore<DeltaUsageHash>> =
			MemMMR::<_, MergeMMRHash>::new(0, &store);
		let leaf_position_map: Vec<(DeltaUsageHash, u64)> =
			leaves.iter().map(|a| (*a, mmr.push(*a).unwrap())).collect();

		let leaf_position: Vec<(u64, DeltaUsageHash)> = leaf_position_map
			.iter()
			.filter(|&(l, _)| l == &c)
			.map(|&(ref l, p)| (p, *l))
			.collect();
		let position: Vec<u64> = leaf_position.clone().into_iter().map(|(p, _)| p).collect();
		let root_hash = mmr.get_root().unwrap();

		assert_eq!(leaf_position.len(), 1);
		assert_eq!(position.len(), 1);

		let max_leaf_index = 4;

		let leaf_index = 2;
		let leaf_hash = c;
		assert!(DdcVerification::_proof_merkle_leaf(
			root_hash,
			leaf_hash,
			leaf_index,
			max_leaf_index,
			&MMRProof { proof: mmr.gen_proof(position.clone()).unwrap().proof_items().to_vec() }
		)
		.unwrap());

		let leaf_index = 5;
		let leaf_hash = f;
		assert_noop!(
			DdcVerification::_proof_merkle_leaf(
				root_hash,
				leaf_hash,
				leaf_index,
				max_leaf_index,
				&MMRProof { proof: mmr.gen_proof(position).unwrap().proof_items().to_vec() }
			),
			Error::<Test>::FailedToVerifyMerkleProof
		);
	});
}

#[ignore = "DAC v5 is in progress"]
#[test]
fn test_single_ocw_pallet_integration() {
	let mut ext = new_test_ext();
	let (offchain, offchain_state) = TestOffchainExt::new();
	let (pool, _pool_state) = TestTransactionPoolExt::new();

	let keystore = MemoryKeystore::new();
	keystore
		.insert(
			DAC_VERIFICATION_KEY_TYPE,
			&format!("0x{}", VALIDATOR_VERIFICATION_PRIV_KEY_HEX),
			&hex::decode(VALIDATOR_VERIFICATION_PUB_KEY_HEX)
				.expect("Test verification pub key to be extracted"),
		)
		.unwrap();

	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(OffchainDbExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));
	ext.register_extension(KeystoreExt::new(keystore));

	ext.execute_with(|| {
		let mut offchain_state = offchain_state.write();
		let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();
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
            response: Some(br#"[{"id":5738616,"status":"PROCESSED","start":1721584800000,"end":1721585099999,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":5738617,"status":"PROCESSED","start":1721585100000,"end":1721585399999,"processing_time":1818,"nodes_total":9,"nodes_processed":9,"records_processed":16,"records_applied":16,"records_discarded":0,"attempt":0},{"id":5738618,"status":"PROCESSED","start":1721585400000,"end":1721585699999,"processing_time":1997,"nodes_total":9,"nodes_processed":9,"records_processed":622,"records_applied":622,"records_discarded":0,"attempt":0},{"id":5738619,"status":"PROCESSED","start":1721585700000,"end":1721585999999,"processing_time":2118,"nodes_total":9,"nodes_processed":9,"records_processed":834,"records_applied":834,"records_discarded":0,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        let pending_request2 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host2, port),
            response: Some(br#"[{"id":5738616,"status":"PROCESSED","start":1721584800000,"end":1721585099999,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":5738617,"status":"PROCESSED","start":1721585100000,"end":1721585399999,"processing_time":1818,"nodes_total":9,"nodes_processed":9,"records_processed":16,"records_applied":16,"records_discarded":0,"attempt":0},{"id":5738618,"status":"PROCESSED","start":1721585400000,"end":1721585699999,"processing_time":1997,"nodes_total":9,"nodes_processed":9,"records_processed":622,"records_applied":622,"records_discarded":0,"attempt":0},{"id":5738619,"status":"PROCESSED","start":1721585700000,"end":1721585999999,"processing_time":2118,"nodes_total":9,"nodes_processed":9,"records_processed":834,"records_applied":834,"records_discarded":0,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        let pending_request3 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host3, port),
            response: Some(br#"[{"id":5738616,"status":"PROCESSED","start":1721584800000,"end":1721585099999,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":5738617,"status":"PROCESSED","start":1721585100000,"end":1721585399999,"processing_time":1818,"nodes_total":9,"nodes_processed":9,"records_processed":16,"records_applied":16,"records_discarded":0,"attempt":0},{"id":5738618,"status":"PROCESSED","start":1721585400000,"end":1721585699999,"processing_time":1997,"nodes_total":9,"nodes_processed":9,"records_processed":622,"records_applied":622,"records_discarded":0,"attempt":0},{"id":5738619,"status":"PROCESSED","start":1721585700000,"end":1721585999999,"processing_time":2118,"nodes_total":9,"nodes_processed":9,"records_processed":834,"records_applied":834,"records_discarded":0,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        let pending_request4 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host4, port),
            response: Some(br#"[{"id":5738616,"status":"PROCESSED","start":1721584800000,"end":1721585099999,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":5738617,"status":"PROCESSED","start":1721585100000,"end":1721585399999,"processing_time":1818,"nodes_total":9,"nodes_processed":9,"records_processed":16,"records_applied":16,"records_discarded":0,"attempt":0},{"id":5738618,"status":"PROCESSED","start":1721585400000,"end":1721585699999,"processing_time":1997,"nodes_total":9,"nodes_processed":9,"records_processed":622,"records_applied":622,"records_discarded":0,"attempt":0},{"id":5738619,"status":"PROCESSED","start":1721585700000,"end":1721585999999,"processing_time":2118,"nodes_total":9,"nodes_processed":9,"records_processed":834,"records_applied":834,"records_discarded":0,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        let pending_request5 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host5, port),
            response: Some(br#"[{"id":5738616,"status":"PROCESSED","start":1721584800000,"end":1721585099999,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":5738617,"status":"PROCESSED","start":1721585100000,"end":1721585399999,"processing_time":1818,"nodes_total":9,"nodes_processed":9,"records_processed":16,"records_applied":16,"records_discarded":0,"attempt":0},{"id":5738618,"status":"PROCESSED","start":1721585400000,"end":1721585699999,"processing_time":1997,"nodes_total":9,"nodes_processed":9,"records_processed":622,"records_applied":622,"records_discarded":0,"attempt":0},{"id":5738619,"status":"PROCESSED","start":1721585700000,"end":1721585999999,"processing_time":2118,"nodes_total":9,"nodes_processed":9,"records_processed":834,"records_applied":834,"records_discarded":0,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        let pending_request6 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host6, port),
            response: Some(br#"[{"id":5738616,"status":"PROCESSED","start":1721584800000,"end":1721585099999,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":5738617,"status":"PROCESSED","start":1721585100000,"end":1721585399999,"processing_time":1818,"nodes_total":9,"nodes_processed":9,"records_processed":16,"records_applied":16,"records_discarded":0,"attempt":0},{"id":5738618,"status":"PROCESSED","start":1721585400000,"end":1721585699999,"processing_time":1997,"nodes_total":9,"nodes_processed":9,"records_processed":622,"records_applied":622,"records_discarded":0,"attempt":0},{"id":5738619,"status":"PROCESSED","start":1721585700000,"end":1721585999999,"processing_time":2118,"nodes_total":9,"nodes_processed":9,"records_processed":834,"records_applied":834,"records_discarded":0,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        let pending_request7 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host7, port),
            response: Some(br#"[{"id":5738616,"status":"PROCESSED","start":1721584800000,"end":1721585099999,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":5738617,"status":"PROCESSED","start":1721585100000,"end":1721585399999,"processing_time":1818,"nodes_total":9,"nodes_processed":9,"records_processed":16,"records_applied":16,"records_discarded":0,"attempt":0},{"id":5738618,"status":"PROCESSED","start":1721585400000,"end":1721585699999,"processing_time":1997,"nodes_total":9,"nodes_processed":9,"records_processed":622,"records_applied":622,"records_discarded":0,"attempt":0},{"id":5738619,"status":"PROCESSED","start":1721585700000,"end":1721585999999,"processing_time":2118,"nodes_total":9,"nodes_processed":9,"records_processed":834,"records_applied":834,"records_discarded":0,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        let pending_request8 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host8, port),
            response: Some(br#"[{"id":5738616,"status":"PROCESSED","start":1721584800000,"end":1721585099999,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":5738617,"status":"PROCESSED","start":1721585100000,"end":1721585399999,"processing_time":1818,"nodes_total":9,"nodes_processed":9,"records_processed":16,"records_applied":16,"records_discarded":0,"attempt":0},{"id":5738618,"status":"PROCESSED","start":1721585400000,"end":1721585699999,"processing_time":1997,"nodes_total":9,"nodes_processed":9,"records_processed":622,"records_applied":622,"records_discarded":0,"attempt":0},{"id":5738619,"status":"PROCESSED","start":1721585700000,"end":1721585999999,"processing_time":2118,"nodes_total":9,"nodes_processed":9,"records_processed":834,"records_applied":834,"records_discarded":0,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };
        let pending_request9 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/eras", host9, port),
            response: Some(br#"[{"id":5738616,"status":"PROCESSED","start":1721584800000,"end":1721585099999,"processing_time":15977,"nodes_total":9,"nodes_processed":9,"records_processed":0,"records_applied":0,"records_discarded":130755,"attempt":0},{"id":5738617,"status":"PROCESSED","start":1721585100000,"end":1721585399999,"processing_time":1818,"nodes_total":9,"nodes_processed":9,"records_processed":16,"records_applied":16,"records_discarded":0,"attempt":0},{"id":5738618,"status":"PROCESSED","start":1721585400000,"end":1721585699999,"processing_time":1997,"nodes_total":9,"nodes_processed":9,"records_processed":622,"records_applied":622,"records_discarded":0,"attempt":0},{"id":5738619,"status":"PROCESSED","start":1721585700000,"end":1721585999999,"processing_time":2118,"nodes_total":9,"nodes_processed":9,"records_processed":834,"records_applied":834,"records_discarded":0,"attempt":0}]"#.to_vec()),
            sent: true,
            ..Default::default()
        };


		let node_pending_request1 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=5738616&limit={}", host1, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=5738616&limit={}", host2, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=5738616&limit={}", host3, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=5738616&limit={}", host4, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request5 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=5738616&limit={}", host5, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request6 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=5738616&limit={}", host6, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request7 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=5738616&limit={}", host7, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request8 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=5738616&limit={}", host8, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let node_pending_request9 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/nodes?eraId=5738616&limit={}", host9, port, pallet::NODES_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"node_id": "0x48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e","stored_bytes": 675613289,"transferred_bytes": 1097091579,"number_of_puts": 889,"number_of_gets": 97},{"node_id": "0x9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a","stored_bytes": 0, "transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request1 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=5738616&limit={}", host1, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"bucket_id": 90235,"stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=5738616&limit={}", host2, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1,"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request3 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=5738616&limit={}", host3, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1,"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request4 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=5738616&limit={}", host4, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1,"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request5 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=5738616&limit={}", host5, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1,"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request6 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=5738616&limit={}", host6, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1,"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request7 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=5738616&limit={}", host7, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1,"bucket_id": 90235,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request8 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=5738616&limit={}", host8, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"bucket_id": 90235,"stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
			sent: true,
			..Default::default()
		};

		let bucket_pending_request9 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets?eraId=5738616&limit={}", host9, port, pallet::BUCKETS_AGGREGATES_FETCH_BATCH_SIZE),
			response: Some(br#"[{"bucket_id": 90235,"stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1,"sub_aggregates": [{"NodeID": "0xbe26b2458fb0c9df4ec26ec5ba083051402b2a3b9d4a7fe6106fe9f8b5efde2c","stored_bytes": 0,"transferred_bytes": 38,"number_of_puts": 0,"number_of_gets": 1}]}]"#.to_vec()),
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

        // Offchain worker should be triggered if block number is  divided by 100
        let block = 500;
        System::set_block_number(block);

        DdcVerification::offchain_worker(block);
    });
}

#[test]
fn fetch_reward_activities_works() {
	let cluster_id = ClusterId::from([12; 20]);
	let a: DeltaUsageHash = H256([0; 32]);
	let b: DeltaUsageHash = H256([1; 32]);
	let c: DeltaUsageHash = H256([2; 32]);
	let d: DeltaUsageHash = H256([3; 32]);
	let e: DeltaUsageHash = H256([4; 32]);
	let g_collector_key: NodePubKey = NodePubKey::StoragePubKey(AccountId32::new([
		0x9e, 0xf9, 0x8a, 0xd9, 0xc3, 0x62, 0x6b, 0xa7, 0x25, 0xe7, 0x8d, 0x76, 0xcf, 0xcf, 0xc4,
		0xb4, 0xd0, 0x7e, 0x84, 0xf0, 0x38, 0x84, 0x65, 0xbc, 0x7e, 0xb9, 0x92, 0xe3, 0xe1, 0x17,
		0x23, 0x4a,
	]));

	let leaves = [a, b, c, d, e];
	let ehd_id = EHDId(cluster_id, g_collector_key, 1);

	let result = DdcVerification::fetch_ehd_charging_loop_input(
		&cluster_id,
		ehd_id.clone(),
		leaves.to_vec(),
	);

	assert_eq!(result.unwrap(), Some((ehd_id, (leaves.len() - 1) as u16)));
}

#[test]
fn test_find_random_merkle_node_ids() {
	let mut ext = TestExternalities::default();
	let (offchain, _offchain_state) = TestOffchainExt::new();
	let (pool, _) = TestTransactionPoolExt::new();

	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(OffchainDbExt::new(Box::new(offchain)));
	ext.register_extension(TransactionPoolExt::new(pool));
	let host1 = "178.251.228.236";

	let port = 8080;
	let node_params1 = StorageNodeParams {
		ssl: false,
		host: host1.as_bytes().to_vec(),
		http_port: port,
		mode: StorageNodeMode::DAC,
		p2p_port: 5555,
		grpc_port: 4444,
		domain: b"example2.com".to_vec(),
	};

	ext.execute_with(|| {
		let deffective_bucket_sub_aggregate = aggregator_client::json::BucketSubAggregate {
			bucket_id: 90235,
			node_id: "0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa319"
				.to_string(),
			stored_bytes: 0,
			transferred_bytes: 505,
			number_of_puts: 12,
			number_of_gets: 13,
			aggregator: AggregatorInfo {
				node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([0; 32])),
				node_params: node_params1.clone(),
			},
		};

		let number_of_leaves = deffective_bucket_sub_aggregate.get_number_of_leaves();

		let ids = DdcVerification::_v4_find_random_merkle_node_ids(
			3,
			number_of_leaves,
			deffective_bucket_sub_aggregate.get_key(),
		);

		for id in ids {
			assert!(id < number_of_leaves);
		}
	});
}

#[test]
fn challenge_bucket_sub_aggregate_works() {
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
			DAC_VERIFICATION_KEY_TYPE,
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
		let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();
		offchain_state.persistent_storage.set(
			b"",
			&key,
			b"9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a".as_ref(),
		);
		offchain_state.timestamp = Timestamp::from_unix_millis(0);
		let host1 = "178.251.228.165";
        let port = 8080;

        //todo! put them in resource file
        let pending_request1 = PendingRequest {
            method: "GET".to_string(),
            uri: format!("http://{}:{}/activity/buckets/123229/challenge?eraId=5757773&nodeId=0x1f50f1455f60f5774564233d321a116ca45ae3188b2200999445706d04839d72&merkleTreeNodeId=0,2,1,3", host1, port),
            response: Some(br#"{"proofs":[{"merkle_tree_node_id":3,"usage":{"stored_bytes":2097152,"transferred_bytes":1048576,"number_of_puts":1,"number_of_gets":1},"path":["hFnZfjnS5bAzgm5tHcWTxuJa5waDcaiU7OhBRofylhQ="],"leafs":[{"record":{"id":"17Z3vSjjRm6mWN3Swpw3Cw==","upstream":{"request":{"requestId":"e9920157-6c6a-485e-9f5a-1685ea6d4ef5","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_PIECE","bucketId":"1","pieceCid":"AQIeIKLbs3OibO5qbLJ/PLCo1m02oFHWCl4s7S59GWgxDUbk","offset":"0","size":"0","timestamp":"1727346880632","signature":{"algorithm":"ED_25519","signer":"iNw0F9UFjsS0UD4MEuoaCom+IA/piSJCPUM0AU+msO4=","value":"KPDnQH5KZZQ2hksJ8F/w3GHwWloAm1QKoLt+SuUNYt3HxsGrh3r3q77COiu0jrwQ7mEsp/FFJp4pDp2Y1j2sDA=="}}},"downstream":[{"request":{"requestId":"a5bcaa37-97a4-45d2-beb9-c11cc955fb78","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_MERKLE_TREE","bucketId":"0","pieceCid":"AQIeIKLbs3OibO5qbLJ/PLCo1m02oFHWCl4s7S59GWgxDUbk","offset":"0","size":"0","timestamp":"1727346880633","signature":{"algorithm":"ED_25519","signer":"CsfLnFNZTp9TjZlQxrzyjwwMe4OF3uouviQGK8ZA574=","value":"ulpjaksvopDDRRfYnrccUg5spkoRpfZlDARbjgfL4Y/X4HZNUp2cL5qQMHUosREB6PSMXr9rQvXYGA9kmrUBDg=="}}},{"request":{"requestId":"8af9ba14-4c49-438c-957d-d1a108a58b85","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","bucketId":"0","pieceCid":"AQIeIKLbs3OibO5qbLJ/PLCo1m02oFHWCl4s7S59GWgxDUbk","offset":"0","size":"524288","timestamp":"1727346880633","signature":{"algorithm":"ED_25519","signer":"CsfLnFNZTp9TjZlQxrzyjwwMe4OF3uouviQGK8ZA574=","value":"CLdw3HaQWVWdDHeog2SZjiEA4NZN6PD8vyw58JuQI7gMDpDXLFslMOcI7p/uNEyeDfNoKTAgNZpWbNR4vSZ/AA=="}}},{"request":{"requestId":"b3dc8833-d5aa-4e33-9afa-54584da29cda","requestType":"REQUEST_TYPE_GET","contentType":"CONTENT_TYPE_SEGMENT","bucketId":"0","pieceCid":"AQIeIKLbs3OibO5qbLJ/PLCo1m02oFHWCl4s7S59GWgxDUbk","offset":"0","size":"524288","timestamp":"1727346880633","signature":{"algorithm":"ED_25519","signer":"CsfLnFNZTp9TjZlQxrzyjwwMe4OF3uouviQGK8ZA574=","value":"5XTnDU/85DqWWpMy1kGRVK6ZHe/EYDeg2p07UbFnIr6xLX7n50k9MslwuF8jMl2/QoBrPnndHdCd5ssqV90kDg=="}}}],"timestamp":"1727346880633","signature":{"algorithm":"ED_25519","signer":"CsfLnFNZTp9TjZlQxrzyjwwMe4OF3uouviQGK8ZA574=","value":"8WWGHaL3n8+bkuYQhTua3l+i3W//XXhlnzCpQ7VJ/BmfXQPFGEjIZsXw0kKr4+VXh/kWAncF3VrvW9nEi6G2CQ=="}},"transferred_bytes":1048576,"stored_bytes":0},{"record":{"id":"8Rg6VlRrSE65NsCY02OnlA==","upstream":{"request":{"requestId":"aacf30c4-b2e9-4f37-826d-0016c280f39b","requestType":"REQUEST_TYPE_PUT","contentType":"CONTENT_TYPE_METADATA","bucketId":"0","pieceCid":"AAAAAAAAAAEBAh4gaLfPG3AA1QwNFQc3VvJYsMAINAN6mMkvo5vk5HP8g/0=","offset":"0","size":"385","timestamp":"1727346880673","signature":{"algorithm":"ED_25519","signer":"xHUfclv0KTLyCz1NjsLAdMrEBfKdlta130WiEBvB14s=","value":"yPZt7Fyfp1aiJL+hYOg5rRtPPTNDMZwgReX2RX4bWbP8+ivreh1cNvSwnM5ln0EFqxTn53iVQpZeMWXUSiJeCw=="}}},"downstream":[],"timestamp":"1727346880673","signature":{"algorithm":"ED_25519","signer":"CsfLnFNZTp9TjZlQxrzyjwwMe4OF3uouviQGK8ZA574=","value":"zX0aGW/FuhddMAtGvN4Gjf6P1JaFGasrwf5yCrQPFv4qUB1GyACynb1s1+Mv0zpMAGOtIOcwaemoPu4fnOByBA=="}},"transferred_bytes":1048576,"stored_bytes":1048576}]}]}"#.to_vec()),
            sent: true,
            ..Default::default()
        };

		let pending_request2 = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/buckets/123229/traverse?eraId=5757773&nodeId=0x1f50f1455f60f5774564233d321a116ca45ae3188b2200999445706d04839d72&merkleTreeNodeId=1&levels=1", host1, port),
			response: Some(br#"{"merkle_tree_node_id":2,"hash":"hkujtYgWP21CrXdRP1rhRPrYR2ooIYCnP5zwCERTePI=","stored_bytes":20913291,"transferred_bytes":20913291,"number_of_puts":61,"number_of_gets":3}"#.to_vec()),
			sent: true,
			..Default::default()
		};

        offchain_state.expect_request(pending_request1);
        offchain_state.expect_request(pending_request2);

        drop(offchain_state);

        let cluster_id = ClusterId::from([1; 20]);
        let era_id = 5757773;
        let host1 = "178.251.228.165";


        let port = 8080;
        let node_params1 = StorageNodeParams {
            ssl: false,
            host: host1.as_bytes().to_vec(),
            http_port: port,
            mode: StorageNodeMode::DAC,
            p2p_port: 5555,
            grpc_port: 4444,
            domain: b"example2.com".to_vec(),
        };

        let deffective_bucket_sub_aggregate = aggregator_client::json::BucketSubAggregate {
            bucket_id: 123229,
            node_id: "0x1f50f1455f60f5774564233d321a116ca45ae3188b2200999445706d04839d72"
                .to_string(),
            stored_bytes: 0,
            transferred_bytes: 25143977,
            number_of_puts: 0,
            number_of_gets: 10,
            aggregator: AggregatorInfo {
                node_pub_key: NodePubKey::StoragePubKey(AccountId32::new([0; 32])),
                node_params: node_params1.clone(),
            },
        };

        let result =
            DdcVerification::_v4_challenge_aggregate(&cluster_id, era_id, &deffective_bucket_sub_aggregate);

        assert!(result.is_ok());

    });
}

use crate::aggregator_client::{
	json::{BucketAggregateResponse, SignedJsonResponse},
	AggregatorClient,
};

#[test]
fn aggregator_client_get_buckets_aggregates_works() {
	let mut ext = TestExternalities::default();
	let (offchain, offchain_state) = TestOffchainExt::new();

	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(OffchainDbExt::new(Box::new(offchain)));

	ext.execute_with(|| {
		let mut offchain_state = offchain_state.write();
		offchain_state.timestamp = Timestamp::from_unix_millis(0);

		let base_url = "http://example.com:8080";
		let era_id = 346524624;
		let activity_buckets_signed_resp =
			include_bytes!("./test_data/activity_buckets_signed_resp.json").as_slice();

		let expected_request = PendingRequest {
			method: "GET".to_string(),
			uri: format!("{}/activity/buckets?eraId={}&sign=true", base_url, era_id),
			response: Some(activity_buckets_signed_resp.to_vec()),
			sent: true,
			..Default::default()
		};

		offchain_state.expect_request(expected_request);
		drop(offchain_state);

		let client = AggregatorClient::new(base_url, Duration::from_millis(1_000), 1, true);

		let expected_response: SignedJsonResponse<Vec<BucketAggregateResponse>> =
			serde_json::from_slice(activity_buckets_signed_resp)
				.expect("json parsing failed, broken test data?");
		let result = client.buckets_aggregates(era_id, None, None);

		assert_eq!(result, Ok(expected_response.payload));
	})
}

#[test]
fn aggregator_client_challenge_bucket_sub_aggregate_works() {
	let mut ext = TestExternalities::default();
	let (offchain, offchain_state) = TestOffchainExt::new();

	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(OffchainDbExt::new(Box::new(offchain)));

	ext.execute_with(|| {
		let mut offchain_state = offchain_state.write();
		offchain_state.timestamp = Timestamp::from_unix_millis(0);

		let base_url = "http://example.com";
		let bucket_id = 1;
		let era_id = 1;
		let merkle_tree_node_id = "2,6";
		let node_id = "0x0ac7cb9c53594e9f538d9950c6bcf28f0c0c7b8385deea2ebe24062bc640e7be";

		let expected_response = proto::ChallengeResponse {
			proofs: vec![
				proto::challenge_response::Proof {
					merkle_tree_node_id: 2,
					usage: Some(proto::Aggregate { stored: 4, delivered: 3, puts: 2, gets: 1 }),
					..Default::default()
				},
				proto::challenge_response::Proof {
					merkle_tree_node_id: 6,
					usage: Some(proto::Aggregate { stored: 8, delivered: 7, puts: 6, gets: 5 }),
					..Default::default()
				},
			],
		};
		let mut expected_response_serialized = Vec::new();
		expected_response.encode(&mut expected_response_serialized).unwrap();

		let expected = PendingRequest {
			method: "GET".into(),
			headers: vec![("Accept".into(), "application/protobuf".into())],
			uri: format!(
				"{}/activity/buckets/{}/challenge?eraId={}&nodeId={}&merkleTreeNodeId={}",
				base_url, bucket_id, era_id, node_id, merkle_tree_node_id
			),
			response: Some(expected_response_serialized),
			sent: true,
			..Default::default()
		};
		offchain_state.expect_request(expected);
		drop(offchain_state);

		let client = AggregatorClient::new(base_url, Duration::from_millis(1_000), 1, false);

		let result = client.challenge_bucket_sub_aggregate(era_id, bucket_id, node_id, vec![2, 6]);
		assert_eq!(result, Ok(expected_response));
	})
}

#[test]
fn aggregator_client_challenge_node_aggregate_works() {
	let mut ext = TestExternalities::default();
	let (offchain, offchain_state) = TestOffchainExt::new();

	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(OffchainDbExt::new(Box::new(offchain)));

	ext.execute_with(|| {
		let mut offchain_state = offchain_state.write();
		offchain_state.timestamp = Timestamp::from_unix_millis(0);

		let base_url = "http://example.com";
		let era_id = 1;
		let merkle_tree_node_id = "2,6";
		let node_id = "0x0ac7cb9c53594e9f538d9950c6bcf28f0c0c7b8385deea2ebe24062bc640e7be";

		let expected_response = proto::ChallengeResponse {
			proofs: vec![
				proto::challenge_response::Proof {
					merkle_tree_node_id: 2,
					usage: Some(proto::Aggregate { stored: 4, delivered: 3, puts: 2, gets: 1 }),
					..Default::default()
				},
				proto::challenge_response::Proof {
					merkle_tree_node_id: 6,
					usage: Some(proto::Aggregate { stored: 8, delivered: 7, puts: 6, gets: 5 }),
					..Default::default()
				},
			],
		};
		let mut expected_response_serialized = Vec::new();
		expected_response.encode(&mut expected_response_serialized).unwrap();

		let expected = PendingRequest {
			method: "GET".into(),
			headers: vec![("Accept".into(), "application/protobuf".into())],
			uri: format!(
				"{}/activity/nodes/{}/challenge?eraId={}&merkleTreeNodeId={}",
				base_url, node_id, era_id, merkle_tree_node_id
			),
			response: Some(expected_response_serialized),
			sent: true,
			..Default::default()
		};
		offchain_state.expect_request(expected);
		drop(offchain_state);

		let client = AggregatorClient::new(base_url, Duration::from_millis(1_000), 1, false);

		let result = client.challenge_node_aggregate(era_id, node_id, vec![2, 6]);
		assert_eq!(result, Ok(expected_response));
	})
}
