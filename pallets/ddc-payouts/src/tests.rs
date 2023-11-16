//! Tests for the module.

use super::{mock::*, *};
use ddc_primitives::ClusterId;
use frame_support::{assert_noop, assert_ok, error::BadOrigin};

#[test]
fn set_authorised_caller_works() {
	ExtBuilder.build_and_execute(|| {
		let root_account = 1u64;
		let dac_account = 2u64;

		assert_noop!(
			DdcPayouts::set_authorised_caller(RuntimeOrigin::signed(root_account), dac_account),
			BadOrigin
		);

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_eq!(DdcPayouts::authorised_caller().unwrap(), dac_account);
	})
}

#[test]
fn begin_billing_report_fails_for_unauthorised() {
	ExtBuilder.build_and_execute(|| {
		let root_account = 1u64;
		let dac_account = 2u64;
		let cluster_id = ClusterId::from([1; 20]);
		let era = 100;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_noop!(
			DdcPayouts::begin_billing_report(
				RuntimeOrigin::signed(dac_account + 1),
				cluster_id,
				era,
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::begin_billing_report(RuntimeOrigin::signed(root_account), cluster_id, era,),
			Error::<Test>::Unauthorised
		);
	})
}

#[test]
fn begin_billing_report_works() {
	ExtBuilder.build_and_execute(|| {
		let dac_account = 2u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		let report = DdcPayouts::active_billing_reports(cluster_id, era);
		assert_eq!(report.state, State::Initialized);
	})
}

#[test]
fn begin_charging_customers_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
		let dac_account = 2u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;

		assert_noop!(
			DdcPayouts::begin_charging_customers(
				RuntimeOrigin::signed(dac_account),
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

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_noop!(
			DdcPayouts::begin_charging_customers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				max_batch_index,
			),
			Error::<Test>::BillingReportDoesNotExist
		);
	})
}

#[test]
fn begin_charging_customers_works() {
	ExtBuilder.build_and_execute(|| {
		let dac_account = 2u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
		));

		let report = DdcPayouts::active_billing_reports(cluster_id, era);
		assert_eq!(report.state, State::ChargingCustomers);
		assert_eq!(report.charging_max_batch_index, max_batch_index);
	})
}

#[test]
fn send_charging_customers_batch_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
		let root_account = 1u64;
		let dac_account = 2u64;
		let user1 = 3u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let batch_index = 1;
		let payers = vec![(user1, CustomerUsage::default())];

		assert_noop!(
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(root_account),
				cluster_id,
				era,
				batch_index,
				payers.clone(),
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::root(),
				cluster_id,
				era,
				batch_index,
				payers.clone(),
			),
			BadOrigin
		);

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_noop!(
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_index,
				payers.clone(),
			),
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_noop!(
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_index,
				payers,
			),
			Error::<Test>::NotExpectedState
		);
	})
}

#[test]
fn end_charging_customers_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
		let root_account = 1u64;
		let dac_account = 2u64;
		let user1 = 3u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;
		let batch_index = 1;
		let payers = vec![(user1, CustomerUsage::default())];

		assert_noop!(
			DdcPayouts::end_charging_customers(
				RuntimeOrigin::signed(root_account),
				cluster_id,
				era,
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::end_charging_customers(RuntimeOrigin::root(), cluster_id, era,),
			BadOrigin
		);

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_noop!(
			DdcPayouts::end_charging_customers(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_noop!(
			DdcPayouts::end_charging_customers(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
		));

		assert_noop!(
			DdcPayouts::end_charging_customers(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::BatchesMissed
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers,
		));

		assert_noop!(
			DdcPayouts::end_charging_customers(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::BatchesMissed
		);
	})
}

#[test]
fn begin_rewarding_providers_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
		let root_account = 1u64;
		let dac_account = 2u64;
		let user1 = 3u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;
		let batch_index = 1;
		let payers = vec![(user1, CustomerUsage::default())];
		let node_usage = NodeUsage::default();

		assert_noop!(
			DdcPayouts::begin_rewarding_providers(
				RuntimeOrigin::signed(root_account),
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

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_noop!(
			DdcPayouts::begin_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				max_batch_index,
				node_usage.clone(),
			),
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_noop!(
			DdcPayouts::begin_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				max_batch_index,
				node_usage.clone(),
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
		));

		assert_noop!(
			DdcPayouts::begin_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				max_batch_index,
				node_usage.clone(),
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers.clone(),
		));

		assert_noop!(
			DdcPayouts::begin_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				max_batch_index,
				node_usage.clone(),
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index + 1,
			payers,
		));

		assert_noop!(
			DdcPayouts::begin_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				max_batch_index,
				node_usage,
			),
			Error::<Test>::NotExpectedState
		);
	})
}

#[test]
fn send_rewarding_providers_batch_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
		let root_account = 1u64;
		let dac_account = 2u64;
		let user1 = 3u64;
		let user2 = 4u64;
		let node1 = 33u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;
		let batch_index = 0;
		let payers1 = vec![(user1, CustomerUsage::default())];
		let payers2 = vec![(user2, CustomerUsage::default())];
		let payees = vec![(node1, NodeUsage::default())];

		assert_noop!(
			DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(root_account),
				cluster_id,
				era,
				batch_index,
				payees.clone(),
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
			),
			BadOrigin
		);

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_noop!(
			DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_index,
				payees.clone(),
			),
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_noop!(
			DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_index,
				payees.clone(),
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
		));

		assert_noop!(
			DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_index,
				payees.clone(),
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers1,
		));

		assert_noop!(
			DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_index,
				payees.clone(),
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index + 1,
			payers2,
		));

		assert_noop!(
			DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_index,
				payees.clone(),
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_noop!(
			DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_index,
				payees,
			),
			Error::<Test>::NotExpectedState
		);
	})
}

#[test]
fn end_rewarding_providers_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
		let root_account = 1u64;
		let dac_account = 2u64;
		let user1 = 3u64;
		let user2 = 4u64;
		let node1 = 33u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;
		let batch_index = 0;
		let payers1 = vec![(user1, CustomerUsage::default())];
		let payers2 = vec![(user2, CustomerUsage::default())];
		let payees = vec![(node1, NodeUsage::default())];
		let total_node_usage = NodeUsage::default();

		assert_noop!(
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(root_account),
				cluster_id,
				era,
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::end_rewarding_providers(RuntimeOrigin::root(), cluster_id, era,),
			BadOrigin
		);

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_noop!(
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
			),
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_noop!(
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
		));

		assert_noop!(
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers1,
		));

		assert_noop!(
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index + 1,
			payers2,
		));

		assert_noop!(
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_noop!(
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::begin_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
			total_node_usage,
		));

		assert_noop!(
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
			),
			Error::<Test>::BatchesMissed
		);

		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payees,
		));

		assert_noop!(
			DdcPayouts::end_rewarding_providers(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
			),
			Error::<Test>::BatchesMissed
		);
	})
}

#[test]
fn end_billing_report_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
		let root_account = 1u64;
		let dac_account = 2u64;
		let user1 = 3u64;
		let user2 = 4u64;
		let node1 = 33u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;
		let batch_index = 0;
		let payers1 = vec![(user1, CustomerUsage::default())];
		let payers2 = vec![(user2, CustomerUsage::default())];
		let payees = vec![(node1, NodeUsage::default())];
		let total_node_usage = NodeUsage::default();

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::signed(root_account), cluster_id, era,),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::root(), cluster_id, era,),
			BadOrigin
		);

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::BillingReportDoesNotExist
		);

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
		));

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers1,
		));

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index + 1,
			payers2,
		));

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::begin_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
			total_node_usage,
		));

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payees.clone(),
		));

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index + 1,
			payees,
		));

		assert_noop!(
			DdcPayouts::end_billing_report(RuntimeOrigin::signed(dac_account), cluster_id, era,),
			Error::<Test>::NotExpectedState
		);
	})
}
