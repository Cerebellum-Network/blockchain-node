#![cfg(test)]

use super::*;
use crate::{
	mock::{Timestamp, *},
	Error as ValidatorError,
};
use codec::Decode;
use frame_support::{assert_noop, assert_ok};
use pallet_ddc_accounts::{BucketsDetails, Error as AccountsError};
use pallet_ddc_staking::{DDC_ERA_DURATION_MS, DDC_ERA_START_MS};
use sp_core::offchain::{testing, OffchainDbExt, OffchainWorkerExt, TransactionPoolExt};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::offchain::storage::StorageValueRef;
use std::sync::Arc;

const OCW_PUB_KEY_STR: &str = "d2bf4b844dfefd6772a8843e669f943408966a977e3ae2af1dd78e0f55f4df67";
const OCW_SEED: &str =
	"news slush supreme milk chapter athlete soap sausage put clutch what kitten";

fn last_event() -> RuntimeEvent {
	System::events().pop().expect("Event expected").event.into()
}

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
	let validator_1_stash = AccountId::from([
		0xd2, 0xbf, 0x4b, 0x84, 0x4d, 0xfe, 0xfd, 0x67, 0x72, 0xa8, 0x84, 0x3e, 0x66, 0x9f, 0x94,
		0x34, 0x08, 0x96, 0x6a, 0x97, 0x7e, 0x3a, 0xe2, 0xaf, 0x1d, 0xd7, 0x8e, 0x0f, 0x55, 0xf4,
		0xdf, 0x67,
	]);
	let validator_1_controller = AccountId::from([0xaa; 32]);
	let validator_2_stash = AccountId::from([0xb; 32]);
	let validator_2_controller = AccountId::from([0xbb; 32]);
	let validator_3_stash = AccountId::from([0xc; 32]);
	let validator_3_controller = AccountId::from([0xcc; 32]);

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
		let era_block_number = 20 as u32 * era_to_validate;
		System::set_block_number(era_block_number); // required for randomness
		assert_ok!(DdcValidator::set_validator_key(
			// register validator 1
			RuntimeOrigin::signed(validator_1_controller),
			validator_1_stash,
		));
		assert_ok!(DdcValidator::set_validator_key(
			// register validator 2
			RuntimeOrigin::signed(validator_2_controller),
			validator_2_stash,
		));
		assert_ok!(DdcValidator::set_validator_key(
			// register validator 3
			RuntimeOrigin::signed(validator_3_controller),
			validator_3_stash,
		));
		Timestamp::set_timestamp(
			(DDC_ERA_START_MS + DDC_ERA_DURATION_MS * (era_to_validate as u128 - 1)) as u64,
		);
		DdcStaking::on_finalize(era_block_number - 1); // set DDC era counter
		DdcValidator::on_initialize(era_block_number - 1); // make assignments

		Timestamp::set_timestamp(
			(DDC_ERA_START_MS + DDC_ERA_DURATION_MS * (era_to_validate as u128 + 1)) as u64,
		);
		DdcStaking::on_finalize(era_block_number + 1); // inc DDC era counter
		StorageValueRef::persistent(ENABLE_DDC_VALIDATION_KEY).set(&true); // enable validation
		DdcValidator::offchain_worker(era_block_number + 1); // execute assignments

		let mut transactions = pool_state.read().transactions.clone();
		transactions.reverse();
		assert_eq!(transactions.len(), 3);

		let tx = transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert!(tx.signature.is_some());

		let bucket_info = BucketsDetails { bucket_id: 5, amount: 600u128 };

		assert_eq!(
			tx.call,
			crate::mock::RuntimeCall::DdcValidator(crate::Call::charge_payments_content_owners {
				paying_accounts: vec![bucket_info]
			})
		);

		let tx = transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert!(tx.signature.is_some());

		let common_decision: ValidationDecision = serde_json::from_slice(include_bytes!(
			"./mock-data/set-1/final-validation-decision.json"
		))
		.unwrap();
		let common_decisions = vec![common_decision.clone()];
		let serialized_decisions = serde_json::to_string(&common_decisions).unwrap();

		assert_eq!(
			tx.call,
			crate::mock::RuntimeCall::DdcValidator(crate::Call::set_validation_decision {
				era: era_to_validate,
				cdn_node: cdn_node_to_validate.clone(),
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

		let tx = transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();

		let stakers_points = vec![(cdn_node_to_validate, common_decision.totals.sent)];

		assert!(tx.signature.is_some());
		assert_eq!(
			tx.call,
			crate::mock::RuntimeCall::DdcValidator(crate::Call::set_era_reward_points {
				era: era_to_validate,
				stakers_points,
			})
		);
	})
}

#[test]
fn send_signal_works_as_expected() {
	let mut t = new_test_ext();

	let validator_controller = AccountId::from([0xaa; 32]);

	t.execute_with(|| {
		assert_eq!(DdcValidator::signal(), None);
		assert_ok!(DdcValidator::send_signal(RuntimeOrigin::signed(validator_controller)));
		assert_eq!(DdcValidator::signal().unwrap(), true);
	})
}

#[test]
fn set_validation_decision_works_as_expected() {
	let mut t = new_test_ext();

	let era_to_validate: EraIndex = 3;
	let cdn_node_to_validate = AccountId::from([0x1; 32]);
	let not_a_cdn_node = AccountId::from([0x2; 32]);
	let validator_1_stash = AccountId::from([
		0xd2, 0xbf, 0x4b, 0x84, 0x4d, 0xfe, 0xfd, 0x67, 0x72, 0xa8, 0x84, 0x3e, 0x66, 0x9f, 0x94,
		0x34, 0x08, 0x96, 0x6a, 0x97, 0x7e, 0x3a, 0xe2, 0xaf, 0x1d, 0xd7, 0x8e, 0x0f, 0x55, 0xf4,
		0xdf, 0x67,
	]);
	let validator_1_controller = AccountId::from([0xaa; 32]);
	let validator_1_not_controller = AccountId::from([0xdd; 32]);

	let decision: ValidationDecision =
		serde_json::from_slice(include_bytes!("./mock-data/set-1/validation-decision.json"))
			.unwrap();

	t.execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			DdcValidator::set_validation_decision(
				RuntimeOrigin::signed(validator_1_controller.clone()),
				era_to_validate,
				cdn_node_to_validate.clone(),
				decision.clone(),
			),
			ValidatorError::<Test>::DDCValidatorKeyNotRegistered
		);

		// Set a mapping from stash to not a validator
		DDCValidatorToStashKeys::<Test>::insert(
			validator_1_stash.clone(),
			validator_1_not_controller.clone(),
		);

		assert_noop!(
			DdcValidator::set_validation_decision(
				RuntimeOrigin::signed(validator_1_stash.clone()),
				era_to_validate,
				cdn_node_to_validate.clone(),
				decision.clone()
			),
			ValidatorError::<Test>::NotValidatorStash
		);

		assert_ok!(DdcValidator::set_validator_key(
			// register validator 1
			RuntimeOrigin::signed(validator_1_controller.clone()),
			validator_1_stash.clone(),
		));

		assert_noop!(
			DdcValidator::set_validation_decision(
				RuntimeOrigin::signed(validator_1_stash.clone()),
				era_to_validate,
				not_a_cdn_node.clone(),
				decision.clone()
			),
			ValidatorError::<Test>::NodeNotActive
		);

		assert_ok!(DdcValidator::set_validation_decision(
			RuntimeOrigin::signed(validator_1_stash.clone()),
			era_to_validate,
			cdn_node_to_validate.clone(),
			decision.clone()
		));

		assert_noop!(
			DdcValidator::set_validation_decision(
				RuntimeOrigin::signed(validator_1_stash),
				era_to_validate,
				cdn_node_to_validate.clone(),
				decision.clone()
			),
			ValidatorError::<Test>::ValidationDecisionAlreadySet
		);

		let evt = last_event();

		assert_eq!(
			evt,
			crate::Event::ValidationDecision(era_to_validate, cdn_node_to_validate, decision)
				.into()
		);
	})
}

#[test]
fn set_era_reward_points_works_as_expected() {
	let mut t = new_test_ext();

	let era_to_validate: EraIndex = 3;
	let cdn_node_to_validate = AccountId::from([0x1; 32]);
	let not_a_cdn_node = AccountId::from([0x2; 32]);
	let validator_1_stash = AccountId::from([
		0xd2, 0xbf, 0x4b, 0x84, 0x4d, 0xfe, 0xfd, 0x67, 0x72, 0xa8, 0x84, 0x3e, 0x66, 0x9f, 0x94,
		0x34, 0x08, 0x96, 0x6a, 0x97, 0x7e, 0x3a, 0xe2, 0xaf, 0x1d, 0xd7, 0x8e, 0x0f, 0x55, 0xf4,
		0xdf, 0x67,
	]);
	let validator_1_controller = AccountId::from([0xaa; 32]);
	let validator_1_not_controller = AccountId::from([0xdd; 32]);

	let decision: ValidationDecision =
		serde_json::from_slice(include_bytes!("./mock-data/set-1/validation-decision.json"))
			.unwrap();

	let stakers_points = vec![(cdn_node_to_validate.clone(), decision.totals.sent)];
	let fake_stakers_points = vec![(not_a_cdn_node.clone(), decision.totals.sent)];

	t.execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			DdcValidator::set_era_reward_points(
				RuntimeOrigin::signed(validator_1_controller.clone()),
				era_to_validate,
				stakers_points.clone(),
			),
			ValidatorError::<Test>::DDCValidatorKeyNotRegistered
		);

		// Set a mapping from stash to not a validator
		DDCValidatorToStashKeys::<Test>::insert(
			validator_1_stash.clone(),
			validator_1_not_controller.clone(),
		);

		assert_noop!(
			DdcValidator::set_era_reward_points(
				RuntimeOrigin::signed(validator_1_stash.clone()),
				era_to_validate,
				stakers_points.clone()
			),
			ValidatorError::<Test>::NotValidatorStash
		);

		assert_ok!(DdcValidator::set_validator_key(
			// register validator 1
			RuntimeOrigin::signed(validator_1_controller),
			validator_1_stash.clone(),
		));

		// attempting to set era reward points for account, which is not an active CDN node
		assert_ok!(DdcValidator::set_era_reward_points(
			RuntimeOrigin::signed(validator_1_stash.clone()),
			era_to_validate,
			fake_stakers_points.clone()
		));

		// rewards points should not be set
		assert_eq!(
			DdcValidator::reward_points_set_for_node(era_to_validate, not_a_cdn_node),
			false
		);

		assert_ok!(DdcValidator::set_era_reward_points(
			RuntimeOrigin::signed(validator_1_stash.clone()),
			era_to_validate,
			stakers_points.clone()
		));

		let reward_points = ddc_staking::pallet::ErasEdgesRewardPointsPerNode::<Test>::get(
			cdn_node_to_validate.clone(),
		);

		assert_eq!(reward_points.len(), 1);

		// Second run will still pass, but state will not change, as for specified node era points
		// were set already
		assert_ok!(DdcValidator::set_era_reward_points(
			RuntimeOrigin::signed(validator_1_stash.clone()),
			era_to_validate,
			stakers_points.clone()
		));

		let reward_points_update_attempt =
			ddc_staking::pallet::ErasEdgesRewardPointsPerNode::<Test>::get(
				cdn_node_to_validate.clone(),
			);

		assert_eq!(reward_points_update_attempt.len(), 1);

		assert_eq!(
			DdcValidator::reward_points_set_for_node(era_to_validate, cdn_node_to_validate),
			true
		);

		assert_eq!(System::events().len(), 1);
		let evt = System::events().pop().expect("Event expected").event;

		assert_eq!(evt, crate::Event::EraRewardPoints(era_to_validate, stakers_points).into());
	})
}

#[test]
fn charge_payments_content_owners_works_as_expected() {
	let mut t = new_test_ext();

	let era_to_validate: EraIndex = 3;
	let validator_1_stash = AccountId::from([
		0xd2, 0xbf, 0x4b, 0x84, 0x4d, 0xfe, 0xfd, 0x67, 0x72, 0xa8, 0x84, 0x3e, 0x66, 0x9f, 0x94,
		0x34, 0x08, 0x96, 0x6a, 0x97, 0x7e, 0x3a, 0xe2, 0xaf, 0x1d, 0xd7, 0x8e, 0x0f, 0x55, 0xf4,
		0xdf, 0x67,
	]);
	let validator_1_controller = AccountId::from([0xaa; 32]);
	let validator_1_not_controller = AccountId::from([0xdd; 32]);

	let bucket_info = BucketsDetails { bucket_id: 5, amount: 600u128 };

	t.execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			DdcValidator::charge_payments_content_owners(
				RuntimeOrigin::signed(validator_1_controller.clone()),
				vec![bucket_info.clone()],
			),
			ValidatorError::<Test>::DDCEraNotSet
		);

		let era_block_number = 20 as u32 * era_to_validate;
		System::set_block_number(era_block_number);
		Timestamp::set_timestamp(
			(DDC_ERA_START_MS + DDC_ERA_DURATION_MS * (era_to_validate as u128 - 1)) as u64,
		);
		DdcStaking::on_finalize(era_block_number - 1);

		assert_noop!(
			DdcValidator::charge_payments_content_owners(
				RuntimeOrigin::signed(validator_1_stash.clone()),
				vec![bucket_info.clone()]
			),
			ValidatorError::<Test>::DDCValidatorKeyNotRegistered
		);

		// Set a mapping from stash to not a validator
		DDCValidatorToStashKeys::<Test>::insert(
			validator_1_stash.clone(),
			validator_1_not_controller.clone(),
		);

		assert_noop!(
			DdcValidator::charge_payments_content_owners(
				RuntimeOrigin::signed(validator_1_stash.clone()),
				vec![bucket_info.clone()]
			),
			ValidatorError::<Test>::NotValidatorStash
		);

		assert_ok!(DdcValidator::set_validator_key(
			// register validator 1
			RuntimeOrigin::signed(validator_1_controller.clone()),
			validator_1_stash.clone(),
		));

		assert_noop!(
			DdcValidator::charge_payments_content_owners(
				RuntimeOrigin::signed(validator_1_stash.clone()),
				vec![bucket_info.clone()]
			),
			ValidatorError::<Test>::PricingNotSet
		);

		ddc_staking::Pricing::<Test>::set(Some(1)); // set CERE token per byte / reward

		// No buckets were created at this point
		assert_noop!(
			DdcValidator::charge_payments_content_owners(
				RuntimeOrigin::signed(validator_1_stash.clone()),
				vec![bucket_info.clone()]
			),
			AccountsError::<Test>::BucketDoesNotExist
		);

		// Create buckets
		for range in 1..6 {
			assert_ok!(ddc_accounts::Pallet::<Test>::create_bucket(
				RuntimeOrigin::signed(validator_1_stash.clone()),
				true,
				range
			));
		}

		// Account to charge payments from is not created
		assert_noop!(
			DdcValidator::charge_payments_content_owners(
				RuntimeOrigin::signed(validator_1_stash.clone()),
				vec![bucket_info.clone()]
			),
			AccountsError::<Test>::NotController
		);

		// Deposit funds for account
		assert_ok!(ddc_accounts::Pallet::<Test>::deposit(
			RuntimeOrigin::signed(validator_1_stash.clone()),
			validator_1_stash.clone(),
			1_000,
		));

		assert_ok!(DdcValidator::charge_payments_content_owners(
			RuntimeOrigin::signed(validator_1_stash.clone()),
			vec![bucket_info.clone()]
		));

		// Should not be able to charge account twice by the same validator during one era
		assert_noop!(
			DdcValidator::charge_payments_content_owners(
				RuntimeOrigin::signed(validator_1_stash.clone()),
				vec![bucket_info.clone()]
			),
			ValidatorError::<Test>::ContentOwnersDoubleSpend
		);

		let last_evt = System::events().pop().expect("Event expected").event;
		assert_eq!(last_evt, ddc_accounts::Event::Charged(bucket_info.amount).into());
	})
}

#[test]
fn set_validator_key_works_as_expected() {
	let mut t = new_test_ext();

	let validator_1_stash = AccountId::from([
		0xd2, 0xbf, 0x4b, 0x84, 0x4d, 0xfe, 0xfd, 0x67, 0x72, 0xa8, 0x84, 0x3e, 0x66, 0x9f, 0x94,
		0x34, 0x08, 0x96, 0x6a, 0x97, 0x7e, 0x3a, 0xe2, 0xaf, 0x1d, 0xd7, 0x8e, 0x0f, 0x55, 0xf4,
		0xdf, 0x67,
	]);
	let validator_1_controller: AccountId32 = AccountId::from([0xaa; 32]);

	t.execute_with(|| {
		assert_noop!(
			DdcValidator::set_validator_key(
				RuntimeOrigin::signed(validator_1_stash.clone()),
				validator_1_stash.clone(),
			),
			ValidatorError::<Test>::NotController
		);

		assert_ok!(DdcValidator::set_validator_key(
			RuntimeOrigin::signed(validator_1_controller.clone()),
			validator_1_stash.clone(),
		));

		staking::Validators::<Test>::remove(validator_1_stash.clone());

		// If stash is not a validator anymore, action will fail
		assert_noop!(
			DdcValidator::set_validator_key(
				RuntimeOrigin::signed(validator_1_controller),
				validator_1_stash,
			),
			ValidatorError::<Test>::NotValidatorStash
		);
	})
}
