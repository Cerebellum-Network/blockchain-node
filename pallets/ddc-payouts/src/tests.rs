//! Tests for the module.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ddc_primitives::ClusterId;
use frame_support::{assert_noop, assert_ok, error::BadOrigin, traits::Randomness};
use sp_core::H256;
use sp_runtime::Perquintill;

use super::{mock::*, *};

#[test]
fn set_authorised_caller_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let root_account = 1u128;
		let dac_account = 2u128;

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
		let root_account = 1u128;
		let dac_account = 2u128;
		let cluster_id = ClusterId::from([1; 20]);
		let era = 100;
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_noop!(
			DdcPayouts::begin_billing_report(
				RuntimeOrigin::signed(dac_account + 1),
				cluster_id,
				era,
				start_era,
				end_era,
			),
			Error::<Test>::Unauthorised
		);

		assert_noop!(
			DdcPayouts::begin_billing_report(
				RuntimeOrigin::signed(root_account),
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
fn begin_billing_report_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let dac_account = 2u128;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		System::assert_last_event(Event::BillingReportInitialized { cluster_id, era }.into());

		let report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report.state, State::Initialized);
		assert_eq!(report.start_era, start_era);
		assert_eq!(report.end_era, end_era);
	})
}

#[test]
fn begin_charging_customers_fails_uninitialised() {
	ExtBuilder.build_and_execute(|| {
		let dac_account = 2u128;
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

		let dac_account = 2u128;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
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
		let root_account = 1u128;
		let dac_account = 2u128;
		let user1 = 3u128;
		let user2 = 4u128;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;
		let batch_index = 1;
		let payers1 = vec![(user1, CustomerUsage::default())];
		let payers2 = vec![(user2, CustomerUsage::default())];
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

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
			start_era,
			end_era,
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

fn calculate_charge_parts_for_day(cluster_id: ClusterId, usage: CustomerUsage) -> CustomerCharge {
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

fn calculate_charge_for_day(cluster_id: ClusterId, usage: CustomerUsage) -> u128 {
	let charge = calculate_charge_parts_for_day(cluster_id, usage);
	charge.transfer + charge.storage + charge.puts + charge.gets
}

fn calculate_charge_parts_for_month(cluster_id: ClusterId, usage: CustomerUsage) -> CustomerCharge {
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

fn calculate_charge_for_month(cluster_id: ClusterId, usage: CustomerUsage) -> u128 {
	let charge = calculate_charge_parts_for_month(cluster_id, usage);
	charge.transfer + charge.storage + charge.puts + charge.gets
}

#[test]
fn send_charging_customers_batch_works1() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let dac_account = 123u128;
		let user1 = 1u128;
		let user2_debtor = 2u128;
		let user3_debtor = 3u128;
		let user4 = 4u128;
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
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
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

		let usage4_charge = calculate_charge_for_month(cluster_id, usage4.clone());
		let user2_debt = DdcPayouts::debtor_customers(cluster_id, user2_debtor).unwrap();
		let expected_charge2 = calculate_charge_for_month(cluster_id, usage2.clone());
		let mut debt = expected_charge2 - USER2_BALANCE;
		assert_eq!(user2_debt, debt);

		let ratio = Perquintill::from_rational(USER2_BALANCE, expected_charge2);
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
				customer_id: user2_debtor,
				batch_index,
				charged: USER2_BALANCE,
				expected_to_charge: expected_charge2,
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

		assert_eq!(System::events().len(), 5 + 3 + 1); // 1 for Currency::transfer

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

		assert_eq!(report.state, State::ChargingCustomers);
		let user1_debt = DdcPayouts::debtor_customers(cluster_id, user1);
		assert_eq!(user1_debt, None);

		let balance_before = Balances::free_balance(DdcPayouts::account_id());

		// batch 3
		batch_index += 1;
		before_total_customer_charge = report.total_customer_charge.clone();
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers3,
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
				charged: PARTIAL_CHARGE,
				expected_to_charge: user3_charge,
			}
			.into(),
		);
	})
}

#[test]
fn send_charging_customers_batch_works1_for_day() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let dac_account = 123u128;
		let user1 = 1u128;
		let user2_debtor = 2u128;
		let user3_debtor = 3u128;
		let user4 = 4u128;
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
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (1.0 * 24.0 * 3600.0) as i64;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
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

		let usage4_charge = calculate_charge_for_day(cluster_id, usage4.clone());
		let user2_debt = DdcPayouts::debtor_customers(cluster_id, user2_debtor).unwrap();
		let expected_charge2 = calculate_charge_for_day(cluster_id, usage2.clone());
		let mut debt = expected_charge2 - USER2_BALANCE;
		assert_eq!(user2_debt, debt);

		let ratio = Perquintill::from_rational(USER2_BALANCE, expected_charge2);
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
				customer_id: user2_debtor,
				batch_index,
				charged: USER2_BALANCE,
				expected_to_charge: expected_charge2,
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

		assert_eq!(System::events().len(), 5 + 3 + 1); // 1 for Currency::transfer

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

		assert_eq!(report.state, State::ChargingCustomers);
		let user1_debt = DdcPayouts::debtor_customers(cluster_id, user1);
		assert_eq!(user1_debt, None);

		let balance_before = Balances::free_balance(DdcPayouts::account_id());

		// batch 3
		batch_index += 1;
		before_total_customer_charge = report.total_customer_charge.clone();
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers3,
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
				charged: PARTIAL_CHARGE,
				expected_to_charge: user3_charge,
			}
			.into(),
		);
	})
}

#[test]
fn send_charging_customers_batch_works2() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let dac_account = 123u128;
		let user5 = 5u128;
		let cluster_id = ONE_CLUSTER_ID;
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
		let usage5 = CustomerUsage {
			// should pass without debt
			transferred_bytes: 1024,
			stored_bytes: 1024,
			number_of_puts: 1,
			number_of_gets: 1,
		};
		let payers5 = vec![(user5, usage5.clone())];
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
		));

		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			max_batch_index,
		));
		assert_eq!(System::events().len(), 3);

		// batch 1
		let mut report = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let before_total_customer_charge = report.total_customer_charge.clone();
		let balance_before = Balances::free_balance(DdcPayouts::account_id());
		assert_ok!(DdcPayouts::send_charging_customers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_index,
			payers5,
		));

		let usage5_charge = calculate_charge_for_month(cluster_id, usage5.clone());
		let charge5 = calculate_charge_parts_for_month(cluster_id, usage5);
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
		let root_account = 100u128;
		let dac_account = 123u128;
		let user1 = 1u128;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;
		let batch_index = 1;
		let payers = vec![(user1, CustomerUsage::default())];
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st
		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

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
			start_era,
			end_era,
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

		let dac_account = 123u128;
		let user1 = 1u128;
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
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
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
		let charge = calculate_charge_for_month(cluster_id, usage1);
		System::assert_last_event(
			Event::Charged { cluster_id, era, batch_index, customer_id: user1, amount: charge }
				.into(),
		);

		let mut balance = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance - Balances::minimum_balance(), charge);
		assert_eq!(System::events().len(), 4 + 1); // 1 for Currency::transfer

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

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

		let transfers = 3 + 3 + 3 * 3; // for Currency::transfer
		assert_eq!(System::events().len(), 5 + 1 + 3 + transfers);

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report_after.state, State::CustomersChargedWithFees);

		let total_left_from_one = (get_fees(&cluster_id).treasury_share +
			get_fees(&cluster_id).validators_share +
			get_fees(&cluster_id).cluster_reserve_share)
			.left_from_one();

		balance = Balances::free_balance(TREASURY_ACCOUNT_ID);
		assert_eq!(balance, get_fees(&cluster_id).treasury_share * charge);

		balance = Balances::free_balance(RESERVE_ACCOUNT_ID);
		assert_eq!(balance, get_fees(&cluster_id).cluster_reserve_share * charge);

		balance = Balances::free_balance(VALIDATOR1_ACCOUNT_ID);
		let mut ratio = Perquintill::from_rational(
			VALIDATOR1_SCORE,
			VALIDATOR1_SCORE + VALIDATOR2_SCORE + VALIDATOR3_SCORE,
		);
		assert_eq!(balance, get_fees(&cluster_id).validators_share * ratio * charge);

		balance = Balances::free_balance(VALIDATOR2_ACCOUNT_ID);
		ratio = Perquintill::from_rational(
			VALIDATOR2_SCORE,
			VALIDATOR1_SCORE + VALIDATOR2_SCORE + VALIDATOR3_SCORE,
		);
		assert_eq!(balance, get_fees(&cluster_id).validators_share * ratio * charge);

		balance = Balances::free_balance(VALIDATOR3_ACCOUNT_ID);
		ratio = Perquintill::from_rational(
			VALIDATOR3_SCORE,
			VALIDATOR1_SCORE + VALIDATOR2_SCORE + VALIDATOR3_SCORE,
		);
		assert_eq!(balance, get_fees(&cluster_id).validators_share * ratio * charge);

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

		let dac_account = 123u128;
		let user1 = 1u128;
		let cluster_id = ClusterId::zero();
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
		let usage1 = CustomerUsage {
			transferred_bytes: 23452345,
			stored_bytes: 3345234523,
			number_of_puts: 1,
			number_of_gets: 1,
		};
		let payers = vec![(user1, usage1.clone())];
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
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
		let charge = calculate_charge_for_month(cluster_id, usage1);
		System::assert_last_event(
			Event::Charged { cluster_id, era, customer_id: user1, batch_index, amount: charge }
				.into(),
		);

		let mut balance = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance - Balances::minimum_balance(), charge);
		assert_eq!(System::events().len(), 4 + 1); // 1 for Currency::transfer

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

		System::assert_has_event(Event::ChargingFinished { cluster_id, era }.into());
		assert_eq!(System::events().len(), 5 + 1);

		let report_after = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		assert_eq!(report_after.state, State::CustomersChargedWithFees);

		let fees = get_fees(&cluster_id);

		let total_left_from_one =
			(fees.treasury_share + fees.validators_share + fees.cluster_reserve_share)
				.left_from_one();

		assert_eq!(total_left_from_one, Perquintill::one());

		assert_eq!(fees.treasury_share, Perquintill::zero());
		assert_eq!(fees.validators_share, Perquintill::zero());
		assert_eq!(fees.cluster_reserve_share, Perquintill::zero());

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
		let root_account = 1u128;
		let dac_account = 2u128;
		let user1 = 3u128;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 2;
		let batch_index = 1;
		let payers = vec![(user1, CustomerUsage::default())];
		let node_usage = NodeUsage::default();
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

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
			start_era,
			end_era,
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

		let dac_account = 123u128;
		let user1 = 1u128;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 0;
		let batch_index = 0;
		let total_node_usage = NodeUsage::default();
		let payers = vec![(user1, CustomerUsage::default())];
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
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
		let root_account = 1u128;
		let dac_account = 2u128;
		let user1 = 3u128;
		let user2 = 4u128;
		let node1 = 33u128;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 1;
		let batch_index = 0;
		let payers1 = vec![(user1, CustomerUsage::default())];
		let payers2 = vec![(user2, CustomerUsage::default())];
		let payees = vec![(node1, NodeUsage::default())];
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

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
			start_era,
			end_era,
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

		let dac_account = 123u128;
		let user1 = 1u128;
		let node1 = 10u128;
		let node2 = 11u128;
		let node3 = 12u128;
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
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
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

		let ratio1_transfer = Perquintill::from_rational(
			node_usage1.transferred_bytes,
			total_nodes_usage.transferred_bytes,
		);
		let mut transfer_charge = ratio1_transfer * report_after.total_customer_charge.transfer;

		let ratio1_storage =
			Perquintill::from_rational(node_usage1.stored_bytes, total_nodes_usage.stored_bytes);
		let mut storage_charge = ratio1_storage * report_after.total_customer_charge.storage;

		let ratio1_puts = Perquintill::from_rational(
			node_usage1.number_of_puts,
			total_nodes_usage.number_of_puts,
		);
		let mut puts_charge = ratio1_puts * report_after.total_customer_charge.puts;

		let ratio1_gets = Perquintill::from_rational(
			node_usage1.number_of_gets,
			total_nodes_usage.number_of_gets,
		);
		let mut gets_charge = ratio1_gets * report_after.total_customer_charge.gets;

		let balance_node1 = Balances::free_balance(node1);
		assert_eq!(balance_node1, transfer_charge + storage_charge + puts_charge + gets_charge);
		let mut report_reward = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();

		let ratio2_transfer = Perquintill::from_rational(
			node_usage2.transferred_bytes,
			total_nodes_usage.transferred_bytes,
		);
		transfer_charge = ratio2_transfer * report_after.total_customer_charge.transfer;

		let ratio2_storage =
			Perquintill::from_rational(node_usage2.stored_bytes, total_nodes_usage.stored_bytes);
		storage_charge = ratio2_storage * report_after.total_customer_charge.storage;

		let ratio2_puts = Perquintill::from_rational(
			node_usage2.number_of_puts,
			total_nodes_usage.number_of_puts,
		);
		puts_charge = ratio2_puts * report_after.total_customer_charge.puts;

		let ratio2_gets = Perquintill::from_rational(
			node_usage2.number_of_gets,
			total_nodes_usage.number_of_gets,
		);
		gets_charge = ratio2_gets * report_after.total_customer_charge.gets;

		let balance_node2 = Balances::free_balance(node2);
		assert_eq!(balance_node2, transfer_charge + storage_charge + puts_charge + gets_charge);
		assert_eq!(report_reward.total_distributed_reward, balance_node1 + balance_node2);

		// batch 2
		assert_ok!(DdcPayouts::send_rewarding_providers_batch(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			batch_node_index + 1,
			payees2,
		));

		let ratio3_transfer = Perquintill::from_rational(
			node_usage3.transferred_bytes,
			total_nodes_usage.transferred_bytes,
		);
		transfer_charge = ratio3_transfer * report_after.total_customer_charge.transfer;

		let ratio3_storage =
			Perquintill::from_rational(node_usage3.stored_bytes, total_nodes_usage.stored_bytes);
		storage_charge = ratio3_storage * report_after.total_customer_charge.storage;

		let ratio3_puts = Perquintill::from_rational(
			node_usage3.number_of_puts,
			total_nodes_usage.number_of_puts,
		);
		puts_charge = ratio3_puts * report_after.total_customer_charge.puts;

		let ratio3_gets = Perquintill::from_rational(
			node_usage3.number_of_gets,
			total_nodes_usage.number_of_gets,
		);
		gets_charge = ratio3_gets * report_after.total_customer_charge.gets;

		report_reward = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let balance_node3 = Balances::free_balance(node3);
		assert_eq!(balance_node3, transfer_charge + storage_charge + puts_charge + gets_charge);
		assert_eq!(
			report_reward.total_distributed_reward,
			balance_node1 + balance_node2 + balance_node3
		);

		let expected_amount_to_reward = report_reward.total_customer_charge.transfer +
			report_reward.total_customer_charge.storage +
			report_reward.total_customer_charge.puts +
			report_reward.total_customer_charge.gets;

		assert!(expected_amount_to_reward - report_reward.total_distributed_reward <= 20000);

		assert_ok!(DdcPayouts::end_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));
	})
}

#[test]
fn send_rewarding_providers_batch_100_nodes_small_usage_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let num_nodes = 100;
		let num_users = 5;
		let dac_account = 123u128;
		let bank = 1u128;
		let cluster_id = ONE_CLUSTER_ID;
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
		let usage1 = CustomerUsage {
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

		let mut payees: Vec<Vec<(u128, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(u128, NodeUsage)> = Vec::new();
		let mut total_nodes_usage = NodeUsage::default();
		for i in 10..10 + num_nodes {
			let node_usage = match i % 3 {
				0 => node_usage1.clone(),
				1 => node_usage2.clone(),
				2 => node_usage3.clone(),
				_ => unreachable!(),
			};
			total_nodes_usage.transferred_bytes += node_usage.transferred_bytes;
			total_nodes_usage.stored_bytes += node_usage.stored_bytes;
			total_nodes_usage.number_of_puts += node_usage.number_of_puts;
			total_nodes_usage.number_of_gets += node_usage.number_of_gets;

			node_batch.push((i, node_usage));
			if node_batch.len() == node_batch_size {
				payees.push(node_batch.clone());
				node_batch.clear();
			}
		}
		if !node_batch.is_empty() {
			payees.push(node_batch.clone());
		}

		let mut total_charge = 0u128;
		let mut payers: Vec<Vec<(u128, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(u128, CustomerUsage)> = Vec::new();
		for user_id in 1000..1000 + num_users {
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
			user_usage.stored_bytes = ratio * user_usage.stored_bytes;
			user_usage.number_of_puts = ratio * user_usage.number_of_puts;
			user_usage.number_of_gets = ratio * user_usage.number_of_gets;

			let expected_charge = calculate_charge_for_month(cluster_id, user_usage.clone());
			Balances::transfer(
				RuntimeOrigin::signed(bank),
				user_id,
				(expected_charge * 2).max(Balances::minimum_balance()),
			)
			.unwrap();
			total_charge += expected_charge;

			user_batch.push((user_id, user_usage));
			if user_batch.len() == user_batch_size {
				payers.push(user_batch.clone());
				user_batch.clear();
			}
		}
		if !user_batch.is_empty() {
			payers.push(user_batch.clone());
		}

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
		));
		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			(payers.len() - 1) as u16,
		));

		for batch in payers.iter() {
			assert_ok!(DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_user_index,
				batch.to_vec(),
			));

			for (customer_id, usage) in batch.iter() {
				let charge = calculate_charge_for_month(cluster_id, usage.clone());

				System::assert_has_event(
					Event::Charged {
						cluster_id,
						era,
						customer_id: *customer_id,
						batch_index: batch_user_index,
						amount: charge,
					}
					.into(),
				);
			}
			batch_user_index += 1;
		}

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let balance1 = Balances::free_balance(report_before.vault);
		let balance2 = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance1, balance2);
		assert_eq!(report_before.vault, DdcPayouts::account_id());
		assert_eq!(balance1 - Balances::minimum_balance(), total_charge);

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

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

		assert_ok!(DdcPayouts::begin_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			(payees.len() - 1) as u16,
			total_nodes_usage.clone(),
		));

		for batch in payees.iter() {
			let before_batch = Balances::free_balance(DdcPayouts::account_id());
			assert_ok!(DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_node_index,
				batch.to_vec(),
			));

			let mut batch_charge = 0;
			for (node1, node_usage1) in batch.iter() {
				let ratio1_transfer = Perquintill::from_rational(
					node_usage1.transferred_bytes,
					total_nodes_usage.transferred_bytes,
				);
				let transfer_charge = ratio1_transfer * report_after.total_customer_charge.transfer;

				let ratio1_storage = Perquintill::from_rational(
					node_usage1.stored_bytes,
					total_nodes_usage.stored_bytes,
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

				let balance_node1 = Balances::free_balance(node1);
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
		let num_nodes = 100;
		let num_users = 5;
		let dac_account = 123u128;
		let bank = 1u128;
		let cluster_id = ONE_CLUSTER_ID;
		let era = 100;
		let user_batch_size = 10;
		let node_batch_size = 10;
		let mut batch_user_index = 0;
		let mut batch_node_index = 0;
		let usage1 = CustomerUsage {
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

		let mut payees: Vec<Vec<(u128, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(u128, NodeUsage)> = Vec::new();
		let mut total_nodes_usage = NodeUsage::default();
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
			node_usage.stored_bytes = ratio * node_usage.stored_bytes;
			node_usage.number_of_puts = ratio * node_usage.number_of_puts;
			node_usage.number_of_gets = ratio * node_usage.number_of_gets;

			total_nodes_usage.transferred_bytes += node_usage.transferred_bytes;
			total_nodes_usage.stored_bytes += node_usage.stored_bytes;
			total_nodes_usage.number_of_puts += node_usage.number_of_puts;
			total_nodes_usage.number_of_gets += node_usage.number_of_gets;

			node_batch.push((i, node_usage));
			if node_batch.len() == node_batch_size {
				payees.push(node_batch.clone());
				node_batch.clear();
			}
		}
		if !node_batch.is_empty() {
			payees.push(node_batch.clone());
		}

		let mut total_charge = 0u128;
		let mut payers: Vec<Vec<(u128, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(u128, CustomerUsage)> = Vec::new();
		for user_id in 1000..1000 + num_users {
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
			user_usage.stored_bytes = ratio * user_usage.stored_bytes;
			user_usage.number_of_puts = ratio * user_usage.number_of_puts;
			user_usage.number_of_gets = ratio * user_usage.number_of_gets;

			let expected_charge = calculate_charge_for_month(cluster_id, user_usage.clone());
			Balances::transfer(
				RuntimeOrigin::signed(bank),
				user_id,
				(expected_charge * 2).max(Balances::minimum_balance()),
			)
			.unwrap();
			total_charge += expected_charge;

			user_batch.push((user_id, user_usage));
			if user_batch.len() == user_batch_size {
				payers.push(user_batch.clone());
				user_batch.clear();
			}
		}
		if !user_batch.is_empty() {
			payers.push(user_batch.clone());
		}

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
		));
		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			(payers.len() - 1) as u16,
		));

		for batch in payers.iter() {
			assert_ok!(DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_user_index,
				batch.to_vec(),
			));

			for (customer_id, usage) in batch.iter() {
				let charge = calculate_charge_for_month(cluster_id, usage.clone());

				System::assert_has_event(
					Event::Charged {
						cluster_id,
						era,
						customer_id: *customer_id,
						batch_index: batch_user_index,
						amount: charge,
					}
					.into(),
				);
			}
			batch_user_index += 1;
		}

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let balance1 = Balances::free_balance(report_before.vault);
		let balance2 = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance1, balance2);
		assert_eq!(report_before.vault, DdcPayouts::account_id());
		assert_eq!(balance1 - Balances::minimum_balance(), total_charge);

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

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

		assert_ok!(DdcPayouts::begin_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			(payees.len() - 1) as u16,
			total_nodes_usage.clone(),
		));

		for batch in payees.iter() {
			let before_batch = Balances::free_balance(DdcPayouts::account_id());
			assert_ok!(DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_node_index,
				batch.to_vec(),
			));

			let mut batch_charge = 0;
			for (node1, node_usage1) in batch.iter() {
				let ratio1_transfer = Perquintill::from_rational(
					node_usage1.transferred_bytes,
					total_nodes_usage.transferred_bytes,
				);
				let transfer_charge = ratio1_transfer * report_after.total_customer_charge.transfer;

				let ratio1_storage = Perquintill::from_rational(
					node_usage1.stored_bytes,
					total_nodes_usage.stored_bytes,
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

				let balance_node1 = Balances::free_balance(node1);
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
fn send_rewarding_providers_batch_100_nodes_small_large_usage_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let num_nodes = 100;
		let num_users = 5;
		let dac_account = 123u128;
		let bank = 1u128;
		let cluster_id = ONE_CLUSTER_ID;
		let era = 100;
		let user_batch_size = 10;
		let node_batch_size = 10;
		let mut batch_user_index = 0;
		let mut batch_node_index = 0;
		let usage1 = CustomerUsage {
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

		let mut payees: Vec<Vec<(u128, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(u128, NodeUsage)> = Vec::new();
		let mut total_nodes_usage = NodeUsage::default();
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
			node_usage.stored_bytes = ratio * node_usage.stored_bytes;
			node_usage.number_of_puts = ratio * node_usage.number_of_puts;
			node_usage.number_of_gets = ratio * node_usage.number_of_gets;

			total_nodes_usage.transferred_bytes += node_usage.transferred_bytes;
			total_nodes_usage.stored_bytes += node_usage.stored_bytes;
			total_nodes_usage.number_of_puts += node_usage.number_of_puts;
			total_nodes_usage.number_of_gets += node_usage.number_of_gets;

			node_batch.push((i, node_usage));
			if node_batch.len() == node_batch_size {
				payees.push(node_batch.clone());
				node_batch.clear();
			}
		}
		if !node_batch.is_empty() {
			payees.push(node_batch.clone());
		}

		let mut total_charge = 0u128;
		let mut payers: Vec<Vec<(u128, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(u128, CustomerUsage)> = Vec::new();
		for user_id in 1000..1000 + num_users {
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
			user_usage.stored_bytes = ratio * user_usage.stored_bytes;
			user_usage.number_of_puts = ratio * user_usage.number_of_puts;
			user_usage.number_of_gets = ratio * user_usage.number_of_gets;

			let expected_charge = calculate_charge_for_month(cluster_id, user_usage.clone());
			Balances::transfer(
				RuntimeOrigin::signed(bank),
				user_id,
				(expected_charge * 2).max(Balances::minimum_balance()),
			)
			.unwrap();
			total_charge += expected_charge;

			user_batch.push((user_id, user_usage));
			if user_batch.len() == user_batch_size {
				payers.push(user_batch.clone());
				user_batch.clear();
			}
		}
		if !user_batch.is_empty() {
			payers.push(user_batch.clone());
		}

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
		));
		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			(payers.len() - 1) as u16,
		));

		for batch in payers.iter() {
			assert_ok!(DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_user_index,
				batch.to_vec(),
			));

			for (customer_id, usage) in batch.iter() {
				let charge = calculate_charge_for_month(cluster_id, usage.clone());

				System::assert_has_event(
					Event::Charged {
						cluster_id,
						era,
						customer_id: *customer_id,
						batch_index: batch_user_index,
						amount: charge,
					}
					.into(),
				);
			}
			batch_user_index += 1;
		}

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let balance1 = Balances::free_balance(report_before.vault);
		let balance2 = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance1, balance2);
		assert_eq!(report_before.vault, DdcPayouts::account_id());
		assert_eq!(balance1 - Balances::minimum_balance(), total_charge);

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

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

		assert_ok!(DdcPayouts::begin_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			(payees.len() - 1) as u16,
			total_nodes_usage.clone(),
		));

		for batch in payees.iter() {
			let before_batch = Balances::free_balance(DdcPayouts::account_id());
			assert_ok!(DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_node_index,
				batch.to_vec(),
			));

			let mut batch_charge = 0;
			for (node1, node_usage1) in batch.iter() {
				let ratio1_transfer = Perquintill::from_rational(
					node_usage1.transferred_bytes,
					total_nodes_usage.transferred_bytes,
				);
				let transfer_charge = ratio1_transfer * report_after.total_customer_charge.transfer;

				let ratio1_storage = Perquintill::from_rational(
					node_usage1.stored_bytes,
					total_nodes_usage.stored_bytes,
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

				let balance_node1 = Balances::free_balance(node1);
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
		let num_nodes = 100;
		let num_users = 100;
		let dac_account = 123u128;
		let bank = 1u128;
		let cluster_id = CERE_CLUSTER_ID;
		let era = 100;
		let user_batch_size = 10;
		let node_batch_size = 10;
		let mut batch_user_index = 0;
		let mut batch_node_index = 0;
		let mut payees: Vec<Vec<(u128, NodeUsage)>> = Vec::new();
		let mut node_batch: Vec<(u128, NodeUsage)> = Vec::new();
		let mut total_nodes_usage = NodeUsage::default();
		for i in 10..10 + num_nodes {
			let node_usage = NodeUsage {
				transferred_bytes: generate_random_u64(&mock_randomness, min, max),
				stored_bytes: generate_random_u64(&mock_randomness, min, max),
				number_of_puts: generate_random_u64(&mock_randomness, min, max),
				number_of_gets: generate_random_u64(&mock_randomness, min, max),
			};

			total_nodes_usage.transferred_bytes += node_usage.transferred_bytes;
			total_nodes_usage.stored_bytes += node_usage.stored_bytes;
			total_nodes_usage.number_of_puts += node_usage.number_of_puts;
			total_nodes_usage.number_of_gets += node_usage.number_of_gets;

			node_batch.push((i, node_usage));
			if node_batch.len() == node_batch_size {
				payees.push(node_batch.clone());
				node_batch.clear();
			}
		}
		if !node_batch.is_empty() {
			payees.push(node_batch.clone());
		}

		let mut total_charge = 0u128;
		let mut payers: Vec<Vec<(u128, CustomerUsage)>> = Vec::new();
		let mut user_batch: Vec<(u128, CustomerUsage)> = Vec::new();
		for user_id in 1000..1000 + num_users {
			let user_usage = CustomerUsage {
				transferred_bytes: generate_random_u64(&mock_randomness, min, max),
				stored_bytes: generate_random_u64(&mock_randomness, min, max),
				number_of_puts: generate_random_u64(&mock_randomness, min, max),
				number_of_gets: generate_random_u64(&mock_randomness, min, max),
			};

			let expected_charge = calculate_charge_for_month(cluster_id, user_usage.clone());
			Balances::transfer(
				RuntimeOrigin::signed(bank),
				user_id,
				(expected_charge * 2).max(Balances::minimum_balance()),
			)
			.unwrap();
			total_charge += expected_charge;

			user_batch.push((user_id, user_usage));
			if user_batch.len() == user_batch_size {
				payers.push(user_batch.clone());
				user_batch.clear();
			}
		}
		if !user_batch.is_empty() {
			payers.push(user_batch.clone());
		}

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));
		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
		));
		assert_ok!(DdcPayouts::begin_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			(payers.len() - 1) as u16,
		));

		for batch in payers.iter() {
			assert_ok!(DdcPayouts::send_charging_customers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_user_index,
				batch.to_vec(),
			));

			for (customer_id, usage) in batch.iter() {
				let charge = calculate_charge_for_month(cluster_id, usage.clone());

				System::assert_has_event(
					Event::Charged {
						cluster_id,
						era,
						customer_id: *customer_id,
						batch_index: batch_user_index,
						amount: charge,
					}
					.into(),
				);
			}
			batch_user_index += 1;
		}

		let report_before = DdcPayouts::active_billing_reports(cluster_id, era).unwrap();
		let balance1 = Balances::free_balance(report_before.vault);
		let balance2 = Balances::free_balance(DdcPayouts::account_id());
		assert_eq!(balance1, balance2);
		assert_eq!(report_before.vault, DdcPayouts::account_id());
		assert_eq!(balance1 - Balances::minimum_balance(), total_charge);

		assert_ok!(DdcPayouts::end_charging_customers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
		));

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

		assert_ok!(DdcPayouts::begin_rewarding_providers(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			(payees.len() - 1) as u16,
			total_nodes_usage.clone(),
		));

		for batch in payees.iter() {
			let before_batch = Balances::free_balance(DdcPayouts::account_id());
			assert_ok!(DdcPayouts::send_rewarding_providers_batch(
				RuntimeOrigin::signed(dac_account),
				cluster_id,
				era,
				batch_node_index,
				batch.to_vec(),
			));

			let mut batch_charge = 0;
			for (node1, node_usage1) in batch.iter() {
				let ratio1_transfer = Perquintill::from_rational(
					node_usage1.transferred_bytes,
					total_nodes_usage.transferred_bytes,
				);
				let transfer_charge = ratio1_transfer * report_after.total_customer_charge.transfer;

				let ratio1_storage = Perquintill::from_rational(
					node_usage1.stored_bytes,
					total_nodes_usage.stored_bytes,
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

				let balance_node1 = Balances::free_balance(node1);
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
		let root_account = 1u128;
		let dac_account = 2u128;
		let user1 = 3u128;
		let user2 = 4u128;
		let node1 = 33u128;
		let cluster_id = ClusterId::from([12; 20]);
		let era = 100;
		let max_batch_index = 1;
		let batch_index = 0;
		let payers1 = vec![(user1, CustomerUsage::default())];
		let payers2 = vec![(user2, CustomerUsage::default())];
		let payees = vec![(node1, NodeUsage::default())];
		let total_node_usage = NodeUsage::default();
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

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
			start_era,
			end_era,
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

		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let dac_account = 2u128;
		let user1 = 1u128;
		let node1 = 33u128;
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

		let node_usage1 = NodeUsage {
			// CDN + Storage
			transferred_bytes: usage1.transferred_bytes * 2 / 3,
			stored_bytes: usage1.stored_bytes * 2 / 3,
			number_of_puts: usage1.number_of_puts * 2 / 3,
			number_of_gets: usage1.number_of_gets * 2 / 3,
		};
		let total_node_usage = node_usage1.clone();
		let payers = vec![(user1, usage1)];
		let payees = vec![(node1, node_usage1)];

		assert_ok!(DdcPayouts::set_authorised_caller(RuntimeOrigin::root(), dac_account));

		assert_ok!(DdcPayouts::begin_billing_report(
			RuntimeOrigin::signed(dac_account),
			cluster_id,
			era,
			start_era,
			end_era,
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
		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let root_account = 1u128;
		let dac_account = 2u128;
		let user1 = 3u128;
		let user2 = 4u128;
		let node1 = 33u128;
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
			start_era,
			end_era,
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

		let start_date = NaiveDate::from_ymd_opt(2023, 4, 1).unwrap(); // April 1st

		let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // Midnight
		let start_era: i64 =
			DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(time), Utc).timestamp();
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;
		let dac_account = 2u128;
		let user1 = 3u128;
		let node1 = 33u128;
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
			start_era,
			end_era,
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
