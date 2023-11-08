//! Tests for the module.

use super::{mock::*, *};
use ddc_primitives::{CDNNodePubKey, NodeType};
use ddc_traits::cluster::{ClusterVisitor, ClusterVisitorError};
use frame_support::{
	assert_noop, assert_ok, assert_storage_noop, error::BadOrigin, traits::ReservableCurrency,
};
use pallet_balances::Error as BalancesError;

pub const BLOCK_TIME: u64 = 1000;
pub const INIT_TIMESTAMP: u64 = 30_000;

#[test]
fn basic_setup_works() {
	// Verifies initial conditions of mock
	ExtBuilder::default().build_and_execute(|| {
		// Account 11 is stashed and locked, and account 10 is the controller
		assert_eq!(DdcStaking::bonded(&11), Some(10));
		// Account 21 is stashed and locked, and account 20 is the controller
		assert_eq!(DdcStaking::bonded(&21), Some(20));
		// Account 1 is not a stashed
		assert_eq!(DdcStaking::bonded(&1), None);

		// Account 10 controls the stash from account 11, which is 100 units
		assert_eq!(
			DdcStaking::ledger(&10),
			Some(StakingLedger {
				stash: 11,
				total: 100,
				active: 100,
				chilling: Default::default(),
				unlocking: Default::default(),
			})
		);
		// Account 20 controls the stash from account 21, which is 100 units
		assert_eq!(
			DdcStaking::ledger(&20),
			Some(StakingLedger {
				stash: 21,
				total: 100,
				active: 100,
				chilling: Default::default(),
				unlocking: Default::default(),
			})
		);
		// Account 1 does not control any stash
		assert_eq!(DdcStaking::ledger(&1), None);
	});
}

#[test]
fn change_controller_works() {
	ExtBuilder::default().build_and_execute(|| {
		// 10 and 11 are bonded as stash controller.
		assert_eq!(DdcStaking::bonded(&11), Some(10));

		// 10 can control 11 who is initially a validator.
		assert_ok!(DdcStaking::withdraw_unbonded(RuntimeOrigin::signed(10)));

		// Change controller.
		assert_ok!(DdcStaking::set_controller(RuntimeOrigin::signed(11), 3));
		assert_eq!(DdcStaking::bonded(&11), Some(3));

		// 10 is no longer in control.
		assert_noop!(
			DdcStaking::serve(RuntimeOrigin::signed(10), ClusterId::from([1; 20])),
			Error::<Test>::NotController
		);
		// 3 is a new controller.
		assert_ok!(DdcStaking::serve(RuntimeOrigin::signed(3), ClusterId::from([1; 20])));
	})
}

#[test]
fn staking_should_work() {
	ExtBuilder::default().build_and_execute(|| {
		// Put some money in account that we'll use.
		for i in 1..5 {
			let _ = Balances::make_free_balance_be(&i, 2000);
		}

		// Add new CDN participant, account 3 controlled by 4 with node 5.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(3),
			4,
			NodePubKey::CDNPubKey(CDNNodePubKey::new([5; 32])),
			1500
		));
		assert_ok!(DdcStaking::serve(RuntimeOrigin::signed(4), ClusterId::from([0; 20])));

		// Account 4 controls the stash from account 3, which is 1500 units, 3 is a CDN
		// participant, 5 is a DDC node.
		assert_eq!(DdcStaking::bonded(&3), Some(4));
		assert_eq!(
			DdcStaking::ledger(&4),
			Some(StakingLedger {
				stash: 3,
				total: 1500,
				active: 1500,
				chilling: Default::default(),
				unlocking: Default::default(),
			})
		);
		assert_eq!(DdcStaking::cdns(3), Some(ClusterId::from([0; 20])));
		assert_eq!(DdcStaking::nodes(NodePubKey::CDNPubKey(CDNNodePubKey::new([5; 32]))), Some(3));

		// Set `CurrentEra`.
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		DdcStaking::on_finalize(System::block_number());

		// Schedule CDN participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(4)));

		// Removal is scheduled, stashed value of 4 is still lock.
		let chilling = DdcStaking::current_era().unwrap() +
			TestClusterVisitor::get_chill_delay(&ClusterId::from([1; 20]), NodeType::CDN)
				.unwrap_or(10_u32);
		assert_eq!(
			DdcStaking::ledger(&4),
			Some(StakingLedger {
				stash: 3,
				total: 1500,
				active: 1500,
				chilling: Some(chilling),
				unlocking: Default::default(),
			})
		);
		// It cannot reserve more than 500 that it has free from the total 2000
		assert_noop!(Balances::reserve(&3, 501), BalancesError::<Test, _>::LiquidityRestrictions);
		assert_ok!(Balances::reserve(&3, 409));

		// Set `CurrentEra` to the value allows us to chill.
		while DdcStaking::current_era().unwrap() < chilling {
			System::set_block_number(System::block_number() + 1);
			Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
			DdcStaking::on_finalize(System::block_number());
		}

		// Ledger is not changed until we make another call to `chill`.
		assert_eq!(
			DdcStaking::ledger(&4),
			Some(StakingLedger {
				stash: 3,
				total: 1500,
				active: 1500,
				chilling: Some(chilling),
				unlocking: Default::default(),
			})
		);

		// Actual CDN participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(4)));

		// Account 3 is no longer a CDN participant.
		assert_eq!(DdcStaking::cdns(3), None);
	});
}
