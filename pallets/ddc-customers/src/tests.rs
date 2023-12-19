//! Tests for the module.

use ddc_primitives::ClusterId;
use frame_support::{assert_noop, assert_ok};
use frame_system::Config;

use super::{mock::*, *};

#[test]
fn create_bucket_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let account_1 = 1;
		let bucket_params = BucketParams { is_public: false };

		// Bucket created
		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(account_1),
			cluster_id,
			bucket_params.clone()
		));

		// Check storage
		assert_eq!(DdcCustomers::buckets_count(), 1);
		assert_eq!(
			DdcCustomers::buckets(1),
			Some(Bucket {
				bucket_id: 1,
				owner_id: account_1,
				cluster_id,
				is_public: bucket_params.is_public
			})
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
		let bucket_1_params = BucketParams { is_public: false };
		let bucket_2_params = BucketParams { is_public: true };

		// Buckets created
		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(account_1),
			cluster_id,
			bucket_1_params.clone()
		));
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(Event::BucketCreated(1u64).into());
		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(account_1),
			cluster_id,
			bucket_2_params.clone()
		));
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(Event::BucketCreated(2u64).into());

		// Check storage
		assert_eq!(DdcCustomers::buckets_count(), 2);
		assert_eq!(
			DdcCustomers::buckets(1),
			Some(Bucket {
				bucket_id: 1,
				owner_id: account_1,
				cluster_id,
				is_public: bucket_1_params.is_public
			})
		);
		assert_eq!(
			DdcCustomers::buckets(2),
			Some(Bucket {
				bucket_id: 2,
				owner_id: account_1,
				cluster_id,
				is_public: bucket_2_params.is_public
			})
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
			Error::<Test>::TransferFailed
		);

		let amount1 = 10_u128;
		// Deposited
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(account_1), amount1));

		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&account_1),
			Some(AccountsLedger {
				owner: account_1,
				total: amount1,
				active: amount1,
				unlocking: Default::default(),
			})
		);

		// Checking that event was emitted
		System::assert_last_event(Event::Deposited(account_1, amount1).into());

		// Deposit should fail when called the second time
		assert_noop!(
			DdcCustomers::deposit(RuntimeOrigin::signed(account_1), amount1),
			Error::<Test>::AlreadyPaired
		);

		// Deposit extra fails if not owner
		assert_noop!(
			DdcCustomers::deposit_extra(RuntimeOrigin::signed(account_2), 10_u128),
			Error::<Test>::NotOwner
		);

		// Deposited extra
		let amount2 = 20_u128;
		assert_ok!(DdcCustomers::deposit_extra(RuntimeOrigin::signed(account_1), amount2));

		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&account_1),
			Some(AccountsLedger {
				owner: account_1,
				total: amount1 + amount2,
				active: amount1 + amount2,
				unlocking: Default::default(),
			})
		);

		// Checking that event was emitted
		System::assert_last_event(Event::Deposited(account_1, amount2).into());
	})
}

#[test]
fn charge_content_owner_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let account_3 = 3;
		let vault = 4;
		let deposit = 100_u128;

		let balance_before_deposit = Balances::free_balance(account_3);
		// Deposited
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(account_3), deposit));
		let balance_after_deposit = Balances::free_balance(account_3);
		assert_eq!(balance_before_deposit - deposit, balance_after_deposit);

		let pallet_balance = Balances::free_balance(DdcCustomers::account_id());
		assert_eq!(deposit, pallet_balance);

		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&account_3),
			Some(AccountsLedger {
				owner: account_3,
				total: deposit,
				active: deposit,
				unlocking: Default::default(),
			})
		);

		// Checking that event was emitted
		System::assert_last_event(Event::Deposited(account_3, deposit).into());

		// successful transfer
		let charge1 = 10;
		let charged = DdcCustomers::charge_content_owner(account_3, vault, charge1).unwrap();
		assert_eq!(charge1, charged);

		let vault_balance = Balances::free_balance(vault);
		assert_eq!(charged, vault_balance);

		let account_balance = Balances::free_balance(account_3);
		assert_eq!(balance_after_deposit, account_balance);

		let pallet_balance_after_charge = Balances::free_balance(DdcCustomers::account_id());
		assert_eq!(pallet_balance - charged, pallet_balance_after_charge);

		// Check storage
		assert_eq!(
			DdcCustomers::ledger(&account_3),
			Some(AccountsLedger {
				owner: account_3,
				total: deposit - charge1,
				active: deposit - charge1,
				unlocking: Default::default(),
			})
		);

		// failed transfer
		let charge2 = 100u128;
		let charge_result = DdcCustomers::charge_content_owner(account_3, vault, charge2).unwrap();
		assert_eq!(
			DdcCustomers::ledger(&account_3),
			Some(AccountsLedger {
				owner: account_3,
				total: 0,
				active: 0,
				unlocking: Default::default(),
			})
		);

		assert_eq!(0, Balances::free_balance(DdcCustomers::account_id()));
		assert_eq!(charge_result, deposit - charge1);

		assert_ok!(DdcCustomers::deposit_extra(RuntimeOrigin::signed(account_3), deposit));
		assert_eq!(
			DdcCustomers::ledger(&account_3),
			Some(AccountsLedger {
				owner: account_3,
				total: deposit,
				active: deposit,
				unlocking: Default::default(),
			})
		);

		assert_eq!(deposit, Balances::free_balance(DdcCustomers::account_id()));
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

#[test]
fn set_bucket_params_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let bucket_owner = 1;
		let bucket_params = BucketParams { is_public: false };

		// Bucket created
		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(bucket_owner),
			cluster_id,
			bucket_params
		));

		// Checking that event was emitted
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(Event::BucketCreated(1u64).into());

		let bucket_id = 1;
		let update_bucket_params = BucketParams { is_public: true };
		assert_ok!(DdcCustomers::set_bucket_params(
			RuntimeOrigin::signed(bucket_owner),
			bucket_id,
			update_bucket_params.clone()
		));

		assert_eq!(DdcCustomers::buckets_count(), 1);
		assert_eq!(
			DdcCustomers::buckets(1),
			Some(Bucket {
				bucket_id,
				owner_id: bucket_owner,
				cluster_id,
				is_public: update_bucket_params.is_public
			})
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(Event::BucketUpdated(bucket_id).into());
	})
}

#[test]
fn set_bucket_params_checks_work() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let bucket_owner = 1;
		let bucket_params = BucketParams { is_public: false };

		// Bucket created
		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(bucket_owner),
			cluster_id,
			bucket_params
		));

		// Checking that event was emitted
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(Event::BucketCreated(1u64).into());
		let bucket_id = 1;

		let non_existent_bucket_id = 2;
		assert_noop!(
			DdcCustomers::set_bucket_params(
				RuntimeOrigin::signed(bucket_owner),
				non_existent_bucket_id,
				BucketParams { is_public: true }
			),
			Error::<Test>::NoBucketWithId
		);

		let not_bucket_owner_id = 2;
		assert_noop!(
			DdcCustomers::set_bucket_params(
				RuntimeOrigin::signed(not_bucket_owner_id),
				bucket_id,
				BucketParams { is_public: true }
			),
			Error::<Test>::NotBucketOwner
		);
	})
}
