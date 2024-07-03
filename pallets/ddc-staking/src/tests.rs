//! Tests for the module.

use ddc_primitives::{
	ClusterNodeKind, ClusterParams, ClusterProtocolParams, ClusterStatus, StorageNodeParams,
	StorageNodePubKey,
};
use frame_support::{assert_noop, assert_ok, traits::ReservableCurrency};
use pallet_balances::Error as BalancesError;

use super::{mock::*, *};

pub const BLOCK_TIME: u64 = 1000;
pub const INIT_TIMESTAMP: u64 = 30_000;

#[test]
fn test_default_staking_ledger() {
	let (clusters, clusters_bonds, nodes_bondes) = build_default_setup();
	ExtBuilder.build_and_execute(clusters, clusters_bonds, nodes_bondes, || {
		let default_staking_ledger = StakingLedger::<
			<Test as frame_system::Config>::AccountId,
			BalanceOf<Test>,
			Test,
		>::default_from(AccountId::from(KEY_1));
		// Account 11 is stashed and locked, and account 10 is the controller
		assert_eq!(default_staking_ledger.stash, AccountId::from(KEY_1));
		assert_eq!(default_staking_ledger.total, Zero::zero());
	});
}

#[test]
fn basic_setup_works() {
	let (clusters, clusters_bonds, nodes_bondes) = build_default_setup();
	ExtBuilder.build_and_execute(clusters, clusters_bonds, nodes_bondes, || {
		// Account 11 is stashed and locked, and account 10 is the controller
		assert_eq!(
			DdcStaking::bonded(AccountId::from(NODE_STASH_1)),
			Some(AccountId::from(NODE_CONTROLLER_1))
		);
		// Account 21 is stashed and locked, and account 20 is the controller
		assert_eq!(
			DdcStaking::bonded(AccountId::from(NODE_STASH_2)),
			Some(AccountId::from(NODE_CONTROLLER_2))
		);
		// Account 1 is not a stashed
		assert_eq!(DdcStaking::bonded(AccountId::from(KEY_1)), None);

		// Account 10 controls the stash from account 11, which is 100 units
		assert_eq!(
			DdcStaking::ledger(AccountId::from(NODE_CONTROLLER_1)),
			Some(StakingLedger {
				stash: AccountId::from(NODE_STASH_1),
				total: 100,
				active: 100,
				chilling: Default::default(),
				unlocking: Default::default(),
			})
		);
		// Account 20 controls the stash from account 21, which is 100 units
		assert_eq!(
			DdcStaking::ledger(AccountId::from(NODE_CONTROLLER_2)),
			Some(StakingLedger {
				stash: AccountId::from(NODE_STASH_2),
				total: 100,
				active: 100,
				chilling: Default::default(),
				unlocking: Default::default(),
			})
		);
		// Account 1 does not control any stash
		assert_eq!(DdcStaking::ledger(AccountId::from(KEY_1)), None);
	});
}

#[test]
fn change_controller_works() {
	let (clusters, clusters_bonds, nodes_bondes) = build_default_setup();
	ExtBuilder.build_and_execute(clusters, clusters_bonds, nodes_bondes, || {
		// 10 and 11 are bonded as stash controller.
		assert_eq!(
			DdcStaking::bonded(AccountId::from(NODE_STASH_1)),
			Some(AccountId::from(NODE_CONTROLLER_1))
		);

		// 10 can control 11 who is initially a validator.
		assert_ok!(DdcStaking::withdraw_unbonded(RuntimeOrigin::signed(AccountId::from(
			NODE_CONTROLLER_1
		))));

		// Change controller.
		assert_ok!(DdcStaking::set_controller(
			RuntimeOrigin::signed(AccountId::from(NODE_STASH_1)),
			AccountId::from(KEY_3)
		));
		assert_noop!(
			DdcStaking::set_controller(
				RuntimeOrigin::signed(AccountId::from(NODE_STASH_1)),
				AccountId::from(KEY_3)
			),
			Error::<Test>::AlreadyPaired
		);
		assert_eq!(DdcStaking::bonded(AccountId::from(NODE_STASH_1)), Some(AccountId::from(KEY_3)));

		// 10 is no longer in control.
		assert_noop!(
			DdcStaking::store(
				RuntimeOrigin::signed(AccountId::from(NODE_CONTROLLER_1)),
				ClusterId::from(CLUSTER_ID)
			),
			Error::<Test>::NotController
		);
		// 3 is a new controller.
		assert_ok!(DdcStaking::store(
			RuntimeOrigin::signed(AccountId::from(KEY_3)),
			ClusterId::from(CLUSTER_ID)
		));
	})
}

#[test]
fn not_enough_inital_bond_flow() {
	let (clusters, clusters_bonds, nodes_bondes) = build_default_setup();
	ExtBuilder.build_and_execute(clusters, clusters_bonds, nodes_bondes, || {
		System::set_block_number(1);

		// Add new Storage participant, account 3 controlled by 4 with node 5.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(AccountId::from(KEY_3)),
			AccountId::from(KEY_4),
			NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32])),
			5
		));

		// Not enough tokens bonded to serve
		assert_noop!(
			DdcStaking::store(
				RuntimeOrigin::signed(AccountId::from(KEY_4)),
				ClusterId::from(CLUSTER_ID)
			),
			Error::<Test>::InsufficientBond
		);

		// Add new Storage participant, account 1 controlled by 2 with node 3.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(AccountId::from(KEY_1)),
			AccountId::from(KEY_2),
			NodePubKey::StoragePubKey(StorageNodePubKey::new([3; 32])),
			100
		));

		// Not enough tokens bonded to store
		assert_noop!(
			DdcStaking::store(
				RuntimeOrigin::signed(AccountId::from(KEY_4)),
				ClusterId::from(CLUSTER_ID)
			),
			Error::<Test>::InsufficientBond
		);

		// Can not bond extra
		assert_noop!(
			DdcStaking::bond(
				RuntimeOrigin::signed(AccountId::from(KEY_3)),
				AccountId::from(KEY_4),
				NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32])),
				5
			),
			Error::<Test>::AlreadyBonded
		);

		// Unbond all bonded amount
		assert_ok!(DdcStaking::unbond(RuntimeOrigin::signed(AccountId::from(KEY_4)), 5));
		System::assert_last_event(Event::Unbonded(AccountId::from(KEY_3), 5).into());
		System::set_block_number(11);
		// Withdraw unbonded tokens to clear up the stash controller pair
		assert_ok!(DdcStaking::withdraw_unbonded(RuntimeOrigin::signed(AccountId::from(KEY_4))));
		System::assert_last_event(Event::Withdrawn(AccountId::from(KEY_3), 5).into());

		// Bond sufficient amount
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(AccountId::from(KEY_3)),
			AccountId::from(KEY_4),
			NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32])),
			10
		));

		// Serving should work
		assert_ok!(DdcStaking::store(
			RuntimeOrigin::signed(AccountId::from(KEY_4)),
			ClusterId::from(CLUSTER_ID)
		));
	})
}

#[test]
fn unbonding_edge_cases_work() {
	let (clusters, clusters_bonds, nodes_bondes) = build_default_setup();
	ExtBuilder.build_and_execute(clusters, clusters_bonds, nodes_bondes, || {
		System::set_block_number(1);

		// Add new Storage participant, account 3 controlled by 4 with node 5.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(AccountId::from(KEY_3)),
			AccountId::from(KEY_4),
			NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32])),
			100
		));

		assert_ok!(DdcStaking::store(
			RuntimeOrigin::signed(AccountId::from(KEY_4)),
			ClusterId::from(CLUSTER_ID)
		));

		assert_ok!(DdcStaking::unbond(RuntimeOrigin::signed(AccountId::from(KEY_4)), 1));
		while System::block_number() < 33 {
			assert_ok!(DdcStaking::unbond(RuntimeOrigin::signed(AccountId::from(KEY_4)), 1));
			System::assert_last_event(Event::Unbonded(AccountId::from(KEY_3), 1).into());
			System::set_block_number(System::block_number() + 1);
		}

		assert_noop!(
			DdcStaking::unbond(RuntimeOrigin::signed(AccountId::from(KEY_4)), 1),
			Error::<Test>::NoMoreChunks
		);
	})
}

#[test]
fn set_node_works() {
	let (clusters, clusters_bonds, nodes_bondes) = build_default_setup();
	ExtBuilder.build_and_execute(clusters, clusters_bonds, nodes_bondes, || {
		System::set_block_number(1);
		// 10 and 11 are bonded as stash controller.
		assert_eq!(
			DdcStaking::bonded(AccountId::from(NODE_STASH_1)),
			Some(AccountId::from(NODE_CONTROLLER_1))
		);

		// Node is already paired
		assert_noop!(
			DdcStaking::set_node(
				RuntimeOrigin::signed(AccountId::from(NODE_CONTROLLER_1)),
				NodePubKey::StoragePubKey(StorageNodePubKey::new(NODE_PUB_KEY_1))
			),
			Error::<Test>::AlreadyPaired
		);

		// Node cannot be changed
		assert_noop!(
			DdcStaking::set_node(
				RuntimeOrigin::signed(AccountId::from(NODE_STASH_1)),
				NodePubKey::StoragePubKey(StorageNodePubKey::new(NODE_PUB_KEY_1))
			),
			Error::<Test>::AlreadyInRole
		);

		// Schedule Storage participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(AccountId::from(NODE_CONTROLLER_1))));
		System::set_block_number(11);
		// Actual Storage participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(AccountId::from(NODE_CONTROLLER_1))));

		// Setting node should work
		assert_ok!(DdcStaking::set_node(
			RuntimeOrigin::signed(AccountId::from(NODE_STASH_1)),
			NodePubKey::StoragePubKey(StorageNodePubKey::new([13; 32]))
		));
	})
}

#[test]
fn cancel_previous_chill_works() {
	let (clusters, clusters_bonds, nodes_bondes) = build_default_setup();
	ExtBuilder.build_and_execute(clusters, clusters_bonds, nodes_bondes, || {
		System::set_block_number(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		// Add new Storage participant, account 3 controlled by 4 with node 5.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(AccountId::from(KEY_3)),
			AccountId::from(KEY_4),
			NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32])),
			100
		));

		// Add new Storage participant, account 1 controlled by 2 with node 3.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(AccountId::from(KEY_1)),
			AccountId::from(KEY_2),
			NodePubKey::StoragePubKey(StorageNodePubKey::new([3; 32])),
			100
		));

		// Not enough tokens bonded to serve
		assert_ok!(DdcStaking::store(RuntimeOrigin::signed(AccountId::from(KEY_4)), cluster_id));

		assert_ok!(DdcStaking::store(
			RuntimeOrigin::signed(AccountId::from(KEY_2)),
			ClusterId::from(CLUSTER_ID)
		));

		// Schedule Storage participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(AccountId::from(KEY_4))));
		// Not enough tokens bonded to serve
		assert_ok!(DdcStaking::store(RuntimeOrigin::signed(AccountId::from(KEY_4)), cluster_id));

		// Schedule Storage participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(AccountId::from(KEY_2))));
		// Not enough tokens bonded to serve
		assert_ok!(DdcStaking::store(RuntimeOrigin::signed(AccountId::from(KEY_2)), cluster_id));
	})
}

// #[test]
// fn staking_should_work() {
// 	let (clusters, clusters_bonds, nodes_bondes) = build_default_setup();
// 	ExtBuilder.build_and_execute(clusters, clusters_bonds, nodes_bondes, || {
// 		System::set_block_number(1);

// 		const KEY_5: [u8; 32] = [5; 32];
// 		const KEY_6: [u8; 32] = [6; 32];

// 		// Put some money in account that we'll use.
// 		let _ = Balances::make_free_balance_be(&AccountId::from(KEY_1), 2000);
// 		let _ = Balances::make_free_balance_be(&AccountId::from(KEY_2), 2000);
// 		let _ = Balances::make_free_balance_be(&AccountId::from(KEY_3), 2000);
// 		let _ = Balances::make_free_balance_be(&AccountId::from(KEY_4), 2000);

// 		// Bond dust should fail
// 		assert_noop!(
// 			DdcStaking::bond(
// 				RuntimeOrigin::signed(AccountId::from(KEY_3)),
// 				AccountId::from(KEY_4),
// 				NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32])),
// 				0
// 			),
// 			Error::<Test>::InsufficientBond
// 		);

// 		// Add new Storage participant, account 3 controlled by 4 with node 5.
// 		assert_ok!(DdcStaking::bond(
// 			RuntimeOrigin::signed(AccountId::from(KEY_3)),
// 			AccountId::from(KEY_4),
// 			NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32])),
// 			1500
// 		));
// 		let events = System::events();
// 		assert_eq!(
// 			events[events.len() - 2].event,
// 			Event::Bonded(AccountId::from(KEY_3), 1500).into()
// 		);
// 		assert_ok!(DdcStaking::store(
// 			RuntimeOrigin::signed(AccountId::from(KEY_4)),
// 			ClusterId::from(CLUSTER_ID)
// 		));
// 		System::assert_last_event(Event::Activated(AccountId::from(KEY_3)).into());

// 		// Controller already paired
// 		assert_noop!(
// 			DdcStaking::bond(
// 				RuntimeOrigin::signed(AccountId::from(KEY_5)),
// 				AccountId::from(KEY_4),
// 				NodePubKey::StoragePubKey(StorageNodePubKey::new([10; 32])),
// 				10
// 			),
// 			Error::<Test>::AlreadyPaired
// 		);

// 		// Node already paired
// 		assert_noop!(
// 			DdcStaking::bond(
// 				RuntimeOrigin::signed(AccountId::from(KEY_5)),
// 				AccountId::from(KEY_6),
// 				NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32])),
// 				10
// 			),
// 			Error::<Test>::AlreadyPaired
// 		);

// 		// Account 4 controls the stash from account 3, which is 1500 units, 3 is a Storage
// 		// participant, 5 is a DDC node.
// 		assert_eq!(DdcStaking::bonded(AccountId::from(KEY_3)), Some(AccountId::from(KEY_4)));
// 		assert_eq!(
// 			DdcStaking::ledger(AccountId::from(KEY_4)),
// 			Some(StakingLedger {
// 				stash: AccountId::from(KEY_3),
// 				total: 1500,
// 				active: 1500,
// 				chilling: Default::default(),
// 				unlocking: Default::default(),
// 			})
// 		);
// 		assert_eq!(DdcStaking::storages(AccountId::from(KEY_3)), Some(ClusterId::from(CLUSTER_ID)));
// 		assert_eq!(
// 			DdcStaking::nodes(NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32]))),
// 			Some(AccountId::from(KEY_3))
// 		);

// 		// Set initial block timestamp.
// 		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);

// 		// Schedule Storage participant removal.
// 		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(AccountId::from(KEY_4))));
// 		System::assert_last_event(
// 			Event::ChillSoon(AccountId::from(KEY_3), ClusterId::from(CLUSTER_ID), 11).into(),
// 		);

// 		// Removal is scheduled, stashed value of 4 is still lock.
// 		let chilling = System::block_number() + 10u64;
// 		// TestClusterProtocol::get_chill_delay(&ClusterId::from([1; 20]), NodeType::Storage)
// 		// 	.unwrap_or(10_u64);
// 		assert_eq!(
// 			DdcStaking::ledger(AccountId::from(KEY_4)),
// 			Some(StakingLedger {
// 				stash: AccountId::from(KEY_3),
// 				total: 1500,
// 				active: 1500,
// 				chilling: Some(chilling),
// 				unlocking: Default::default(),
// 			})
// 		);
// 		// It cannot reserve more than 500 that it has free from the total 2000
// 		assert_noop!(
// 			Balances::reserve(&AccountId::from(KEY_3), 501),
// 			BalancesError::<Test, _>::LiquidityRestrictions
// 		);
// 		assert_ok!(Balances::reserve(&AccountId::from(KEY_3), 409));

// 		// Too early to call chill the second time
// 		assert_noop!(
// 			DdcStaking::chill(RuntimeOrigin::signed(AccountId::from(KEY_4))),
// 			Error::<Test>::TooEarly
// 		);

// 		// Fast chill should not be allowed
// 		assert_noop!(
// 			DdcStaking::fast_chill(RuntimeOrigin::signed(AccountId::from(KEY_4))),
// 			Error::<Test>::FastChillProhibited
// 		);

// 		// Set the block number that allows us to chill.
// 		while System::block_number() < chilling {
// 			System::set_block_number(System::block_number() + 1);
// 			Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
// 		}

// 		// Ledger is not changed until we make another call to `chill`.
// 		assert_eq!(
// 			DdcStaking::ledger(AccountId::from(KEY_4)),
// 			Some(StakingLedger {
// 				stash: AccountId::from(KEY_3),
// 				total: 1500,
// 				active: 1500,
// 				chilling: Some(chilling),
// 				unlocking: Default::default(),
// 			})
// 		);

// 		// Actual Storage participant removal.
// 		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(AccountId::from(KEY_4))));
// 		System::assert_last_event(Event::Chilled(AccountId::from(KEY_3)).into());

// 		// Account 3 is no longer a Storage participant.
// 		assert_eq!(DdcStaking::storages(AccountId::from(KEY_3)), None);
// 	});
// }

#[test]
fn storage_full_unbonding_works() {
	let (clusters, clusters_bonds, nodes_bondes) = build_default_setup();
	ExtBuilder.build_and_execute(clusters, clusters_bonds, nodes_bondes, || {
		System::set_block_number(1);

		let provider_stash = AccountId::from(KEY_3);
		let provider_controller = AccountId::from(KEY_4);
		let cluster_id = ClusterId::from(CLUSTER_ID);
		let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([2; 32]));

		let lock = MockNodeVisitor::set_and_hold_lock(MockNode {
			cluster_id: Some(cluster_id),
			exists: true,
			node_provider_id: provider_controller.clone(),
		});

		let storage_bond_size = 10_u128;
		let storage_chill_delay = 10_u64;
		let storage_unbond_delay = 10_u64;

		// Put some money in account that we'll use.
		let _ = Balances::make_free_balance_be(&provider_controller.clone(), 2000);
		let _ = Balances::make_free_balance_be(&provider_stash.clone(), 2000);

		// Add new Storage participant, account 1 controlled by 2 with node 1.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(provider_stash.clone()),
			provider_controller.clone(),
			node_pub_key.clone(),
			storage_bond_size, // min bond size
		));
		let events = System::events();
		assert_eq!(
			events[events.len() - 2].event,
			Event::Bonded(provider_stash.clone(), storage_bond_size).into()
		);
		assert_ok!(DdcStaking::store(
			RuntimeOrigin::signed(provider_controller.clone()),
			cluster_id
		));
		System::assert_last_event(Event::Activated(provider_stash.clone()).into());

		assert_eq!(DdcStaking::storages(provider_stash.clone()), Some(cluster_id));
		assert_eq!(DdcStaking::nodes(node_pub_key), Some(provider_stash.clone()));

		// Set block timestamp.
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);

		// Schedule Storage participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(provider_controller.clone())));
		let chilling = System::block_number() + storage_chill_delay;
		System::assert_last_event(
			Event::ChillSoon(provider_stash.clone(), cluster_id, chilling).into(),
		);

		// Set the block number that allows us to chill.
		while System::block_number() < chilling {
			System::set_block_number(System::block_number() + 1);
			Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		}

		// Actual Storage participant removal.
		assert_ok!(DdcStaking::chill(RuntimeOrigin::signed(provider_controller.clone())));
		System::assert_last_event(Event::Chilled(provider_stash.clone()).into());

		// Account is no longer a Storage participant.
		assert_eq!(DdcStaking::storages(provider_stash.clone()), None);

		// Start unbonding all tokens
		assert_ok!(DdcStaking::unbond(
			RuntimeOrigin::signed(provider_controller.clone()),
			storage_bond_size
		));
		System::assert_has_event(Event::LeaveSoon(provider_stash.clone()).into());
		assert_eq!(DdcStaking::leaving_storages(provider_stash.clone()), Some(cluster_id));
		System::assert_last_event(
			Event::Unbonded(provider_stash.clone(), storage_bond_size).into(),
		);

		let unbonding = System::block_number() + storage_unbond_delay;
		// Set the block number that allows us to chill.
		while System::block_number() < unbonding {
			System::set_block_number(System::block_number() + 1);
			Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		}

		assert_ok!(DdcStaking::withdraw_unbonded(RuntimeOrigin::signed(
			provider_controller.clone()
		)));
		System::assert_has_event(
			Event::Withdrawn(provider_stash.clone(), storage_bond_size).into(),
		);
		assert_eq!(DdcStaking::leaving_storages(provider_stash.clone()), None);
		System::assert_last_event(Event::Left(provider_stash).into());

		MockNodeVisitor::reset_and_release_lock(lock);
	});
}

#[test]
fn staking_creator_works() {
	let (clusters, clusters_bonds, nodes_bondes) = build_default_setup();
	ExtBuilder.build_and_execute(clusters, clusters_bonds, nodes_bondes, || {
		let stash = AccountId::from(KEY_1);
		let controller = AccountId::from(KEY_2);
		let cluster_id = ClusterId::from(CLUSTER_ID);
		let value = 5;
		let storage_node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([2; 32]));

		assert_ok!(
			<DdcStaking as StakerCreator<Test, BalanceOf<Test>>>::bond_stake_and_participate(
				stash,
				controller,
				storage_node_pub_key,
				value,
				cluster_id,
			)
		);
	});
}

#[test]
fn staking_visitor_works() {
	let (clusters, clusters_bonds, nodes_bondes) = build_default_setup();
	ExtBuilder.build_and_execute(clusters, clusters_bonds, nodes_bondes, || {
		let cluster_id = ClusterId::from(CLUSTER_ID);
		let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([5; 32]));

		// Add new Storage participant, account 3 controlled by 4 with node 5.
		assert_ok!(DdcStaking::bond(
			RuntimeOrigin::signed(AccountId::from(KEY_3)),
			AccountId::from(KEY_4),
			node_pub_key.clone(),
			100
		));

		assert!(<DdcStaking as StakingVisitor<Test>>::has_stake(&node_pub_key,));

		if let Ok(result) =
			<DdcStaking as StakingVisitor<Test>>::has_chilling_attempt(&node_pub_key)
		{
			assert!(!result);
		}

		assert_ok!(DdcStaking::store(
			RuntimeOrigin::signed(AccountId::from(KEY_4)),
			ClusterId::from(CLUSTER_ID)
		));

		if let Ok(result) =
			<DdcStaking as StakingVisitor<Test>>::has_activated_stake(&node_pub_key, &cluster_id)
		{
			assert!(result);
		}
	});
}
