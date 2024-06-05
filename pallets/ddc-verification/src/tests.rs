use ddc_primitives::{ClusterId, StorageNodeParams};
use frame_support::{assert_noop, assert_ok};
use sp_core::{
	offchain::{
		testing::{PendingRequest, TestOffchainExt, TestTransactionPoolExt},
		OffchainDbExt, OffchainWorkerExt, Timestamp, TransactionPoolExt,
	},
	Pair, H256,
};
use sp_io::TestExternalities;

use crate::{mock::*, Error, Event, NodeActivity, *};

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

		assert_ok!(DdcVerification::create_billing_reports(
			RuntimeOrigin::signed(dac_account.clone()),
			cluster_id,
			era,
			merkel_root_hash,
			merkel_root_hash,
		));

		System::assert_last_event(Event::BillingReportCreated { cluster_id, era }.into());

		let report = DdcVerification::active_billing_reports(cluster_id, dac_account.clone()).unwrap();
		assert_eq!(report.payers_merkle_root_hash, merkel_root_hash);

		assert_noop!(
			DdcVerification::create_billing_reports(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				merkel_root_hash,
				merkel_root_hash
			),
			Error::<Test>::BillingReportAlreadyExist
		);
	})
}

#[test]
fn set_validate_payout_batch_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let (pair1, _seed) = sp_core::sr25519::Pair::from_phrase(
			"spider sell nice animal border success square soda stem charge caution echo",
			None,
		)
		.unwrap();
		let (pair2, _seed) = sp_core::sr25519::Pair::from_phrase(
			"ketchup route purchase humble harsh true glide chef buyer crane infant sponsor",
			None,
		)
		.unwrap();
		let (pair3, _seed) = sp_core::sr25519::Pair::from_phrase(
			"hamster diamond design extra december body action relax front sustain heavy gaze",
			None,
		)
		.unwrap();
		let (pair4, _seed) = sp_core::sr25519::Pair::from_phrase(
			"clip olympic snack fringe critic claim chaos mother twist shy rule violin",
			None,
		)
		.unwrap();
		let (pair5, _seed) = sp_core::sr25519::Pair::from_phrase(
			"bamboo fish such plug arrive vague umbrella today glass venture hour ginger",
			None,
		)
		.unwrap();
		let (pair6, _seed) = sp_core::sr25519::Pair::from_phrase(
			"shallow radio below sudden unlock apology brisk shiver hill amateur tiny judge",
			None,
		)
		.unwrap();

		let account_id1 = AccountId::from(pair1.public().0);
		let account_id2 = AccountId::from(pair2.public().0);
		let account_id3 = AccountId::from(pair3.public().0);
		let account_id4 = AccountId::from(pair4.public().0);
		let account_id6 = AccountId::from(pair6.public().0);

		ValidatorSet::<Test>::put(vec![
			account_id1.clone(),
			account_id2.clone(),
			account_id3.clone(),
			account_id4.clone(),
			account_id6.clone(),
		]);
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let payout_data = PayoutData { hash: MmrRootHash::default() };
		let payout_data1 = PayoutData { hash: MmrRootHash::from_low_u64_ne(1) };

		// 1. If user is not part of validator, he/she won't be able to sign extrinsic
		assert_noop!(
			DdcVerification::set_validate_payout_batch(
				RuntimeOrigin::signed(AccountId::from(pair5.public().0)),
				cluster_id,
				era,
				payout_data.clone(),
			),
			Error::<Test>::NotAValidator
		);

		// 2. send signed transaction from valid validator
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
			Error::<Test>::AlreadySigned
		);

		// 4. send signed transaction from second valid validator
		assert_ok!(DdcVerification::set_validate_payout_batch(
			RuntimeOrigin::signed(account_id2.clone()),
			cluster_id,
			era,
			payout_data.clone(),
		));

		// 5. 2/3 rd validators have not signed yet the same data
		assert_eq!(DdcVerification::payout_batch(cluster_id, era), None);

		assert_ok!(DdcVerification::set_validate_payout_batch(
			RuntimeOrigin::signed(account_id3.clone()),
			cluster_id,
			era,
			payout_data.clone(),
		));

		// 6. send signed transaction from third valid validator but different hash
		assert_ok!(DdcVerification::set_validate_payout_batch(
			RuntimeOrigin::signed(account_id6.clone()),
			cluster_id,
			era,
			payout_data1.clone(),
		));

		// 7. 2/3 rd validators have not signed yet the same data
		assert_eq!(DdcVerification::payout_batch(cluster_id, era), None);

		// 8. send signed transaction from fourth valid validator
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
		let node_activity = NodeActivity {
			stored_bytes: 100,
			transferred_bytes: 50,
			number_of_puts: 10,
			number_of_gets: 20,
			proof: vec![1, 2, 3],
		};
		let node_activity_json = serde_json::to_string(&node_activity).unwrap();

		// Mock HTTP request and response
		let pending_request = PendingRequest {
			method: "GET".to_string(),
			uri: format!("http://{}:{}/activity/node?eraId={}", host, port, era_id),
			response: Some(node_activity_json.as_bytes().to_vec()),
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
		let activity = result.unwrap();
		assert_eq!(activity.number_of_gets, node_activity.number_of_gets);
		assert_eq!(activity.number_of_puts, node_activity.number_of_puts);
		assert_eq!(activity.transferred_bytes, node_activity.transferred_bytes);
		assert_eq!(activity.stored_bytes, node_activity.stored_bytes);
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
			proof: vec![1, 2, 3],
		};
		let customer_activity2 = CustomerActivity {
			bucket_id: 222,
			customer_id: [2; 32],
			stored_bytes: 1000,
			transferred_bytes: 500,
			number_of_puts: 100,
			number_of_gets: 200,
			proof: vec![4, 5, 6, 7],
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
