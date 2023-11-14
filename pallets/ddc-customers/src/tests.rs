//! Tests for the module.

use super::{mock::*, *};
use ddc_primitives::ClusterId;

use frame_support::{assert_noop, assert_ok};
use pallet_balances::Error as BalancesError;

#[test]
fn create_bucket_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);

		// Bucket created
		assert_ok!(DdcCustomers::create_bucket(RuntimeOrigin::signed(1), cluster_id.clone()));

		// Check storage
		assert_eq!(DdcCustomers::buckets_count(), 1);
		assert_eq!(
			DdcCustomers::buckets(&1),
			Some(Bucket { bucket_id: 1, owner_id: 1, cluster_id })
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(Event::BucketCreated { 0: 1u64.into() }.into())

		// let bytes = [0u8; 32];
		// let node_pub_key = AccountId32::from(bytes);
		// let cdn_node_params = CDNNodeParams {
		// 	host: vec![1u8, 255],
		// 	http_port: 35000u16,
		// 	grpc_port: 25000u16,
		// 	p2p_port: 15000u16,
		// };

		// // Node params are not valid
		// assert_noop!(
		// 	DdcNodes::create_node(
		// 		RuntimeOrigin::signed(1),
		// 		NodePubKey::CDNPubKey(node_pub_key.clone()),
		// 		NodeParams::StorageParams(StorageNodeParams {
		// 			host: vec![1u8, 255],
		// 			http_port: 35000u16,
		// 			grpc_port: 25000u16,
		// 			p2p_port: 15000u16,
		// 		})
		// 	),
		// 	Error::<Test>::InvalidNodeParams
		// );

		// // Node already exists
		// assert_noop!(
		// 	DdcNodes::create_node(
		// 		RuntimeOrigin::signed(1),
		// 		NodePubKey::CDNPubKey(node_pub_key.clone()),
		// 		NodeParams::CDNParams(cdn_node_params)
		// 	),
		// 	Error::<Test>::NodeAlreadyExists
		// );

		// // Checking that event was emitted
		// assert_eq!(System::events().len(), 1);
		// System::assert_last_event(
		// 	Event::NodeCreated { node_pub_key: NodePubKey::CDNPubKey(node_pub_key) }.into(),
		// )
	})
}

#[test]
fn deposit_and_deposit_extra_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);
		// let bytes = [0u8; 32];
		// let node_pub_key = AccountId32::from(bytes);
		// let storage_node_params = StorageNodeParams {
		// 	host: vec![1u8, 255],
		// 	http_port: 35000u16,
		// 	grpc_port: 25000u16,
		// 	p2p_port: 15000u16,
		// };
		// let cdn_node_params = CDNNodeParams {
		// 	host: vec![1u8, 255],
		// 	http_port: 35000u16,
		// 	grpc_port: 25000u16,
		// 	p2p_port: 15000u16,
		// };

		// Deposit dust
		assert_noop!(
			DdcCustomers::deposit(RuntimeOrigin::signed(1), 0_u128.into()),
			Error::<Test>::InsufficientDeposit
		);

		// Deposit all tokens fails (should not kill account)
		assert_noop!(
			DdcCustomers::deposit(RuntimeOrigin::signed(1), 100_u128.into()),
			BalancesError::<Test, _>::KeepAlive
		);

		// Deposited
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(1), 10_u128.into()));

		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&1),
			Some(AccountsLedger {
				owner: 1,
				total: 10_u128.into(),
				active: 10_u128.into(),
				unlocking: Default::default(),
			})
		);

		// Checking that event was emitted
		System::assert_last_event(Event::Deposited { 0: 1, 1: 10 }.into());

		// Deposit should fail when called the second time
		assert_noop!(
			DdcCustomers::deposit(RuntimeOrigin::signed(1), 10_u128.into()),
			Error::<Test>::AlreadyPaired
		);

		// Deposit extra fails if not owner
		assert_noop!(
			DdcCustomers::deposit_extra(RuntimeOrigin::signed(2), 10_u128.into()),
			Error::<Test>::NotOwner
		);

		// Deposited extra
		assert_ok!(DdcCustomers::deposit_extra(RuntimeOrigin::signed(1), 20_u128.into()));

		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&1),
			Some(AccountsLedger {
				owner: 1,
				total: 30_u128.into(),
				active: 30_u128.into(),
				unlocking: Default::default(),
			})
		);

		// Checking that event was emitted
		System::assert_last_event(Event::Deposited { 0: 1, 1: 20 }.into());

		// // Set node params
		// assert_ok!(DdcNodes::set_node_params(
		// 	RuntimeOrigin::signed(1),
		// 	NodePubKey::CDNPubKey(node_pub_key.clone()),
		// 	NodeParams::CDNParams(cdn_node_params.clone())
		// ));

		// // Node params are not valid
		// assert_noop!(
		// 	DdcNodes::set_node_params(
		// 		RuntimeOrigin::signed(1),
		// 		NodePubKey::CDNPubKey(node_pub_key.clone()),
		// 		NodeParams::StorageParams(storage_node_params)
		// 	),
		// 	Error::<Test>::InvalidNodeParams
		// );

		// // Only node provider can set params
		// assert_noop!(
		// 	DdcNodes::set_node_params(
		// 		RuntimeOrigin::signed(2),
		// 		NodePubKey::CDNPubKey(node_pub_key.clone()),
		// 		NodeParams::CDNParams(cdn_node_params.clone())
		// 	),
		// 	Error::<Test>::OnlyNodeProvider
		// );
	})
}

#[test]
fn unlock_and_withdraw_deposit_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		// Deposited
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(1), 35_u128.into()));
		// So there is always positive balance within pallet
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(2), 10_u128.into()));

		// Unlock chunk
		assert_ok!(DdcCustomers::unlock_deposit(RuntimeOrigin::signed(1), 1_u128.into()));
		System::set_block_number(2);

		let mut unlocking_chunks: BoundedVec<UnlockChunk<Balance, Test>, MaxUnlockingChunks> =
			Default::default();
		match unlocking_chunks.try_push(UnlockChunk { value: 1, block: 11 }) {
			Ok(_) => (),
			Err(_) => println!("No more chunks"),
		};
		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&1),
			Some(AccountsLedger {
				owner: 1,
				total: 35_u128.into(),
				active: 34_u128.into(),
				unlocking: unlocking_chunks.clone(),
			})
		);

		// Reach max unlock chunks
		for i in 1..32 {
			assert_ok!(DdcCustomers::unlock_deposit(RuntimeOrigin::signed(1), 1_u128.into()));
			System::set_block_number(i + 2);
		}

		// No more chunks can be added
		assert_noop!(
			DdcCustomers::unlock_deposit(RuntimeOrigin::signed(1), 1_u128.into()),
			Error::<Test>::NoMoreChunks
		);

		// Set the block to withdraw all unlocked chunks
		System::set_block_number(42);

		assert_ok!(DdcCustomers::withdraw_unlocked_deposit(RuntimeOrigin::signed(1)));
		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&1),
			Some(AccountsLedger {
				owner: 1,
				total: 3_u128.into(),
				active: 3_u128.into(),
				unlocking: Default::default(),
			})
		);

		// Unlock remaining chuncks & withdraw
		assert_ok!(DdcCustomers::unlock_deposit(RuntimeOrigin::signed(1), 3_u128.into()));
		System::set_block_number(52);
		assert_ok!(DdcCustomers::withdraw_unlocked_deposit(RuntimeOrigin::signed(1)));

		// Check storage
		assert_eq!(DdcCustomers::ledger(&1), None);

		// let bytes = [0u8; 32];
		// let node_pub_key = AccountId32::from(bytes);
		// let cdn_node_params = CDNNodeParams {
		// 	host: vec![1u8, 255],
		// 	http_port: 35000u16,
		// 	grpc_port: 25000u16,
		// 	p2p_port: 15000u16,
		// };

		// // Node doesn't exist
		// assert_noop!(
		// 	DdcNodes::delete_node(
		// 		RuntimeOrigin::signed(1),
		// 		NodePubKey::CDNPubKey(node_pub_key.clone())
		// 	),
		// 	Error::<Test>::NodeDoesNotExist
		// );

		// // Create node
		// assert_ok!(DdcNodes::create_node(
		// 	RuntimeOrigin::signed(1),
		// 	NodePubKey::CDNPubKey(node_pub_key.clone()),
		// 	NodeParams::CDNParams(cdn_node_params.clone())
		// ));

		// // Only node provider can delete
		// assert_noop!(
		// 	DdcNodes::delete_node(
		// 		RuntimeOrigin::signed(2),
		// 		NodePubKey::CDNPubKey(node_pub_key.clone())
		// 	),
		// 	Error::<Test>::OnlyNodeProvider
		// );

		// // Delete node
		// assert_ok!(DdcNodes::delete_node(
		// 	RuntimeOrigin::signed(1),
		// 	NodePubKey::CDNPubKey(node_pub_key.clone()),
		// ));

		// // Checking that event was emitted
		// assert_eq!(System::events().len(), 2);
		// System::assert_last_event(
		// 	Event::NodeDeleted { node_pub_key: NodePubKey::CDNPubKey(node_pub_key) }.into(),
		// )
	})
}
