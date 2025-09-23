//! Tests for the withdrawal-fix pallet

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use crate::{Error, WithdrawalState};

#[test]
fn test_fix_withdrawal_success() {
	new_test_ext().execute_with(|| {
		let account = 1;
		let amount = 100;

		// Set up pending withdrawal
		PendingWithdrawals::<Test>::insert(account, amount);

		// Fix withdrawal
		assert_ok!(WithdrawalFix::fix_withdrawal(
			RuntimeOrigin::root(),
			account,
			amount
		));

		// Check state
		assert_eq!(WithdrawalStates::<Test>::get(account), Some(WithdrawalState::Processed));
		assert!(!PendingWithdrawals::<Test>::contains_key(account));

		// Check event
		System::assert_last_event(
			Event::WithdrawalFixed { account, amount }.into()
		);
	});
}

#[test]
fn test_fix_withdrawal_not_found() {
	new_test_ext().execute_with(|| {
		let account = 1;
		let amount = 100;

		// Try to fix non-existent withdrawal
		assert_noop!(
			WithdrawalFix::fix_withdrawal(RuntimeOrigin::root(), account, amount),
			Error::<Test>::WithdrawalNotFound
		);
	});
}

#[test]
fn test_fix_withdrawal_already_processed() {
	new_test_ext().execute_with(|| {
		let account = 1;
		let amount = 100;

		// Set up already processed withdrawal
		WithdrawalStates::<Test>::insert(account, WithdrawalState::Processed);
		// Also need to set up a pending withdrawal to pass the first check
		PendingWithdrawals::<Test>::insert(account, amount);

		// Try to fix already processed withdrawal
		assert_noop!(
			WithdrawalFix::fix_withdrawal(RuntimeOrigin::root(), account, amount),
			Error::<Test>::WithdrawalAlreadyProcessed
		);
	});
}

#[test]
fn test_update_withdrawal_state_success() {
	new_test_ext().execute_with(|| {
		let account = 1;
		let state = WithdrawalState::Pending;

		// Update state
		assert_ok!(WithdrawalFix::update_withdrawal_state(
			RuntimeOrigin::root(),
			account,
			state.clone()
		));

		// Check state
		assert_eq!(WithdrawalStates::<Test>::get(account), Some(state.clone()));

		// Check event
		System::assert_last_event(
			Event::WithdrawalStateUpdated { account, state }.into()
		);
	});
}

#[test]
fn test_update_withdrawal_state_invalid_transition() {
	new_test_ext().execute_with(|| {
		let account = 1;

		// Set initial state to Processed
		WithdrawalStates::<Test>::insert(account, WithdrawalState::Processed);

		// Try invalid transition from Processed to Pending
		assert_noop!(
			WithdrawalFix::update_withdrawal_state(
				RuntimeOrigin::root(),
				account,
				WithdrawalState::Pending
			),
			Error::<Test>::InvalidStateTransition
		);
	});
}

#[test]
fn test_valid_state_transitions() {
	new_test_ext().execute_with(|| {
		let account = 1;

		// Test valid transitions
		let valid_transitions = vec![
			(None, WithdrawalState::Pending),
			(Some(WithdrawalState::Pending), WithdrawalState::Processing),
			(Some(WithdrawalState::Processing), WithdrawalState::Processed),
			(Some(WithdrawalState::Processing), WithdrawalState::Failed),
			(Some(WithdrawalState::Failed), WithdrawalState::Pending),
		];

		for (from, to) in valid_transitions {
			if let Some(state) = from {
				WithdrawalStates::<Test>::insert(account, state);
			}
			
			assert_ok!(WithdrawalFix::update_withdrawal_state(
				RuntimeOrigin::root(),
				account,
				to.clone()
			));
			
			assert_eq!(WithdrawalStates::<Test>::get(account), Some(to));
		}
	});
}

#[test]
fn test_unauthorized_access() {
	new_test_ext().execute_with(|| {
		let account = 1;
		let amount = 100;

		// Test unauthorized fix_withdrawal
		assert_noop!(
			WithdrawalFix::fix_withdrawal(RuntimeOrigin::signed(account), account, amount),
			sp_runtime::traits::BadOrigin
		);

		// Test unauthorized update_withdrawal_state
		assert_noop!(
			WithdrawalFix::update_withdrawal_state(
				RuntimeOrigin::signed(account),
				account,
				WithdrawalState::Pending
			),
			sp_runtime::traits::BadOrigin
		);
	});
}
