#![allow(clippy::get_first)]
//! Tests for the module.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ddc_primitives::{ClusterId, Fingerprint, PayableUsageHash};
use frame_support::{assert_noop, assert_ok, traits::Randomness};
use polkadot_ckb_merkle_mountain_range::{
	util::{MemMMR, MemStore},
	MMR,
};
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::Perquintill;

use super::{mock::*, *};

fn get_fingerprint(
	cluster_id: &ClusterId,
	era_id: DdcEra,
	start_era: i64,
	end_era: i64,
	payers_merkle_root: PayableUsageHash,
	payees_merkle_root: PayableUsageHash,
	cluster_usage: &NodeUsage,
) -> Fingerprint {
	let fingerprint = BillingFingerprint::<AccountId> {
		cluster_id: *cluster_id,
		era_id,
		start_era,
		end_era,
		payers_merkle_root,
		payees_merkle_root,
		cluster_usage: cluster_usage.clone(),
		validators: Default::default(),
	};

	fingerprint.selective_hash::<Test>()
}

fn hash_bucket_payable_usage_batch(usages: Vec<(BucketId, BucketUsage)>) -> (H256, MMRProof, H256) {
	if usages.len() > MAX_PAYOUT_BATCH_SIZE.into() {
		panic!("Batch size is reached")
	}

	let store1 = MemStore::default();
	let mut mmr1: MMR<PayableUsageHash, MergeMMRHash, &MemStore<PayableUsageHash>> =
		MemMMR::<PayableUsageHash, MergeMMRHash>::new(0, &store1);

	for usage in usages {
		let mut data = usage.0.encode(); // bucket_id
		data.extend_from_slice(&usage.1.stored_bytes.encode());
		data.extend_from_slice(&usage.1.transferred_bytes.encode());
		data.extend_from_slice(&usage.1.number_of_puts.encode());
		data.extend_from_slice(&usage.1.number_of_gets.encode());
		let hash = blake2_256(&data);
		let _pos: u64 = mmr1.push(H256(hash)).unwrap();
	}

	let payers_batch_root = mmr1.get_root().unwrap();

	let store2 = MemStore::default();
	let mut mmr2: MMR<PayableUsageHash, MergeMMRHash, &MemStore<PayableUsageHash>> =
		MemMMR::<PayableUsageHash, MergeMMRHash>::new(0, &store2);
	let batch_pos = mmr2.push(payers_batch_root).unwrap();

	let payers_root = mmr2.get_root().unwrap();

	let payers_batch_proof =
		MMRProof { proof: mmr2.gen_proof(vec![batch_pos]).unwrap().proof_items().to_vec() };

	(payers_batch_root, payers_batch_proof, payers_root)
}

fn hash_node_payable_usage_batch(usages: Vec<(NodePubKey, NodeUsage)>) -> (H256, MMRProof, H256) {
	if usages.len() > MAX_PAYOUT_BATCH_SIZE.into() {
		panic!("Batch size is reached")
	}

	let store1 = MemStore::default();
	let mut mmr1: MMR<PayableUsageHash, MergeMMRHash, &MemStore<PayableUsageHash>> =
		MemMMR::<PayableUsageHash, MergeMMRHash>::new(0, &store1);

	for usage in usages {
		let mut data = usage.0.encode(); // node_key
		data.extend_from_slice(&usage.1.stored_bytes.encode());
		data.extend_from_slice(&usage.1.transferred_bytes.encode());
		data.extend_from_slice(&usage.1.number_of_puts.encode());
		data.extend_from_slice(&usage.1.number_of_gets.encode());
		let hash = blake2_256(&data);
		let _pos: u64 = mmr1.push(H256(hash)).unwrap();
	}

	let payees_batch_root = mmr1.get_root().unwrap();

	let store2 = MemStore::default();
	let mut mmr2: MMR<PayableUsageHash, MergeMMRHash, &MemStore<PayableUsageHash>> =
		MemMMR::<PayableUsageHash, MergeMMRHash>::new(0, &store2);
	let batch_pos = mmr2.push(payees_batch_root).unwrap();

	let payees_root = mmr2.get_root().unwrap();

	let payees_batch_proof =
		MMRProof { proof: mmr2.gen_proof(vec![batch_pos]).unwrap().proof_items().to_vec() };

	(payees_batch_root, payees_batch_proof, payees_root)
}

fn get_root_with_proofs(hashes: Vec<H256>) -> (H256, Vec<MMRProof>) {
	let store = MemStore::default();
	let mut mmr: MMR<PayableUsageHash, MergeMMRHash, &MemStore<PayableUsageHash>> =
		MemMMR::<PayableUsageHash, MergeMMRHash>::new(0, &store);

	let mut positions = vec![];
	for hash in hashes {
		let pos: u64 = mmr.push(hash).unwrap();
		positions.push(pos);
	}

	let mut proofs = vec![];
	for pos in positions {
		let proof = MMRProof { proof: mmr.gen_proof(vec![pos]).unwrap().proof_items().to_vec() };
		proofs.push(proof);
	}

	let root = mmr.get_root().unwrap();

	(root, proofs)
}

#[test]
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
fn set_authorised_caller_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let root_account = AccountId::from([1; 32]);
		let dac_account = AccountId::from([2; 32]);

		assert_noop!(
			DdcPayouts::set_authorised_caller(
				RuntimeOrigin::signed(root_account),
				dac_account.clone()
			),
			BadOrigin
		);

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));

		System::assert_last_event(
			Event::AuthorisedCaller { authorised_caller: dac_account.clone() }.into(),
		);

		assert_eq!(DdcPayouts::authorised_caller().unwrap(), dac_account);
	})
}

#[test]
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
fn begin_billing_report_fails_for_unauthorised() {
	ExtBuilder.build_and_execute(|| {
		let cluster_id = ClusterId::from([1; 20]);
		let era = 100;
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		assert_noop!(
			DdcPayouts::begin_billing_report(
				RuntimeOrigin::signed(CUSTOMER3_KEY_32),
				cluster_id,
				era,
				start_era,
				end_era,
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::begin_billing_report(
				RuntimeOrigin::signed(AccountId::from([0; 32])),
				cluster_id,
				era,
				start_era,
				end_era,
			),
			Error::<Test>::Unauthorised
		);
	})
}

#[test]
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
fn begin_billing_report_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
=======
		let dac_account = AccountId::from([2; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();
		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
<<<<<<< HEAD
			cluster_id, era, start_era, end_era,
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));

=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
			cluster_id,
			era,
			start_era,
			end_era,
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
=======
			cluster_id,
			era,
			fingerprint
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		System::assert_last_event(Event::BillingReportInitialized { cluster_id, era }.into());

		let report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, PayoutState::Initialized);

		let fingerprint = DdcPayouts::billing_fingerprints(fingerprint).unwrap();

		assert_eq!(fingerprint.start_era, start_era);
		assert_eq!(fingerprint.end_era, end_era);
	})
}

#[test]
fn begin_charging_customers_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let dac_account = 3u128;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let dac_account = AccountId::from([3; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let fake_ocw_account = AccountId::from([124; 32]);
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
>>>>>>> f82a1179 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_charging_batch_index = 2;

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
			DdcPayouts::begin_charging_customers(
				RuntimeOrigin::signed(AccountId::from([124; 32])),
				cluster_id,
				era,
				max_batch_index,
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::begin_charging_customers(
				RuntimeOrigin::root(),
				cluster_id,
				era,
				max_batch_index,
			),
			BadOrigin
		);

		assert_noop!(
			DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				RuntimeOrigin::signed(DAC_ACCOUNT_ID),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				RuntimeOrigin::signed(DAC_ACCOUNT_ID.into()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.into()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> 8a36f637 (fix: clippy and formatiing)
=======
			<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				cluster_id,
				era,
				max_charging_batch_index,
			),
			Error::<Test>::BillingReportDoesNotExist
		);
	})
}

#[test]
fn begin_charging_customers_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
=======
		let dac_account = AccountId::from([2; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_charging_batch_index = 2;
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();
		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));

=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		));

<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));

		System::assert_last_event(Event::ChargingStarted { cluster_id, era }.into());

		let report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, PayoutState::ChargingCustomers);
		assert_eq!(report.charging_max_batch_index, max_charging_batch_index);
	})
}

#[test]
fn send_charging_customers_batch_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let root_account = AccountId::from([1; 32]);
<<<<<<< HEAD
		let dac_account = AccountId::from([2; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([3; 32]);
		let user2 = AccountId::from([4; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
>>>>>>> f82a1179 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
<<<<<<< HEAD
		let max_batch_index = 2;
		let batch_index = 1;
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let max_charging_batch_index = 1;
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
		let customer_usage = BucketUsage {
			transferred_bytes: 100,
			stored_bytes: 800,
			number_of_gets: 100,
			number_of_puts: 200,
		};
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
		let payers1 = vec![(node_key.clone(), bucket_id3, customer_usage)];
		let payers2 = vec![(node_key.clone(), bucket_id4, BucketUsage::default())];
<<<<<<< HEAD
=======
		let bucket_id1: BucketId = 1;
		let bucket_id2: BucketId = 2;
=======
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let customer_usage = CustomerUsage {
			transferred_bytes: 100,
			stored_bytes: -800,
			number_of_gets: 100,
			number_of_puts: 200,
		};
<<<<<<< HEAD
<<<<<<< HEAD
		let payers1 = vec![(user1, bucket_id1, customer_usage)];
<<<<<<< HEAD
		let payers2 = vec![(user2, bucket_id2, CustomerUsage::default())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers2 = vec![(user2.clone(), bucket_id2, CustomerUsage::default())];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
		let payers1 = vec![(user1, node_id.clone(), bucket_id1, customer_usage)];
		let payers2 = vec![(user2.clone(), node_id.clone(), bucket_id2, CustomerUsage::default())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
		let payers1 = vec![(node_key.clone(), bucket_id3, customer_usage)];
		let payers2 = vec![(node_key.clone(), bucket_id4, CustomerUsage::default())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
=======
		let payers1 = vec![(bucket_id3, customer_usage)];
		let payers2 = vec![(bucket_id4, BucketUsage::default())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

<<<<<<< HEAD
		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(AccountId::from([0; 32])),
>>>>>>> f82a1179 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payers1.clone(),
=======
				payers1.clone(),
				MMRProof::default(),
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::root(),
				cluster_id,
				era,
				batch_index,
				payers1.clone(),
				MMRProof::default(),
			),
			BadOrigin
		);

		assert_noop!(
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
				batch_index,
				payers1.clone(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				batch_index,
				&payers1.clone(),
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
			),
			Error::<Test>::BillingReportDoesNotExist
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
=======
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			cluster_id,
			era,
			start_era,
			end_era,
<<<<<<< HEAD
		));

		assert_noop!(
			DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payers1.clone(),
=======
				payers1.clone(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
=======
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				0,
				&payers1.clone(),
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
=======
				payers_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				0,
				&payers1.clone(),
				payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
		let payers1 = vec![(node_key, bucket_id4, BucketUsage::default())];
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
=======
		let payers1 = vec![(user2, node_id, bucket_id2, CustomerUsage::default())];
=======
		let payers1 = vec![(node_key, bucket_id4, CustomerUsage::default())];
<<<<<<< HEAD
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			0,
			&payers1.clone(),
			payers_proofs.get(0).unwrap().clone(),
		));

>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
<<<<<<< HEAD
				batch_index,
<<<<<<< HEAD
				payers1.clone(),
				MMRProof::default(),
			),
			Error::<Test>::ArithmeticOverflow
		);

=======
>>>>>>> e0ce0e5b (node integer delta usage (#412))
		let payers1 = vec![(user2, bucket_id2, CustomerUsage::default())];
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account),
>>>>>>> 00eed38c (Changes to accept stored_bytes as signed input (#410))
=======
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1.clone(),
=======
			payers1.clone(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
			MMRProof::default(),
		));

		assert_noop!(
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payers1,
=======
				payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				&payers1,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
=======
				0,
				&payers1,
				payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			),
			Error::<Test>::BatchIndexAlreadyProcessed
		);

<<<<<<< HEAD
		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payers2,
=======
				payers2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				batch_index,
				&payers2,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
			),
			Error::<Test>::BatchIndexAlreadyProcessed
		);
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			1,
			&payers2,
			payers_proofs.get(1).unwrap().clone(),
		));
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
	})
}

fn calculate_charge_parts_for_day(cluster_id: ClusterId, usage: BucketUsage) -> CustomerCharge {
	let pricing_params = get_pricing(&cluster_id);

	// Calculate the duration of the period in seconds
	let duration_seconds = 1.0 * 24.0 * 3600.0;
	let seconds_in_month = 30.44 * 24.0 * 3600.0;
	let fraction_of_month =
		Perquintill::from_rational(duration_seconds as u64, seconds_in_month as u64);

	let storage = fraction_of_month *
		(|| -> Option<u128> {
			(usage.stored_bytes as u128)
				.checked_mul(pricing_params.unit_per_mb_stored)?
				.checked_div(byte_unit::MEBIBYTE)
		})()
		.unwrap();

	CustomerCharge {
		transfer: pricing_params.unit_per_mb_streamed * (usage.transferred_bytes as u128) /
			byte_unit::MEBIBYTE,
		storage,
		puts: pricing_params.unit_per_put_request * (usage.number_of_puts as u128),
		gets: pricing_params.unit_per_get_request * (usage.number_of_gets as u128),
	}
}

fn calculate_charge_for_day(cluster_id: ClusterId, usage: BucketUsage) -> u128 {
	let charge = calculate_charge_parts_for_day(cluster_id, usage);
	charge.transfer + charge.storage + charge.puts + charge.gets
}

fn calculate_charge_parts_for_month(cluster_id: ClusterId, usage: BucketUsage) -> CustomerCharge {
	let pricing_params = get_pricing(&cluster_id);

	let fraction_of_month = Perquintill::one();
	let storage = fraction_of_month *
		(|| -> Option<u128> {
			(usage.stored_bytes as u128)
				.checked_mul(pricing_params.unit_per_mb_stored)?
				.checked_div(byte_unit::MEBIBYTE)
		})()
		.unwrap();

	CustomerCharge {
		transfer: pricing_params.unit_per_mb_streamed * (usage.transferred_bytes as u128) /
			byte_unit::MEBIBYTE,
		storage,
		puts: pricing_params.unit_per_put_request * (usage.number_of_puts as u128),
		gets: pricing_params.unit_per_get_request * (usage.number_of_gets as u128),
	}
}

fn calculate_charge_parts_for_hour(cluster_id: ClusterId, usage: BucketUsage) -> CustomerCharge {
	let pricing_params = get_pricing(&cluster_id);

	let duration_seconds = 1.0 * 1.0 * 3600.0;
	let seconds_in_month = 30.44 * 24.0 * 3600.0;
	let fraction_of_hour =
		Perquintill::from_rational(duration_seconds as u64, seconds_in_month as u64);
	let storage = fraction_of_hour *
		(|| -> Option<u128> {
			(usage.stored_bytes as u128)
				.checked_mul(pricing_params.unit_per_mb_stored)?
				.checked_div(byte_unit::MEBIBYTE)
		})()
		.unwrap();

	CustomerCharge {
		transfer: pricing_params.unit_per_mb_streamed * (usage.transferred_bytes as u128) /
			byte_unit::MEBIBYTE,
		storage,
		puts: pricing_params.unit_per_put_request * (usage.number_of_puts as u128),
		gets: pricing_params.unit_per_get_request * (usage.number_of_gets as u128),
	}
}

fn calculate_charge_for_month(cluster_id: ClusterId, usage: BucketUsage) -> u128 {
	let charge = calculate_charge_parts_for_month(cluster_id, usage);
	charge.transfer + charge.storage + charge.puts + charge.gets
}

fn calculate_charge_for_hour(cluster_id: ClusterId, usage: BucketUsage) -> u128 {
	let charge = calculate_charge_parts_for_hour(cluster_id, usage);
	charge.transfer + charge.storage + charge.puts + charge.gets
}

#[test]
fn send_charging_customers_batch_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
		let user2_debtor = AccountId::from([2; 32]);
		let user3_debtor = AccountId::from([3; 32]);
		let user4 = AccountId::from([4; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let cluster_id = ClusterId::from([12; 20]);
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let era = 100;
		let max_charging_batch_index = 2;
		let mut batch_index = 0;
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)

		let bucket_id1: BucketId = BUCKET_ID1;
		let bucket_id2: BucketId = BUCKET_ID2;
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
<<<<<<< HEAD

		let usage1 = BucketUsage {
=======
		let bucket_id1: BucketId = 1;
		let bucket_id2: BucketId = 2;
		let bucket_id3: BucketId = 3;
		let bucket_id4: BucketId = 4;
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)

<<<<<<< HEAD
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			// should pass without debt
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};
		let usage2 = BucketUsage {
			// should fail as not enough balance
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage3 = BucketUsage {
			// should pass but with debt (partial charge)
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage4 = BucketUsage {
			// should pass without debt
			transferred_bytes: 467457,
			stored_bytes: 45674567456,
			number_of_puts: 3456345,
			number_of_gets: 242334563456423,
		};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
		];
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
=======
		let payers1 =
			vec![(user2_debtor, bucket_id2, usage2.clone()), (user4, bucket_id4, usage4.clone())];
		let payers2 = vec![(user1, bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor, bucket_id3, usage3.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
		];
<<<<<<< HEAD
<<<<<<< HEAD
		let payers2 = vec![(user1.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), bucket_id3, usage3.clone())];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let payers2 = vec![(user1.clone(), node_id.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), node_id, bucket_id3, usage3.clone())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers1 = vec![(bucket_id2, usage2.clone()), (bucket_id4, usage4.clone())];
		let payers2 = vec![(bucket_id1, usage1.clone())];
		let payers3 = vec![(bucket_id3, usage3.clone())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers3_batch_root, _, _) = hash_bucket_payable_usage_batch(payers3.clone());

		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root, payers3_batch_root]);

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));
		assert_eq!(System::events().len(), 3);

		// batch 1
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1,
=======
			payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers1,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let usage4_charge = calculate_charge_for_month(cluster_id, usage4.clone());
		let user2_debt = DdcPayouts::debtor_customers(cluster_id, user2_debtor.clone()).unwrap();
		let expected_charge2 = calculate_charge_for_month(cluster_id, usage2.clone());
		let mut debt = expected_charge2 - CUSTOMER2_BALANCE;
		assert_eq!(user2_debt, debt);

		let ratio = Perquintill::from_rational(CUSTOMER2_BALANCE, expected_charge2);
		let mut charge2 = calculate_charge_parts_for_month(cluster_id, usage2);
		charge2.storage = ratio * charge2.storage;
		charge2.transfer = ratio * charge2.transfer;
		charge2.gets = ratio * charge2.gets;
		charge2.puts = ratio * charge2.puts;

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge4 = calculate_charge_parts_for_month(cluster_id, usage4);
		assert_eq!(charge2.puts + charge4.puts, report.total_customer_charge.puts);
		assert_eq!(charge2.gets + charge4.gets, report.total_customer_charge.gets);
		assert_eq!(charge2.storage + charge4.storage, report.total_customer_charge.storage);
		assert_eq!(charge2.transfer + charge4.transfer, report.total_customer_charge.transfer);

		System::assert_has_event(
			Event::ChargeFailed {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user2_debtor.clone(),
=======
				customer_id: user2_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user2_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id2,
				batch_index,
				charged: CUSTOMER2_BALANCE,
				expected_to_charge: expected_charge2,
			}
			.into(),
		);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
				customer_id: user2_debtor,
				bucket_id: bucket_id2,
				batch_index,
				amount: debt,
			}
			.into(),
		);
		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				customer_id: user4,
				bucket_id: bucket_id4,
				batch_index,
				amount: usage4_charge,
			}
			.into(),
		);

		assert_eq!(System::events().len(), 5 + 3 + 1); // 1 for Currency::transfer

		// batch 2
		let mut before_total_customer_charge = report.total_customer_charge;
		batch_index += 1;
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers2,
=======
			payers2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers2,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(1).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				batch_index,
				bucket_id: bucket_id1,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user1.clone(),
=======
				customer_id: user1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user1.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				amount: calculate_charge_for_month(cluster_id, usage1.clone()),
			}
			.into(),
		);

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge1 = calculate_charge_parts_for_month(cluster_id, usage1);
		assert_eq!(
			charge1.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			charge1.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			charge1.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			charge1.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		assert_eq!(report.state, PayoutState::ChargingCustomers);
		let user1_debt = DdcPayouts::debtor_customers(cluster_id, user1);
		assert_eq!(user1_debt, None);

		let balance_before = Balances::free_balance(DdcPayouts::account_id());

		// batch 3
		batch_index += 1;
		before_total_customer_charge = report.total_customer_charge.clone();
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers3,
=======
			payers3,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers3,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(2).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let user3_charge = calculate_charge_for_month(cluster_id, usage3.clone());
		let charge3 = calculate_charge_parts_for_month(cluster_id, usage3);
		let ratio = Perquintill::from_rational(PARTIAL_CHARGE, user3_charge);
		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(
			ratio * charge3.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			ratio * charge3.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			ratio * charge3.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			ratio * charge3.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		let balance = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance, balance_before + PARTIAL_CHARGE);

		let user3_debt = DdcPayouts::debtor_customers(cluster_id, user3_debtor.clone()).unwrap();
		debt = user3_charge - PARTIAL_CHARGE;
		assert_eq!(user3_debt, debt);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user3_debtor.clone(),
=======
				customer_id: user3_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user3_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id3,
				batch_index,
				amount: user3_debt,
			}
			.into(),
		);

		System::assert_last_event(
			Event::ChargeFailed {
				cluster_id,
				era,
				batch_index,
				customer_id: user3_debtor,
				bucket_id: bucket_id3,
				charged: PARTIAL_CHARGE,
				expected_to_charge: user3_charge,
			}
			.into(),
		);
	})
}

#[test]
fn end_charging_customers_works_small_usage_1_hour() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
		let user6: AccountId = CUSTOMER6_KEY_32;
		let user7 = CUSTOMER7_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user6 = AccountId::from([6; 32]);
		let user7 = AccountId::from([7; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		let cluster_id = HIGH_FEES_CLUSTER_ID;
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
<<<<<<< HEAD
		let bucket_id6: BucketId = BUCKET_ID6;
		let bucket_id7: BucketId = BUCKET_ID7;

		let usage6 = BucketUsage {
=======
		let bucket_id6: BucketId = 6;
		let bucket_id7: BucketId = 7;
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let user6: AccountId = CUSTOMER6_KEY_32;
		let user7 = CUSTOMER7_KEY_32;
		let cluster_id = HIGH_FEES_CLUSTER_ID;
		let era = 100;
		let max_charging_batch_index = 0;
		let bucket_id6: BucketId = BUCKET_ID6;
		let bucket_id7: BucketId = BUCKET_ID7;
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)

<<<<<<< HEAD
		let usage6 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage6 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			transferred_bytes: 0,
			stored_bytes: 474_957,
			number_of_puts: 0,
			number_of_gets: 0,
		};
		let usage7 = BucketUsage {
			transferred_bytes: 474_957,
			stored_bytes: 0,
			number_of_puts: 0,
			number_of_gets: 0,
		};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD

<<<<<<< HEAD
		let payers1 = vec![
			(node_key.clone(), bucket_id6, usage6.clone()),
			(node_key.clone(), bucket_id7, usage7.clone()),
<<<<<<< HEAD
		];
=======
		let payers1 =
			vec![(user6, bucket_id6, usage6.clone()), (user7, bucket_id7, usage7.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
=======

>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
		let payers1 = vec![
			(user6.clone(), node_id.clone(), bucket_id6, usage6.clone()),
			(user7.clone(), node_id.clone(), bucket_id7, usage7.clone()),
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let payers1 = vec![(bucket_id6, usage6.clone()), (bucket_id7, usage7.clone())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (1.0 * 1.0 * 3600.0) as i64; // 1 hour
		let cluster_usage = NodeUsage::default();

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers1_batch_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers1_batch_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));
		assert_eq!(System::events().len(), 3);

		// batch 1
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1,
=======
			payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			0,
			&payers1,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
		));

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let usage6_charge = calculate_charge_for_hour(cluster_id, usage6.clone());
		let usage7_charge = calculate_charge_for_hour(cluster_id, usage7.clone());
		let charge6 = calculate_charge_parts_for_hour(cluster_id, usage6);
		let charge7 = calculate_charge_parts_for_hour(cluster_id, usage7);
		assert_eq!(charge7.puts + charge6.puts, report_before.total_customer_charge.puts);
		assert_eq!(charge7.gets + charge6.gets, report_before.total_customer_charge.gets);
		assert_eq!(charge7.storage + charge6.storage, report_before.total_customer_charge.storage);
		assert_eq!(
			charge7.transfer + charge6.transfer,
			report_before.total_customer_charge.transfer
		);

		System::assert_has_event(
			Event::Charged {
				cluster_id,
				era,
				customer_id: user6,
				bucket_id: bucket_id6,
				batch_index: 0,
				amount: usage6_charge,
			}
			.into(),
		);

		System::assert_has_event(
			Event::Charged {
				cluster_id,
				era,
				customer_id: user7,
				bucket_id: bucket_id7,
				batch_index: 0,
				amount: usage7_charge,
			}
			.into(),
		);

		let mut balance = Balances::free_balance(DdcPayouts::account_id());
		let charge = usage7_charge + usage6_charge;
		assert_eq!(balance - Balances::minimum_balance(), charge);

		balance = Balances::free_balance(AccountId::from(TREASURY_ACCOUNT_ID));
		assert_eq!(balance, 0);

		balance = Balances::free_balance(AccountId::from(RESERVE_ACCOUNT_ID));
		assert_eq!(balance, 0);

		balance = Balances::free_balance(AccountId::from(VALIDATOR1_ACCOUNT_ID));
		assert_eq!(balance, 0);

		balance = Balances::free_balance(AccountId::from(VALIDATOR2_ACCOUNT_ID));
		assert_eq!(balance, 0);

		balance = Balances::free_balance(AccountId::from(VALIDATOR3_ACCOUNT_ID));
		assert_eq!(balance, 0);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
			cluster_id,
			era,
		));
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

		System::assert_has_event(Event::ChargingFinished { cluster_id, era }.into());
		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report_after.state, PayoutState::CustomersChargedWithFees);

		let fees = get_fees(&cluster_id);
		let total_left_from_one =
			(fees.treasury_share + fees.validators_share + fees.cluster_reserve_share)
				.left_from_one();

		assert_eq!(
			total_left_from_one,
			Perquintill::one() -
				(PRICING_FEES_HIGH.treasury_share +
					PRICING_FEES_HIGH.validators_share +
					PRICING_FEES_HIGH.cluster_reserve_share)
		);
		assert_eq!(fees.treasury_share, PRICING_FEES_HIGH.treasury_share);
		assert_eq!(fees.validators_share, PRICING_FEES_HIGH.validators_share);
		assert_eq!(fees.cluster_reserve_share, PRICING_FEES_HIGH.cluster_reserve_share);

		balance = Balances::free_balance(AccountId::from(TREASURY_ACCOUNT_ID));
		assert_eq!(balance, get_fees(&cluster_id).treasury_share * charge);
		assert!(balance > 0);

		balance = Balances::free_balance(AccountId::from(RESERVE_ACCOUNT_ID));
		assert_eq!(balance, get_fees(&cluster_id).cluster_reserve_share * charge);
		assert!(balance > 0);

		balance = Balances::free_balance(AccountId::from(VALIDATOR1_ACCOUNT_ID));
		let mut ratio = Perquintill::from_rational(
			VALIDATOR1_SCORE,
			VALIDATOR1_SCORE + VALIDATOR2_SCORE + VALIDATOR3_SCORE,
		);
		assert_eq!(balance, get_fees(&cluster_id).validators_share * ratio * charge);
		assert!(balance > 0);

		balance = Balances::free_balance(AccountId::from(VALIDATOR2_ACCOUNT_ID));
		ratio = Perquintill::from_rational(
			VALIDATOR2_SCORE,
			VALIDATOR1_SCORE + VALIDATOR2_SCORE + VALIDATOR3_SCORE,
		);
		assert_eq!(balance, get_fees(&cluster_id).validators_share * ratio * charge);
		assert!(balance > 0);

		balance = Balances::free_balance(AccountId::from(VALIDATOR3_ACCOUNT_ID));
		ratio = Perquintill::from_rational(
			VALIDATOR3_SCORE,
			VALIDATOR1_SCORE + VALIDATOR2_SCORE + VALIDATOR3_SCORE,
		);
		assert_eq!(balance, get_fees(&cluster_id).validators_share * ratio * charge);
		assert!(balance > 0);

		assert_eq!(
			report_after.total_customer_charge.transfer,
			total_left_from_one * report_before.total_customer_charge.transfer
		);
		assert!(report_after.total_customer_charge.transfer > 0);
		assert_eq!(
			report_after.total_customer_charge.storage,
			total_left_from_one * report_before.total_customer_charge.storage
		);
		assert!(report_after.total_customer_charge.storage > 0);
		assert_eq!(
			report_after.total_customer_charge.puts,
			total_left_from_one * report_before.total_customer_charge.puts
		);
		assert_eq!(
			report_after.total_customer_charge.gets,
			total_left_from_one * report_before.total_customer_charge.gets
		);
	})
}

#[test]
fn send_charging_customers_batch_works_for_day() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
		let user2_debtor = AccountId::from([2; 32]);
		let user3_debtor = AccountId::from([3; 32]);
		let user4 = AccountId::from([4; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_charging_batch_index = 2;
		let mut batch_index = 0;
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let bucket_id1: BucketId = BUCKET_ID1;
		let bucket_id2: BucketId = BUCKET_ID2;
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;

<<<<<<< HEAD
<<<<<<< HEAD
		let usage1 = BucketUsage {
=======
		let bucket_id1: BucketId = 1;
		let bucket_id2: BucketId = 2;
		let bucket_id3: BucketId = 3;
		let bucket_id4: BucketId = 4;
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			// should pass without debt
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};
		let usage2 = BucketUsage {
			// should fail as not enough balance
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage3 = BucketUsage {
			// should pass but with debt (partial charge)
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage4 = BucketUsage {
			// should pass without debt
			transferred_bytes: 467457,
			stored_bytes: 45674567456,
			number_of_puts: 3456345,
			number_of_gets: 242334563456423,
		};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
		];

		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
=======
		let payers1 =
			vec![(user2_debtor, bucket_id2, usage2.clone()), (user4, bucket_id4, usage4.clone())];
		let payers2 = vec![(user1, bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor, bucket_id3, usage3.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));

>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
		];
<<<<<<< HEAD
		let payers2 = vec![(user1.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), bucket_id3, usage3.clone())];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======

<<<<<<< HEAD
		let payers2 = vec![(user1.clone(), node_id.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), node_id.clone(), bucket_id3, usage3.clone())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers1 = vec![(bucket_id2, usage2.clone()), (bucket_id4, usage4.clone())];
		let payers2 = vec![(bucket_id1, usage1.clone())];
		let payers3 = vec![(bucket_id3, usage3.clone())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (1.0 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers3_batch_root, _, _) = hash_bucket_payable_usage_batch(payers3.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root, payers3_batch_root]);

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));
		assert_eq!(System::events().len(), 3);

		// batch 1
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1,
=======
			payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers1,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let usage4_charge = calculate_charge_for_day(cluster_id, usage4.clone());
		let user2_debt = DdcPayouts::debtor_customers(cluster_id, user2_debtor.clone()).unwrap();
		let expected_charge2 = calculate_charge_for_day(cluster_id, usage2.clone());
		let mut debt = expected_charge2 - CUSTOMER2_BALANCE;
		assert_eq!(user2_debt, debt);

		let ratio = Perquintill::from_rational(CUSTOMER2_BALANCE, expected_charge2);
		let mut charge2 = calculate_charge_parts_for_day(cluster_id, usage2);
		charge2.storage = ratio * charge2.storage;
		charge2.transfer = ratio * charge2.transfer;
		charge2.gets = ratio * charge2.gets;
		charge2.puts = ratio * charge2.puts;

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge4 = calculate_charge_parts_for_day(cluster_id, usage4);
		assert_eq!(charge2.puts + charge4.puts, report.total_customer_charge.puts);
		assert_eq!(charge2.gets + charge4.gets, report.total_customer_charge.gets);
		assert_eq!(charge2.storage + charge4.storage, report.total_customer_charge.storage);
		assert_eq!(charge2.transfer + charge4.transfer, report.total_customer_charge.transfer);

		System::assert_has_event(
			Event::ChargeFailed {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user2_debtor.clone(),
=======
				customer_id: user2_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user2_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id2,
				batch_index,
				charged: CUSTOMER2_BALANCE,
				expected_to_charge: expected_charge2,
			}
			.into(),
		);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
				customer_id: user2_debtor,
				bucket_id: bucket_id2,
				batch_index,
				amount: debt,
			}
			.into(),
		);
		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				customer_id: user4,
				bucket_id: bucket_id4,
				batch_index,
				amount: usage4_charge,
			}
			.into(),
		);

		assert_eq!(System::events().len(), 5 + 3 + 1); // 1 for Currency::transfer

		// batch 2
		let mut before_total_customer_charge = report.total_customer_charge;
		batch_index += 1;
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers2,
=======
			payers2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers2,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(1).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user1.clone(),
=======
				customer_id: user1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user1.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id1,
				amount: calculate_charge_for_day(cluster_id, usage1.clone()),
			}
			.into(),
		);

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge1 = calculate_charge_parts_for_day(cluster_id, usage1);
		assert_eq!(
			charge1.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			charge1.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			charge1.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			charge1.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		assert_eq!(report.state, PayoutState::ChargingCustomers);
		let user1_debt = DdcPayouts::debtor_customers(cluster_id, user1);
		assert_eq!(user1_debt, None);

		let balance_before = Balances::free_balance(DdcPayouts::account_id());

		// batch 3
		batch_index += 1;
		before_total_customer_charge = report.total_customer_charge.clone();
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers3,
=======
			payers3,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers3,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(2).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let user3_charge = calculate_charge_for_day(cluster_id, usage3.clone());
		let charge3 = calculate_charge_parts_for_day(cluster_id, usage3);
		let ratio = Perquintill::from_rational(PARTIAL_CHARGE, user3_charge);
		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(
			ratio * charge3.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			ratio * charge3.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			ratio * charge3.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			ratio * charge3.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		let balance = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance, balance_before + PARTIAL_CHARGE);

		let user3_debt = DdcPayouts::debtor_customers(cluster_id, user3_debtor.clone()).unwrap();
		debt = user3_charge - PARTIAL_CHARGE;
		assert_eq!(user3_debt, debt);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user3_debtor.clone(),
=======
				customer_id: user3_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user3_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id3,
				batch_index,
				amount: user3_debt,
			}
			.into(),
		);

		System::assert_last_event(
			Event::ChargeFailed {
				cluster_id,
				era,
				batch_index,
				customer_id: user3_debtor,
				bucket_id: bucket_id3,
				charged: PARTIAL_CHARGE,
				expected_to_charge: user3_charge,
			}
			.into(),
		);
	})
}

#[test]
fn send_charging_customers_batch_works_for_day_free_storage() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
		let user2_debtor = AccountId::from([2; 32]);
		let user3_debtor = AccountId::from([3; 32]);
		let user4 = AccountId::from([4; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		let cluster_id = STORAGE_ZERO_CLUSTER_ID;
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));

>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let max_batch_index = 3;
		let mut batch_index = 0;
<<<<<<< HEAD
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
		let cluster_id = STORAGE_ZERO_CLUSTER_ID;
		let era = 100;
		let max_charging_batch_index = 2;
		let mut batch_index = 0;
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let bucket_id1: BucketId = BUCKET_ID1;
		let bucket_id2: BucketId = BUCKET_ID2;
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
<<<<<<< HEAD

		let usage1 = BucketUsage {
=======
		let bucket_id1: BucketId = 1;
		let bucket_id2: BucketId = 2;
		let bucket_id3: BucketId = 3;
		let bucket_id4: BucketId = 4;
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)

<<<<<<< HEAD
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			// should pass without debt
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};
		let usage2 = BucketUsage {
			// should fail as not enough balance
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage3 = BucketUsage {
			// should pass but with debt (partial charge)
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage4 = BucketUsage {
			// should pass without debt
			transferred_bytes: 467457,
			stored_bytes: 45674567456,
			number_of_puts: 3456345,
			number_of_gets: 242334563456423,
		};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
		];

		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
=======
		let payers1 =
			vec![(user2_debtor, bucket_id2, usage2.clone()), (user4, bucket_id4, usage4.clone())];
		let payers2 = vec![(user1, bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor, bucket_id3, usage3.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
		];
<<<<<<< HEAD
		let payers2 = vec![(user1.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), bucket_id3, usage3.clone())];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======

<<<<<<< HEAD
		let payers2 = vec![(user1.clone(), node_id.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), node_id.clone(), bucket_id3, usage3.clone())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers1 = vec![(bucket_id2, usage2.clone()), (bucket_id4, usage4.clone())];
		let payers2 = vec![(bucket_id1, usage1.clone())];
		let payers3 = vec![(bucket_id3, usage3.clone())];

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers3_batch_root, _, _) = hash_bucket_payable_usage_batch(payers3.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root, payers3_batch_root]);

>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (1.0 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();
		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));
		assert_eq!(System::events().len(), 3);

		// batch 1
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1,
=======
			payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers1,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let usage4_charge = calculate_charge_for_day(cluster_id, usage4.clone());
		let user2_debt = DdcPayouts::debtor_customers(cluster_id, user2_debtor.clone()).unwrap();
		let expected_charge2 = calculate_charge_for_day(cluster_id, usage2.clone());
		let mut debt = expected_charge2 - CUSTOMER2_BALANCE;
		assert_eq!(user2_debt, debt);

		let ratio = Perquintill::from_rational(CUSTOMER2_BALANCE, expected_charge2);
		let mut charge2 = calculate_charge_parts_for_day(cluster_id, usage2);
		charge2.storage = ratio * charge2.storage;
		charge2.transfer = ratio * charge2.transfer;
		charge2.gets = ratio * charge2.gets;
		charge2.puts = ratio * charge2.puts;

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge4 = calculate_charge_parts_for_day(cluster_id, usage4);
		assert_eq!(charge2.puts + charge4.puts, report.total_customer_charge.puts);
		assert_eq!(charge2.gets + charge4.gets, report.total_customer_charge.gets);
		assert_eq!(charge2.storage + charge4.storage, report.total_customer_charge.storage);
		assert_eq!(charge2.transfer + charge4.transfer, report.total_customer_charge.transfer);

		System::assert_has_event(
			Event::ChargeFailed {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user2_debtor.clone(),
=======
				customer_id: user2_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user2_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id2,
				batch_index,
				charged: CUSTOMER2_BALANCE,
				expected_to_charge: expected_charge2,
			}
			.into(),
		);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
				customer_id: user2_debtor,
				bucket_id: bucket_id2,
				batch_index,
				amount: debt,
			}
			.into(),
		);
		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				customer_id: user4,
				bucket_id: bucket_id4,
				batch_index,
				amount: usage4_charge,
			}
			.into(),
		);

		assert_eq!(System::events().len(), 5 + 3 + 1); // 1 for Currency::transfer

		// batch 2
		let mut before_total_customer_charge = report.total_customer_charge;
		batch_index += 1;
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers2,
=======
			payers2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers2,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(1).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user1.clone(),
=======
				customer_id: user1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user1.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id1,
				amount: calculate_charge_for_day(cluster_id, usage1.clone()),
			}
			.into(),
		);

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge1 = calculate_charge_parts_for_day(cluster_id, usage1);
		assert_eq!(
			charge1.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			charge1.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			charge1.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			charge1.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		assert_eq!(report.state, PayoutState::ChargingCustomers);
		let user1_debt = DdcPayouts::debtor_customers(cluster_id, user1);
		assert_eq!(user1_debt, None);

		let balance_before = Balances::free_balance(DdcPayouts::account_id());

		// batch 3
		batch_index += 1;
		before_total_customer_charge = report.total_customer_charge.clone();
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers3,
=======
			payers3,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers3,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(2).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let user3_charge = calculate_charge_for_day(cluster_id, usage3.clone());
		let charge3 = calculate_charge_parts_for_day(cluster_id, usage3);
		let ratio = Perquintill::from_rational(PARTIAL_CHARGE, user3_charge);
		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(
			ratio * charge3.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			ratio * charge3.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			ratio * charge3.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			ratio * charge3.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		let balance = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance, balance_before + PARTIAL_CHARGE);

		let user3_debt = DdcPayouts::debtor_customers(cluster_id, user3_debtor.clone()).unwrap();
		debt = user3_charge - PARTIAL_CHARGE;
		assert_eq!(user3_debt, debt);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user3_debtor.clone(),
=======
				customer_id: user3_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user3_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id3,
				batch_index,
				amount: user3_debt,
			}
			.into(),
		);

		System::assert_last_event(
			Event::ChargeFailed {
				cluster_id,
				era,
				batch_index,
				customer_id: user3_debtor,
				bucket_id: bucket_id3,
				charged: PARTIAL_CHARGE,
				expected_to_charge: user3_charge,
			}
			.into(),
		);
	})
}

#[test]
fn send_charging_customers_batch_works_for_day_free_stream() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
		let user2_debtor = AccountId::from([2; 32]);
		let user3_debtor = AccountId::from([3; 32]);
		let user4 = AccountId::from([4; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		let cluster_id = STREAM_ZERO_CLUSTER_ID;
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let max_batch_index = 3;
		let mut batch_index = 0;
<<<<<<< HEAD
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
		let cluster_id = STREAM_ZERO_CLUSTER_ID;
		let era = 100;
		let max_charging_batch_index = 2;
		let mut batch_index = 0;
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let bucket_id1: BucketId = BUCKET_ID1;
		let bucket_id2: BucketId = BUCKET_ID2;
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
<<<<<<< HEAD

		let usage1 = BucketUsage {
=======
		let bucket_id1: BucketId = 1;
		let bucket_id2: BucketId = 2;
		let bucket_id3: BucketId = 3;
		let bucket_id4: BucketId = 4;
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)

<<<<<<< HEAD
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			// should pass without debt
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};
		let usage2 = BucketUsage {
			// should fail as not enough balance
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage3 = BucketUsage {
			// should pass but with debt (partial charge)
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage4 = BucketUsage {
			// should pass without debt
			transferred_bytes: 467457,
			stored_bytes: 45674567456,
			number_of_puts: 3456345,
			number_of_gets: 242334563456423,
		};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
		];
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
=======
		let payers1 =
			vec![(user2_debtor, bucket_id2, usage2.clone()), (user4, bucket_id4, usage4.clone())];
		let payers2 = vec![(user1, bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor, bucket_id3, usage3.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
		];
<<<<<<< HEAD
<<<<<<< HEAD
		let payers2 = vec![(user1.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), bucket_id3, usage3.clone())];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let payers2 = vec![(user1.clone(), node_id.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), node_id, bucket_id3, usage3.clone())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers1 = vec![(bucket_id2, usage2.clone()), (bucket_id4, usage4.clone())];
		let payers2 = vec![(bucket_id1, usage1.clone())];
		let payers3 = vec![(bucket_id3, usage3.clone())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (1.0 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers3_batch_root, _, _) = hash_bucket_payable_usage_batch(payers3.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root, payers3_batch_root]);

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));
		assert_eq!(System::events().len(), 3);

		// batch 1
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1,
=======
			payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers1,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let usage4_charge = calculate_charge_for_day(cluster_id, usage4.clone());
		let user2_debt = DdcPayouts::debtor_customers(cluster_id, user2_debtor.clone()).unwrap();
		let expected_charge2 = calculate_charge_for_day(cluster_id, usage2.clone());
		let mut debt = expected_charge2 - CUSTOMER2_BALANCE;
		assert_eq!(user2_debt, debt);

		let ratio = Perquintill::from_rational(CUSTOMER2_BALANCE, expected_charge2);
		let mut charge2 = calculate_charge_parts_for_day(cluster_id, usage2);
		charge2.storage = ratio * charge2.storage;
		charge2.transfer = ratio * charge2.transfer;
		charge2.gets = ratio * charge2.gets;
		charge2.puts = ratio * charge2.puts;

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge4 = calculate_charge_parts_for_day(cluster_id, usage4);
		assert_eq!(charge2.puts + charge4.puts, report.total_customer_charge.puts);
		assert_eq!(charge2.gets + charge4.gets, report.total_customer_charge.gets);
		assert_eq!(charge2.storage + charge4.storage, report.total_customer_charge.storage);
		assert_eq!(charge2.transfer + charge4.transfer, report.total_customer_charge.transfer);

		System::assert_has_event(
			Event::ChargeFailed {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user2_debtor.clone(),
=======
				customer_id: user2_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user2_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id2,
				batch_index,
				charged: CUSTOMER2_BALANCE,
				expected_to_charge: expected_charge2,
			}
			.into(),
		);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
				customer_id: user2_debtor,
				bucket_id: bucket_id2,
				batch_index,
				amount: debt,
			}
			.into(),
		);
		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				customer_id: user4,
				bucket_id: bucket_id4,
				batch_index,
				amount: usage4_charge,
			}
			.into(),
		);

		assert_eq!(System::events().len(), 5 + 3 + 1); // 1 for Currency::transfer

		// batch 2
		let mut before_total_customer_charge = report.total_customer_charge;
		batch_index += 1;
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers2,
=======
			payers2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers2,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(1).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user1.clone(),
=======
				customer_id: user1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user1.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id1,
				amount: calculate_charge_for_day(cluster_id, usage1.clone()),
			}
			.into(),
		);

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge1 = calculate_charge_parts_for_day(cluster_id, usage1);
		assert_eq!(
			charge1.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			charge1.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			charge1.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			charge1.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		assert_eq!(report.state, PayoutState::ChargingCustomers);
		let user1_debt = DdcPayouts::debtor_customers(cluster_id, user1);
		assert_eq!(user1_debt, None);

		let balance_before = Balances::free_balance(DdcPayouts::account_id());

		// batch 3
		batch_index += 1;
		before_total_customer_charge = report.total_customer_charge.clone();
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers3,
=======
			payers3,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers3,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(2).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let user3_charge = calculate_charge_for_day(cluster_id, usage3.clone());
		let charge3 = calculate_charge_parts_for_day(cluster_id, usage3);
		let ratio = Perquintill::from_rational(PARTIAL_CHARGE, user3_charge);
		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(
			ratio * charge3.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			ratio * charge3.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			ratio * charge3.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			ratio * charge3.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		let balance = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance, balance_before + PARTIAL_CHARGE);

		let user3_debt = DdcPayouts::debtor_customers(cluster_id, user3_debtor.clone()).unwrap();
		debt = user3_charge - PARTIAL_CHARGE;
		assert_eq!(user3_debt, debt);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user3_debtor.clone(),
=======
				customer_id: user3_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user3_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id3,
				batch_index,
				amount: user3_debt,
			}
			.into(),
		);

		System::assert_last_event(
			Event::ChargeFailed {
				cluster_id,
				era,
				batch_index,
				customer_id: user3_debtor,
				bucket_id: bucket_id3,
				charged: PARTIAL_CHARGE,
				expected_to_charge: user3_charge,
			}
			.into(),
		);
	})
}

#[test]
fn send_charging_customers_batch_works_for_day_free_get() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
		let user2_debtor = AccountId::from([2; 32]);
		let user3_debtor = AccountId::from([3; 32]);
		let user4 = AccountId::from([4; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		let cluster_id = GET_ZERO_CLUSTER_ID;
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let max_batch_index = 3;
		let mut batch_index = 0;
<<<<<<< HEAD
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
		let cluster_id = GET_ZERO_CLUSTER_ID;
		let era = 100;
		let max_charging_batch_index = 2;
		let mut batch_index = 0;
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let bucket_id1: BucketId = BUCKET_ID1;
		let bucket_id2: BucketId = BUCKET_ID2;
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
<<<<<<< HEAD

		let usage1 = BucketUsage {
=======
		let bucket_id1: BucketId = 1;
		let bucket_id2: BucketId = 2;
		let bucket_id3: BucketId = 3;
		let bucket_id4: BucketId = 4;
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)

<<<<<<< HEAD
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			// should pass without debt
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};
		let usage2 = BucketUsage {
			// should fail as not enough balance
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage3 = BucketUsage {
			// should pass but with debt (partial charge)
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage4 = BucketUsage {
			// should pass without debt
			transferred_bytes: 467457,
			stored_bytes: 45674567456,
			number_of_puts: 3456345,
			number_of_gets: 242334563456423,
		};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD

<<<<<<< HEAD
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
<<<<<<< HEAD
		];
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
=======
		let payers1 =
			vec![(user2_debtor, bucket_id2, usage2.clone()), (user4, bucket_id4, usage4.clone())];
		let payers2 = vec![(user1, bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor, bucket_id3, usage3.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
=======

>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
		let payers1 = vec![
			(user2_debtor.clone(), node_id.clone(), bucket_id2, usage2.clone()),
			(user4.clone(), node_id.clone(), bucket_id4, usage4.clone()),
		];
<<<<<<< HEAD
		let payers2 = vec![(user1.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), bucket_id3, usage3.clone())];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let payers2 = vec![(user1.clone(), node_id.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), node_id, bucket_id3, usage3.clone())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		];
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers1 = vec![(bucket_id2, usage2.clone()), (bucket_id4, usage4.clone())];
		let payers2 = vec![(bucket_id1, usage1.clone())];
		let payers3 = vec![(bucket_id3, usage3.clone())];

>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (1.0 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers3_batch_root, _, _) = hash_bucket_payable_usage_batch(payers3.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root, payers3_batch_root]);

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));
		assert_eq!(System::events().len(), 3);

		// batch 1
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1,
=======
			payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers1,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let usage4_charge = calculate_charge_for_day(cluster_id, usage4.clone());
		let user2_debt = DdcPayouts::debtor_customers(cluster_id, user2_debtor.clone()).unwrap();
		let expected_charge2 = calculate_charge_for_day(cluster_id, usage2.clone());
		let mut debt = expected_charge2 - CUSTOMER2_BALANCE;
		assert_eq!(user2_debt, debt);

		let ratio = Perquintill::from_rational(CUSTOMER2_BALANCE, expected_charge2);
		let mut charge2 = calculate_charge_parts_for_day(cluster_id, usage2);
		charge2.storage = ratio * charge2.storage;
		charge2.transfer = ratio * charge2.transfer;
		charge2.gets = ratio * charge2.gets;
		charge2.puts = ratio * charge2.puts;

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge4 = calculate_charge_parts_for_day(cluster_id, usage4);
		assert_eq!(charge2.puts + charge4.puts, report.total_customer_charge.puts);
		assert_eq!(charge2.gets + charge4.gets, report.total_customer_charge.gets);
		assert_eq!(charge2.storage + charge4.storage, report.total_customer_charge.storage);
		assert_eq!(charge2.transfer + charge4.transfer, report.total_customer_charge.transfer);

		System::assert_has_event(
			Event::ChargeFailed {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user2_debtor.clone(),
=======
				customer_id: user2_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user2_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id2,
				batch_index,
				charged: CUSTOMER2_BALANCE,
				expected_to_charge: expected_charge2,
			}
			.into(),
		);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user2_debtor.clone(),
=======
				customer_id: user2_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user2_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id2,
				batch_index,
				amount: debt,
			}
			.into(),
		);
		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				customer_id: user4,
				bucket_id: bucket_id4,
				batch_index,
				amount: usage4_charge,
			}
			.into(),
		);

		assert_eq!(System::events().len(), 5 + 3 + 1); // 1 for Currency::transfer

		// batch 2
		let mut before_total_customer_charge = report.total_customer_charge;
		batch_index += 1;
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers2,
=======
			payers2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers2,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(1).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user1.clone(),
=======
				customer_id: user1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user1.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id1,
				amount: calculate_charge_for_day(cluster_id, usage1.clone()),
			}
			.into(),
		);

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge1 = calculate_charge_parts_for_day(cluster_id, usage1);
		assert_eq!(
			charge1.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			charge1.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			charge1.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			charge1.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		assert_eq!(report.state, PayoutState::ChargingCustomers);
		let user1_debt = DdcPayouts::debtor_customers(cluster_id, user1);
		assert_eq!(user1_debt, None);

		let balance_before = Balances::free_balance(DdcPayouts::account_id());

		// batch 3
		batch_index += 1;
		before_total_customer_charge = report.total_customer_charge.clone();
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers3,
=======
			payers3,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers3,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(2).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let user3_charge = calculate_charge_for_day(cluster_id, usage3.clone());
		let charge3 = calculate_charge_parts_for_day(cluster_id, usage3);
		let ratio = Perquintill::from_rational(PARTIAL_CHARGE, user3_charge);
		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(
			ratio * charge3.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			ratio * charge3.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			ratio * charge3.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			ratio * charge3.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		let balance = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance, balance_before + PARTIAL_CHARGE);

		let user3_debt = DdcPayouts::debtor_customers(cluster_id, user3_debtor.clone()).unwrap();
		debt = user3_charge - PARTIAL_CHARGE;
		assert_eq!(user3_debt, debt);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user3_debtor.clone(),
=======
				customer_id: user3_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user3_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id3,
				batch_index,
				amount: user3_debt,
			}
			.into(),
		);

		System::assert_last_event(
			Event::ChargeFailed {
				cluster_id,
				era,
				batch_index,
				customer_id: user3_debtor,
				bucket_id: bucket_id3,
				charged: PARTIAL_CHARGE,
				expected_to_charge: user3_charge,
			}
			.into(),
		);
	})
}

#[test]
fn send_charging_customers_batch_works_for_day_free_put() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
		let user2_debtor = AccountId::from([2; 32]);
		let user3_debtor = AccountId::from([3; 32]);
		let user4 = AccountId::from([4; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		let cluster_id = PUT_ZERO_CLUSTER_ID;
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let max_batch_index = 3;
		let mut batch_index = 0;
<<<<<<< HEAD
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
		let cluster_id = PUT_ZERO_CLUSTER_ID;
		let era = 100;
		let max_charging_batch_index = 2;
		let mut batch_index = 0;
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let bucket_id1: BucketId = BUCKET_ID1;
		let bucket_id2: BucketId = BUCKET_ID2;
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
<<<<<<< HEAD

		let usage1 = BucketUsage {
=======
		let bucket_id1: BucketId = 1;
		let bucket_id2: BucketId = 2;
		let bucket_id3: BucketId = 3;
		let bucket_id4: BucketId = 4;
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)

<<<<<<< HEAD
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			// should pass without debt
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};
		let usage2 = BucketUsage {
			// should fail as not enough balance
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage3 = BucketUsage {
			// should pass but with debt (partial charge)
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage4 = BucketUsage {
			// should pass without debt
			transferred_bytes: 467457,
			stored_bytes: 45674567456,
			number_of_puts: 3456345,
			number_of_gets: 242334563456423,
		};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD

<<<<<<< HEAD
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
<<<<<<< HEAD
		];
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
=======
		let payers1 =
			vec![(user2_debtor, bucket_id2, usage2.clone()), (user4, bucket_id4, usage4.clone())];
		let payers2 = vec![(user1, bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor, bucket_id3, usage3.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
=======

>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
		let payers1 = vec![
			(user2_debtor.clone(), node_id.clone(), bucket_id2, usage2.clone()),
			(user4.clone(), node_id.clone(), bucket_id4, usage4.clone()),
		];
<<<<<<< HEAD
		let payers2 = vec![(user1.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), bucket_id3, usage3.clone())];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let payers2 = vec![(user1.clone(), node_id.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), node_id, bucket_id3, usage3.clone())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		];
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers1 = vec![(bucket_id2, usage2.clone()), (bucket_id4, usage4.clone())];
		let payers2 = vec![(bucket_id1, usage1.clone())];
		let payers3 = vec![(bucket_id3, usage3.clone())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (1.0 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers3_batch_root, _, _) = hash_bucket_payable_usage_batch(payers3.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root, payers3_batch_root]);

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));
		assert_eq!(System::events().len(), 3);

		// batch 1
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1,
=======
			payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers1,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let usage4_charge = calculate_charge_for_day(cluster_id, usage4.clone());
		let user2_debt = DdcPayouts::debtor_customers(cluster_id, user2_debtor.clone()).unwrap();
		let expected_charge2 = calculate_charge_for_day(cluster_id, usage2.clone());
		let mut debt = expected_charge2 - CUSTOMER2_BALANCE;
		assert_eq!(user2_debt, debt);

		let ratio = Perquintill::from_rational(CUSTOMER2_BALANCE, expected_charge2);
		let mut charge2 = calculate_charge_parts_for_day(cluster_id, usage2);
		charge2.storage = ratio * charge2.storage;
		charge2.transfer = ratio * charge2.transfer;
		charge2.gets = ratio * charge2.gets;
		charge2.puts = ratio * charge2.puts;

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge4 = calculate_charge_parts_for_day(cluster_id, usage4);
		assert_eq!(charge2.puts + charge4.puts, report.total_customer_charge.puts);
		assert_eq!(charge2.gets + charge4.gets, report.total_customer_charge.gets);
		assert_eq!(charge2.storage + charge4.storage, report.total_customer_charge.storage);
		assert_eq!(charge2.transfer + charge4.transfer, report.total_customer_charge.transfer);

		System::assert_has_event(
			Event::ChargeFailed {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user2_debtor.clone(),
=======
				customer_id: user2_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user2_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id2,
				batch_index,
				charged: CUSTOMER2_BALANCE,
				expected_to_charge: expected_charge2,
			}
			.into(),
		);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
				customer_id: user2_debtor,
				bucket_id: bucket_id2,
				batch_index,
				amount: debt,
			}
			.into(),
		);
		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				customer_id: user4,
				bucket_id: bucket_id4,
				batch_index,
				amount: usage4_charge,
			}
			.into(),
		);

		assert_eq!(System::events().len(), 5 + 3 + 1); // 1 for Currency::transfer

		// batch 2
		let mut before_total_customer_charge = report.total_customer_charge;
		batch_index += 1;
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers2,
=======
			payers2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers2,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(1).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				batch_index,
				bucket_id: bucket_id1,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user1.clone(),
=======
				customer_id: user1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user1.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				amount: calculate_charge_for_day(cluster_id, usage1.clone()),
			}
			.into(),
		);

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge1 = calculate_charge_parts_for_day(cluster_id, usage1);
		assert_eq!(
			charge1.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			charge1.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			charge1.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			charge1.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		assert_eq!(report.state, PayoutState::ChargingCustomers);
		let user1_debt = DdcPayouts::debtor_customers(cluster_id, user1);
		assert_eq!(user1_debt, None);

		let balance_before = Balances::free_balance(DdcPayouts::account_id());

		// batch 3
		batch_index += 1;
		before_total_customer_charge = report.total_customer_charge.clone();
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers3,
=======
			payers3,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers3,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(2).unwrap().clone()
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let user3_charge = calculate_charge_for_day(cluster_id, usage3.clone());
		let charge3 = calculate_charge_parts_for_day(cluster_id, usage3);
		let ratio = Perquintill::from_rational(PARTIAL_CHARGE, user3_charge);
		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(
			ratio * charge3.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			ratio * charge3.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			ratio * charge3.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			ratio * charge3.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		let balance = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance, balance_before + PARTIAL_CHARGE);

		let user3_debt = DdcPayouts::debtor_customers(cluster_id, user3_debtor.clone()).unwrap();
		debt = user3_charge - PARTIAL_CHARGE;
		assert_eq!(user3_debt, debt);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user3_debtor.clone(),
=======
				customer_id: user3_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user3_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id3,
				batch_index,
				amount: user3_debt,
			}
			.into(),
		);

		System::assert_last_event(
			Event::ChargeFailed {
				cluster_id,
				era,
				batch_index,
				customer_id: user3_debtor,
				bucket_id: bucket_id3,
				charged: PARTIAL_CHARGE,
				expected_to_charge: user3_charge,
			}
			.into(),
		);
	})
}

#[test]
fn send_charging_customers_batch_works_for_day_free_storage_stream() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
		let user2_debtor = AccountId::from([2; 32]);
		let user3_debtor = AccountId::from([3; 32]);
		let user4 = AccountId::from([4; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		let cluster_id = STORAGE_STREAM_ZERO_CLUSTER_ID;
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let max_batch_index = 3;
		let mut batch_index = 0;
<<<<<<< HEAD
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let user1: AccountId = CUSTOMER1_KEY_32;
		let user2_debtor: AccountId = CUSTOMER2_KEY_32;
		let user3_debtor: AccountId = CUSTOMER3_KEY_32;
		let user4: AccountId = CUSTOMER4_KEY_32;
		let cluster_id = STORAGE_STREAM_ZERO_CLUSTER_ID;
		let era = 100;
		let max_charging_batch_index = 2;
		let mut batch_index = 0;
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let bucket_id1: BucketId = BUCKET_ID1;
		let bucket_id2: BucketId = BUCKET_ID2;
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
<<<<<<< HEAD

		let usage1 = BucketUsage {
=======
		let bucket_id1: BucketId = 1;
		let bucket_id2: BucketId = 2;
		let bucket_id3: BucketId = 3;
		let bucket_id4: BucketId = 4;
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)

<<<<<<< HEAD
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			// should pass without debt
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};
		let usage2 = BucketUsage {
			// should fail as not enough balance
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage3 = BucketUsage {
			// should pass but with debt (partial charge)
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage4 = BucketUsage {
			// should pass without debt
			transferred_bytes: 467457,
			stored_bytes: 45674567456,
			number_of_puts: 3456345,
			number_of_gets: 242334563456423,
		};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
		];
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
=======
		let payers1 =
			vec![(user2_debtor, bucket_id2, usage2.clone()), (user4, bucket_id4, usage4.clone())];
		let payers2 = vec![(user1, bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor, bucket_id3, usage3.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers1 = vec![
			(node_key.clone(), bucket_id2, usage2.clone()),
			(node_key.clone(), bucket_id4, usage4.clone()),
		];
<<<<<<< HEAD
<<<<<<< HEAD
		let payers2 = vec![(user1.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), bucket_id3, usage3.clone())];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let payers2 = vec![(user1.clone(), node_id.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(user3_debtor.clone(), node_id, bucket_id3, usage3.clone())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payers2 = vec![(node_key.clone(), bucket_id1, usage1.clone())];
		let payers3 = vec![(node_key.clone(), bucket_id3, usage3.clone())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers1 = vec![(bucket_id2, usage2.clone()), (bucket_id4, usage4.clone())];
		let payers2 = vec![(bucket_id1, usage1.clone())];
		let payers3 = vec![(bucket_id3, usage3.clone())];

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers3_batch_root, _, _) = hash_bucket_payable_usage_batch(payers3.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root, payers3_batch_root]);

>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (1.0 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();
		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));
		assert_eq!(System::events().len(), 3);

		// batch 1
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1,
=======
			payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers1,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let usage4_charge = calculate_charge_for_day(cluster_id, usage4.clone());
		let user2_debt = DdcPayouts::debtor_customers(cluster_id, user2_debtor.clone()).unwrap();
		let expected_charge2 = calculate_charge_for_day(cluster_id, usage2.clone());
		let mut debt = expected_charge2 - CUSTOMER2_BALANCE;
		assert_eq!(user2_debt, debt);

		let ratio = Perquintill::from_rational(CUSTOMER2_BALANCE, expected_charge2);
		let mut charge2 = calculate_charge_parts_for_day(cluster_id, usage2);
		charge2.storage = ratio * charge2.storage;
		charge2.transfer = ratio * charge2.transfer;
		charge2.gets = ratio * charge2.gets;
		charge2.puts = ratio * charge2.puts;

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge4 = calculate_charge_parts_for_day(cluster_id, usage4);
		assert_eq!(charge2.puts + charge4.puts, report.total_customer_charge.puts);
		assert_eq!(charge2.gets + charge4.gets, report.total_customer_charge.gets);
		assert_eq!(charge2.storage + charge4.storage, report.total_customer_charge.storage);
		assert_eq!(charge2.transfer + charge4.transfer, report.total_customer_charge.transfer);

		System::assert_has_event(
			Event::ChargeFailed {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user2_debtor.clone(),
=======
				customer_id: user2_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user2_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id2,
				batch_index,
				charged: CUSTOMER2_BALANCE,
				expected_to_charge: expected_charge2,
			}
			.into(),
		);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
				customer_id: user2_debtor,
				bucket_id: bucket_id2,
				batch_index,
				amount: debt,
			}
			.into(),
		);
		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				customer_id: user4,
				bucket_id: bucket_id4,
				batch_index,
				amount: usage4_charge,
			}
			.into(),
		);

		assert_eq!(System::events().len(), 5 + 3 + 1); // 1 for Currency::transfer

		// batch 2
		let mut before_total_customer_charge = report.total_customer_charge;
		batch_index += 1;
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers2,
=======
			payers2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers2,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(1).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				batch_index,
				bucket_id: bucket_id1,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user1.clone(),
=======
				customer_id: user1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user1.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				amount: calculate_charge_for_day(cluster_id, usage1.clone()),
			}
			.into(),
		);

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge1 = calculate_charge_parts_for_day(cluster_id, usage1);
		assert_eq!(
			charge1.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			charge1.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			charge1.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			charge1.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		assert_eq!(report.state, PayoutState::ChargingCustomers);
		let user1_debt = DdcPayouts::debtor_customers(cluster_id, user1);
		assert_eq!(user1_debt, None);

		let balance_before = Balances::free_balance(DdcPayouts::account_id());

		// batch 3
		batch_index += 1;
		before_total_customer_charge = report.total_customer_charge.clone();
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers3,
=======
			payers3,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers3,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(2).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let user3_charge = calculate_charge_for_day(cluster_id, usage3.clone());
		let charge3 = calculate_charge_parts_for_day(cluster_id, usage3);
		let ratio = Perquintill::from_rational(PARTIAL_CHARGE, user3_charge);
		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(
			ratio * charge3.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			ratio * charge3.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			ratio * charge3.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			ratio * charge3.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);

		let balance = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance, balance_before + PARTIAL_CHARGE);

		let user3_debt = DdcPayouts::debtor_customers(cluster_id, user3_debtor.clone()).unwrap();
		debt = user3_charge - PARTIAL_CHARGE;
		assert_eq!(user3_debt, debt);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				customer_id: user3_debtor.clone(),
=======
				customer_id: user3_debtor,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
				customer_id: user3_debtor.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				bucket_id: bucket_id3,
				batch_index,
				amount: user3_debt,
			}
			.into(),
		);

		System::assert_last_event(
			Event::ChargeFailed {
				cluster_id,
				era,
				batch_index,
				bucket_id: bucket_id3,
				customer_id: user3_debtor,
				charged: PARTIAL_CHARGE,
				expected_to_charge: user3_charge,
			}
			.into(),
		);
	})
}

#[test]
fn send_charging_customers_batch_works_zero_fees() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
<<<<<<< HEAD
<<<<<<< HEAD

<<<<<<< HEAD
=======
		let dac_account = AccountId::from([123; 32]);
		let user5 = AccountId::from([5; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		let cluster_id = ONE_CLUSTER_ID;
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
<<<<<<< HEAD
		let bucket_id5: BucketId = BUCKET_ID5;
		let usage5 = BucketUsage {
=======
		let bucket_id5: BucketId = 5;
=======
		let dac_account = AccountId::from([123; 32]);
=======

>>>>>>> fd642396 (chore: unused account is removed from tests)
		let cluster_id = ONE_CLUSTER_ID;
		let era = 100;
		let max_charging_batch_index = 0;
		let batch_index = 0;
		let bucket_id5: BucketId = BUCKET_ID5;
<<<<<<< HEAD
<<<<<<< HEAD
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let usage5 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage5 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
=======
		let usage1 = BucketUsage {
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			// should pass without debt
			transferred_bytes: 1024,
			stored_bytes: 1024,
			number_of_puts: 1,
			number_of_gets: 1,
		};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD

		let payers5 = vec![(node_key.clone(), bucket_id5, usage5.clone())];
=======
		let payers5 = vec![(user5, bucket_id5, usage5.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======

<<<<<<< HEAD
		let payers5 = vec![(user5, node_id.clone(), bucket_id5, usage5.clone())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payers5 = vec![(node_key.clone(), bucket_id5, usage5.clone())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers1 = vec![(bucket_id5, usage1.clone())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();
		let (_, payers_batch_proof, payers_root) = hash_bucket_payable_usage_batch(payers1.clone());

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));
		assert_eq!(System::events().len(), 3);

		// batch 1
		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let before_total_customer_charge = report.total_customer_charge;
		let balance_before = Balances::free_balance(DdcPayouts::account_id());
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers5,
=======
			payers5,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers5,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			&payers1,
			payers_batch_proof,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let usage5_charge = calculate_charge_for_month(cluster_id, usage1.clone());
		let charge5 = calculate_charge_parts_for_month(cluster_id, usage1);
		let balance = Balances::free_balance(DdcPayouts::account_id());
		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(balance, usage5_charge + balance_before);
		assert_eq!(
			charge5.puts + before_total_customer_charge.puts,
			report.total_customer_charge.puts
		);
		assert_eq!(
			charge5.gets + before_total_customer_charge.gets,
			report.total_customer_charge.gets
		);
		assert_eq!(
			charge5.storage + before_total_customer_charge.storage,
			report.total_customer_charge.storage
		);
		assert_eq!(
			charge5.transfer + before_total_customer_charge.transfer,
			report.total_customer_charge.transfer
		);
	})
}

#[test]
fn end_charging_customers_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let root_account = AccountId::from([100; 32]);
<<<<<<< HEAD
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		let cluster_id = ClusterId::from([12; 20]);
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let max_batch_index = 2;
		let batch_index = 1;
<<<<<<< HEAD
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
>>>>>>> f82a1179 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
<<<<<<< HEAD
		let max_batch_index = 2;
		let batch_index = 1;
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let bucket_id1: BucketId = BUCKET_ID1;

		let payers = vec![(node_key.clone(), bucket_id1, BucketUsage::default())];
<<<<<<< HEAD
=======
		let bucket_id1: BucketId = 1;
<<<<<<< HEAD
		let payers = vec![(user1, bucket_id1, CustomerUsage::default())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======

<<<<<<< HEAD
		let payers = vec![(user1, node_id.clone(), bucket_id1, CustomerUsage::default())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payers = vec![(node_key.clone(), bucket_id1, CustomerUsage::default())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
=======
		let max_charging_batch_index = 1;
		let bucket_id1: BucketId = BUCKET_ID1;
		let bucket_id2: BucketId = BUCKET_ID2;
		let payers1 = vec![(
			bucket_id1,
			BucketUsage {
				transferred_bytes: 1,
				stored_bytes: 1,
				number_of_gets: 1,
				number_of_puts: 1,
			},
		)];
		let payers2 = vec![(
			bucket_id2,
			BucketUsage {
				transferred_bytes: 2,
				stored_bytes: 2,
				number_of_gets: 2,
				number_of_puts: 2,
			},
		)];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,),
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
			DdcPayouts::end_charging_customers(
				RuntimeOrigin::signed(AccountId::from([0; 32])),
				cluster_id,
				era,
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::end_charging_customers(RuntimeOrigin::root(), cluster_id, era,),
			BadOrigin
		);

		assert_noop!(
			DdcPayouts::end_charging_customers(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,),
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,),
			Error::<Test>::BatchesMissed
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			DdcPayouts::end_charging_customers(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
			Error::<Test>::BatchesMissed
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers,
=======
			payers,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,),
			Error::<Test>::BatchesMissed
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
<<<<<<< HEAD
			batch_index,
			&payers,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			0,
			&payers1,
			payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,),
=======
			DdcPayouts::end_charging_customers(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
				cluster_id,
				era,
			),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,),
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			Error::<Test>::BatchesMissed
		);
	})
}

#[test]
fn end_charging_customers_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
		let user1 = CUSTOMER1_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		let cluster_id = ClusterId::from([12; 20]);
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
<<<<<<< HEAD
		let bucket_id1: BucketId = BUCKET_ID1;
		let usage1 = BucketUsage {
=======
		let bucket_id1: BucketId = 1;
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let user1 = CUSTOMER1_KEY_32;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_charging_batch_index = 0;
		let batch_index = 0;
		let bucket_id1: BucketId = BUCKET_ID1;
<<<<<<< HEAD
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let payers = vec![(node_key.clone(), bucket_id1, usage1.clone())];
=======
		let payers = vec![(user1, bucket_id1, usage1.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers = vec![(user1.clone(), bucket_id1, usage1.clone())];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let payers = vec![(user1.clone(), node_id.clone(), bucket_id1, usage1.clone())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payers = vec![(node_key.clone(), bucket_id1, usage1.clone())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers = vec![(bucket_id1, usage1.clone())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		let (payers_batch_root, _, _) = hash_bucket_payable_usage_batch(payers.clone());

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_batch_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_batch_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));

=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers,
=======
			payers,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
		));

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge = calculate_charge_for_month(cluster_id, usage1);
		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				batch_index,
				customer_id: user1,
				amount: charge,
				bucket_id: bucket_id1,
			}
			.into(),
		);

		let mut balance = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance - Balances::minimum_balance(), charge);
		assert_eq!(System::events().len(), 4 + 1); // 1 for Currency::transfer

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
			cluster_id,
			era,
		));
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

		System::assert_has_event(Event::ChargingFinished { cluster_id, era }.into());

		let treasury_fee = get_fees(&cluster_id).treasury_share * charge;
		let reserve_fee = get_fees(&cluster_id).cluster_reserve_share * charge;
		let validator_fee = get_fees(&cluster_id).validators_share * charge;

		System::assert_has_event(
			Event::TreasuryFeesCollected { cluster_id, era, amount: treasury_fee }.into(),
		);

		System::assert_has_event(
			Event::ClusterReserveFeesCollected { cluster_id, era, amount: reserve_fee }.into(),
		);

		System::assert_has_event(
			Event::ValidatorFeesCollected { cluster_id, era, amount: validator_fee }.into(),
		);

<<<<<<< HEAD
<<<<<<< HEAD
		let transfers = 3 + 3 + 3 * 3; // for Currency::transfer
<<<<<<< HEAD
		assert_eq!(System::events().len(), 7 + 1 + 3 + transfers);
=======
		let transfers = 3 + 3 + 3 + 3 * 3; // for Currency::transfer
		assert_eq!(System::events().len(), 5 + 1 + 3 + transfers);
>>>>>>> d97d5c7e (Feature/token utility events (#331))
=======
		let transfers = 3 + 3 + 3 * 3; // for Currency::transfer
<<<<<<< HEAD
		assert_eq!(System::events().len(), 8 + 1 + 3 + transfers);
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_eq!(System::events().len(), 7 + 1 + 3 + transfers);
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
=======
		assert_eq!(System::events().len(), 7 + 2 + 3 + transfers);
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report_after.state, PayoutState::CustomersChargedWithFees);

		let total_left_from_one = (get_fees(&cluster_id).treasury_share +
			get_fees(&cluster_id).validators_share +
			get_fees(&cluster_id).cluster_reserve_share)
			.left_from_one();

<<<<<<< HEAD
<<<<<<< HEAD
		balance = Balances::free_balance(AccountId::from(TREASURY_ACCOUNT_ID));
		let mut expected_fees = get_fees(&cluster_id).treasury_share * charge;
		assert_eq!(balance, expected_fees);

		balance = Balances::free_balance(AccountId::from(RESERVE_ACCOUNT_ID));
=======
		balance = Balances::free_balance(TREASURY_ACCOUNT_ID);
		let mut expected_fees = get_fees(&cluster_id).treasury_share * charge;
		assert_eq!(balance, expected_fees);

		balance = Balances::free_balance(RESERVE_ACCOUNT_ID);
>>>>>>> d97d5c7e (Feature/token utility events (#331))
=======
		balance = Balances::free_balance(AccountId::from(TREASURY_ACCOUNT_ID));
		let mut expected_fees = get_fees(&cluster_id).treasury_share * charge;
		assert_eq!(balance, expected_fees);

		balance = Balances::free_balance(AccountId::from(RESERVE_ACCOUNT_ID));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		expected_fees = get_fees(&cluster_id).cluster_reserve_share * charge;
		assert_eq!(balance, expected_fees);

		balance = Balances::free_balance(AccountId::from(VALIDATOR1_ACCOUNT_ID));
		let mut ratio = Perquintill::from_rational(
			VALIDATOR1_SCORE,
			VALIDATOR1_SCORE + VALIDATOR2_SCORE + VALIDATOR3_SCORE,
		);
		expected_fees = get_fees(&cluster_id).validators_share * ratio * charge;
		assert_eq!(balance, expected_fees);
		System::assert_has_event(
			Event::ValidatorRewarded {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				validator_id: AccountId::from(VALIDATOR1_ACCOUNT_ID),
=======
				validator_id: VALIDATOR1_ACCOUNT_ID,
>>>>>>> d97d5c7e (Feature/token utility events (#331))
=======
				validator_id: AccountId::from(VALIDATOR1_ACCOUNT_ID),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				amount: expected_fees,
			}
			.into(),
		);

		balance = Balances::free_balance(AccountId::from(VALIDATOR2_ACCOUNT_ID));
		ratio = Perquintill::from_rational(
			VALIDATOR2_SCORE,
			VALIDATOR1_SCORE + VALIDATOR2_SCORE + VALIDATOR3_SCORE,
		);
		expected_fees = get_fees(&cluster_id).validators_share * ratio * charge;
		assert_eq!(balance, expected_fees);
		System::assert_has_event(
			Event::ValidatorRewarded {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				validator_id: AccountId::from(VALIDATOR2_ACCOUNT_ID),
=======
				validator_id: VALIDATOR2_ACCOUNT_ID,
>>>>>>> d97d5c7e (Feature/token utility events (#331))
=======
				validator_id: AccountId::from(VALIDATOR2_ACCOUNT_ID),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				amount: expected_fees,
			}
			.into(),
		);

		balance = Balances::free_balance(AccountId::from(VALIDATOR3_ACCOUNT_ID));
		ratio = Perquintill::from_rational(
			VALIDATOR3_SCORE,
			VALIDATOR1_SCORE + VALIDATOR2_SCORE + VALIDATOR3_SCORE,
		);
		expected_fees = get_fees(&cluster_id).validators_share * ratio * charge;
		assert_eq!(balance, expected_fees);
		System::assert_has_event(
			Event::ValidatorRewarded {
				cluster_id,
				era,
<<<<<<< HEAD
<<<<<<< HEAD
				validator_id: AccountId::from(VALIDATOR3_ACCOUNT_ID),
=======
				validator_id: VALIDATOR3_ACCOUNT_ID,
>>>>>>> d97d5c7e (Feature/token utility events (#331))
=======
				validator_id: AccountId::from(VALIDATOR3_ACCOUNT_ID),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
				amount: expected_fees,
			}
			.into(),
		);

		assert_eq!(
			report_after.total_customer_charge.transfer,
			total_left_from_one * report_before.total_customer_charge.transfer
		);
		assert_eq!(
			report_after.total_customer_charge.storage,
			total_left_from_one * report_before.total_customer_charge.storage
		);
		assert_eq!(
			report_after.total_customer_charge.puts,
			total_left_from_one * report_before.total_customer_charge.puts
		);
		assert_eq!(
			report_after.total_customer_charge.gets,
			total_left_from_one * report_before.total_customer_charge.gets
		);
	})
}

#[test]
fn end_charging_customers_works_zero_fees() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
		let user1: AccountId = CUSTOMER1_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
		let cluster_id = ClusterId::zero();
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
<<<<<<< HEAD
		let bucket_id1: BucketId = BUCKET_ID1;
		let usage1 = BucketUsage {
=======
		let bucket_id1: BucketId = 1;
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let user1: AccountId = CUSTOMER1_KEY_32;
		let cluster_id = ClusterId::zero();
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
		let bucket_id1: BucketId = BUCKET_ID1;
<<<<<<< HEAD
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 1,
			number_of_gets: 1,
		};
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let payers = vec![(node_key.clone(), bucket_id1, usage1.clone())];
=======
		let payers = vec![(user1, bucket_id1, usage1.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers = vec![(user1.clone(), bucket_id1, usage1.clone())];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let payers = vec![(user1.clone(), node_id.clone(), bucket_id1, usage1.clone())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payers = vec![(node_key.clone(), bucket_id1, usage1.clone())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers = vec![(bucket_id1, usage1.clone())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		let (payers_batch_root, _, _) = hash_bucket_payable_usage_batch(payers.clone());

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_batch_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_batch_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));

=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_batch_index,
		));

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers,
=======
			payers,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
		));

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge = calculate_charge_for_month(cluster_id, usage1);
		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				customer_id: user1,
				bucket_id: bucket_id1,
				batch_index,
				amount: charge,
			}
			.into(),
		);

		let mut balance = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance - Balances::minimum_balance(), charge);
		assert_eq!(System::events().len(), 4 + 1); // 1 for Currency::transfer

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
			cluster_id,
			era,
		));
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

		System::assert_has_event(Event::ChargingFinished { cluster_id, era }.into());
		assert_eq!(System::events().len(), 5 + 1);

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report_after.state, PayoutState::CustomersChargedWithFees);

		let fees = get_fees(&cluster_id);

		let total_left_from_one =
			(fees.treasury_share + fees.validators_share + fees.cluster_reserve_share)
				.left_from_one();

		assert_eq!(total_left_from_one, Perquintill::one());

		assert_eq!(fees.treasury_share, Perquintill::zero());
		assert_eq!(fees.validators_share, Perquintill::zero());
		assert_eq!(fees.cluster_reserve_share, Perquintill::zero());

		balance = Balances::free_balance(AccountId::from(TREASURY_ACCOUNT_ID));
		assert_eq!(balance, 0);

		balance = Balances::free_balance(AccountId::from(RESERVE_ACCOUNT_ID));
		assert_eq!(balance, 0);

		balance = Balances::free_balance(AccountId::from(VALIDATOR1_ACCOUNT_ID));
		assert_eq!(balance, 0);

		balance = Balances::free_balance(AccountId::from(VALIDATOR2_ACCOUNT_ID));
		assert_eq!(balance, 0);

		balance = Balances::free_balance(AccountId::from(VALIDATOR3_ACCOUNT_ID));
		assert_eq!(balance, 0);

		assert_eq!(
			report_after.total_customer_charge.transfer,
			total_left_from_one * report_before.total_customer_charge.transfer
		);
		assert_eq!(
			report_after.total_customer_charge.storage,
			total_left_from_one * report_before.total_customer_charge.storage
		);
		assert_eq!(
			report_after.total_customer_charge.puts,
			total_left_from_one * report_before.total_customer_charge.puts
		);
		assert_eq!(
			report_after.total_customer_charge.gets,
			total_left_from_one * report_before.total_customer_charge.gets
		);
	})
}

#[test]
fn begin_rewarding_providers_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let root_account = AccountId::from([1; 32]);
<<<<<<< HEAD
		let dac_account = AccountId::from([2; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([3; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
>>>>>>> f82a1179 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
<<<<<<< HEAD
		let max_batch_index = 2;
		let batch_index = 1;
<<<<<<< HEAD
<<<<<<< HEAD
		let bucket_id1: BucketId = BUCKET_ID1;
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
		let payers = vec![(node_key.clone(), bucket_id1, BucketUsage::default())];
<<<<<<< HEAD
=======
		let bucket_id1: BucketId = 1;
<<<<<<< HEAD
		let payers = vec![(user1, bucket_id1, CustomerUsage::default())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
		let payers = vec![(user1, node_id.clone(), bucket_id1, CustomerUsage::default())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
=======
		let bucket_id1: BucketId = BUCKET_ID1;
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let payers = vec![(node_key.clone(), bucket_id1, CustomerUsage::default())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
		let node_usage = NodeUsage::default();
=======
		let max_batch_index = 0;
		let batch_index = 0;
		let bucket_id1: BucketId = BUCKET_ID1;
		let payers = vec![(bucket_id1, BucketUsage::default())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		let (_, payers_batch_proof, payers_root) = hash_bucket_payable_usage_batch(payers.clone());

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
			DdcPayouts::begin_rewarding_providers(
				RuntimeOrigin::signed(AccountId::from([0; 32])),
				cluster_id,
				era,
				max_batch_index,
				node_usage.clone(),
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::begin_rewarding_providers(
				RuntimeOrigin::root(),
				cluster_id,
				era,
				max_batch_index,
				node_usage.clone(),
			),
			BadOrigin
		);

		assert_noop!(
			DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				cluster_id,
				era,
				max_batch_index,
			),
			Error::<Test>::BillingReportDoesNotExist
		);

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_noop!(
			DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				cluster_id,
				era,
				max_batch_index,
			),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_batch_index,
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
			DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				cluster_id,
				era,
				max_batch_index,
			),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers,
=======
			payers.clone(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_batch_proof.clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
			DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				cluster_id,
				era,
				max_batch_index,
			),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index + 1,
<<<<<<< HEAD
			&payers,
=======
			payers,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index + 1,
			&payers,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
		));

=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
			DdcPayouts::begin_rewarding_providers(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				cluster_id,
				era,
				max_batch_index,
			),
			Error::<Test>::NotExpectedState
		);
	})
}

#[test]
fn begin_rewarding_providers_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

<<<<<<< HEAD
<<<<<<< HEAD
=======
		let dac_account = AccountId::from([123; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_charging_batch_index = 0;
		let max_rewarding_batch_index = 0;
		let batch_index = 0;
<<<<<<< HEAD
<<<<<<< HEAD
		let bucket_id1: BucketId = BUCKET_ID1;
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
		let total_node_usage = NodeUsage::default();
		let payers = vec![(node_key.clone(), bucket_id1, BucketUsage::default())];
<<<<<<< HEAD
=======
		let bucket_id1: BucketId = 1;
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
=======
		let bucket_id1: BucketId = BUCKET_ID1;
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let total_node_usage = NodeUsage::default();
<<<<<<< HEAD
<<<<<<< HEAD
		let payers = vec![(user1, bucket_id1, CustomerUsage::default())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers = vec![(user1, node_id.clone(), bucket_id1, CustomerUsage::default())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payers = vec![(node_key.clone(), bucket_id1, CustomerUsage::default())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
=======
		let payers = vec![(bucket_id1, BucketUsage::default())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		let (_, payers_batch_proof, payers_root) = hash_bucket_payable_usage_batch(payers.clone());

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage,
		));

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
<<<<<<< HEAD
			cluster_id, era, start_era, end_era,
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));

=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
=======
			cluster_id,
			era,
			fingerprint
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, PayoutState::Initialized);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers,
=======
			payers,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
			MMRProof::default(),
		));

<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
		));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))

<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
		assert_ok!(DdcPayouts::begin_rewarding_providers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers,
			payers_batch_proof,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_rewarding_batch_index,
		));

		System::assert_last_event(Event::RewardingStarted { cluster_id, era }.into());

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, PayoutState::RewardingProviders);
	})
}

#[test]
fn send_rewarding_providers_batch_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let root_account = AccountId::from([1; 32]);
<<<<<<< HEAD
		let dac_account = AccountId::from([2; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([3; 32]);
		let user2 = AccountId::from([4; 32]);
<<<<<<< HEAD
		let node1 = AccountId::from([33; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let cluster_id = ClusterId::from([12; 20]);
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
<<<<<<< HEAD
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			format!("0x{}", "302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let era = 100;
		let max_batch_index = 1;
		let batch_index = 0;
<<<<<<< HEAD
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
		let payers1 = vec![(node_key.clone(), bucket_id3, BucketUsage::default())];
		let payers2 = vec![(node_key.clone(), bucket_id4, BucketUsage::default())];

		let payees = vec![(node_key, NodeUsage::default())];
=======
		let bucket_id1: BucketId = 1;
		let bucket_id2: BucketId = 2;
<<<<<<< HEAD
		let payers1 = vec![(user1, bucket_id1, CustomerUsage::default())];
		let payers2 = vec![(user2, bucket_id2, CustomerUsage::default())];
		let payees = vec![(node1, NodeUsage::default())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======

		let payers1 = vec![(user1, node_id.clone(), bucket_id1, CustomerUsage::default())];
		let payers2 = vec![(user2, node_id.clone(), bucket_id2, CustomerUsage::default())];
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
>>>>>>> f82a1179 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
		let era = 100;
		let max_charging_batch_index = 1;
		let batch_index = 0;
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
<<<<<<< HEAD
<<<<<<< HEAD
		let payers1 = vec![(node_key.clone(), bucket_id3, CustomerUsage::default())];
		let payers2 = vec![(node_key.clone(), bucket_id4, CustomerUsage::default())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers1 = vec![(node_key.clone(), bucket_id3, BucketUsage::default())];
		let payers2 = vec![(node_key.clone(), bucket_id4, BucketUsage::default())];
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')

<<<<<<< HEAD
		let payees = vec![(node1, node_id, NodeUsage::default())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payees = vec![(node_key, NodeUsage::default())];
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
=======
		let payers1 = vec![(bucket_id3, BucketUsage::default())];
		let payers2 = vec![(bucket_id4, BucketUsage::default())];

		let payees1 = vec![(node_key, NodeUsage::default())];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		let (_, payees1_batch_proof, payees_root) = hash_node_payable_usage_batch(payees1.clone());

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			&cluster_usage,
		);

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(AccountId::from([0; 32])),
>>>>>>> f82a1179 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
=======
				payees.clone(),
				MMRProof::default(),
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::root(),
				cluster_id,
				era,
				batch_index,
				payees.clone(),
				MMRProof::default(),
			),
			BadOrigin
		);

		assert_noop!(
			DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
				batch_index,
				payees.clone(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
=======
				&payees1,
				payees1_batch_proof.clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			),
			Error::<Test>::BillingReportDoesNotExist
		);

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			cluster_id,
			era,
			start_era,
			end_era,
<<<<<<< HEAD
		));

		assert_noop!(
			DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
=======
				payees.clone(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
=======
			payers_root,
			payees_root,
			cluster_usage,
		));

>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
=======
				&payees1,
				payees1_batch_proof.clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
=======
				payees.clone(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
=======
				&payees1,
				payees1_batch_proof.clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1,
=======
			payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			0,
			&payers1,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
=======
				payees.clone(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
=======
				&payees1,
				payees1_batch_proof.clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index + 1,
<<<<<<< HEAD
			&payers2,
=======
			payers2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			1,
			&payers2,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(1).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
=======
				payees.clone(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
=======
				&payees1,
				payees1_batch_proof.clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
		));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))

		assert_noop!(
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
=======
				payees,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				batch_index,
<<<<<<< HEAD
				&payees,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
=======
				&payees1,
				payees1_batch_proof,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			),
			Error::<Test>::NotExpectedState
		);
	})
}

#[test]
fn send_rewarding_providers_batch_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======

		let dac_account = AccountId::from([123; 32]);
		let user1 = AccountId::from([1; 32]);
<<<<<<< HEAD
		let node1 = AccountId::from([10; 32]);
		let node2 = AccountId::from([11; 32]);
		let node3 = AccountId::from([12; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let cluster_id = ClusterId::from([12; 20]);
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let dac_account = AccountId::from([123; 32]);
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let era = 100;
		let max_charging_batch_index = 0;
		let max_node_batch_index = 1;
		let batch_node_index = 0;
		let bucket_id1: BucketId = 1;
<<<<<<< HEAD
<<<<<<< HEAD
		let usage1 = BucketUsage {
=======
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};

		let node_usage1 = NodeUsage {
			// Storage 1
			transferred_bytes: usage1.transferred_bytes * 2 / 3,
			stored_bytes: 0,
			number_of_puts: usage1.number_of_puts * 2 / 3,
			number_of_gets: usage1.number_of_gets * 2 / 3,
		};

		let node_usage2 = NodeUsage {
			// Storage 2
			transferred_bytes: 0,
			stored_bytes: usage1.stored_bytes * 2,
			number_of_puts: 0,
			number_of_gets: 0,
		};

		let node_usage3 = NodeUsage {
			// Storage 1 + Storage 2
			transferred_bytes: usage1.transferred_bytes * 2,
			stored_bytes: usage1.stored_bytes * 3,
			number_of_puts: usage1.number_of_puts * 2,
			number_of_gets: usage1.number_of_gets * 2,
		};

		let cluster_usage = NodeUsage {
			transferred_bytes: node_usage1.transferred_bytes +
				node_usage2.transferred_bytes +
				node_usage3.transferred_bytes,
			stored_bytes: node_usage1.stored_bytes +
				node_usage2.stored_bytes +
				node_usage3.stored_bytes,
			number_of_puts: node_usage1.number_of_puts +
				node_usage2.number_of_puts +
				node_usage3.number_of_puts,
			number_of_gets: node_usage1.number_of_gets +
				node_usage2.number_of_gets +
				node_usage3.number_of_gets,
		};

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let payers = vec![(node_key.clone(), bucket_id1, usage1)];
=======
		let payers = vec![(bucket_id1, usage1)];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let payees1 = vec![
			(NodePubKey::StoragePubKey(NODE1_PUB_KEY_32.clone()), node_usage1.clone()),
			(NodePubKey::StoragePubKey(NODE2_PUB_KEY_32.clone()), node_usage2.clone()),
		];
		let payees2 =
			vec![(NodePubKey::StoragePubKey(NODE3_PUB_KEY_32.clone()), node_usage3.clone())];

=======
		let payers = vec![(user1, bucket_id1, usage1)];
<<<<<<< HEAD
		let payees1 = vec![(node1, node_usage1.clone()), (node2, node_usage2.clone())];
		let payees2 = vec![(node3, node_usage3.clone())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payees1 =
			vec![(node1.clone(), node_usage1.clone()), (node2.clone(), node_usage2.clone())];
		let payees2 = vec![(node3.clone(), node_usage3.clone())];
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let payers = vec![(user1, node_id.clone(), bucket_id1, usage1)];
		let payees1 = vec![
			(NodePubKey::StoragePubKey(NODE1_PUB_KEY_32.clone()), node_usage1.clone()),
			(NodePubKey::StoragePubKey(NODE2_PUB_KEY_32.clone()), node_usage2.clone()),
		];
<<<<<<< HEAD
		let payees2 = vec![(node3.clone(), node_id, node_usage3.clone())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payees2 =
			vec![(NodePubKey::StoragePubKey(NODE3_PUB_KEY_32.clone()), node_usage3.clone())];

>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let (_, payers1_batch_proof, payers_root) = hash_bucket_payable_usage_batch(payers.clone());

		let (payees1_batch_root, _, _) = hash_node_payable_usage_batch(payees1.clone());
		let (payees2_batch_root, _, _) = hash_node_payable_usage_batch(payees2.clone());
		let (payees_root, payees_proofs) =
			get_root_with_proofs(vec![payees1_batch_root, payees2_batch_root]);

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			cluster_usage.clone(),
		));

>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));

=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers,
=======
			payers,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			0,
			&payers,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers1_batch_proof,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
		));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let total_left_from_one = (get_fees(&cluster_id).treasury_share +
			get_fees(&cluster_id).validators_share +
			get_fees(&cluster_id).cluster_reserve_share)
			.left_from_one();

		assert_eq!(
			report_after.total_customer_charge.transfer,
			total_left_from_one * report_before.total_customer_charge.transfer
		);
		assert_eq!(
			report_after.total_customer_charge.storage,
			total_left_from_one * report_before.total_customer_charge.storage
		);
		assert_eq!(
			report_after.total_customer_charge.puts,
			total_left_from_one * report_before.total_customer_charge.puts
		);
		assert_eq!(
			report_after.total_customer_charge.gets,
			total_left_from_one * report_before.total_customer_charge.gets
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
		assert_ok!(DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_node_batch_index,
		));

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_node_index,
<<<<<<< HEAD
			&payees1,
=======
			payees1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			0,
			&payees1,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payees_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let ratio1_transfer = Perquintill::from_rational(
			node_usage1.transferred_bytes,
			cluster_usage.transferred_bytes,
		);
		let mut transfer_charge = ratio1_transfer * report_after.total_customer_charge.transfer;

		let ratio1_storage = Perquintill::from_rational(
			node_usage1.stored_bytes as u64,
			cluster_usage.stored_bytes as u64,
		);
		let mut storage_charge = ratio1_storage * report_after.total_customer_charge.storage;

		let ratio1_puts =
			Perquintill::from_rational(node_usage1.number_of_puts, cluster_usage.number_of_puts);
		let mut puts_charge = ratio1_puts * report_after.total_customer_charge.puts;

		let ratio1_gets =
			Perquintill::from_rational(node_usage1.number_of_gets, cluster_usage.number_of_gets);
		let mut gets_charge = ratio1_gets * report_after.total_customer_charge.gets;

<<<<<<< HEAD
<<<<<<< HEAD
		let balance_node1 = Balances::free_balance(NODE_PROVIDER1_KEY_32.clone());
=======
		let balance_node1 = Balances::free_balance(node1.clone());
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let balance_node1 = Balances::free_balance(NODE_PROVIDER1_KEY_32.clone());
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		assert_eq!(balance_node1, transfer_charge + storage_charge + puts_charge + gets_charge);
		let mut report_reward = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();

		System::assert_has_event(
			Event::Rewarded {
				cluster_id,
				era,
				node_provider_id: NODE_PROVIDER1_KEY_32.clone(),
				batch_index: batch_node_index,
				rewarded: balance_node1,
				expected_to_reward: balance_node1,
			}
			.into(),
		);

		let ratio2_transfer = Perquintill::from_rational(
			node_usage2.transferred_bytes,
			cluster_usage.transferred_bytes,
		);
		transfer_charge = ratio2_transfer * report_after.total_customer_charge.transfer;

		let ratio2_storage = Perquintill::from_rational(
			node_usage2.stored_bytes as u64,
			cluster_usage.stored_bytes as u64,
		);
		storage_charge = ratio2_storage * report_after.total_customer_charge.storage;

		let ratio2_puts =
			Perquintill::from_rational(node_usage2.number_of_puts, cluster_usage.number_of_puts);
		puts_charge = ratio2_puts * report_after.total_customer_charge.puts;

		let ratio2_gets =
			Perquintill::from_rational(node_usage2.number_of_gets, cluster_usage.number_of_gets);
		gets_charge = ratio2_gets * report_after.total_customer_charge.gets;

<<<<<<< HEAD
<<<<<<< HEAD
		let balance_node2 = Balances::free_balance(NODE_PROVIDER2_KEY_32.clone());
=======
		let balance_node2 = Balances::free_balance(node2.clone());
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let balance_node2 = Balances::free_balance(NODE_PROVIDER2_KEY_32.clone());
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		assert_eq!(balance_node2, transfer_charge + storage_charge + puts_charge + gets_charge);
		assert_eq!(report_reward.total_distributed_reward, balance_node1 + balance_node2);

		System::assert_has_event(
			Event::Rewarded {
				cluster_id,
				era,
				node_provider_id: NODE_PROVIDER2_KEY_32,
				batch_index: batch_node_index,
				rewarded: balance_node2,
				expected_to_reward: balance_node2,
			}
			.into(),
		);

		// batch 2
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_node_index + 1,
<<<<<<< HEAD
			&payees2,
=======
			payees2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			1,
			&payees2,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payees_proofs.get(1).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let ratio3_transfer = Perquintill::from_rational(
			node_usage3.transferred_bytes,
			cluster_usage.transferred_bytes,
		);
		transfer_charge = ratio3_transfer * report_after.total_customer_charge.transfer;

		let ratio3_storage = Perquintill::from_rational(
			node_usage3.stored_bytes as u64,
			cluster_usage.stored_bytes as u64,
		);
		storage_charge = ratio3_storage * report_after.total_customer_charge.storage;

		let ratio3_puts =
			Perquintill::from_rational(node_usage3.number_of_puts, cluster_usage.number_of_puts);
		puts_charge = ratio3_puts * report_after.total_customer_charge.puts;

		let ratio3_gets =
			Perquintill::from_rational(node_usage3.number_of_gets, cluster_usage.number_of_gets);
		gets_charge = ratio3_gets * report_after.total_customer_charge.gets;

		report_reward = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
<<<<<<< HEAD
<<<<<<< HEAD
		let balance_node3 = Balances::free_balance(NODE_PROVIDER3_KEY_32.clone());
=======
		let balance_node3 = Balances::free_balance(node3.clone());
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let balance_node3 = Balances::free_balance(NODE_PROVIDER3_KEY_32.clone());
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		assert_eq!(balance_node3, transfer_charge + storage_charge + puts_charge + gets_charge);

		System::assert_has_event(
			Event::Rewarded {
				cluster_id,
				era,
				node_provider_id: NODE_PROVIDER3_KEY_32.clone(),
				batch_index: batch_node_index + 1,
				rewarded: balance_node3,
				expected_to_reward: balance_node3,
			}
			.into(),
		);

		assert_eq!(
			report_reward.total_distributed_reward,
			balance_node1 + balance_node2 + balance_node3
		);

		let expected_amount_to_reward = report_reward.total_customer_charge.transfer +
			report_reward.total_customer_charge.storage +
			report_reward.total_customer_charge.puts +
			report_reward.total_customer_charge.gets;

		assert!(expected_amount_to_reward - report_reward.total_distributed_reward <= 20000);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,)
		);
=======
		assert_ok!(DdcPayouts::end_rewarding_providers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
			cluster_id,
			era,
		));
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,)
		);
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
	})
}

#[test]
fn send_rewarding_providers_batch_100_nodes_small_usage_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let num_nodes = 10;
		let num_users = 5;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let bank: AccountId = BANK_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let bank = AccountId::from([1; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let dac_account = 123u128;
<<<<<<< HEAD
>>>>>>> 5f953312 (bump dependnecies and fix implementation)
=======
		let bank = 1u128;
>>>>>>> e1534a4c (fix tests)
=======
		let bank: AccountId = BANK_KEY_32;
>>>>>>> c0869191 (chore: unified faucet account in tests)
		let cluster_id = ONE_CLUSTER_ID;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let nodes_keys = [
			NODE1_PUB_KEY_32,
			NODE2_PUB_KEY_32,
			NODE3_PUB_KEY_32,
			NODE4_PUB_KEY_32,
			NODE5_PUB_KEY_32,
			NODE6_PUB_KEY_32,
			NODE7_PUB_KEY_32,
			NODE8_PUB_KEY_32,
			NODE9_PUB_KEY_32,
			NODE10_PUB_KEY_32,
		];
		let storage_nodes: Vec<NodePubKey> =
			nodes_keys.iter().map(|key| NodePubKey::StoragePubKey(key.clone())).collect();
		let providers_accounts = [
			NODE_PROVIDER1_KEY_32,
			NODE_PROVIDER2_KEY_32,
			NODE_PROVIDER3_KEY_32,
			NODE_PROVIDER4_KEY_32,
			NODE_PROVIDER5_KEY_32,
			NODE_PROVIDER6_KEY_32,
			NODE_PROVIDER7_KEY_32,
			NODE_PROVIDER8_KEY_32,
			NODE_PROVIDER9_KEY_32,
			NODE_PROVIDER10_KEY_32,
		];
		let customers_accounts = [
			CUSTOMER100_KEY_32,
			CUSTOMER101_KEY_32,
			CUSTOMER102_KEY_32,
			CUSTOMER103_KEY_32,
			CUSTOMER104_KEY_32,
		];
<<<<<<< HEAD
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let nodes_keys = [
			NODE1_PUB_KEY_32,
			NODE2_PUB_KEY_32,
			NODE3_PUB_KEY_32,
			NODE4_PUB_KEY_32,
			NODE5_PUB_KEY_32,
			NODE6_PUB_KEY_32,
			NODE7_PUB_KEY_32,
			NODE8_PUB_KEY_32,
			NODE9_PUB_KEY_32,
			NODE10_PUB_KEY_32,
		];
		let storage_nodes: Vec<NodePubKey> =
			nodes_keys.iter().map(|key| NodePubKey::StoragePubKey(key.clone())).collect();
		let providers_accounts = [
			NODE_PROVIDER1_KEY_32,
			NODE_PROVIDER2_KEY_32,
			NODE_PROVIDER3_KEY_32,
			NODE_PROVIDER4_KEY_32,
			NODE_PROVIDER5_KEY_32,
			NODE_PROVIDER6_KEY_32,
			NODE_PROVIDER7_KEY_32,
			NODE_PROVIDER8_KEY_32,
			NODE_PROVIDER9_KEY_32,
			NODE_PROVIDER10_KEY_32,
		];
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let user_batch_size = 10;
		let node_batch_size = 10;
		let mut batch_user_index = 0;
		let mut batch_node_index = 0;
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let usage1 = BucketUsage {
			transferred_bytes: 1024,
			stored_bytes: 1024,
			number_of_puts: 1,
			number_of_gets: 1,
		};

		let node_usage1 = NodeUsage {
			// CDN
			transferred_bytes: Perquintill::from_float(0.75) * usage1.transferred_bytes,
			stored_bytes: 0,
			number_of_puts: Perquintill::from_float(0.75) * usage1.number_of_puts,
			number_of_gets: Perquintill::from_float(0.75) * usage1.number_of_gets,
		};

		let node_usage2 = NodeUsage {
			// Storage
			transferred_bytes: 0,
			stored_bytes: usage1.stored_bytes * 2,
			number_of_puts: 0,
			number_of_gets: 0,
		};

		let node_usage3 = NodeUsage {
			// CDN + Storage
			transferred_bytes: usage1.transferred_bytes * 2,
			stored_bytes: usage1.stored_bytes * 3,
			number_of_puts: usage1.number_of_puts * 2,
			number_of_gets: usage1.number_of_gets * 2,
		};

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let mut payees: Vec<Vec<(NodePubKey, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(NodePubKey, NodeUsage)> = Vec::new();
<<<<<<< HEAD
=======
		let mut payees: Vec<Vec<(AccountId, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(AccountId, NodeUsage)> = Vec::new();
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let mut payees: Vec<Vec<(AccountId, String, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(AccountId, String, NodeUsage)> = Vec::new();
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let mut payees: Vec<Vec<(NodePubKey, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(NodePubKey, NodeUsage)> = Vec::new();
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let mut total_nodes_usage = NodeUsage::default();
=======
		let mut cluster_usage = NodeUsage::default();
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		for i in 10..10 + num_nodes {
			let node_usage = match i % 3 {
				0 => node_usage1.clone(),
				1 => node_usage2.clone(),
				2 => node_usage3.clone(),
				_ => unreachable!(),
			};
			cluster_usage.transferred_bytes += node_usage.transferred_bytes;
			cluster_usage.stored_bytes += node_usage.stored_bytes;
			cluster_usage.number_of_puts += node_usage.number_of_puts;
			cluster_usage.number_of_gets += node_usage.number_of_gets;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			let node_key = storage_nodes[i - num_nodes].clone();
			node_batch.push((node_key.clone(), node_usage));
=======
			node_batch.push((AccountId::from([i; 32]), node_usage));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			node_batch.push((AccountId::from([i; 32]), node_id.clone(), node_usage));
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			let node_key = storage_nodes[i - num_nodes].clone();
			node_batch.push((node_key.clone(), node_usage));
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
			if node_batch.len() == node_batch_size {
				payees.push(node_batch.clone());
				node_batch.clear();
			}
		}
		if !node_batch.is_empty() {
			payees.push(node_batch.clone());
		}

		let mut payees_batch_roots = vec![];
		for batch in payees.iter() {
			let (payers_batch_root, _, _) = hash_node_payable_usage_batch(batch.clone());
			payees_batch_roots.push(payers_batch_root);
		}
		let (payees_root, payees_proofs) = get_root_with_proofs(payees_batch_roots);

		let mut total_charge = 0u128;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let mut payers: Vec<Vec<(NodePubKey, BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, BucketUsage)> = Vec::new();
=======
		let mut payers: Vec<Vec<(BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(BucketId, BucketUsage)> = Vec::new();
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		for user_id in 100..100 + num_users {
=======
		let mut payers: Vec<Vec<(u128, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(u128, BucketId, CustomerUsage)> = Vec::new();
		for user_id in 1000..1000 + num_users {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let mut payers: Vec<Vec<(AccountId, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(AccountId, BucketId, CustomerUsage)> = Vec::new();
=======
		let mut payers: Vec<Vec<(AccountId, String, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(AccountId, String, BucketId, CustomerUsage)> = Vec::new();
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
		for user_id in 100u8..100 + num_users {
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let mut payers: Vec<Vec<(NodePubKey, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, CustomerUsage)> = Vec::new();
=======
		let mut payers: Vec<Vec<(NodePubKey, BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, BucketUsage)> = Vec::new();
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
		for user_id in 100..100 + num_users {
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
			let ratio = match user_id % 5 {
				0 => Perquintill::one(),
				1 => Perquintill::from_float(0.5),
				2 => Perquintill::from_float(2f64),
				3 => Perquintill::from_float(0.25),
				4 => Perquintill::from_float(0.001),
				_ => unreachable!(),
			};

			let mut user_usage = usage1.clone();
			user_usage.transferred_bytes = ratio * user_usage.transferred_bytes;
			user_usage.stored_bytes = (ratio * user_usage.stored_bytes as u64) as i64;
			user_usage.number_of_puts = ratio * user_usage.number_of_puts;
			user_usage.number_of_gets = ratio * user_usage.number_of_gets;

			let expected_charge = calculate_charge_for_month(cluster_id, user_usage.clone());
			Balances::transfer(
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				&bank.clone(),
				&AccountId::from([user_id; 32]),
=======
				RuntimeOrigin::signed(bank.clone()),
				AccountId::from([user_id; 32]),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				&bank,
				&user_id,
>>>>>>> 447b5301 (Polkadot v1.1. to v1.2 upgrade)
=======
				&bank.clone(),
				&AccountId::from([user_id; 32]),
>>>>>>> 716ee103 (fix: clippy and tests)
=======
				&1,
=======
				&bank,
>>>>>>> e1534a4c (fix tests)
				&user_id,
>>>>>>> 5f953312 (bump dependnecies and fix implementation)
				(expected_charge * 2).max(Balances::minimum_balance()),
				ExistenceRequirement::KeepAlive,
			)
			.unwrap();
			total_charge += expected_charge;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			let bucket_id = user_id.into();
<<<<<<< HEAD
			user_batch.push((node_key.clone(), bucket_id, user_usage));
=======
			user_batch.push((user_id, bucketid1, user_usage));
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			user_batch.push((AccountId::from([user_id; 32]), bucketid1, user_usage));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			user_batch.push((
				AccountId::from([user_id; 32]),
				node_id.clone(),
				bucketid1,
				user_usage,
			));
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			let bucket_id = user_id.into();
			user_batch.push((node_key.clone(), bucket_id, user_usage));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
			user_batch.push((bucket_id, user_usage));
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			if user_batch.len() == user_batch_size {
				payers.push(user_batch.clone());
				user_batch.clear();
			}
		}
		if !user_batch.is_empty() {
			payers.push(user_batch.clone());
		}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
=======
		let mut payers_batch_roots = vec![];
		for batch in payers.iter() {
			let (payers_batch_root, _, _) = hash_bucket_payable_usage_batch(batch.clone());
			payers_batch_roots.push(payers_batch_root);
		}
		let (payers_root, payers_proofs) = get_root_with_proofs(payers_batch_roots);

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			cluster_usage.clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			(payers.len() - 1) as u16,
		));

		for batch in payers.iter() {
<<<<<<< HEAD
<<<<<<< HEAD
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_user_index,
<<<<<<< HEAD
=======
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				batch_user_index,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				&batch.to_vec(),
				payers_proofs.get(batch_user_index as usize).unwrap().clone(),
			));

<<<<<<< HEAD
			for (i, (_node_key, bucket_id, usage)) in batch.iter().enumerate() {
=======
				batch.to_vec(),
				MMRProof::default(),
			));

<<<<<<< HEAD
<<<<<<< HEAD
			for (customer_id, _bucket_id, usage) in batch.iter() {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			for (customer_id, _node_id, _bucket_id, usage) in batch.iter() {
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			for (i, (_node_key, bucket_id, usage)) in batch.iter().enumerate() {
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
			for (i, (bucket_id, usage)) in batch.iter().enumerate() {
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
				let charge = calculate_charge_for_month(cluster_id, usage.clone());

				let customer_id = customers_accounts[i].clone();
				System::assert_has_event(
					Event::Charged {
						cluster_id,
						era,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
						bucket_id: *bucket_id,
=======
						bucket_id: bucket_id.clone(),
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
						bucket_id: *bucket_id,
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
						customer_id: customer_id.clone(),
=======
						bucket_id: bucketid1,
<<<<<<< HEAD
						customer_id: *customer_id,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						customer_id: customer_id.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
						batch_index: batch_user_index,
						amount: charge,
					}
					.into(),
				);
			}
			batch_user_index += 1;
		}

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let balance1 = Balances::free_balance(report_before.vault.clone());
		let balance2 = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance1, balance2);
		assert_eq!(report_before.vault, DdcPayouts::account_id());
		assert_eq!(balance1 - Balances::minimum_balance(), total_charge);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
		));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let total_left_from_one = (get_fees(&cluster_id).treasury_share +
			get_fees(&cluster_id).validators_share +
			get_fees(&cluster_id).cluster_reserve_share)
			.left_from_one();

		let total_charge = report_after.total_customer_charge.transfer +
			report_before.total_customer_charge.storage +
			report_before.total_customer_charge.puts +
			report_before.total_customer_charge.gets;
		let balance_after = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(total_charge, balance_after - Balances::minimum_balance());

		assert_eq!(
			report_after.total_customer_charge.transfer,
			total_left_from_one * report_before.total_customer_charge.transfer
		);
		assert_eq!(
			report_after.total_customer_charge.storage,
			total_left_from_one * report_before.total_customer_charge.storage
		);
		assert_eq!(
			report_after.total_customer_charge.puts,
			total_left_from_one * report_before.total_customer_charge.puts
		);
		assert_eq!(
			report_after.total_customer_charge.gets,
			total_left_from_one * report_before.total_customer_charge.gets
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
		assert_ok!(DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			(payees.len() - 1) as u16,
		));

		for batch in payees.iter() {
			let before_batch = Balances::free_balance(DdcPayouts::account_id());
<<<<<<< HEAD
<<<<<<< HEAD
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			assert_ok!(DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_node_index,
<<<<<<< HEAD
				&batch.to_vec(),
=======
				batch.to_vec(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				batch_node_index,
				&batch.to_vec(),
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
=======
				payees_proofs.get(batch_node_index as usize).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			));

			let mut batch_charge = 0;
<<<<<<< HEAD
<<<<<<< HEAD
			for (i, (_node_key, node_usage1)) in batch.iter().enumerate() {
=======
			for (node1, _node_id, node_usage1) in batch.iter() {
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			for (i, (_node_key, node_usage1)) in batch.iter().enumerate() {
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
				let ratio1_transfer = Perquintill::from_rational(
					node_usage1.transferred_bytes,
					cluster_usage.transferred_bytes,
				);
				let transfer_charge = ratio1_transfer * report_after.total_customer_charge.transfer;

				let ratio1_storage = Perquintill::from_rational(
					node_usage1.stored_bytes as u64,
					cluster_usage.stored_bytes as u64,
				);
				let storage_charge = ratio1_storage * report_after.total_customer_charge.storage;

				let ratio1_puts = Perquintill::from_rational(
					node_usage1.number_of_puts,
					cluster_usage.number_of_puts,
				);
				let puts_charge = ratio1_puts * report_after.total_customer_charge.puts;

				let ratio1_gets = Perquintill::from_rational(
					node_usage1.number_of_gets,
					cluster_usage.number_of_gets,
				);
				let gets_charge = ratio1_gets * report_after.total_customer_charge.gets;

				let balance_node1 = Balances::free_balance(
					providers_accounts.get(i).expect("Node provider to be found"),
				);
				assert!(
					(transfer_charge + storage_charge + puts_charge + gets_charge) - balance_node1 <
						MAX_DUST.into()
				);

				batch_charge += transfer_charge + storage_charge + puts_charge + gets_charge;
			}
			let after_batch = Balances::free_balance(DdcPayouts::account_id());
			assert!(batch_charge + after_batch - before_batch < MAX_DUST.into());

			batch_node_index += 1;
		}
		assert!(Balances::free_balance(DdcPayouts::account_id()) < MAX_DUST.into());
	})
}

#[test]
fn send_rewarding_providers_batch_100_nodes_large_usage_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let num_nodes = 10;
		let num_users = 5;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let bank: AccountId = BANK_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let bank = AccountId::from([1; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let dac_account = 123u128;
<<<<<<< HEAD
>>>>>>> 5f953312 (bump dependnecies and fix implementation)
=======
		let bank = 1u128;
>>>>>>> e1534a4c (fix tests)
=======
		let bank: AccountId = BANK_KEY_32;
>>>>>>> c0869191 (chore: unified faucet account in tests)
		let cluster_id = ONE_CLUSTER_ID;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)

		let nodes_keys = [
			NODE1_PUB_KEY_32,
			NODE2_PUB_KEY_32,
			NODE3_PUB_KEY_32,
			NODE4_PUB_KEY_32,
			NODE5_PUB_KEY_32,
			NODE6_PUB_KEY_32,
			NODE7_PUB_KEY_32,
			NODE8_PUB_KEY_32,
			NODE9_PUB_KEY_32,
			NODE10_PUB_KEY_32,
		];

		let storage_nodes: Vec<NodePubKey> =
			nodes_keys.iter().map(|key| NodePubKey::StoragePubKey(key.clone())).collect();

		let customers_accounts = [
			CUSTOMER100_KEY_32,
			CUSTOMER101_KEY_32,
			CUSTOMER102_KEY_32,
			CUSTOMER103_KEY_32,
			CUSTOMER104_KEY_32,
		];

=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
=======
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);

>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let nodes_keys = [
			NODE1_PUB_KEY_32,
			NODE2_PUB_KEY_32,
			NODE3_PUB_KEY_32,
			NODE4_PUB_KEY_32,
			NODE5_PUB_KEY_32,
			NODE6_PUB_KEY_32,
			NODE7_PUB_KEY_32,
			NODE8_PUB_KEY_32,
			NODE9_PUB_KEY_32,
			NODE10_PUB_KEY_32,
		];

		let storage_nodes: Vec<NodePubKey> =
			nodes_keys.iter().map(|key| NodePubKey::StoragePubKey(key.clone())).collect();

		let providers_accounts = [
			NODE_PROVIDER1_KEY_32,
			NODE_PROVIDER2_KEY_32,
			NODE_PROVIDER3_KEY_32,
			NODE_PROVIDER4_KEY_32,
			NODE_PROVIDER5_KEY_32,
			NODE_PROVIDER6_KEY_32,
			NODE_PROVIDER7_KEY_32,
			NODE_PROVIDER8_KEY_32,
			NODE_PROVIDER9_KEY_32,
			NODE_PROVIDER10_KEY_32,
		];
<<<<<<< HEAD
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
=======

		let customers_accounts = [
			CUSTOMER100_KEY_32,
			CUSTOMER101_KEY_32,
			CUSTOMER102_KEY_32,
			CUSTOMER103_KEY_32,
			CUSTOMER104_KEY_32,
		];

>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let user_batch_size = 10;
		let node_batch_size = 10;
		let mut batch_user_index = 0;
<<<<<<< HEAD
		let mut batch_node_index = 0;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let usage1 = BucketUsage {
=======
		let bucket_id: BucketId = 1;
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			transferred_bytes: 1024,
			stored_bytes: 1024,
			number_of_puts: 1,
			number_of_gets: 1,
		};

		let node_usage1 = NodeUsage {
			// CDN
			transferred_bytes: Perquintill::from_float(0.75) * usage1.transferred_bytes,
			stored_bytes: 0,
			number_of_puts: Perquintill::from_float(0.75) * usage1.number_of_puts,
			number_of_gets: Perquintill::from_float(0.75) * usage1.number_of_gets,
		};

		let node_usage2 = NodeUsage {
			// Storage
			transferred_bytes: 0,
			stored_bytes: usage1.stored_bytes * 2,
			number_of_puts: 0,
			number_of_gets: 0,
		};

		let node_usage3 = NodeUsage {
			// CDN + Storage
			transferred_bytes: usage1.transferred_bytes * 2,
			stored_bytes: usage1.stored_bytes * 3,
			number_of_puts: usage1.number_of_puts * 2,
			number_of_gets: usage1.number_of_gets * 2,
		};

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let mut payees: Vec<Vec<(NodePubKey, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(NodePubKey, NodeUsage)> = Vec::new();
<<<<<<< HEAD
=======
		let mut payees: Vec<Vec<(AccountId, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(AccountId, NodeUsage)> = Vec::new();
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let mut payees: Vec<Vec<(AccountId, String, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(AccountId, String, NodeUsage)> = Vec::new();
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let mut payees: Vec<Vec<(NodePubKey, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(NodePubKey, NodeUsage)> = Vec::new();
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let mut total_nodes_usage = NodeUsage::default();
=======
		let mut cluster_usage = NodeUsage::default();
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		for i in 10..10 + num_nodes {
			let ratio = match i % 5 {
				0 => Perquintill::from_float(1_000_000.0),
				1 => Perquintill::from_float(10_000_000.0),
				2 => Perquintill::from_float(100_000_000.0),
				3 => Perquintill::from_float(1_000_000_000.0),
				4 => Perquintill::from_float(10_000_000_000.0),
				_ => unreachable!(),
			};
			let mut node_usage = match i % 3 {
				0 => node_usage1.clone(),
				1 => node_usage2.clone(),
				2 => node_usage3.clone(),
				_ => unreachable!(),
			};
			node_usage.transferred_bytes = ratio * node_usage.transferred_bytes;
			node_usage.stored_bytes = (ratio * node_usage.stored_bytes as u64) as i64;
			node_usage.number_of_puts = ratio * node_usage.number_of_puts;
			node_usage.number_of_gets = ratio * node_usage.number_of_gets;

			cluster_usage.transferred_bytes += node_usage.transferred_bytes;
			cluster_usage.stored_bytes += node_usage.stored_bytes;
			cluster_usage.number_of_puts += node_usage.number_of_puts;
			cluster_usage.number_of_gets += node_usage.number_of_gets;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			let node_key = storage_nodes[i - num_nodes].clone();
			node_batch.push((node_key.clone(), node_usage));
=======
			node_batch.push((AccountId::from([i; 32]), node_usage));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			node_batch.push((AccountId::from([i; 32]), node_id.clone(), node_usage));
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			let node_key = storage_nodes[i - num_nodes].clone();
			node_batch.push((node_key.clone(), node_usage));
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
			if node_batch.len() == node_batch_size {
				payees.push(node_batch.clone());
				node_batch.clear();
			}
		}
		if !node_batch.is_empty() {
			payees.push(node_batch.clone());
		}

		let mut total_charge = 0u128;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let mut payers: Vec<Vec<(NodePubKey, BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, BucketUsage)> = Vec::new();
=======
		let mut payers: Vec<Vec<(BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(BucketId, BucketUsage)> = Vec::new();
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)

		for user_id in 100..100 + num_users {
=======
		let mut payers: Vec<Vec<(u128, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(u128, BucketId, CustomerUsage)> = Vec::new();
		for user_id in 1000..1000 + num_users {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let mut payers: Vec<Vec<(AccountId, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(AccountId, BucketId, CustomerUsage)> = Vec::new();
=======
		let mut payers: Vec<Vec<(AccountId, String, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(AccountId, String, BucketId, CustomerUsage)> = Vec::new();

>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
		for user_id in 100u8..100 + num_users {
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let mut payers: Vec<Vec<(NodePubKey, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, CustomerUsage)> = Vec::new();
=======
		let mut payers: Vec<Vec<(NodePubKey, BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, BucketUsage)> = Vec::new();
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')

		for user_id in 100..100 + num_users {
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
			let ratio = match user_id % 5 {
				0 => Perquintill::from_float(1_000_000.0),
				1 => Perquintill::from_float(10_000_000.0),
				2 => Perquintill::from_float(100_000_000.0),
				3 => Perquintill::from_float(1_000_000_000.0),
				4 => Perquintill::from_float(10_000_000_000.0),
				_ => unreachable!(),
			};

			let mut user_usage = usage1.clone();
			user_usage.transferred_bytes = ratio * user_usage.transferred_bytes;
			user_usage.stored_bytes = (ratio * user_usage.stored_bytes as u64) as i64;
			user_usage.number_of_puts = ratio * user_usage.number_of_puts;
			user_usage.number_of_gets = ratio * user_usage.number_of_gets;

			let expected_charge = calculate_charge_for_month(cluster_id, user_usage.clone());
			Balances::transfer(
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				&bank.clone(),
				&AccountId::from([user_id; 32]),
=======
				RuntimeOrigin::signed(bank.clone()),
				AccountId::from([user_id; 32]),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				&bank,
				&user_id,
>>>>>>> 447b5301 (Polkadot v1.1. to v1.2 upgrade)
=======
				&bank.clone(),
				&AccountId::from([user_id; 32]),
>>>>>>> 716ee103 (fix: clippy and tests)
=======
				&1,
=======
				&bank,
>>>>>>> e1534a4c (fix tests)
				&user_id,
>>>>>>> 5f953312 (bump dependnecies and fix implementation)
				(expected_charge * 2).max(Balances::minimum_balance()),
				ExistenceRequirement::KeepAlive,
			)
			.unwrap();
			total_charge += expected_charge;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			let bucket_id = user_id.into();
<<<<<<< HEAD
			user_batch.push((node_key.clone(), bucket_id, user_usage));
=======
			user_batch.push((user_id, bucket_id, user_usage));
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			user_batch.push((AccountId::from([user_id; 32]), bucket_id, user_usage));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			user_batch.push((
				AccountId::from([user_id; 32]),
				node_id.clone(),
				bucket_id,
				user_usage,
			));
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			let bucket_id = user_id.into();
			user_batch.push((node_key.clone(), bucket_id, user_usage));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
			user_batch.push((bucket_id, user_usage));
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			if user_batch.len() == user_batch_size {
				payers.push(user_batch.clone());
				user_batch.clear();
			}
		}
		if !user_batch.is_empty() {
			payers.push(user_batch.clone());
		}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
=======
		let mut payers_batch_roots = vec![];
		for batch in payers.iter() {
			let (payers_batch_root, _, _) = hash_bucket_payable_usage_batch(batch.clone());
			payers_batch_roots.push(payers_batch_root);
		}
		let (payers_root, payers_proofs) = get_root_with_proofs(payers_batch_roots);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			cluster_usage.clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			DEFAULT_PAYEES_ROOT,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			(payers.len() - 1) as u16,
		));

		for batch in payers.iter() {
<<<<<<< HEAD
<<<<<<< HEAD
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_user_index,
<<<<<<< HEAD
=======
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				batch_user_index,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				&batch.to_vec(),
				payers_proofs.get(batch_user_index as usize).unwrap().clone(),
			));

<<<<<<< HEAD
			for (i, (_node_key, bucket_id, usage)) in batch.iter().enumerate() {
=======
				batch.to_vec(),
				MMRProof::default(),
			));

<<<<<<< HEAD
<<<<<<< HEAD
			for (customer_id, _bucket_id, usage) in batch.iter() {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			for (customer_id, _node_id, _bucket_id, usage) in batch.iter() {
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			for (i, (_node_key, bucket_id, usage)) in batch.iter().enumerate() {
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
			for (i, (bucket_id, usage)) in batch.iter().enumerate() {
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
				let charge = calculate_charge_for_month(cluster_id, usage.clone());

				let customer_id = customers_accounts[i].clone();
				System::assert_has_event(
					Event::Charged {
						cluster_id,
						era,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
						bucket_id: *bucket_id,
=======
						bucket_id: bucket_id.clone(),
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
						bucket_id: *bucket_id,
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
						customer_id: customer_id.clone(),
=======
						bucket_id: bucketid1,
<<<<<<< HEAD
						customer_id: *customer_id,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						customer_id: customer_id.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
						batch_index: batch_user_index,
						amount: charge,
					}
					.into(),
				);
			}
			batch_user_index += 1;
		}

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let balance1 = Balances::free_balance(report_before.vault.clone());
		let balance2 = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance1, balance2);
		assert_eq!(report_before.vault, DdcPayouts::account_id());
		assert_eq!(balance1 - Balances::minimum_balance(), total_charge);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
		));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let total_left_from_one = (get_fees(&cluster_id).treasury_share +
			get_fees(&cluster_id).validators_share +
			get_fees(&cluster_id).cluster_reserve_share)
			.left_from_one();

		let total_charge = report_after.total_customer_charge.transfer +
			report_before.total_customer_charge.storage +
			report_before.total_customer_charge.puts +
			report_before.total_customer_charge.gets;
		let balance_after = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(total_charge, balance_after - Balances::minimum_balance());

		assert_eq!(
			report_after.total_customer_charge.transfer,
			total_left_from_one * report_before.total_customer_charge.transfer
		);
		assert_eq!(
			report_after.total_customer_charge.storage,
			total_left_from_one * report_before.total_customer_charge.storage
		);
		assert_eq!(
			report_after.total_customer_charge.puts,
			total_left_from_one * report_before.total_customer_charge.puts
		);
		assert_eq!(
			report_after.total_customer_charge.gets,
			total_left_from_one * report_before.total_customer_charge.gets
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
		assert_ok!(DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			(payees.len() - 1) as u16,
		));
<<<<<<< HEAD

		for batch in payees.iter() {
			let before_batch = Balances::free_balance(DdcPayouts::account_id());
<<<<<<< HEAD
<<<<<<< HEAD
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			assert_ok!(DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_node_index,
<<<<<<< HEAD
				&batch.to_vec(),
=======
				batch.to_vec(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				batch_node_index,
				&batch.to_vec(),
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
			));

			let mut batch_charge = 0;
<<<<<<< HEAD
<<<<<<< HEAD
			for (i, (_node_key, node_usage1)) in batch.iter().enumerate() {
=======
			for (node1, _node_id, node_usage1) in batch.iter() {
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			for (i, (_node_key, node_usage1)) in batch.iter().enumerate() {
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
				let ratio1_transfer = Perquintill::from_rational(
					node_usage1.transferred_bytes,
					total_nodes_usage.transferred_bytes,
				);
				let transfer_charge = ratio1_transfer * report_after.total_customer_charge.transfer;

				let ratio1_storage = Perquintill::from_rational(
					node_usage1.stored_bytes as u64,
					total_nodes_usage.stored_bytes as u64,
				);
				let storage_charge = ratio1_storage * report_after.total_customer_charge.storage;

				let ratio1_puts = Perquintill::from_rational(
					node_usage1.number_of_puts,
					total_nodes_usage.number_of_puts,
				);
				let puts_charge = ratio1_puts * report_after.total_customer_charge.puts;

				let ratio1_gets = Perquintill::from_rational(
					node_usage1.number_of_gets,
					total_nodes_usage.number_of_gets,
				);
				let gets_charge = ratio1_gets * report_after.total_customer_charge.gets;

				let balance_node1 = Balances::free_balance(
					providers_accounts.get(i).expect("Node provider to be found"),
				);
				assert!(
					(transfer_charge + storage_charge + puts_charge + gets_charge) - balance_node1 <
						MAX_DUST.into()
				);

				batch_charge += transfer_charge + storage_charge + puts_charge + gets_charge;
			}
			let after_batch = Balances::free_balance(DdcPayouts::account_id());
			assert!(batch_charge + after_batch - before_batch < MAX_DUST.into());

			batch_node_index += 1;
		}
		assert!(Balances::free_balance(DdcPayouts::account_id()) < MAX_DUST.into());
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
	})
}

#[test]
fn send_rewarding_providers_batch_100_nodes_small_large_usage_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let num_nodes = 10;
		let num_users = 5;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let bank: AccountId = BANK_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let bank = AccountId::from([1; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let dac_account = 123u128;
<<<<<<< HEAD
>>>>>>> 5f953312 (bump dependnecies and fix implementation)
=======
		let bank = 1u128;
>>>>>>> e1534a4c (fix tests)
=======
		let bank: AccountId = BANK_KEY_32;
>>>>>>> c0869191 (chore: unified faucet account in tests)
		let cluster_id = ONE_CLUSTER_ID;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let nodes_keys = [
			NODE1_PUB_KEY_32,
			NODE2_PUB_KEY_32,
			NODE3_PUB_KEY_32,
			NODE4_PUB_KEY_32,
			NODE5_PUB_KEY_32,
			NODE6_PUB_KEY_32,
			NODE7_PUB_KEY_32,
			NODE8_PUB_KEY_32,
			NODE9_PUB_KEY_32,
			NODE10_PUB_KEY_32,
		];
		let storage_nodes: Vec<NodePubKey> =
			nodes_keys.iter().map(|key| NodePubKey::StoragePubKey(key.clone())).collect();
		let providers_accounts = [
			NODE_PROVIDER1_KEY_32,
			NODE_PROVIDER2_KEY_32,
			NODE_PROVIDER3_KEY_32,
			NODE_PROVIDER4_KEY_32,
			NODE_PROVIDER5_KEY_32,
			NODE_PROVIDER6_KEY_32,
			NODE_PROVIDER7_KEY_32,
			NODE_PROVIDER8_KEY_32,
			NODE_PROVIDER9_KEY_32,
			NODE_PROVIDER10_KEY_32,
		];
		let customers_accounts = [
			CUSTOMER100_KEY_32,
			CUSTOMER101_KEY_32,
			CUSTOMER102_KEY_32,
			CUSTOMER103_KEY_32,
			CUSTOMER104_KEY_32,
		];

<<<<<<< HEAD
=======
		let node_id: String =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let nodes_keys = [
			NODE1_PUB_KEY_32,
			NODE2_PUB_KEY_32,
			NODE3_PUB_KEY_32,
			NODE4_PUB_KEY_32,
			NODE5_PUB_KEY_32,
			NODE6_PUB_KEY_32,
			NODE7_PUB_KEY_32,
			NODE8_PUB_KEY_32,
			NODE9_PUB_KEY_32,
			NODE10_PUB_KEY_32,
		];
		let storage_nodes: Vec<NodePubKey> =
			nodes_keys.iter().map(|key| NodePubKey::StoragePubKey(key.clone())).collect();
		let providers_accounts = [
			NODE_PROVIDER1_KEY_32,
			NODE_PROVIDER2_KEY_32,
			NODE_PROVIDER3_KEY_32,
			NODE_PROVIDER4_KEY_32,
			NODE_PROVIDER5_KEY_32,
			NODE_PROVIDER6_KEY_32,
			NODE_PROVIDER7_KEY_32,
			NODE_PROVIDER8_KEY_32,
			NODE_PROVIDER9_KEY_32,
			NODE_PROVIDER10_KEY_32,
		];
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let user_batch_size = 10;
		let node_batch_size = 10;
		let mut batch_user_index = 0;
		let mut batch_node_index = 0;
		let usage1 = BucketUsage {
			transferred_bytes: 1024,
			stored_bytes: 1024,
			number_of_puts: 1,
			number_of_gets: 1,
		};

		let node_usage1 = NodeUsage {
			// CDN
			transferred_bytes: Perquintill::from_float(0.75) * usage1.transferred_bytes,
			stored_bytes: 0,
			number_of_puts: Perquintill::from_float(0.75) * usage1.number_of_puts,
			number_of_gets: Perquintill::from_float(0.75) * usage1.number_of_gets,
		};

		let node_usage2 = NodeUsage {
			// Storage
			transferred_bytes: 0,
			stored_bytes: usage1.stored_bytes * 2,
			number_of_puts: 0,
			number_of_gets: 0,
		};

		let node_usage3 = NodeUsage {
			// CDN + Storage
			transferred_bytes: usage1.transferred_bytes * 2,
			stored_bytes: usage1.stored_bytes * 3,
			number_of_puts: usage1.number_of_puts * 2,
			number_of_gets: usage1.number_of_gets * 2,
		};

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let mut payees: Vec<Vec<(NodePubKey, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(NodePubKey, NodeUsage)> = Vec::new();
<<<<<<< HEAD
=======
		let mut payees: Vec<Vec<(AccountId, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(AccountId, NodeUsage)> = Vec::new();
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let mut payees: Vec<Vec<(AccountId, String, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(AccountId, String, NodeUsage)> = Vec::new();
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let mut payees: Vec<Vec<(NodePubKey, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(NodePubKey, NodeUsage)> = Vec::new();
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let mut total_nodes_usage = NodeUsage::default();
=======
		let mut cluster_usage = NodeUsage::default();
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		for i in 10..10 + num_nodes {
			let ratio = match i % 5 {
				0 => Perquintill::from_float(1_000_000.0),
				1 => Perquintill::from_float(0.5),
				2 => Perquintill::from_float(100_000_000.0),
				3 => Perquintill::from_float(0.25),
				4 => Perquintill::from_float(10_000_000_000.0),
				_ => unreachable!(),
			};
			let mut node_usage = match i % 3 {
				0 => node_usage1.clone(),
				1 => node_usage2.clone(),
				2 => node_usage3.clone(),
				_ => unreachable!(),
			};
			node_usage.transferred_bytes = ratio * node_usage.transferred_bytes;
			node_usage.stored_bytes = (ratio * node_usage.stored_bytes as u64) as i64;
			node_usage.number_of_puts = ratio * node_usage.number_of_puts;
			node_usage.number_of_gets = ratio * node_usage.number_of_gets;

			cluster_usage.transferred_bytes += node_usage.transferred_bytes;
			cluster_usage.stored_bytes += node_usage.stored_bytes;
			cluster_usage.number_of_puts += node_usage.number_of_puts;
			cluster_usage.number_of_gets += node_usage.number_of_gets;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			let node_key = storage_nodes[i - num_nodes].clone();
			node_batch.push((node_key.clone(), node_usage));
=======
			node_batch.push((AccountId::from([i; 32]), node_usage));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			node_batch.push((AccountId::from([i; 32]), node_id.clone(), node_usage));
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			let node_key = storage_nodes[i - num_nodes].clone();
			node_batch.push((node_key.clone(), node_usage));
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
			if node_batch.len() == node_batch_size {
				payees.push(node_batch.clone());
				node_batch.clear();
			}
		}
		if !node_batch.is_empty() {
			payees.push(node_batch.clone());
		}

		let mut payees_batch_roots = vec![];
		for batch in payees.iter() {
			let (payers_batch_root, _, _) = hash_node_payable_usage_batch(batch.clone());
			payees_batch_roots.push(payers_batch_root);
		}
		let (payees_root, payees_proofs) = get_root_with_proofs(payees_batch_roots);

		let mut total_charge = 0u128;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let mut payers: Vec<Vec<(NodePubKey, BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, BucketUsage)> = Vec::new();
=======
		let mut payers: Vec<Vec<(BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(BucketId, BucketUsage)> = Vec::new();
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		for user_id in 100u8..100 + num_users {
=======
		let mut payers: Vec<Vec<(u128, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(u128, BucketId, CustomerUsage)> = Vec::new();
		for user_id in 1000..1000 + num_users {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let mut payers: Vec<Vec<(AccountId, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(AccountId, BucketId, CustomerUsage)> = Vec::new();
=======
		let mut payers: Vec<Vec<(AccountId, String, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(AccountId, String, BucketId, CustomerUsage)> = Vec::new();
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let mut payers: Vec<Vec<(NodePubKey, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, CustomerUsage)> = Vec::new();
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let mut payers: Vec<Vec<(NodePubKey, BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, BucketUsage)> = Vec::new();
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
		for user_id in 100u8..100 + num_users {
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
			let ratio = match user_id % 5 {
				0 => Perquintill::from_float(1_000_000.0),
				1 => Perquintill::from_float(10_000_000.0),
				2 => Perquintill::from_float(100_000_000.0),
				3 => Perquintill::from_float(1_000_000_000.0),
				4 => Perquintill::from_float(10_000_000_000.0),
				_ => unreachable!(),
			};

			let mut user_usage = usage1.clone();
			user_usage.transferred_bytes = ratio * user_usage.transferred_bytes;
			user_usage.stored_bytes = (ratio * user_usage.stored_bytes as u64) as i64;
			user_usage.number_of_puts = ratio * user_usage.number_of_puts;
			user_usage.number_of_gets = ratio * user_usage.number_of_gets;

			let expected_charge = calculate_charge_for_month(cluster_id, user_usage.clone());
			Balances::transfer(
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				&bank.clone(),
				&AccountId::from([user_id; 32]),
=======
				RuntimeOrigin::signed(bank.clone()),
				AccountId::from([user_id; 32]),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				&bank,
				&user_id,
>>>>>>> 447b5301 (Polkadot v1.1. to v1.2 upgrade)
=======
				&bank.clone(),
				&AccountId::from([user_id; 32]),
>>>>>>> 716ee103 (fix: clippy and tests)
=======
				&1,
=======
				&bank,
>>>>>>> e1534a4c (fix tests)
				&user_id,
>>>>>>> 5f953312 (bump dependnecies and fix implementation)
				(expected_charge * 2).max(Balances::minimum_balance()),
				ExistenceRequirement::KeepAlive,
			)
			.unwrap();
			total_charge += expected_charge;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			let bucket_id = user_id.into();
<<<<<<< HEAD
			user_batch.push((node_key.clone(), bucket_id, user_usage));
=======
			user_batch.push((user_id, bucketid1, user_usage));
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			user_batch.push((AccountId::from([user_id; 32]), bucketid1, user_usage));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			user_batch.push((
				AccountId::from([user_id; 32]),
				node_id.clone(),
				bucketid1,
				user_usage,
			));
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			let bucket_id = user_id.into();
			user_batch.push((node_key.clone(), bucket_id, user_usage));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
			user_batch.push((bucket_id, user_usage));
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			if user_batch.len() == user_batch_size {
				payers.push(user_batch.clone());
				user_batch.clear();
			}
		}
		if !user_batch.is_empty() {
			payers.push(user_batch.clone());
		}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
=======
		let mut payers_batch_roots = vec![];
		for batch in payers.iter() {
			let (payers_batch_root, _, _) = hash_bucket_payable_usage_batch(batch.clone());
			payers_batch_roots.push(payers_batch_root);
		}
		let (payers_root, payers_proofs) = get_root_with_proofs(payers_batch_roots);

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			cluster_usage.clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			(payers.len() - 1) as u16,
		));

		for batch in payers.iter() {
<<<<<<< HEAD
<<<<<<< HEAD
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_user_index,
<<<<<<< HEAD
=======
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				batch_user_index,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				&batch.to_vec(),
				payers_proofs.get(batch_user_index as usize).unwrap().clone(),
			));

<<<<<<< HEAD
			for (i, (_node_key, bucket_id, usage)) in batch.iter().enumerate() {
=======
				batch.to_vec(),
				MMRProof::default(),
			));

<<<<<<< HEAD
<<<<<<< HEAD
			for (customer_id, _bucket_id, usage) in batch.iter() {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			for (customer_id, _node_id, _bucket_id, usage) in batch.iter() {
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			for (i, (_node_key, bucket_id, usage)) in batch.iter().enumerate() {
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
			for (i, (bucket_id, usage)) in batch.iter().enumerate() {
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
				let charge = calculate_charge_for_month(cluster_id, usage.clone());

				let customer_id = customers_accounts[i].clone();
				System::assert_has_event(
					Event::Charged {
						cluster_id,
						era,
<<<<<<< HEAD
<<<<<<< HEAD
						customer_id: customer_id.clone(),
<<<<<<< HEAD
<<<<<<< HEAD
						bucket_id: *bucket_id,
=======
						customer_id: *customer_id,
=======
						customer_id: customer_id.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
						bucket_id: bucketid1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						bucket_id: bucket_id.clone(),
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
						bucket_id: *bucket_id,
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
						batch_index: batch_user_index,
						amount: charge,
					}
					.into(),
				);
			}
			batch_user_index += 1;
		}

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let balance1 = Balances::free_balance(report_before.vault.clone());
		let balance2 = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance1, balance2);
		assert_eq!(report_before.vault, DdcPayouts::account_id());
		assert_eq!(balance1 - Balances::minimum_balance(), total_charge);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
		));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let total_left_from_one = (get_fees(&cluster_id).treasury_share +
			get_fees(&cluster_id).validators_share +
			get_fees(&cluster_id).cluster_reserve_share)
			.left_from_one();

		let total_charge = report_after.total_customer_charge.transfer +
			report_before.total_customer_charge.storage +
			report_before.total_customer_charge.puts +
			report_before.total_customer_charge.gets;
		let balance_after = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(total_charge, balance_after - Balances::minimum_balance());

		assert_eq!(
			report_after.total_customer_charge.transfer,
			total_left_from_one * report_before.total_customer_charge.transfer
		);
		assert_eq!(
			report_after.total_customer_charge.storage,
			total_left_from_one * report_before.total_customer_charge.storage
		);
		assert_eq!(
			report_after.total_customer_charge.puts,
			total_left_from_one * report_before.total_customer_charge.puts
		);
		assert_eq!(
			report_after.total_customer_charge.gets,
			total_left_from_one * report_before.total_customer_charge.gets
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
		assert_ok!(DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			(payees.len() - 1) as u16,
		));

		for batch in payees.iter() {
			let before_batch = Balances::free_balance(DdcPayouts::account_id());
<<<<<<< HEAD
<<<<<<< HEAD
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			assert_ok!(DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_node_index,
<<<<<<< HEAD
				&batch.to_vec(),
=======
				batch.to_vec(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				batch_node_index,
				&batch.to_vec(),
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
=======
				payees_proofs.get(batch_node_index as usize).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			));

			let mut batch_charge = 0;
<<<<<<< HEAD
<<<<<<< HEAD
			for (i, (_node_key, node_usage1)) in batch.iter().enumerate() {
=======
			for (node1, _node_id, node_usage1) in batch.iter() {
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			for (i, (_node_key, node_usage1)) in batch.iter().enumerate() {
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
				let ratio1_transfer = Perquintill::from_rational(
					node_usage1.transferred_bytes,
					cluster_usage.transferred_bytes,
				);
				let transfer_charge = ratio1_transfer * report_after.total_customer_charge.transfer;

				let ratio1_storage = Perquintill::from_rational(
					node_usage1.stored_bytes as u64,
					cluster_usage.stored_bytes as u64,
				);
				let storage_charge = ratio1_storage * report_after.total_customer_charge.storage;

				let ratio1_puts = Perquintill::from_rational(
					node_usage1.number_of_puts,
					cluster_usage.number_of_puts,
				);
				let puts_charge = ratio1_puts * report_after.total_customer_charge.puts;

				let ratio1_gets = Perquintill::from_rational(
					node_usage1.number_of_gets,
					cluster_usage.number_of_gets,
				);
				let gets_charge = ratio1_gets * report_after.total_customer_charge.gets;

				let balance_node1 = Balances::free_balance(
					providers_accounts.get(i).expect("Node provider to be found"),
				);
				assert!(
					(transfer_charge + storage_charge + puts_charge + gets_charge) - balance_node1 <
						MAX_DUST.into()
				);

				batch_charge += transfer_charge + storage_charge + puts_charge + gets_charge;
			}
			let after_batch = Balances::free_balance(DdcPayouts::account_id());
			assert!(batch_charge + after_batch - before_batch < MAX_DUST.into());

			batch_node_index += 1;
		}
		assert!(Balances::free_balance(DdcPayouts::account_id()) < MAX_DUST.into());
	})
}

fn generate_random_u64<T: Randomness<H256, BlockNumber>>(_: &T, min: u64, max: u64) -> u64 {
	let (random_seed, _) = T::random_seed();
	let random_raw = u64::from_be_bytes(random_seed.as_bytes()[0..8].try_into().unwrap());

	min.saturating_add(random_raw % (max.saturating_sub(min).saturating_add(1)))
}

#[test]
fn send_rewarding_providers_batch_100_nodes_random_usage_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let mock_randomness = MockRandomness::default();
		let min: u64 = 1024;
		let max: u64 = 1024 * 1024;
<<<<<<< HEAD
		let num_nodes = 10;
		let num_users = 10;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let bank: AccountId = BANK_KEY_32;
=======
		let dac_account = AccountId::from([123; 32]);
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let bank = AccountId::from([1; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let num_nodes = 100;
		let num_users = 100;
		let dac_account = 123u128;
<<<<<<< HEAD
>>>>>>> 5f953312 (bump dependnecies and fix implementation)
=======
		let bank = 1u128;
>>>>>>> e1534a4c (fix tests)
=======
		let bank: AccountId = BANK_KEY_32;
>>>>>>> c0869191 (chore: unified faucet account in tests)
		let cluster_id = CERE_CLUSTER_ID;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let nodes_keys = [
			NODE1_PUB_KEY_32,
			NODE2_PUB_KEY_32,
			NODE3_PUB_KEY_32,
			NODE4_PUB_KEY_32,
			NODE5_PUB_KEY_32,
			NODE6_PUB_KEY_32,
			NODE7_PUB_KEY_32,
			NODE8_PUB_KEY_32,
			NODE9_PUB_KEY_32,
			NODE10_PUB_KEY_32,
		];
		let storage_nodes: Vec<NodePubKey> =
			nodes_keys.iter().map(|key| NodePubKey::StoragePubKey(key.clone())).collect();
		let providers_accounts = [
			NODE_PROVIDER1_KEY_32,
			NODE_PROVIDER2_KEY_32,
			NODE_PROVIDER3_KEY_32,
			NODE_PROVIDER4_KEY_32,
			NODE_PROVIDER5_KEY_32,
			NODE_PROVIDER6_KEY_32,
			NODE_PROVIDER7_KEY_32,
			NODE_PROVIDER8_KEY_32,
			NODE_PROVIDER9_KEY_32,
			NODE_PROVIDER10_KEY_32,
		];
		let customers_accounts = [
			CUSTOMER100_KEY_32,
			CUSTOMER101_KEY_32,
			CUSTOMER102_KEY_32,
			CUSTOMER103_KEY_32,
			CUSTOMER104_KEY_32,
			CUSTOMER105_KEY_32,
			CUSTOMER106_KEY_32,
			CUSTOMER107_KEY_32,
			CUSTOMER108_KEY_32,
			CUSTOMER109_KEY_32,
		];

<<<<<<< HEAD
=======
		let node_id: String =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let nodes_keys = [
			NODE1_PUB_KEY_32,
			NODE2_PUB_KEY_32,
			NODE3_PUB_KEY_32,
			NODE4_PUB_KEY_32,
			NODE5_PUB_KEY_32,
			NODE6_PUB_KEY_32,
			NODE7_PUB_KEY_32,
			NODE8_PUB_KEY_32,
			NODE9_PUB_KEY_32,
			NODE10_PUB_KEY_32,
		];
		let storage_nodes: Vec<NodePubKey> =
			nodes_keys.iter().map(|key| NodePubKey::StoragePubKey(key.clone())).collect();
		let providers_accounts = [
			NODE_PROVIDER1_KEY_32,
			NODE_PROVIDER2_KEY_32,
			NODE_PROVIDER3_KEY_32,
			NODE_PROVIDER4_KEY_32,
			NODE_PROVIDER5_KEY_32,
			NODE_PROVIDER6_KEY_32,
			NODE_PROVIDER7_KEY_32,
			NODE_PROVIDER8_KEY_32,
			NODE_PROVIDER9_KEY_32,
			NODE_PROVIDER10_KEY_32,
		];
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
		let user_batch_size = 10;
		let node_batch_size = 10;
		let mut batch_user_index = 0;
		let mut batch_node_index = 0;
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let mut payees: Vec<Vec<(NodePubKey, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(NodePubKey, NodeUsage)> = Vec::new();
<<<<<<< HEAD
=======
		let bucket_id1: BucketId = 1;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let mut payees: Vec<Vec<(u128, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(u128, NodeUsage)> = Vec::new();
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let mut payees: Vec<Vec<(AccountId, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(AccountId, NodeUsage)> = Vec::new();
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let mut payees: Vec<Vec<(AccountId, String, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(AccountId, String, NodeUsage)> = Vec::new();
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let mut payees: Vec<Vec<(NodePubKey, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(NodePubKey, NodeUsage)> = Vec::new();
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let mut total_nodes_usage = NodeUsage::default();
=======
		let mut cluster_usage = NodeUsage::default();
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		for i in 10..10 + num_nodes {
			let node_usage = NodeUsage {
				transferred_bytes: generate_random_u64(&mock_randomness, min, max),
				stored_bytes: (generate_random_u64(&mock_randomness, min, max)) as i64,
				number_of_puts: generate_random_u64(&mock_randomness, min, max),
				number_of_gets: generate_random_u64(&mock_randomness, min, max),
			};

			cluster_usage.transferred_bytes += node_usage.transferred_bytes;
			cluster_usage.stored_bytes += node_usage.stored_bytes;
			cluster_usage.number_of_puts += node_usage.number_of_puts;
			cluster_usage.number_of_gets += node_usage.number_of_gets;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			let node_key = storage_nodes[i - num_nodes].clone();
			node_batch.push((node_key.clone(), node_usage));
=======
			node_batch.push((AccountId::from([i; 32]), node_usage));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			node_batch.push((AccountId::from([i; 32]), node_id.clone(), node_usage));
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			let node_key = storage_nodes[i - num_nodes].clone();
			node_batch.push((node_key.clone(), node_usage));
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
			if node_batch.len() == node_batch_size {
				payees.push(node_batch.clone());
				node_batch.clear();
			}
		}
		if !node_batch.is_empty() {
			payees.push(node_batch.clone());
		}

		let mut payees_batch_roots = vec![];
		for batch in payees.iter() {
			let (payers_batch_root, _, _) = hash_node_payable_usage_batch(batch.clone());
			payees_batch_roots.push(payers_batch_root);
		}
		let (payees_root, payees_proofs) = get_root_with_proofs(payees_batch_roots);

		let mut total_charge = 0u128;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		let mut payers: Vec<Vec<(NodePubKey, BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, BucketUsage)> = Vec::new();
=======
		let mut payers: Vec<Vec<(BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(BucketId, BucketUsage)> = Vec::new();
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		for user_id in 100..100 + num_users {
			let user_usage = BucketUsage {
=======
		let mut payers: Vec<Vec<(u128, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(u128, BucketId, CustomerUsage)> = Vec::new();
		for user_id in 1000..1000 + num_users {
=======
		let mut payers: Vec<Vec<(AccountId, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(AccountId, BucketId, CustomerUsage)> = Vec::new();
=======
		let mut payers: Vec<Vec<(AccountId, String, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(AccountId, String, BucketId, CustomerUsage)> = Vec::new();
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
		for user_id in 100u8..100 + num_users {
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		let mut payers: Vec<Vec<(NodePubKey, BucketId, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, CustomerUsage)> = Vec::new();
		for user_id in 100..100 + num_users {
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
			let user_usage = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let mut payers: Vec<Vec<(NodePubKey, BucketId, BucketUsage)>> = Vec::new();
		let mut user_batch: Vec<(NodePubKey, BucketId, BucketUsage)> = Vec::new();
		for user_id in 100..100 + num_users {
			let user_usage = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
				transferred_bytes: generate_random_u64(&mock_randomness, min, max),
				stored_bytes: (generate_random_u64(&mock_randomness, min, max)) as i64,
				number_of_puts: generate_random_u64(&mock_randomness, min, max),
				number_of_gets: generate_random_u64(&mock_randomness, min, max),
			};

			let expected_charge = calculate_charge_for_month(cluster_id, user_usage.clone());
			Balances::transfer(
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
				&bank.clone(),
				&AccountId::from([user_id; 32]),
=======
				RuntimeOrigin::signed(bank.clone()),
				AccountId::from([user_id; 32]),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				&bank,
				&user_id,
>>>>>>> 447b5301 (Polkadot v1.1. to v1.2 upgrade)
=======
				&bank.clone(),
				&AccountId::from([user_id; 32]),
>>>>>>> 716ee103 (fix: clippy and tests)
=======
				&1,
=======
				&bank,
>>>>>>> e1534a4c (fix tests)
				&user_id,
>>>>>>> 5f953312 (bump dependnecies and fix implementation)
				(expected_charge * 2).max(Balances::minimum_balance()),
				ExistenceRequirement::KeepAlive,
			)
			.unwrap();
			total_charge += expected_charge;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			let bucket_id = user_id.into();
<<<<<<< HEAD
			user_batch.push((node_key.clone(), bucket_id, user_usage));
=======
			user_batch.push((user_id, bucket_id1, user_usage));
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			user_batch.push((AccountId::from([user_id; 32]), bucket_id1, user_usage));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			user_batch.push((
				AccountId::from([user_id; 32]),
				node_id.clone(),
				bucket_id1,
				user_usage,
			));
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			let bucket_id = user_id.into();
			user_batch.push((node_key.clone(), bucket_id, user_usage));
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
			user_batch.push((bucket_id, user_usage));
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			if user_batch.len() == user_batch_size {
				payers.push(user_batch.clone());
				user_batch.clear();
			}
		}
		if !user_batch.is_empty() {
			payers.push(user_batch.clone());
		}

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
=======
		let mut payers_batch_roots = vec![];
		for batch in payers.iter() {
			let (payers_batch_root, _, _) = hash_bucket_payable_usage_batch(batch.clone());
			payers_batch_roots.push(payers_batch_root);
		}
		let (payers_root, payers_proofs) = get_root_with_proofs(payers_batch_roots);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			cluster_usage.clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));
=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
		));
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			(payers.len() - 1) as u16,
		));

		for batch in payers.iter() {
<<<<<<< HEAD
<<<<<<< HEAD
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_user_index,
<<<<<<< HEAD
=======
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				batch_user_index,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				&batch.to_vec(),
				payers_proofs.get(batch_user_index as usize).unwrap().clone(),
			));

<<<<<<< HEAD
			for (i, (_node_key, bucket_id, usage)) in batch.iter().enumerate() {
=======
				batch.to_vec(),
				MMRProof::default(),
			));

<<<<<<< HEAD
<<<<<<< HEAD
			for (customer_id, _bucket_id, usage) in batch.iter() {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			for (customer_id, _node_id, _bucket_id, usage) in batch.iter() {
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			for (i, (_node_key, bucket_id, usage)) in batch.iter().enumerate() {
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
			for (i, (bucket_id, usage)) in batch.iter().enumerate() {
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
				let charge = calculate_charge_for_month(cluster_id, usage.clone());
				let customer_id = customers_accounts[i].clone();

				System::assert_has_event(
					Event::Charged {
						cluster_id,
						era,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
						bucket_id: *bucket_id,
=======
						bucket_id: bucket_id.clone(),
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
						bucket_id: *bucket_id,
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
						customer_id: customer_id.clone(),
=======
						bucket_id: bucket_id1,
<<<<<<< HEAD
						customer_id: *customer_id,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
						customer_id: customer_id.clone(),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
						batch_index: batch_user_index,
						amount: charge,
					}
					.into(),
				);
			}
			batch_user_index += 1;
		}

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let balance1 = Balances::free_balance(report_before.vault.clone());
		let balance2 = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance1, balance2);
		assert_eq!(report_before.vault, DdcPayouts::account_id());
		assert_eq!(balance1 - Balances::minimum_balance(), total_charge);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
		));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let total_left_from_one = (get_fees(&cluster_id).treasury_share +
			get_fees(&cluster_id).validators_share +
			get_fees(&cluster_id).cluster_reserve_share)
			.left_from_one();

		let total_charge = report_after.total_customer_charge.transfer +
			report_before.total_customer_charge.storage +
			report_before.total_customer_charge.puts +
			report_before.total_customer_charge.gets;
		let balance_after = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(total_charge, balance_after - Balances::minimum_balance());

		assert_eq!(
			report_after.total_customer_charge.transfer,
			total_left_from_one * report_before.total_customer_charge.transfer
		);
		assert_eq!(
			report_after.total_customer_charge.storage,
			total_left_from_one * report_before.total_customer_charge.storage
		);
		assert_eq!(
			report_after.total_customer_charge.puts,
			total_left_from_one * report_before.total_customer_charge.puts
		);
		assert_eq!(
			report_after.total_customer_charge.gets,
			total_left_from_one * report_before.total_customer_charge.gets
		);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
		assert_ok!(DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			(payees.len() - 1) as u16,
		));

		for batch in payees.iter() {
			let before_batch = Balances::free_balance(DdcPayouts::account_id());
<<<<<<< HEAD
<<<<<<< HEAD
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			assert_ok!(DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
				RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
				cluster_id,
				era,
				batch_node_index,
<<<<<<< HEAD
				&batch.to_vec(),
=======
				batch.to_vec(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				batch_node_index,
				&batch.to_vec(),
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
				MMRProof::default(),
=======
				payees_proofs.get(batch_node_index as usize).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			));

			let mut batch_charge = 0;
<<<<<<< HEAD
<<<<<<< HEAD
			for (i, (_node_key, node_usage1)) in batch.iter().enumerate() {
=======
			for (node1, _node_id, node_usage1) in batch.iter() {
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
			for (i, (_node_key, node_usage1)) in batch.iter().enumerate() {
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
				let ratio1_transfer = Perquintill::from_rational(
					node_usage1.transferred_bytes,
					cluster_usage.transferred_bytes,
				);
				let transfer_charge = ratio1_transfer * report_after.total_customer_charge.transfer;

				let ratio1_storage = Perquintill::from_rational(
					node_usage1.stored_bytes as u64,
					cluster_usage.stored_bytes as u64,
				);
				let storage_charge = ratio1_storage * report_after.total_customer_charge.storage;

				let ratio1_puts = Perquintill::from_rational(
					node_usage1.number_of_puts,
					cluster_usage.number_of_puts,
				);
				let puts_charge = ratio1_puts * report_after.total_customer_charge.puts;

				let ratio1_gets = Perquintill::from_rational(
					node_usage1.number_of_gets,
					cluster_usage.number_of_gets,
				);
				let gets_charge = ratio1_gets * report_after.total_customer_charge.gets;

				let balance_node1 = Balances::free_balance(
					providers_accounts.get(i).expect("Node provider to be found"),
				);
				assert!(
					(transfer_charge + storage_charge + puts_charge + gets_charge) - balance_node1 <
						MAX_DUST.into()
				);

				batch_charge += transfer_charge + storage_charge + puts_charge + gets_charge;
			}
			let after_batch = Balances::free_balance(DdcPayouts::account_id());
			assert!(batch_charge + after_batch - before_batch < MAX_DUST.into());

			batch_node_index += 1;
		}
		assert!(Balances::free_balance(DdcPayouts::account_id()) < MAX_DUST.into());
	})
}

#[test]
fn end_rewarding_providers_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let root_account = AccountId::from([1; 32]);
<<<<<<< HEAD
		let dac_account = AccountId::from([2; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([3; 32]);
		let user2 = AccountId::from([4; 32]);
<<<<<<< HEAD
		let node1 = AccountId::from([33; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let cluster_id = ClusterId::from([12; 20]);
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id: String =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let era = 100;
		let max_batch_index = 1;
		let batch_index = 0;
<<<<<<< HEAD
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
		let payers1 = vec![(node_key.clone(), bucket_id3, BucketUsage::default())];
		let payers2 = vec![(node_key.clone(), bucket_id4, BucketUsage::default())];
		let payees = vec![(node_key, NodeUsage::default())];
=======
		let bucket_id1: BucketId = 1;
		let bucket_id2: BucketId = 2;
<<<<<<< HEAD
		let payers1 = vec![(user1, bucket_id1, CustomerUsage::default())];
		let payers2 = vec![(user2, bucket_id2, CustomerUsage::default())];
		let payees = vec![(node1, NodeUsage::default())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers1 = vec![(user1, node_id.clone(), bucket_id1, CustomerUsage::default())];
		let payers2 = vec![(user2, node_id.clone(), bucket_id2, CustomerUsage::default())];
<<<<<<< HEAD
		let payees = vec![(node1, node_id, NodeUsage::default())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
>>>>>>> f82a1179 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let node_key1 = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
		let node_key2 = NodePubKey::StoragePubKey(NODE2_PUB_KEY_32);

		let era = 100;
		let max_charging_batch_index = 1;
		let max_rewarding_batch_index = 1;
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;
<<<<<<< HEAD
<<<<<<< HEAD
		let payers1 = vec![(node_key.clone(), bucket_id3, CustomerUsage::default())];
		let payers2 = vec![(node_key.clone(), bucket_id4, CustomerUsage::default())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers1 = vec![(node_key.clone(), bucket_id3, BucketUsage::default())];
		let payers2 = vec![(node_key.clone(), bucket_id4, BucketUsage::default())];
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
		let payees = vec![(node_key, NodeUsage::default())];
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let total_node_usage = NodeUsage::default();
=======
		let payers1 = vec![(bucket_id3, BucketUsage::default())];
		let payers2 = vec![(bucket_id4, BucketUsage::default())];
		let payees1 = vec![(node_key1, NodeUsage::default())];
		let payees2 = vec![(node_key2, NodeUsage::default())];

>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let cluster_usage = NodeUsage::default();

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::BillingReportDoesNotExist
		);

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		let (payees1_batch_root, _, _) = hash_node_payable_usage_batch(payees1.clone());
		let (payees2_batch_root, _, _) = hash_node_payable_usage_batch(payees2.clone());

		let (payees_root, payees_proofs) =
			get_root_with_proofs(vec![payees1_batch_root, payees2_batch_root]);

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			cluster_usage,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(AccountId::from([0; 32])),
				cluster_id,
				era,
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::end_rewarding_providers(RuntimeOrigin::root(), cluster_id, era,),
			BadOrigin
		);

		assert_noop!(
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1,
=======
			payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			0,
			&payers1,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(0).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index + 1,
<<<<<<< HEAD
			&payers2,
=======
			payers2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			1,
			&payers2,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
=======
			payers_proofs.get(1).unwrap().clone(),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
		assert_ok!(DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_rewarding_batch_index,
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::BatchesMissed
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
			Error::<Test>::BatchesMissed
		);

		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payees,
=======
			payees,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			Error::<Test>::BatchesMissed
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
<<<<<<< HEAD
			batch_index,
			&payees,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
=======
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
				cluster_id,
				era,
			),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
=======
			0,
			&payees1,
			payees_proofs.get(0).unwrap().clone(),
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			Error::<Test>::BatchesMissed
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			1,
			&payees2,
			payees_proofs.get(1).unwrap().clone(),
		));
	})
}

#[test]
fn end_rewarding_providers_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let dac_account = AccountId::from([2; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([1; 32]);
<<<<<<< HEAD
		let node1 = AccountId::from([33; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let cluster_id = ClusterId::from([12; 20]);
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
>>>>>>> 4c37f795 (fix: unussed variables are removed and clippy issues fixed)
		let era = 100;
		let max_charging_batch_index = 0;
		let max_rewarding_batch_index = 0;
		let batch_index = 0;
		let bucket_id1: BucketId = 1;
<<<<<<< HEAD
<<<<<<< HEAD
		let usage1 = BucketUsage {
=======
		let usage1 = CustomerUsage {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let usage1 = BucketUsage {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};

		let node_usage1 = NodeUsage {
			// CDN + Storage
			transferred_bytes: usage1.transferred_bytes * 2 / 3,
			stored_bytes: usage1.stored_bytes * 2 / 3,
			number_of_puts: usage1.number_of_puts * 2 / 3,
			number_of_gets: usage1.number_of_gets * 2 / 3,
		};
<<<<<<< HEAD
		let total_node_usage = node_usage1.clone();
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let payers = vec![(node_key.clone(), bucket_id1, usage1)];
=======
		let payers = vec![(bucket_id1, usage1)];
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let payees = vec![(node_key, node_usage1)];
=======
		let payers = vec![(user1, bucket_id1, usage1)];
		let payees = vec![(node1, node_usage1)];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers = vec![(user1, node_id.clone(), bucket_id1, usage1)];
<<<<<<< HEAD
		let payees = vec![(node1, node_id, node_usage1)];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payees = vec![(node_key, node_usage1)];
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));

=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
=======
		let (_, payers_batch_proof, payers_root) = hash_bucket_payable_usage_batch(payers.clone());
		let (_, payees_batch_proof, payees_root) = hash_node_payable_usage_batch(payees.clone());

		let cluster_usage = NodeUsage::default();

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			cluster_usage,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, PayoutState::Initialized);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers,
=======
			payers,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
			MMRProof::default(),
		));

<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
		));

		assert_ok!(DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			batch_index,
			&payers,
			payers_batch_proof,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_rewarding_batch_index,
		));

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payees,
=======
			payees,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
			MMRProof::default(),
		));

<<<<<<< HEAD
		assert_ok!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,)
		);
=======
		assert_ok!(DdcPayouts::end_rewarding_providers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
		));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			batch_index,
			&payees,
			payees_batch_proof,
		));

		assert_ok!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,)
		);
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

		System::assert_last_event(Event::RewardingFinished { cluster_id, era }.into());

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, PayoutState::ProvidersRewarded);
	})
}

#[test]
fn end_billing_report_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let root_account = AccountId::from([1; 32]);
<<<<<<< HEAD
		let dac_account = AccountId::from([2; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([3; 32]);
		let user2 = AccountId::from([4; 32]);
<<<<<<< HEAD
		let node1 = AccountId::from([33; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let cluster_id = ClusterId::from([12; 20]);
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
<<<<<<< HEAD
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let node_key = NodePubKey::StoragePubKey(AccountId32::from([
			48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
			235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
		]));
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let era = 100;
		let max_batch_index = 1;
		let batch_index = 0;
<<<<<<< HEAD
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;

		let payers1 = vec![(node_key.clone(), bucket_id3, BucketUsage::default())];
		let payers2 = vec![(node_key.clone(), bucket_id4, BucketUsage::default())];
		let payees = vec![(node_key, NodeUsage::default())];
=======
		let bucket_id1: BucketId = 1;
		let bucket_id2: BucketId = 2;
<<<<<<< HEAD
		let payers1 = vec![(user1, bucket_id1, CustomerUsage::default())];
		let payers2 = vec![(user2, bucket_id2, CustomerUsage::default())];
		let payees = vec![(node1, NodeUsage::default())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers1 = vec![(user1, node_id.clone(), bucket_id1, CustomerUsage::default())];
		let payers2 = vec![(user2, node_id.clone(), bucket_id2, CustomerUsage::default())];
<<<<<<< HEAD
		let payees = vec![(node1, node_id, NodeUsage::default())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
>>>>>>> f82a1179 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
		let era = 100;
		let max_charging_batch_index = 1;
		let max_rewarding_batch_index = 0;
		let bucket_id3: BucketId = BUCKET_ID3;
		let bucket_id4: BucketId = BUCKET_ID4;

<<<<<<< HEAD
<<<<<<< HEAD
		let payers1 = vec![(node_key.clone(), bucket_id3, CustomerUsage::default())];
		let payers2 = vec![(node_key.clone(), bucket_id4, CustomerUsage::default())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let payers1 = vec![(node_key.clone(), bucket_id3, BucketUsage::default())];
		let payers2 = vec![(node_key.clone(), bucket_id4, BucketUsage::default())];
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
		let payees = vec![(node_key, NodeUsage::default())];
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let total_node_usage = NodeUsage::default();
=======
		let payers1 = vec![(
			bucket_id3,
			BucketUsage {
				transferred_bytes: 1,
				stored_bytes: 1,
				number_of_gets: 1,
				number_of_puts: 1,
			},
		)];
		let payers2 = vec![(
			bucket_id4,
			BucketUsage {
				transferred_bytes: 2,
				stored_bytes: 2,
				number_of_gets: 2,
				number_of_puts: 2,
			},
		)];
		let payees1 = vec![(
			node_key,
			NodeUsage {
				transferred_bytes: 3,
				stored_bytes: 3,
				number_of_gets: 3,
				number_of_puts: 3,
			},
		)];

		let cluster_usage = NodeUsage::default();

		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		let (_, payees1_batch_proof, payees_root) = hash_node_payable_usage_batch(payees1.clone());

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			&cluster_usage,
		);
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			cluster_usage,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id,
			era,
			fingerprint
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
			DdcPayouts::end_billing_report(RuntimeOrigin::signed(root_account), cluster_id, era,),
=======
			DdcPayouts::end_billing_report(
				RuntimeOrigin::signed(AccountId::from([0; 32])),
				cluster_id,
				era,
			),
>>>>>>> f82a1179 (chore: unused account is removed from tests)
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::root(), cluster_id, era,),
			BadOrigin
		);

		assert_noop!(
			DdcPayouts::end_billing_report(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			DdcPayouts::end_billing_report(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers1,
=======
			payers1,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			0,
			&payers1,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
			DdcPayouts::end_billing_report(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index + 1,
<<<<<<< HEAD
			&payers2,
=======
			payers2,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
=======
			payers_proofs.get(0).unwrap().clone(),
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			1,
			&payers2,
<<<<<<< HEAD
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
=======
			payers_proofs.get(1).unwrap().clone(),
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
			DdcPayouts::end_billing_report(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

<<<<<<< HEAD
		assert_ok!(DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_rewarding_batch_index
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			DdcPayouts::end_billing_report(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payees,
=======
			payees.clone(),
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
<<<<<<< HEAD
			batch_index,
			&payees,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
			DdcPayouts::end_billing_report(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
				cluster_id,
				era,
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index + 1,
<<<<<<< HEAD
			&payees,
=======
			payees,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			batch_index + 1,
			&payees,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			MMRProof::default(),
		));

		assert_noop!(
<<<<<<< HEAD
<<<<<<< HEAD
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
=======
			DdcPayouts::end_billing_report(
				RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
				cluster_id,
				era,
			),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,),
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
=======
			0,
			&payees1,
			payees1_batch_proof,
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era),
			Error::<Test>::NotExpectedState
		);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era),
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
			Error::<Test>::NotExpectedState
		);
	})
}

#[test]
fn end_billing_report_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let dac_account = AccountId::from([2; 32]);
<<<<<<< HEAD
		let user1 = AccountId::from([3; 32]);
<<<<<<< HEAD
		let node1 = AccountId::from([33; 32]);
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		let cluster_id = ClusterId::from([12; 20]);
<<<<<<< HEAD
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
=======
		let node_id =
			String::from("0x302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395");
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
=======
>>>>>>> fd642396 (chore: unused account is removed from tests)
		let cluster_id = ClusterId::from([12; 20]);
		let node_key = NodePubKey::StoragePubKey(NODE1_PUB_KEY_32);
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
		let era = 100;
<<<<<<< HEAD
		let max_batch_index = 0;
		let batch_index = 0;
		let total_node_usage = NodeUsage::default();
<<<<<<< HEAD
<<<<<<< HEAD
=======
		let max_charging_batch_index = 0;
		let max_rewarding_batch_index = 0;
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		let bucket_id3 = BUCKET_ID3;
		let payers = vec![(bucket_id3, BucketUsage::default())];
		let payees = vec![(node_key.clone(), NodeUsage::default())];
<<<<<<< HEAD
=======
		let bucket_id1 = 1;
<<<<<<< HEAD
		let payers = vec![(user1, bucket_id1, CustomerUsage::default())];
		let payees = vec![(node1, NodeUsage::default())];
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		let payers = vec![(user1, node_id.clone(), bucket_id1, CustomerUsage::default())];
<<<<<<< HEAD
		let payees = vec![(node1, node_id, NodeUsage::default())];
>>>>>>> 29f4f8d6 (fix: tests and benchmarks are fixed)
=======
		let payees = vec![(NodePubKey::StoragePubKey(NODE1_PUB_KEY_32), NodeUsage::default())];
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
=======
		let bucket_id3 = BUCKET_ID3;
		let payers = vec![(node_key.clone(), bucket_id3, BucketUsage::default())];
		let payees = vec![(node_key.clone(), NodeUsage::default())];
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
		let cluster_usage = NodeUsage::default();

		let (_, payers_batch_proof, payers_root) = hash_bucket_payable_usage_batch(payers.clone());
		let (_, payees_batch_proof, payees_root) = hash_node_payable_usage_batch(payees.clone());

		let fingerprint = get_fingerprint(
			&cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			&cluster_usage,
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_billing_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			era,
			start_era,
			end_era,
			payers_root,
			payees_root,
			cluster_usage,
		));
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
<<<<<<< HEAD
			cluster_id, era, start_era, end_era,
=======
		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account.clone()));

=======
>>>>>>> c0ba34cb (chore: deprecated extrinsic is removed from tests)
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
			start_era,
			end_era,
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_billing_report(
			cluster_id, era, start_era, end_era,
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
=======
			cluster_id,
			era,
			fingerprint
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		));

		let report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, PayoutState::Initialized);

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
=======
		assert_ok!(DdcPayouts::begin_charging_customers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_charging_batch_index,
		));

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
=======
		assert_ok!(DdcPayouts::send_charging_customers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payers,
=======
			payers,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
			MMRProof::default(),
		));

<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
=======
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
		));

		assert_ok!(DdcPayouts::begin_rewarding_providers(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			0,
			&payers,
			payers_batch_proof,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
			cluster_id,
			era,
			max_rewarding_batch_index,
		));

<<<<<<< HEAD
<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
=======
		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
<<<<<<< HEAD
			RuntimeOrigin::signed(dac_account.clone()),
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))
=======
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
>>>>>>> fd642396 (chore: unused account is removed from tests)
			cluster_id,
			era,
			batch_index,
<<<<<<< HEAD
			&payees,
=======
			payees,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
			MMRProof::default(),
		));

<<<<<<< HEAD
		assert_ok!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,)
		);
=======
		assert_ok!(DdcPayouts::end_rewarding_providers(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32.clone()),
			cluster_id,
			era,
		));
>>>>>>> 32dbf724 (Added Logs and Events for OCW-DAC Validation (#411))

<<<<<<< HEAD
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,));
=======
		assert_ok!(DdcPayouts::end_billing_report(
			RuntimeOrigin::signed(VALIDATOR_OCW_KEY_32),
			cluster_id,
			era,
		));
>>>>>>> fd642396 (chore: unused account is removed from tests)
=======
		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			0,
			&payees,
			payees_batch_proof,
		));

		assert_ok!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,)
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_billing_report(cluster_id, era,));
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)

		System::assert_last_event(Event::BillingReportFinalized { cluster_id, era }.into());

		let report_end = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert!(report_end.rewarding_processed_batches.is_empty());
		assert!(report_end.charging_processed_batches.is_empty());
		assert_eq!(report_end.state, PayoutState::Finalized);
	})
}
