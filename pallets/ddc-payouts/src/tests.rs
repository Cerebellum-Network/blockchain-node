//! Tests for the module.

use super::{mock::*, *};
use ddc_primitives::ClusterId;

use frame_support::{assert_noop, assert_ok};
use pallet_balances::Error as BalancesError;

#[test]
fn create_bucket_works() {
	ExtBuilder.build_and_execute(|| {

		assert_ok!(Ok(()));
	})
}