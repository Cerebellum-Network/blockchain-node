//! Tests for the module.

use ddc_primitives::ClusterId;
use frame_support::{assert_noop, assert_ok, error::BadOrigin};

use super::{mock::*, *};

#[test]
fn set_authorised_caller_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let root_account = 1u64;
		let dac_account = 2u64;

		assert_noop!(
			DdcPayouts::set_authorised_caller(RuntimeOrigin::signed(root_account), dac_account),
			BadOrigin
		);

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		System::assert_last_event(
			Event::AuthorisedCaller { authorised_caller: dac_account }.into(),
		);

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
		System::set_block_number(1);

		let dac_account = 2u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		System::assert_last_event(Event::BillingReportInitialized { cluster_id, era }.into());

		let report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
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
		System::set_block_number(1);

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

		System::assert_last_event(Event::ChargingStarted { cluster_id, era }.into());

		let report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
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
		let user2 = 4u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;
		let batch_index = 1;
		let payers1 = vec![(user1, CustomerUsage::default())];
		let payers2 = vec![(user2, CustomerUsage::default())];

		assert_noop!(
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(root_account),
				cluster_id,
				era,
				batch_index,
				payers1.clone(),
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
				payers1.clone(),
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
				payers1.clone(),
			),
			Error::<Test>::NotExpectedState
		);

		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
		));

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers1.clone(),
		));

		assert_noop!(
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_index,
				payers1,
			),
			Error::<Test>::BatchIndexAlreadyProcessed
		);

		assert_noop!(
			DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_index,
				payers2,
			),
			Error::<Test>::BatchIndexAlreadyProcessed
		);
	})
}

fn calculate_charge_parts(usage: CustomerUsage) -> CustomerCharge {
	CustomerCharge {
		transfer: PRICING_PARAMS.unit_per_mb_streamed * (usage.transferred_bytes as u128) /
			byte_unit::MEBIBYTE,
		storage: (PRICING_PARAMS.unit_per_mb_stored * usage.stored_bytes as u128) /
			byte_unit::MEBIBYTE,
		puts: PRICING_PARAMS.unit_per_put_request * usage.number_of_puts,
		gets: PRICING_PARAMS.unit_per_get_request * usage.number_of_gets,
	}
}

fn calculate_charge(usage: CustomerUsage) -> u128 {
	let charge = calculate_charge_parts(usage);
	charge.transfer + charge.storage + charge.puts + charge.gets
}

#[test]
fn send_charging_customers_batch_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let dac_account = 123u64;
		let user1 = 1u64;
		let user2_debtor = 2u64;
		let user3_debtor = 3u64;
		let user4 = 4u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 3;
		let mut batch_index = 0;
		let usage1 = CustomerUsage {
			// should pass without debt
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};
		let usage2 = CustomerUsage {
			// should fail as not enough balance
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage3 = CustomerUsage {
			// should pass but with debt (partial charge)
			transferred_bytes: 1,
			stored_bytes: 2,
			number_of_puts: 3,
			number_of_gets: 4,
		};
		let usage4 = CustomerUsage {
			// should pass without debt
			transferred_bytes: 467457,
			stored_bytes: 45674567456,
			number_of_puts: 3456345,
			number_of_gets: 242334563456423,
		};
		let payers1 = vec![(user2_debtor, usage2.clone()), (user4, usage4.clone())];
		let payers2 = vec![(user1, usage1.clone())];
		let payers3 = vec![(user3_debtor, usage3.clone())];

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
		assert_eq!(System::events().len(), 3);

		// batch 1
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers1,
		));

		let usage4_charge = calculate_charge(usage4.clone());
		let mut balance = Balances::free_balance(DdcPayouts::sub_account_id(cluster_id, era));
		assert_eq!(balance, usage4_charge);

		let user2_debt = DdcPayouts::debtor_customers(cluster_id, user2_debtor).unwrap();
		let mut debt = calculate_charge(usage2.clone());
		assert_eq!(user2_debt, debt);

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge2 = calculate_charge_parts(usage2);
		let charge4 = calculate_charge_parts(usage4);
		assert_eq!(charge2.puts + charge4.puts, report.total_customer_charge.puts);
		assert_eq!(charge2.gets + charge4.gets, report.total_customer_charge.gets);
		assert_eq!(charge2.storage + charge4.storage, report.total_customer_charge.storage);
		assert_eq!(charge2.transfer + charge4.transfer, report.total_customer_charge.transfer);

		System::assert_has_event(
			Event::ChargeFailed {
				cluster_id,
				era,
				customer_id: user2_debtor,
				batch_index,
				amount: debt,
			}
			.into(),
		);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
				customer_id: user2_debtor,
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
				batch_index,
				amount: usage4_charge,
			}
			.into(),
		);

		assert_eq!(System::events().len(), 5 + 3 + 1); // 3 for Currency::transfer

		// batch 2
		let mut before_total_customer_charge = report.total_customer_charge.clone();
		batch_index += 1;
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers2,
		));

		System::assert_last_event(
			Event::Charged {
				cluster_id,
				era,
				batch_index,
				customer_id: user1,
				amount: calculate_charge(usage1.clone()),
			}
			.into(),
		);

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge1 = calculate_charge_parts(usage1);
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

		assert_eq!(report.state, State::ChargingCustomers);
		let user1_debt = DdcPayouts::debtor_customers(cluster_id, user1);
		assert_eq!(user1_debt, None);

		let balance_before = Balances::free_balance(DdcPayouts::sub_account_id(cluster_id, era));

		// batch 3
		batch_index += 2;
		before_total_customer_charge = report.total_customer_charge.clone();
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers3,
		));

		let user3_charge = calculate_charge(usage3.clone());
		let charge3 = calculate_charge_parts(usage3);
		let ratio = Perbill::from_rational(PARTIAL_CHARGE, user3_charge);
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

		balance = Balances::free_balance(DdcPayouts::sub_account_id(cluster_id, era));
		assert_eq!(balance, balance_before + PARTIAL_CHARGE);

		let user3_debt = DdcPayouts::debtor_customers(cluster_id, user3_debtor).unwrap();
		debt = user3_charge - PARTIAL_CHARGE;
		assert_eq!(user3_debt, debt);

		System::assert_has_event(
			Event::Indebted {
				cluster_id,
				era,
				customer_id: user3_debtor,
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
				amount: user3_charge,
			}
			.into(),
		);
	})
}

#[test]
fn end_charging_customers_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
		let root_account = 100u64;
		let dac_account = 123u64;
		let user1 = 1u64;
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
fn end_charging_customers_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let dac_account = 123u64;
		let user1 = 1u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
		let usage1 = CustomerUsage {
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};
		let payers = vec![(user1, usage1.clone())];

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

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers,
		));

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge = calculate_charge(usage1);
		System::assert_last_event(
			Event::Charged { cluster_id, era, batch_index, customer_id: user1, amount: charge }
				.into(),
		);

		let mut balance = Balances::free_balance(DdcPayouts::sub_account_id(cluster_id, era));
		assert_eq!(balance, charge);
		assert_eq!(System::events().len(), 4 + 3); // 3 for Currency::transfer

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		System::assert_has_event(Event::ChargingFinished { cluster_id, era }.into());

		let treasury_fee = PRICING_FEES.treasury_share * charge;
		let reserve_fee = PRICING_FEES.cluster_reserve_share * charge;
		let validator_fee = PRICING_FEES.validators_share * charge;

		System::assert_has_event(
			Event::TreasuryFeesCollected { cluster_id, era, amount: treasury_fee }.into(),
		);

		System::assert_has_event(
			Event::ClusterReserveFeesCollected { cluster_id, era, amount: reserve_fee }.into(),
		);

		System::assert_has_event(
			Event::ValidatorFeesCollected { cluster_id, era, amount: validator_fee }.into(),
		);

		let transfers = 3 + 3 + 3 * 3; // for Currency::transfer
		assert_eq!(System::events().len(), 7 + 1 + 3 + transfers);

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report_after.state, State::CustomersChargedWithFees);

		let total_left_from_one = (PRICING_FEES.treasury_share +
			PRICING_FEES.validators_share +
			PRICING_FEES.cluster_reserve_share)
			.left_from_one();

		balance = Balances::free_balance(TREASURY_ACCOUNT_ID);
		assert_eq!(balance, PRICING_FEES.treasury_share * charge);

		balance = Balances::free_balance(RESERVE_ACCOUNT_ID);
		assert_eq!(balance, PRICING_FEES.cluster_reserve_share * charge);

		balance = Balances::free_balance(VALIDATOR1_ACCOUNT_ID);
		assert_eq!(balance, PRICING_FEES.validators_share * charge / 3);

		balance = Balances::free_balance(VALIDATOR2_ACCOUNT_ID);
		assert_eq!(balance, PRICING_FEES.validators_share * charge / 3);

		balance = Balances::free_balance(VALIDATOR3_ACCOUNT_ID);
		assert_eq!(balance, PRICING_FEES.validators_share * charge / 3);

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

		let dac_account = 123u64;
		let user1 = 1u64;
		let cluster_id = ClusterId::zero();
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
		let usage1 = CustomerUsage {
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};
		let payers = vec![(user1, usage1.clone())];

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

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers,
		));

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let charge = calculate_charge(usage1);
		System::assert_last_event(
			Event::Charged { cluster_id, era, customer_id: user1, batch_index, amount: charge }
				.into(),
		);

		let mut balance = Balances::free_balance(DdcPayouts::sub_account_id(cluster_id, era));
		assert_eq!(balance, charge);
		assert_eq!(System::events().len(), 4 + 3); // 3 for Currency::transfer

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		System::assert_has_event(Event::ChargingFinished { cluster_id, era }.into());
		assert_eq!(System::events().len(), 7 + 1);

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report_after.state, State::CustomersChargedWithFees);

		let fees = get_fees(&cluster_id).unwrap();

		let total_left_from_one =
			(fees.treasury_share + fees.validators_share + fees.cluster_reserve_share)
				.left_from_one();

		assert_eq!(total_left_from_one, Perbill::one());

		assert_eq!(fees.treasury_share, Perbill::zero());
		assert_eq!(fees.validators_share, Perbill::zero());
		assert_eq!(fees.cluster_reserve_share, Perbill::zero());

		balance = Balances::free_balance(TREASURY_ACCOUNT_ID);
		assert_eq!(balance, 0);

		balance = Balances::free_balance(RESERVE_ACCOUNT_ID);
		assert_eq!(balance, 0);

		balance = Balances::free_balance(VALIDATOR1_ACCOUNT_ID);
		assert_eq!(balance, 0);

		balance = Balances::free_balance(VALIDATOR2_ACCOUNT_ID);
		assert_eq!(balance, 0);

		balance = Balances::free_balance(VALIDATOR3_ACCOUNT_ID);
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
fn begin_rewarding_providers_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let dac_account = 123u64;
		let user1 = 1u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
		let total_node_usage = NodeUsage::default();
		let payers = vec![(user1, CustomerUsage::default())];

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, State::Initialized);

		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
		));

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers,
		));

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_ok!(DdcPayouts::begin_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
			total_node_usage,
		));

		System::assert_last_event(Event::RewardingStarted { cluster_id, era }.into());

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, State::RewardingProviders);
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
		let max_batch_index = 1;
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
fn send_rewarding_providers_batch_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let dac_account = 123u64;
		let user1 = 1u64;
		let node1 = 10u64;
		let node2 = 11u64;
		let node3 = 12u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 0;
		let max_node_batch_index = 1;
		let batch_index = 0;
		let batch_node_index = 0;
		let usage1 = CustomerUsage {
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 4456456345234523,
			number_of_gets: 523423,
		};

		let node_usage1 = NodeUsage {
			// CDN
			transferred_bytes: usage1.transferred_bytes * 2 / 3,
			stored_bytes: 0,
			number_of_puts: usage1.number_of_puts * 2 / 3,
			number_of_gets: usage1.number_of_gets * 2 / 3,
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

		let total_nodes_usage = NodeUsage {
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

		let payers = vec![(user1, usage1)];
		let payees1 = vec![(node1, node_usage1.clone()), (node2, node_usage2.clone())];
		let payees2 = vec![(node3, node_usage3.clone())];

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

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers,
		));

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let total_left_from_one = (PRICING_FEES.treasury_share +
			PRICING_FEES.validators_share +
			PRICING_FEES.cluster_reserve_share)
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

		assert_ok!(DdcPayouts::begin_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_node_batch_index,
			total_nodes_usage.clone(),
		));

		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_node_index,
			payees1,
		));

		let mut ratio = Perbill::from_rational(
			node_usage1.transferred_bytes,
			total_nodes_usage.transferred_bytes,
		);
		let mut transfer_charge = ratio * report_after.total_customer_charge.transfer;

		ratio = Perbill::from_rational(node_usage1.stored_bytes, total_nodes_usage.stored_bytes);
		let mut storage_charge = ratio * report_after.total_customer_charge.storage;

		ratio =
			Perbill::from_rational(node_usage1.number_of_puts, total_nodes_usage.number_of_puts);
		let mut puts_charge = ratio * report_after.total_customer_charge.puts;

		ratio =
			Perbill::from_rational(node_usage1.number_of_gets, total_nodes_usage.number_of_gets);
		let mut gets_charge = ratio * report_after.total_customer_charge.gets;

		let mut balance = Balances::free_balance(node1);
		assert_eq!(balance, transfer_charge + storage_charge + puts_charge + gets_charge);

		ratio = Perbill::from_rational(
			node_usage2.transferred_bytes,
			total_nodes_usage.transferred_bytes,
		);
		transfer_charge = ratio * report_after.total_customer_charge.transfer;

		ratio = Perbill::from_rational(node_usage2.stored_bytes, total_nodes_usage.stored_bytes);
		storage_charge = ratio * report_after.total_customer_charge.storage;

		ratio =
			Perbill::from_rational(node_usage2.number_of_puts, total_nodes_usage.number_of_puts);
		puts_charge = ratio * report_after.total_customer_charge.puts;

		ratio =
			Perbill::from_rational(node_usage2.number_of_gets, total_nodes_usage.number_of_gets);
		gets_charge = ratio * report_after.total_customer_charge.gets;

		balance = Balances::free_balance(node2);
		assert_eq!(balance, transfer_charge + storage_charge + puts_charge + gets_charge);

		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_node_index + 1,
			payees2,
		));

		ratio = Perbill::from_rational(
			node_usage3.transferred_bytes,
			total_nodes_usage.transferred_bytes,
		);
		transfer_charge = ratio * report_after.total_customer_charge.transfer;

		ratio = Perbill::from_rational(node_usage3.stored_bytes, total_nodes_usage.stored_bytes);
		storage_charge = ratio * report_after.total_customer_charge.storage;

		ratio =
			Perbill::from_rational(node_usage3.number_of_puts, total_nodes_usage.number_of_puts);
		puts_charge = ratio * report_after.total_customer_charge.puts;

		ratio =
			Perbill::from_rational(node_usage3.number_of_gets, total_nodes_usage.number_of_gets);
		gets_charge = ratio * report_after.total_customer_charge.gets;

		balance = Balances::free_balance(node3);
		assert_eq!(balance, transfer_charge + storage_charge + puts_charge + gets_charge);

		assert_ok!(DdcPayouts::end_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));
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
		let max_batch_index = 1;
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
fn end_rewarding_providers_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let dac_account = 2u64;
		let user1 = 3u64;
		let node1 = 33u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
		let total_node_usage = NodeUsage::default();
		let payers = vec![(user1, CustomerUsage::default())];
		let payees = vec![(node1, NodeUsage::default())];

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, State::Initialized);

		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
		));

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers,
		));

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_ok!(DdcPayouts::begin_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
			total_node_usage,
		));

		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payees,
		));

		assert_ok!(DdcPayouts::end_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		System::assert_last_event(Event::RewardingFinished { cluster_id, era }.into());

		report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, State::ProvidersRewarded);
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
		let max_batch_index = 1;
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

#[test]
fn end_billing_report_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let dac_account = 2u64;
		let user1 = 3u64;
		let node1 = 33u64;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
		let total_node_usage = NodeUsage::default();
		let payers = vec![(user1, CustomerUsage::default())];
		let payees = vec![(node1, NodeUsage::default())];

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		let report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, State::Initialized);

		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
		));

		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers,
		));

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_ok!(DdcPayouts::begin_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
			total_node_usage,
		));

		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payees,
		));

		assert_ok!(DdcPayouts::end_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		assert_ok!(DdcPayouts::end_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		System::assert_last_event(Event::BillingReportFinalized { cluster_id, era }.into());

		let report_end = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert!(report_end.rewarding_processed_batches.is_empty());
		assert!(report_end.charging_processed_batches.is_empty());
		assert_eq!(report_end.state, State::Finalized);
	})
}
