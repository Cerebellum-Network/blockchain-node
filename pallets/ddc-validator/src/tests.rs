use crate::{
	mock::{Timestamp, *},
	shm, utils, DacTotalAggregates, EraIndex, ValidationDecision, DEFAULT_DATA_PROVIDER_URL,
	ERA_DURATION_MS, ERA_IN_BLOCKS, KEY_TYPE, TIME_START_MS,
};
use codec::Decode;
use frame_support::{
	assert_ok,
	traits::{OffchainWorker, OnInitialize},
};
use pallet_ddc_accounts::BucketsDetails;
use sp_core::offchain::{testing, OffchainDbExt, OffchainWorkerExt, TransactionPoolExt};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use std::sync::Arc;

const OCW_PUB_KEY_STR: &str = "d2bf4b844dfefd6772a8843e669f943408966a977e3ae2af1dd78e0f55f4df67";
const OCW_SEED: &str =
	"news slush supreme milk chapter athlete soap sausage put clutch what kitten";

#[test]
fn it_sets_validation_decision_with_one_validator_in_quorum() {
	let mut t = new_test_ext();

	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainDbExt::new(offchain.clone()));
	t.register_extension(OffchainWorkerExt::new(offchain));

	let keystore = KeyStore::new();
	keystore.sr25519_generate_new(KEY_TYPE, Some(OCW_SEED)).unwrap();
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	t.register_extension(TransactionPoolExt::new(pool));

	let era_to_validate: EraIndex = 3;
	let cdn_node_to_validate = AccountId::from([0x1; 32]);
	let cdn_node_to_validate_str = utils::account_to_string::<Test>(cdn_node_to_validate.clone());

	{
		let mut state = offchain_state.write();

		let mut expect_request = |url: &str, response: &[u8]| {
			state.expect_request(testing::PendingRequest {
				method: "GET".into(),
				uri: url.to_string(),
				response: Some(response.to_vec()),
				sent: true,
				..Default::default()
			});
		};

		expect_request(
			&format!(
				"{}/JSON.GET/ddc:dac:aggregation:nodes:{}/$.{}",
				DEFAULT_DATA_PROVIDER_URL, era_to_validate, cdn_node_to_validate_str
			),
			include_bytes!("./mock-data/set-1/aggregated-node-data-for-era.json"),
		);

		expect_request(
			&format!(
				"{}/JSON.GET/ddc:dac:data:file:84640a53-fc1f-4ac5-921c-6695056840bc",
				DEFAULT_DATA_PROVIDER_URL
			),
			include_bytes!("./mock-data/set-1/file-request1.json"),
		);

		expect_request(
			&format!(
				"{}/JSON.GET/ddc:dac:data:file:d0a55c8b-fcb9-41b5-aa9a-8b40e9c4edf7",
				DEFAULT_DATA_PROVIDER_URL
			),
			include_bytes!("./mock-data/set-1/file-request2.json"),
		);

		expect_request(
			&format!(
				"{}/JSON.GET/ddc:dac:data:file:80a62530-fd76-40b5-bc53-dd82365e89ce",
				DEFAULT_DATA_PROVIDER_URL
			),
			include_bytes!("./mock-data/set-1/file-request3.json"),
		);

		let decision: ValidationDecision =
			serde_json::from_slice(include_bytes!("./mock-data/set-1/validation-decision.json"))
				.unwrap();
		let serialized_decision = serde_json::to_string(&decision).unwrap();
		let encoded_decision_vec =
			shm::base64_encode(&serialized_decision.as_bytes().to_vec()).unwrap();
		let encoded_decision_str = encoded_decision_vec.iter().cloned().collect::<String>();
		let result_json = serde_json::json!({
			"result": decision.result,
			"data": encoded_decision_str,
		});
		let result_json_str = serde_json::to_string(&result_json).unwrap();
		let unescaped_result_json = utils::unescape(&result_json_str);
		let url_encoded_result_json = utils::url_encode(&unescaped_result_json);

		expect_request(
			&format!(
				"{}/FCALL/save_validation_result_by_node/1/{}:{}:{}/{}",
				DEFAULT_DATA_PROVIDER_URL,
				OCW_PUB_KEY_STR,
				cdn_node_to_validate_str,
				era_to_validate,
				url_encoded_result_json,
			),
			include_bytes!("./mock-data/set-1/save-validation-decision-result.json"),
		);

		expect_request(
			&format!(
				"{}/JSON.GET/ddc:dac:shared:nodes:{}",
				DEFAULT_DATA_PROVIDER_URL, era_to_validate
			),
			include_bytes!("./mock-data/set-1/shared-validation-decisions-for-era.json"),
		);
	}

	t.execute_with(|| {
		let era_block_number = ERA_IN_BLOCKS as u32 * era_to_validate;
		System::set_block_number(era_block_number); // required for randomness

		Timestamp::set_timestamp(
			(TIME_START_MS + ERA_DURATION_MS * (era_to_validate as u128 - 1)) as u64,
		);
		DdcValidator::on_initialize(era_block_number - 1); // make assignments

		Timestamp::set_timestamp(
			(TIME_START_MS + ERA_DURATION_MS * (era_to_validate as u128 + 1)) as u64,
		);
		DdcValidator::offchain_worker(era_block_number + 1); // execute assignments

		let mut transactions = pool_state.read().transactions.clone();
		transactions.reverse();
		assert_eq!(transactions.len(), 3);

		let tx = transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert!(tx.signature.is_some());

		let bucket_info1 = BucketsDetails { bucket_id: 5, amount: 100u128 };
		let bucket_info2 = BucketsDetails { bucket_id: 5, amount: 200u128 };
		let bucket_info3 = BucketsDetails { bucket_id: 5, amount: 300u128 };

		assert_eq!(
			tx.call,
			crate::mock::RuntimeCall::DdcValidator(crate::Call::charge_payments_content_owners {
				paying_accounts: vec![bucket_info3, bucket_info1, bucket_info2,]
			})
		);

		let tx = transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert!(tx.signature.is_some());
		assert_eq!(
			tx.call,
			crate::mock::RuntimeCall::DdcValidator(crate::Call::payout_cdn_owners {
				era: era_to_validate + 1
			})
		);

		let tx = transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert!(tx.signature.is_some());

		let common_decision: ValidationDecision =
			serde_json::from_slice(include_bytes!("./mock-data/set-1/validation-decision.json"))
				.unwrap();
		let common_decisions = vec![common_decision.clone()];
		let serialized_decisions = serde_json::to_string(&common_decisions).unwrap();

		assert_eq!(
			tx.call,
			crate::mock::RuntimeCall::DdcValidator(crate::Call::set_validation_decision {
				era: era_to_validate + 1,
				cdn_node: cdn_node_to_validate,
				validation_decision: ValidationDecision {
					edge: cdn_node_to_validate_str,
					result: true,
					payload: utils::hash(&serialized_decisions),
					totals: DacTotalAggregates {
						received: common_decision.totals.received,
						sent: common_decision.totals.sent,
						failed_by_client: common_decision.totals.failed_by_client,
						failure_rate: common_decision.totals.failure_rate,
					}
				}
			})
		);
	})
}
