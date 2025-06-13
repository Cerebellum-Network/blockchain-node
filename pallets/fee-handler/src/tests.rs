use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::AccountIdConversion;

#[test]
fn manual_topup_works() {
    new_test_ext().execute_with(|| {
        let user = 1;
        let amount = 100;
        
        // Set up initial balance for the user
        Balances::mint_into(&user, amount).unwrap();
        
        // Test manual topup
        assert_ok!(FeeHandler::manual_topup(RuntimeOrigin::signed(user), amount));
        
        // Check that the fee pot account received the funds
        let fee_pot_account = FeeHandler::fee_pot_account_id();
        assert_eq!(Balances::balance(&fee_pot_account), amount);
        
        // Check that the user's balance was reduced
        assert_eq!(Balances::balance(&user), 0);
        
        // Verify event was emitted
        System::assert_last_event(Event::ManualFeeAccountTopUp { source: user, amount }.into());
    });
}

#[test]
fn manual_topup_fails_with_insufficient_balance() {
    new_test_ext().execute_with(|| {
        let user = 1;
        let amount = 100;
        
        // Don't set up any balance for the user
        
        // Test manual topup should fail
        assert_noop!(
            FeeHandler::manual_topup(RuntimeOrigin::signed(user), amount),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn burn_native_tokens_works() {
    new_test_ext().execute_with(|| {
        let user = 1;
        let amount = 100;
        
        // Set up initial balance for the user
        Balances::mint_into(&user, amount).unwrap();
        
        // Test burning tokens
        assert_ok!(FeeHandler::burn_native_tokens(RuntimeOrigin::root(), user, amount));
        
        // Check that the user's balance was reduced
        assert_eq!(Balances::balance(&user), 0);
        
        // Verify event was emitted
        System::assert_last_event(Event::NativeTokenBurned(user, amount).into());
    });
}

#[test]
fn burn_native_tokens_fails_with_unauthorized_origin() {
    new_test_ext().execute_with(|| {
        let user = 1;
        let amount = 100;
        
        // Test burning tokens with non-root origin
        assert_noop!(
            FeeHandler::burn_native_tokens(RuntimeOrigin::signed(user), user, amount),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn burn_native_tokens_fails_with_insufficient_balance() {
    new_test_ext().execute_with(|| {
        let user = 1;
        let amount = 100;
        
        // Don't set up any balance for the user
        
        // Test burning tokens
        assert_noop!(
            FeeHandler::burn_native_tokens(RuntimeOrigin::root(), user, amount),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
    });
} 