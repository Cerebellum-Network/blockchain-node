use ddc_primitives::ClusterId;
use frame_support::{assert_noop, assert_ok};
use sp_core::{Pair, H256};

use crate::{mock::*, Error, Event, *};

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
		));

		System::assert_last_event(Event::BillingReportCreated { cluster_id, era }.into());

		let report = DdcVerification::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.merkle_root_hash, merkel_root_hash);

		assert_noop!(
			DdcVerification::create_billing_reports(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				merkel_root_hash,
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

		let validator_one: <Test as Config>::AuthorityId = pair1.public().into();
		let validator_two: <Test as Config>::AuthorityId = pair2.public().into();
		let validator_three: <Test as Config>::AuthorityId = pair3.public().into();
		let validator_four: <Test as Config>::AuthorityId = pair4.public().into();
		let validator_six: <Test as Config>::AuthorityId = pair6.public().into();

		ValidatorSet::<Test>::put(vec![
			validator_one,
			validator_two,
			validator_three,
			validator_four,
			validator_six,
		]);

		let account_id1 = AccountId::from(pair1.public().0);
		let account_id2 = AccountId::from(pair2.public().0);
		let account_id3 = AccountId::from(pair3.public().0);
		let account_id4 = AccountId::from(pair4.public().0);
		let account_id6 = AccountId::from(pair6.public().0);
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
