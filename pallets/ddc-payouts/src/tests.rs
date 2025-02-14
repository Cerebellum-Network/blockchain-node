#![allow(clippy::get_first)]
//! Tests for the module.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ddc_primitives::{BucketId, BucketUsage, ClusterId, Fingerprint, PayableUsageHash};
use frame_support::{assert_noop, assert_ok, storage::unhashed::get, traits::Randomness};
use polkadot_ckb_merkle_mountain_range::{
	util::{MemMMR, MemStore},
	MMR,
};
use sp_core::{ByteArray, H256};
use sp_io::hashing::blake2_256;
use sp_runtime::{PerThing, Perquintill};

use super::{mock::*, *};
use crate::migrations::v3::PayoutFingerprint;

fn get_fingerprint(
	cluster_id: &ClusterId,
	ehd_id: String,
	payers_merkle_root: PayableUsageHash,
	payees_merkle_root: PayableUsageHash,
) -> Fingerprint {
	let fingerprint = PayoutFingerprint::<AccountId> {
		cluster_id: *cluster_id,
		ehd_id,
		payers_merkle_root,
		payees_merkle_root,
		validators: Default::default(),
	};

	fingerprint.selective_hash::<Test>()
}

fn hash_bucket_payable_usage_batch(usages: Vec<(AccountId, u128)>) -> (H256, MMRProof, H256) {
	if usages.len() > MAX_PAYOUT_BATCH_SIZE.into() {
		panic!("Batch size is reached")
	}

	let store1 = MemStore::default();
	let mut mmr1: MMR<PayableUsageHash, MergeMMRHash, &MemStore<PayableUsageHash>> =
		MemMMR::<PayableUsageHash, MergeMMRHash>::new(0, &store1);

	for (customer_id, amount) in usages {
		// let mut data = usage.0.encode(); // bucket_id
		// data.extend_from_slice(&usage.1.stored_bytes.encode());
		// data.extend_from_slice(&usage.1.transferred_bytes.encode());
		// data.extend_from_slice(&usage.1.number_of_puts.encode());
		// data.extend_from_slice(&usage.1.number_of_gets.encode());
		let mut data = customer_id.encode();
		data.extend_from_slice(&amount.encode());
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

fn hash_node_payable_usage_batch(usages: Vec<(AccountId, u128)>) -> (H256, MMRProof, H256) {
	if usages.len() > MAX_PAYOUT_BATCH_SIZE.into() {
		panic!("Batch size is reached")
	}

	let store1 = MemStore::default();
	let mut mmr1: MMR<PayableUsageHash, MergeMMRHash, &MemStore<PayableUsageHash>> =
		MemMMR::<PayableUsageHash, MergeMMRHash>::new(0, &store1);

	for (provider_id, amount) in usages {
		// let mut data = usage.0.encode(); // node_key
		// data.extend_from_slice(&usage.1.stored_bytes.encode());
		// data.extend_from_slice(&usage.1.transferred_bytes.encode());
		// data.extend_from_slice(&usage.1.number_of_puts.encode());
		// data.extend_from_slice(&usage.1.number_of_gets.encode());
		let mut data = provider_id.encode();
		data.extend_from_slice(&amount.encode());

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
fn test_commit_payout_fingerprint_happy_path() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);

		let mut expected_validators = BTreeSet::new();
		expected_validators.insert(VALIDATOR1_ACCOUNT_ID.into());
		let expected_fingerprint = pallet::PayoutFingerprint::<sp_runtime::AccountId32> {
			ehd_id: ehd_id.clone(),
			cluster_id,
			payers_merkle_root: DEFAULT_PAYERS_ROOT,
			payees_merkle_root: DEFAULT_PAYEES_ROOT,
			validators: expected_validators,
		};
		let fingerprint_hash = expected_fingerprint.selective_hash::<Test>();

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
		));

		let stored_fingerprint = PayoutFingerprints::<Test>::get(&fingerprint_hash);
		assert_eq!(stored_fingerprint, Some(expected_fingerprint));
	});
}

//TODO: HAs to be tested with whole data
// #[test]
// fn test_commit_payout_fingerprint_error_era_id_ge_last_paid_era() {
//     ExtBuilder.build_and_execute(|| {
//         System::set_block_number(1);
//         let collector_id = [1u8; 32];
//         let cluster_id = ClusterId::from([12; 20]);
//         let era = 100;
//         let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id),era);
//         assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
//             VALIDATOR1_ACCOUNT_ID.into(),
//             cluster_id,
//             ehd_id.clone(),
//             DEFAULT_PAYERS_ROOT,
//             DEFAULT_PAYEES_ROOT,
//         ));
//         let previous_era = 99;
//         let previous_ehd_id = format!("{:?}-0x{}-{}", cluster_id,
// hex::encode(collector_id),previous_era);         assert_noop!(
//             <DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
//                 VALIDATOR1_ACCOUNT_ID.into(),
//                 cluster_id,
//                 previous_ehd_id,
//                 DEFAULT_PAYERS_ROOT,
//                 DEFAULT_PAYEES_ROOT,
//             ),
//             Error::<Test>::BadRequest
//         );
//     });
// }

#[test]
fn test_commit_payout_fingerprint_error_payout_fingerprint_committed() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
				VALIDATOR1_ACCOUNT_ID.into(),
				cluster_id,
				ehd_id,
				DEFAULT_PAYERS_ROOT,
				DEFAULT_PAYEES_ROOT,
			),
			Error::<Test>::PayoutFingerprintCommitted
		);
	});
}

#[test]
fn test_begin_payout_happy_path() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), DEFAULT_PAYERS_ROOT, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		let expected_payout = PayoutReceipt::<Test> {
			vault: DdcPayouts::account_id(),
			fingerprint,
			state: PayoutState::Initialized,
			..Default::default()
		};
		let actual_payout = PayoutReceipts::<Test>::get(&cluster_id, era);

		assert_eq!(expected_payout, actual_payout.unwrap());
	});
}

//FIXME: this should pass
#[ignore]
#[test]
fn test_begin_payout_error_without_payout_receipt() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 101;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), DEFAULT_PAYERS_ROOT, DEFAULT_PAYEES_ROOT);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::begin_payout(cluster_id, era, fingerprint,),
			Error::<Test>::NotExpectedState
		);
	});
}

#[test]
fn test_begin_payout_error_with_wrong_fingerprint() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
		));
		let wrong_ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era + 1);
		let wrong_fingerprint = get_fingerprint(
			&cluster_id,
			wrong_ehd_id.clone(),
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
		);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::begin_payout(cluster_id, era, wrong_fingerprint,),
			Error::<Test>::PayoutFingerprintDoesNotExist
		);
	});
}

#[test]
fn test_begin_charging_customers_happy_path() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), DEFAULT_PAYERS_ROOT, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		let expected_payout = PayoutReceipt::<Test> {
			vault: DdcPayouts::account_id(),
			fingerprint,
			state: PayoutState::ChargingCustomers,
			charging_max_batch_index: max_charging_batch_index,
			..Default::default()
		};
		let actual_payout = PayoutReceipts::<Test>::get(&cluster_id, era);

		assert_eq!(expected_payout, actual_payout.unwrap());
	});
}

#[test]
fn test_begin_charging_customers_error_batch_index_overflow() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2000;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), DEFAULT_PAYERS_ROOT, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
				cluster_id,
				era,
				max_charging_batch_index,
			),
			Error::<Test>::BatchIndexOverflow
		);
	});
}

#[test]
fn test_begin_charging_customers_error_payout_receipt_does_not_exist() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), DEFAULT_PAYERS_ROOT, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		let wrong_era_ud = 101;

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
				cluster_id,
				wrong_era_ud,
				max_charging_batch_index,
			),
			Error::<Test>::PayoutReceiptDoesNotExist
		);
	});
}

#[test]
fn test_begin_charging_customers_error_not_expected_state() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), DEFAULT_PAYERS_ROOT, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		let mut actual_payout = PayoutReceipts::<Test>::get(&cluster_id, era);
		actual_payout.as_mut().unwrap().state = PayoutState::NotInitialized;
		PayoutReceipts::<Test>::insert(&cluster_id, era, actual_payout.unwrap());

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
				cluster_id,
				era,
				max_charging_batch_index,
			),
			Error::<Test>::NotExpectedState
		);
	});
}

#[test]
fn test_send_charging_customers_batch_happy_path() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			0,
			&payers1,
			payers_proofs.get(0).unwrap().clone(),
		));
	});
}

fn test_send_charging_customers_batch_error_batch_size_out_of_index() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				3,
				&payers1,
				payers_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::BatchSizeIsOutOfBounds
		);
	});
}

#[test]
fn test_send_charging_customers_batch_error_payout_receipt_does_not_exist() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		let wrong_era_ud = 101;

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				wrong_era_ud,
				0,
				&payers1,
				payers_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::PayoutReceiptDoesNotExist
		);
	});
}

#[test]
fn test_send_charging_customers_batch_error_not_expected_state() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		let mut actual_payout = PayoutReceipts::<Test>::get(&cluster_id, era);
		actual_payout.as_mut().unwrap().state = PayoutState::NotInitialized;
		PayoutReceipts::<Test>::insert(&cluster_id, era, actual_payout.unwrap());

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				0,
				&payers1,
				payers_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::NotExpectedState
		);
	});
}

#[test]
fn test_send_charging_customers_batch_error_batch_index_is_out_of_range() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id, era, 2,
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				3,
				&payers1,
				payers_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::BatchIndexIsOutOfRange
		);
	});
}

#[test]
fn test_send_charging_customers_batch_error_batch_index_already_processed() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id, era, 2,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			0,
			&payers1,
			payers_proofs.get(0).unwrap().clone(),
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				0,
				&payers1,
				payers_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::BatchIndexAlreadyProcessed
		);
	});
}

#[test]
fn test_send_charging_customers_batch_error_payout_fingerprint_does_not_exist() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				0,
				&payers1,
				payers_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::PayoutReceiptDoesNotExist
		);
	});
}

#[test]
fn test_send_charging_customers_batch_error_failed_to_verify_merkle_proof() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let collector_id = [1u8; 32];
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id, era, 2,
		));

		let wrong_payers_proofs = vec![MMRProof { proof: vec![] }];

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
				cluster_id,
				era,
				0,
				&payers1,
				wrong_payers_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::FailedToVerifyMerkleProof
		);
	});
}

#[ignore]
#[test]
fn test_end_charging_customers_happy_path() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			0,
			&payers1,
			payers_proofs.get(0).unwrap().clone(),
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_charging_customers_batch(
			cluster_id,
			era,
			1,
			&payers1,
			payers_proofs.get(1).unwrap().clone(),
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_charging_customers(cluster_id, era,));

		let expected_payout = PayoutReceipt::<Test> {
			vault: DdcPayouts::account_id(),
			fingerprint,
			state: PayoutState::CustomersChargedWithFees,
			charging_max_batch_index: max_charging_batch_index,
			..Default::default()
		};
		let actual_payout = PayoutReceipts::<Test>::get(&cluster_id, era);

		assert_eq!(expected_payout, actual_payout.unwrap());
	})
}

#[test]
fn test_begin_rewarding_providers_happy_path() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		let mut payout = PayoutReceipt::<Test> {
			vault: DdcPayouts::account_id(),
			fingerprint,
			state: PayoutState::CustomersChargedWithFees,
			charging_max_batch_index: max_charging_batch_index,
			..Default::default()
		};
		payout.state = PayoutState::CustomersChargedWithFees;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
			cluster_id,
			era,
			max_charging_batch_index
		));
	});
}

#[test]
fn test_begin_rewarding_providers_error_batch_index_overflow() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		let mut payout = PayoutReceipt::<Test> {
			vault: DdcPayouts::account_id(),
			fingerprint,
			state: PayoutState::CustomersChargedWithFees,
			charging_max_batch_index: max_charging_batch_index,
			..Default::default()
		};
		payout.state = PayoutState::CustomersChargedWithFees;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(cluster_id, era, 3000),
			Error::<Test>::BatchIndexOverflow
		);
	});
}

#[test]
fn test_begin_rewarding_providers_error_payout_receipt_does_not_exist() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let max_charging_batch_index = 2;

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
				cluster_id,
				era,
				max_charging_batch_index
			),
			Error::<Test>::PayoutReceiptDoesNotExist
		);
	});
}

#[test]
fn test_begin_rewarding_providers_error_not_expected_state() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let max_charging_batch_index = 2;
		let fingerprint = get_fingerprint(
			&cluster_id,
			"ehd_id".to_string(),
			DEFAULT_PAYERS_ROOT,
			DEFAULT_PAYEES_ROOT,
		);

		let mut payout = PayoutReceipt::<Test> {
			vault: DdcPayouts::account_id(),
			fingerprint,
			state: PayoutState::NotInitialized,
			charging_max_batch_index: 0,
			..Default::default()
		};
		payout.state = PayoutState::NotInitialized;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
				cluster_id,
				era,
				max_charging_batch_index
			),
			Error::<Test>::NotExpectedState
		);
	});
}

#[test]
fn test_send_rewarding_providers_batch_happy_path() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);
		let payees1 = vec![(AccountId32::new([3u8; 32]), 100)];
		let payees2 = vec![(AccountId32::new([4u8; 32]), 100)];
		let (payees1_batch_root, _, _) = hash_bucket_payable_usage_batch(payees1.clone());
		let (payees2_batch_root, _, _) = hash_bucket_payable_usage_batch(payees2.clone());
		let (payees_root, payees_proofs) =
			get_root_with_proofs(vec![payees1_batch_root, payees2_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			payees_root,
		));

		let fingerprint = get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, payees_root);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		let mut payout = PayoutReceipts::<Test>::get(&cluster_id, era).unwrap();
		payout.state = PayoutState::CustomersChargedWithFees;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
			cluster_id,
			era,
			max_charging_batch_index
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			0,
			&payees1,
			payees_proofs.get(0).unwrap().clone(),
		));
	});
}

#[test]
fn test_send_rewarding_providers_batch_error_batch_size_is_out_of_bounds() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) = get_root_with_proofs(vec![payers1_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			DEFAULT_PAYEES_ROOT,
		));

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		let mut payout = PayoutReceipt::<Test> {
			vault: DdcPayouts::account_id(),
			fingerprint,
			state: PayoutState::CustomersChargedWithFees,
			charging_max_batch_index: max_charging_batch_index,
			..Default::default()
		};
		payout.state = PayoutState::CustomersChargedWithFees;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				0,
				&vec![],
				payers_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::BatchSizeIsOutOfBounds
		);
	});
}

#[test]
fn test_send_rewarding_providers_batch_error_payout_receipt_does_not_exist() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let payees1 = vec![(AccountId32::new([3u8; 32]), 100)];
		let (payees1_batch_root, _, _) = hash_bucket_payable_usage_batch(payees1.clone());
		let (payees_root, payees_proofs) = get_root_with_proofs(vec![payees1_batch_root]);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				0,
				&payees1,
				payees_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::PayoutReceiptDoesNotExist
		);
	});
}

#[test]
fn test_send_rewarding_providers_batch_error_not_expected_state() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) = get_root_with_proofs(vec![payers1_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		let mut payout = PayoutReceipt::<Test> {
			vault: DdcPayouts::account_id(),
			fingerprint,
			state: PayoutState::NotInitialized,
			charging_max_batch_index: 0,
			..Default::default()
		};
		payout.state = PayoutState::NotInitialized;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				0,
				&payers1,
				payers_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::NotExpectedState
		);
	});
}

#[test]
fn test_send_rewarding_providers_batch_error_batch_index_is_out_of_range() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers_root, payers_proofs) = get_root_with_proofs(vec![payers1_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;

		let fingerprint =
			get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, DEFAULT_PAYEES_ROOT);

		let mut payout = PayoutReceipt::<Test> {
			vault: DdcPayouts::account_id(),
			fingerprint,
			state: PayoutState::CustomersChargedWithFees,
			charging_max_batch_index: max_charging_batch_index,
			..Default::default()
		};
		payout.state = PayoutState::RewardingProviders;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				3,
				&payers1,
				payers_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::BatchIndexIsOutOfRange
		);
	});
}

#[test]
fn test_send_rewarding_providers_batch_error_batch_index_already_processed() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);
		let payees1 = vec![(AccountId32::new([3u8; 32]), 100)];
		let payees2 = vec![(AccountId32::new([4u8; 32]), 100)];
		let (payees1_batch_root, _, _) = hash_bucket_payable_usage_batch(payees1.clone());
		let (payees2_batch_root, _, _) = hash_bucket_payable_usage_batch(payees2.clone());
		let (payees_root, payees_proofs) =
			get_root_with_proofs(vec![payees1_batch_root, payees2_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 2;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			payees_root,
		));

		let fingerprint = get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, payees_root);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		let mut payout = PayoutReceipts::<Test>::get(&cluster_id, era).unwrap();
		payout.state = PayoutState::CustomersChargedWithFees;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
			cluster_id,
			era,
			max_charging_batch_index
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			0,
			&payees1,
			payees_proofs.get(0).unwrap().clone(),
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				0,
				&payees1,
				payees_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::BatchIndexAlreadyProcessed
		);
	});
}

#[test]
fn test_send_rewarding_providers_batch_error_payout_fingerprint_does_not_exist() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let payees1 = vec![(AccountId32::new([3u8; 32]), 100)];
		let (payees1_batch_root, _, _) = hash_bucket_payable_usage_batch(payees1.clone());
		let (payees_root, payees_proofs) = get_root_with_proofs(vec![payees1_batch_root]);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
				cluster_id,
				era,
				0,
				&payees1,
				payees_proofs.get(0).unwrap().clone(),
			),
			Error::<Test>::PayoutReceiptDoesNotExist
		);
	});
}

#[test]
fn test_end_rewarding_providers_happy_path() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);
		let payees1 = vec![(AccountId32::new([3u8; 32]), 100)];
		let payees2 = vec![(AccountId32::new([4u8; 32]), 100)];
		let (payees1_batch_root, _, _) = hash_bucket_payable_usage_batch(payees1.clone());
		let (payees2_batch_root, _, _) = hash_bucket_payable_usage_batch(payees2.clone());
		let (payees_root, payees_proofs) =
			get_root_with_proofs(vec![payees1_batch_root, payees2_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 1;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			payees_root,
		));

		let fingerprint = get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, payees_root);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		let mut payout = PayoutReceipts::<Test>::get(&cluster_id, era).unwrap();
		payout.state = PayoutState::CustomersChargedWithFees;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
			cluster_id,
			era,
			max_charging_batch_index
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			0,
			&payees1,
			payees_proofs.get(0).unwrap().clone(),
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			1,
			&payees2,
			payees_proofs.get(1).unwrap().clone(),
		));

		assert_ok!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,)
		);
	});
}

#[test]
fn test_end_rewarding_providers_error_not_expected_state() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);
		let payees1 = vec![(AccountId32::new([3u8; 32]), 100)];
		let payees2 = vec![(AccountId32::new([4u8; 32]), 100)];
		let (payees1_batch_root, _, _) = hash_bucket_payable_usage_batch(payees1.clone());
		let (payees2_batch_root, _, _) = hash_bucket_payable_usage_batch(payees2.clone());
		let (payees_root, payees_proofs) =
			get_root_with_proofs(vec![payees1_batch_root, payees2_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 1;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			payees_root,
		));

		let fingerprint = get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, payees_root);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		let mut payout = PayoutReceipts::<Test>::get(&cluster_id, era).unwrap();
		payout.state = PayoutState::CustomersChargedWithFees;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::NotExpectedState
		);
	});
}

#[test]
fn test_end_rewarding_providers_validation_issue() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);
		let payees1 = vec![(AccountId32::new([3u8; 32]), 100)];
		let payees2 = vec![(AccountId32::new([4u8; 32]), 100)];
		let (payees1_batch_root, _, _) = hash_bucket_payable_usage_batch(payees1.clone());
		let (payees2_batch_root, _, _) = hash_bucket_payable_usage_batch(payees2.clone());
		let (payees_root, payees_proofs) =
			get_root_with_proofs(vec![payees1_batch_root, payees2_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 1;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			payees_root,
		));

		let fingerprint = get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, payees_root);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		let mut payout = PayoutReceipts::<Test>::get(&cluster_id, era).unwrap();
		payout.state = PayoutState::CustomersChargedWithFees;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
			cluster_id,
			era,
			max_charging_batch_index
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			0,
			&payees1,
			payees_proofs.get(0).unwrap().clone(),
		));

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,),
			Error::<Test>::BatchesMissed
		);
	});
}

#[test]
fn test_end_payout_happy_path() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;
		let user1_account = AccountId32::new([1u8; 32]);
		let user1_amount = 100;
		let user2_account = AccountId32::new([2u8; 32]);
		let user2_amount = 100;
		let payers1 = vec![(user1_account, user1_amount)];
		let payers2 = vec![(user2_account, user2_amount)];
		let (payers1_batch_root, _, _) = hash_bucket_payable_usage_batch(payers1.clone());
		let (payers2_batch_root, _, _) = hash_bucket_payable_usage_batch(payers2.clone());
		let (payers_root, payers_proofs) =
			get_root_with_proofs(vec![payers1_batch_root, payers2_batch_root]);
		let payees1 = vec![(AccountId32::new([3u8; 32]), 100)];
		let payees2 = vec![(AccountId32::new([4u8; 32]), 100)];
		let (payees1_batch_root, _, _) = hash_bucket_payable_usage_batch(payees1.clone());
		let (payees2_batch_root, _, _) = hash_bucket_payable_usage_batch(payees2.clone());
		let (payees_root, payees_proofs) =
			get_root_with_proofs(vec![payees1_batch_root, payees2_batch_root]);

		let collector_id = [1u8; 32];
		let ehd_id = format!("{:?}-0x{}-{}", cluster_id, hex::encode(collector_id), era);
		let max_charging_batch_index = 1;

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::commit_payout_fingerprint(
			VALIDATOR1_ACCOUNT_ID.into(),
			cluster_id,
			ehd_id.clone(),
			payers_root,
			payees_root,
		));

		let fingerprint = get_fingerprint(&cluster_id, ehd_id.clone(), payers_root, payees_root);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_payout(
			cluster_id,
			era,
			fingerprint,
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_charging_customers(
			cluster_id,
			era,
			max_charging_batch_index,
		));

		let mut payout = PayoutReceipts::<Test>::get(&cluster_id, era).unwrap();
		payout.state = PayoutState::CustomersChargedWithFees;
		PayoutReceipts::<Test>::insert(&cluster_id, era, payout);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::begin_rewarding_providers(
			cluster_id,
			era,
			max_charging_batch_index
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			0,
			&payees1,
			payees_proofs.get(0).unwrap().clone(),
		));

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::send_rewarding_providers_batch(
			cluster_id,
			era,
			1,
			&payees2,
			payees_proofs.get(1).unwrap().clone(),
		));

		assert_ok!(
			<DdcPayouts as PayoutProcessor<Test>>::end_rewarding_providers(cluster_id, era,)
		);

		assert_ok!(<DdcPayouts as PayoutProcessor<Test>>::end_payout(cluster_id, era,));
	});
}

#[test]
fn test_end_payout_error_payout_receipt_does_not_exist() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		let cluster_id = ClusterId::from([1u8; 20]);
		let era = 100;

		assert_noop!(
			<DdcPayouts as PayoutProcessor<Test>>::end_payout(cluster_id, era,),
			Error::<Test>::PayoutReceiptDoesNotExist
		);
	});
}
