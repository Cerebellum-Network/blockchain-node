//! Tests for the module.

use ddc_primitives::ClusterId;
use frame_support::{assert_noop, assert_ok};
use frame_system::Config;
use pallet_balances::Error as BalancesError;

use super::{mock::*, *};

#[test]
fn create_bucket_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let account_1 = 1;

		// Bucket created
		assert_ok!(DdcCustomers::create_bucket(RuntimeOrigin::signed(account_1), cluster_id));

		// Check storage
		assert_eq!(DdcCustomers::buckets_count(), 1);
		assert_eq!(
			DdcCustomers::buckets(1),
			Some(Bucket { bucket_id: 1, owner_id: account_1, cluster_id })
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(Event::BucketCreated(1u64).into())
	})
}

#[test]
fn create_two_buckets_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let account_1 = 1;

		// Buckets created
		assert_ok!(DdcCustomers::create_bucket(RuntimeOrigin::signed(account_1), cluster_id));
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(Event::BucketCreated(1u64).into());
		assert_ok!(DdcCustomers::create_bucket(RuntimeOrigin::signed(account_1), cluster_id));
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(Event::BucketCreated(2u64).into());

		// Check storage
		assert_eq!(DdcCustomers::buckets_count(), 2);
		assert_eq!(
			DdcCustomers::buckets(1),
			Some(Bucket { bucket_id: 1, owner_id: account_1, cluster_id })
		);
		assert_eq!(
			DdcCustomers::buckets(2),
			Some(Bucket { bucket_id: 2, owner_id: account_1, cluster_id })
		);
	})
}

#[test]
fn deposit_and_deposit_extra_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let account_1 = 1;
		let account_2 = 2;

		// Deposit dust
		assert_noop!(
			DdcCustomers::deposit(RuntimeOrigin::signed(account_1), 0_u128),
			Error::<Test>::InsufficientDeposit
		);

		// Deposit all tokens fails (should not kill account)
		assert_noop!(
			DdcCustomers::deposit(RuntimeOrigin::signed(account_1), 100_u128),
			BalancesError::<Test, _>::KeepAlive
		);

		// Deposited
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(account_1), 10_u128));

		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&account_1),
			Some(AccountsLedger {
				owner: 1,
				total: 10_u128,
				active: 10_u128,
				unlocking: Default::default(),
			})
		);

		// Checking that event was emitted
		System::assert_last_event(Event::Deposited(account_1, 10).into());

		// Deposit should fail when called the second time
		assert_noop!(
			DdcCustomers::deposit(RuntimeOrigin::signed(account_1), 10_u128),
			Error::<Test>::AlreadyPaired
		);

		// Deposit extra fails if not owner
		assert_noop!(
			DdcCustomers::deposit_extra(RuntimeOrigin::signed(account_2), 10_u128),
			Error::<Test>::NotOwner
		);

		// Deposited extra
		assert_ok!(DdcCustomers::deposit_extra(RuntimeOrigin::signed(account_1), 20_u128));

		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&account_1),
			Some(AccountsLedger {
				owner: 1,
				total: 30_u128,
				active: 30_u128,
				unlocking: Default::default(),
			})
		);

		// Checking that event was emitted
		System::assert_last_event(Event::Deposited(account_1, 20).into());
	})
}

#[test]
fn unlock_and_withdraw_deposit_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let account_1 = 1;
		let account_2 = 2;

		// Deposited
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(account_1), 35_u128));
		// So there is always positive balance within pallet
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(account_2), 10_u128));

		// Unlock chunk
		assert_ok!(DdcCustomers::unlock_deposit(RuntimeOrigin::signed(account_1), 1_u128));
		System::set_block_number(2);

		let mut unlocking_chunks: BoundedVec<
			UnlockChunk<Balance, <Test as Config>::BlockNumber>,
			MaxUnlockingChunks,
		> = Default::default();
		match unlocking_chunks.try_push(UnlockChunk { value: 1, block: 11 }) {
			Ok(_) => (),
			Err(_) => println!("No more chunks"),
		};
		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&1),
			Some(AccountsLedger {
				owner: account_1,
				total: 35_u128,
				active: 34_u128,
				unlocking: unlocking_chunks.clone(),
			})
		);

		// Reach max unlock chunks
		for i in 1..32 {
			assert_ok!(DdcCustomers::unlock_deposit(RuntimeOrigin::signed(account_1), 1_u128));
			System::set_block_number(i + 2);
		}

		// No more chunks can be added
		assert_noop!(
			DdcCustomers::unlock_deposit(RuntimeOrigin::signed(account_1), 1_u128),
			Error::<Test>::NoMoreChunks
		);

		// Set the block to withdraw all unlocked chunks
		System::set_block_number(42);

		assert_ok!(DdcCustomers::withdraw_unlocked_deposit(RuntimeOrigin::signed(account_1)));
		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&1),
			Some(AccountsLedger {
				owner: account_1,
				total: 3_u128,
				active: 3_u128,
				unlocking: Default::default(),
			})
		);

		// Unlock remaining chunks & withdraw
		assert_ok!(DdcCustomers::unlock_deposit(RuntimeOrigin::signed(account_1), 3_u128));
		System::set_block_number(52);
		assert_ok!(DdcCustomers::withdraw_unlocked_deposit(RuntimeOrigin::signed(account_1)));

		// Check storage
		assert_eq!(DdcCustomers::ledger(&account_1), None);
	})
}
