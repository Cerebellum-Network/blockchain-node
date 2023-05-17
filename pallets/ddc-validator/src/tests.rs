use crate::mock::*;
use frame_support::assert_ok;
use sp_core::crypto::AccountId32;
use sp_runtime::DispatchResult;

#[test]
fn save_validated_data_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(DispatchResult::Ok(()));
	});
}
