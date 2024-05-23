use ddc_primitives::ClusterId;
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

use crate::{mock::*, Error, Event};

#[test]
fn create_billing_reports_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let dac_account = 2u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let merkel_root_hash: H256 = array_bytes::hex_n_into_unchecked(
			"95803defe6ea9f41e7ec6afa497064f21bfded027d8812efacbdf984e630cbdc",
		);

		assert_ok!(DdcVerification::create_billing_reports(
			RuntimeOrigin::signed(dac_account),
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
