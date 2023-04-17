use crate::{mock::*};
use frame_support::{assert_ok};
use sp_core::crypto::AccountId32;

#[test]
fn save_validated_data_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(DdcValidator::save_validated_data(
            Origin::signed(AccountId32::from([1; 32])),
            true,
            String::from("0xab1"),
            1,
        ));
    });
}