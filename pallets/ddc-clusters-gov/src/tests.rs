//! Tests for the module.

use frame_support::{assert_noop, assert_ok};
use frame_system::Config;
use hex_literal::hex;
use sp_runtime::{traits::Hash, Perquintill};

use super::{mock::*, *};

#[test]
fn cluster_activation_proposal_works() {
	ExtBuilder.build_and_execute(|| {
		// todo
	})
}
