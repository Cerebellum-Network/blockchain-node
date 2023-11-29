//! Tests for the module.

use ddc_primitives::{CDNNodePubKey, StorageNodePubKey};
use frame_support::{assert_noop, assert_ok, traits::ReservableCurrency};
use pallet_balances::Error as BalancesError;

use super::{mock::*, *};

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
fn not_enough_inital_bond_flow() {
	ExtBuilder::default().build_and_execute(|| {
		System::set_block_number(1);

		// Add new CDN participant, account 3 controlled by 4 with node 5.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(3),
			4,
			NodePubKey::CDNPubKey(CDNNodePubKey::new([5; 32])),
			5
		));

		// Not enough tokens bonded to serve
		assert_noop!(
			DdcStaking::serve(RuntimeOrigin::signed(4), ClusterId::from([1; 20])),
			Error::<Test>::InsufficientBond
		);

		// Can not bond extra
		assert_noop!(
			DdcStaking::bond(
				RuntimeOrigin::signed(3),
				4,
				NodePubKey::CDNPubKey(CDNNodePubKey::new([5; 32])),
				5
			),
			Error::<Test>::AlreadyBonded
		);

		// Unbond all bonded amount
		assert_ok!(DdcStaking::unbond(RuntimeOrigin::signed(4), 5));
		System::assert_last_event(Event::Unbonded(3, 5).into());
		System::set_block_number(11);
		// Withdraw unbonded tokens to clear up the stash controller pair
		assert_ok!(DdcStaking::withdraw_unbonded(RuntimeOrigin::signed(4)));
		System::assert_last_event(Event::Withdrawn(3, 5).into());

		// Bond sufficient amount
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(3),
			4,
			NodePubKey::CDNPubKey(CDNNodePubKey::new([5; 32])),
			10
		));

		// Serving should work
		assert_ok!(DdcStaking::serve(RuntimeOrigin::signed(4), ClusterId::from([1; 20])));
	})
}

#[test]
fn set_node_works() {
	ExtBuilder::default().build_and_execute(|| {
		System::set_block_number(1);
		// 10 and 11 are bonded as stash controller.
		assert_eq!(DdcStaking::bonded(&11), Some(10));

		// Node is already paired
		assert_noop!(
			DdcStaking::set_node(
				RuntimeOrigin::signed(10),
				NodePubKey::CDNPubKey(CDNNodePubKey::new([12; 32]))
			),
			Error::<Test>::AlreadyPaired
		);

		// Node cannot be changed
		assert_noop!(
			DdcStaking::set_node(
				RuntimeOrigin::signed(11),
				NodePubKey::CDNPubKey(CDNNodePubKey::new([12; 32]))
			),
			Error::<Test>::AlreadyInRole
		);

		// Schedule CDN participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(10)));
		System::set_block_number(11);
		// Actual CDN participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(10)));

		// Setting node should work
		assert_ok!(DdcStaking::set_node(
			RuntimeOrigin::signed(11),
			NodePubKey::CDNPubKey(CDNNodePubKey::new([13; 32]))
		));
	})
}

#[test]
fn staking_should_work() {
	ExtBuilder::default().build_and_execute(|| {
		System::set_block_number(1);

		// Put some money in account that we'll use.
		for i in 1..5 {
			let _ = Balances::make_free_balance_be(&i, 2000);
		}

		// Bond dust should fail
		assert_noop!(
			DdcStaking::bond(
				RuntimeOrigin::signed(3),
				4,
				NodePubKey::CDNPubKey(CDNNodePubKey::new([5; 32])),
				0
			),
			Error::<Test>::InsufficientBond
		);

		// Add new CDN participant, account 3 controlled by 4 with node 5.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(3),
			4,
			NodePubKey::CDNPubKey(CDNNodePubKey::new([5; 32])),
			1500
		));
		System::assert_last_event(Event::Bonded(3, 1500).into());
		assert_ok!(DdcStaking::serve(RuntimeOrigin::signed(4), ClusterId::from([0; 20])));
		System::assert_last_event(Event::Activated(3).into());

		// Controller already paired
		assert_noop!(
			DdcStaking::bond(
				RuntimeOrigin::signed(5),
				4,
				NodePubKey::CDNPubKey(CDNNodePubKey::new([10; 32])),
				10
			),
			Error::<Test>::AlreadyPaired
		);

		// Node already paired
		assert_noop!(
			DdcStaking::bond(
				RuntimeOrigin::signed(5),
				6,
				NodePubKey::CDNPubKey(CDNNodePubKey::new([5; 32])),
				10
			),
			Error::<Test>::AlreadyPaired
		);

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

		// Set initial block timestamp.
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);

		// Schedule CDN participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(4)));
		System::assert_last_event(Event::ChillSoon(3, ClusterId::from([0; 20]), 11).into());

		// Removal is scheduled, stashed value of 4 is still lock.
		let chilling = System::block_number() + 10u64;
		// TestClusterVisitor::get_chill_delay(&ClusterId::from([1; 20]), NodeType::CDN)
		// 	.unwrap_or(10_u64);
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

		// Too early to call chill the second time
		assert_noop!(DdcStaking::chill(RuntimeOrigin::signed(4)), Error::<Test>::TooEarly);

		// Fast chill should not be allowed
		assert_noop!(
			DdcStaking::fast_chill(RuntimeOrigin::signed(4)),
			Error::<Test>::FastChillProhibited
		);

		// Set the block number that allows us to chill.
		while System::block_number() < chilling {
			System::set_block_number(System::block_number() + 1);
			Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
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
		System::assert_last_event(Event::Chilled(3).into());

		// Account 3 is no longer a CDN participant.
		assert_eq!(DdcStaking::cdns(3), None);
	});
}

#[test]
fn cdn_full_unbonding_works() {
	ExtBuilder::default().build_and_execute(|| {
		System::set_block_number(1);

		let provider_stash: u64 = 1;
		let provider_controller: u64 = 2;
		let cluster_id = ClusterId::from([1; 20]);
		let node_pub_key = NodePubKey::CDNPubKey(CDNNodePubKey::new([1; 32]));

		let lock = MockNodeVisitor::set_and_hold_lock(MockNode {
			cluster_id: Some(cluster_id),
			exists: true,
		});

		let cdn_bond_size = 10_u128;
		let cdn_chill_delay = 10_u64;
		let cdn_unbond_delay = 10_u64;

		// Put some money in account that we'll use.
		let _ = Balances::make_free_balance_be(&provider_controller, 2000);
		let _ = Balances::make_free_balance_be(&provider_stash, 2000);

		// Add new CDN participant, account 1 controlled by 2 with node 1.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(provider_stash),
			provider_controller,
			node_pub_key.clone(),
			cdn_bond_size, // min bond size
		));
		System::assert_last_event(Event::Bonded(provider_stash, cdn_bond_size).into());
		assert_ok!(DdcStaking::serve(RuntimeOrigin::signed(provider_controller), cluster_id));
		System::assert_last_event(Event::Activated(provider_stash).into());

		assert_eq!(DdcStaking::cdns(provider_stash), Some(cluster_id));
		assert_eq!(DdcStaking::nodes(node_pub_key), Some(provider_stash));

		// Set block timestamp.
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);

		// Schedule CDN participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(provider_controller)));
		let chilling = System::block_number() + cdn_chill_delay;
		System::assert_last_event(Event::ChillSoon(provider_stash, cluster_id, chilling).into());

		// Set the block number that allows us to chill.
		while System::block_number() < chilling {
			System::set_block_number(System::block_number() + 1);
			Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		}

		// Actual CDN participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(provider_controller)));
		System::assert_last_event(Event::Chilled(provider_stash).into());

		// Account is no longer a CDN participant.
		assert_eq!(DdcStaking::cdns(provider_stash), None);

		// Start unbonding all tokens
		assert_ok!(DdcStaking::unbond(RuntimeOrigin::signed(provider_controller), cdn_bond_size));
		System::assert_has_event(Event::LeaveSoon(provider_stash).into());
		assert_eq!(DdcStaking::leaving_cdns(provider_stash), Some(cluster_id));
		System::assert_last_event(Event::Unbonded(provider_stash, cdn_bond_size).into());

		let unbonding = System::block_number() + cdn_unbond_delay;
		// Set the block number that allows us to chill.
		while System::block_number() < unbonding {
			System::set_block_number(System::block_number() + 1);
			Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		}

		assert_ok!(DdcStaking::withdraw_unbonded(RuntimeOrigin::signed(provider_controller)));
		System::assert_has_event(Event::Withdrawn(provider_stash, cdn_bond_size).into());
		assert_eq!(DdcStaking::leaving_cdns(provider_stash), None);
		System::assert_last_event(Event::Left(provider_stash).into());

		MockNodeVisitor::reset_and_release_lock(lock);
	});
}

#[test]
fn storage_full_unbonding_works() {
	ExtBuilder::default().build_and_execute(|| {
		System::set_block_number(1);

		let provider_stash: u64 = 3;
		let provider_controller: u64 = 4;
		let cluster_id = ClusterId::from([1; 20]);
		let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([2; 32]));

		let lock = MockNodeVisitor::set_and_hold_lock(MockNode {
			cluster_id: Some(cluster_id),
			exists: true,
		});

		let storage_bond_size = 10_u128;
		let storage_chill_delay = 10_u64;
		let storage_unbond_delay = 10_u64;

		// Put some money in account that we'll use.
		let _ = Balances::make_free_balance_be(&provider_controller, 2000);
		let _ = Balances::make_free_balance_be(&provider_stash, 2000);

		// Add new Storage participant, account 1 controlled by 2 with node 1.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(provider_stash),
			provider_controller,
			node_pub_key.clone(),
			storage_bond_size, // min bond size
		));
		System::assert_last_event(Event::Bonded(provider_stash, storage_bond_size).into());
		assert_ok!(DdcStaking::store(RuntimeOrigin::signed(provider_controller), cluster_id));
		System::assert_last_event(Event::Activated(provider_stash).into());

		assert_eq!(DdcStaking::storages(provider_stash), Some(cluster_id));
		assert_eq!(DdcStaking::nodes(node_pub_key), Some(provider_stash));

		// Set block timestamp.
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);

		// Schedule Storage participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(provider_controller)));
		let chilling = System::block_number() + storage_chill_delay;
		System::assert_last_event(Event::ChillSoon(provider_stash, cluster_id, chilling).into());

		// Set the block number that allows us to chill.
		while System::block_number() < chilling {
			System::set_block_number(System::block_number() + 1);
			Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		}

		// Actual Storage participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(provider_controller)));
		System::assert_last_event(Event::Chilled(provider_stash).into());

		// Account is no longer a Storage participant.
		assert_eq!(DdcStaking::storages(provider_stash), None);

		// Start unbonding all tokens
		assert_ok!(DdcStaking::unbond(
			RuntimeOrigin::signed(provider_controller),
			storage_bond_size
		));
		System::assert_has_event(Event::LeaveSoon(provider_stash).into());
		assert_eq!(DdcStaking::leaving_storages(provider_stash), Some(cluster_id));
		System::assert_last_event(Event::Unbonded(provider_stash, storage_bond_size).into());

		let unbonding = System::block_number() + storage_unbond_delay;
		// Set the block number that allows us to chill.
		while System::block_number() < unbonding {
			System::set_block_number(System::block_number() + 1);
			Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		}

		assert_ok!(DdcStaking::withdraw_unbonded(RuntimeOrigin::signed(provider_controller)));
		System::assert_has_event(Event::Withdrawn(provider_stash, storage_bond_size).into());
		assert_eq!(DdcStaking::leaving_storages(provider_stash), None);
		System::assert_last_event(Event::Left(provider_stash).into());

		MockNodeVisitor::reset_and_release_lock(lock);
	});
}
