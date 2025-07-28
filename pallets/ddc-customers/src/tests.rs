//! Tests for the module.

use frame_support::{assert_noop, assert_ok};

use super::{mock::*, *};

#[test]
fn create_bucket_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let account1 = 1;
		let bucket_params = BucketParams { is_public: false };

		// Bucket created
		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(account1),
			cluster_id,
			bucket_params.clone()
		));

		// Check storage
		assert_eq!(BucketsCount::<Test>::get(), 1);
		assert_eq!(
			Buckets::<Test>::get(1),
			Some(Bucket {
				bucket_id: 1,
				owner_id: account1,
				cluster_id,
				is_public: bucket_params.is_public,
				is_removed: false,
			})
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(Event::BucketCreated { cluster_id, bucket_id: 1u64 }.into())
	})
}

#[test]
fn create_two_buckets_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let account1 = 1;
		let bucket_1_params = BucketParams { is_public: false };
		let bucket_2_params = BucketParams { is_public: true };

		// Buckets created
		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(account1),
			cluster_id,
			bucket_1_params.clone()
		));
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(Event::BucketCreated { cluster_id, bucket_id: 1u64 }.into());
		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(account1),
			cluster_id,
			bucket_2_params.clone()
		));
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(Event::BucketCreated { cluster_id, bucket_id: 2u64 }.into());

		// Check storage
		assert_eq!(BucketsCount::<Test>::get(), 2);
		assert_eq!(
			Buckets::<Test>::get(1),
			Some(Bucket {
				bucket_id: 1,
				owner_id: account1,
				cluster_id,
				is_public: bucket_1_params.is_public,
				is_removed: false,
			})
		);
		assert_eq!(
			Buckets::<Test>::get(2),
			Some(Bucket {
				bucket_id: 2,
				owner_id: account1,
				cluster_id,
				is_public: bucket_2_params.is_public,
				is_removed: false,
			})
		);
	})
}

#[test]
fn deposit_and_deposit_extra_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let account1 = 1;
		let account2 = 2;

		// Deposit dust
		assert_noop!(
			DdcCustomers::deposit(RuntimeOrigin::signed(account1), cluster_id, 0_u128),
			Error::<Test>::InsufficientDeposit
		);

		// Deposit all tokens fails (should not kill account)
		assert_noop!(
			DdcCustomers::deposit(RuntimeOrigin::signed(account1), cluster_id, 100_u128),
			Error::<Test>::TransferFailed
		);

		let amount1 = 90_u128;
		// Deposited
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(account1), cluster_id, amount1));

		// Check storage
		assert_eq!(
			ClusterLedger::<Test>::get(cluster_id, account1),
			Some(CustomerLedger {
				owner: account1,
				total: amount1,
				active: amount1,
				unlocking: Default::default()
			})
		);

		// Checking that event was emitted
		System::assert_last_event(
			Event::Deposited { cluster_id, owner_id: account1, amount: amount1 }.into(),
		);

		// Deposit should fail when called the second time
		assert_noop!(
			DdcCustomers::deposit(RuntimeOrigin::signed(account1), cluster_id, amount1),
			Error::<Test>::AlreadyPaired
		);

		// Deposit extra fails if not owner
		assert_noop!(
			DdcCustomers::deposit_extra(RuntimeOrigin::signed(account2), cluster_id, 10_u128),
			Error::<Test>::NotOwner
		);

		// Deposit of an extra amount that is more than the customer's total balance fails
		let extra_amount1 = 20_u128;
		assert_noop!(
			DdcCustomers::deposit_extra(RuntimeOrigin::signed(account1), cluster_id, extra_amount1),
			Error::<Test>::TransferFailed
		);

		let extra_amount2 = 5_u128;

		// Deposited extra
		assert_ok!(DdcCustomers::deposit_extra(
			RuntimeOrigin::signed(account1),
			cluster_id,
			extra_amount2
		));

		// Check storage
		assert_eq!(
			ClusterLedger::<Test>::get(cluster_id, account1),
			Some(CustomerLedger {
				owner: account1,
				total: amount1 + extra_amount2,
				active: amount1 + extra_amount2,
				unlocking: Default::default(),
			})
		);

		// Checking that event was emitted
		System::assert_last_event(
			Event::Deposited { cluster_id, owner_id: account1, amount: extra_amount2 }.into(),
		);
	})
}

#[test]
fn deposit_for_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let account1 = 1;
		let funder_account = 99;

		// Deposit dust
		assert_noop!(
			DdcCustomers::deposit_for(
				RuntimeOrigin::signed(funder_account),
				account1,
				cluster_id,
				0_u128
			),
			Error::<Test>::InsufficientDeposit
		);

		// Deposit all tokens fails
		assert_noop!(
			DdcCustomers::deposit_for(
				RuntimeOrigin::signed(funder_account),
				account1,
				cluster_id,
				1_000_001_u128
			),
			Error::<Test>::NotEnoughBalance
		);

		// Deposit with unsufficiect tokens for owner existensial deposit fails
		let non_existing_account = 50;
		assert_noop!(
			DdcCustomers::deposit_for(
				RuntimeOrigin::signed(funder_account),
				non_existing_account,
				cluster_id,
				1_u128
			),
			Error::<Test>::InsufficientDeposit
		);

		// Deposit with sufficiect tokens for owner existensial deposit works
		let non_existing_account = 50;
		assert_ok!(DdcCustomers::deposit_for(
			RuntimeOrigin::signed(funder_account),
			non_existing_account,
			cluster_id,
			2_u128
		));

		let amount1 = 5000_u128;
		// Deposited
		assert_ok!(DdcCustomers::deposit_for(
			RuntimeOrigin::signed(funder_account),
			account1,
			cluster_id,
			amount1
		));

		// Check storage
		assert_eq!(
			ClusterLedger::<Test>::get(cluster_id, account1),
			Some(CustomerLedger {
				owner: account1,
				total: amount1,
				active: amount1,
				unlocking: Default::default()
			})
		);

		// Checking that event was emitted
		System::assert_last_event(
			Event::Deposited { cluster_id, owner_id: account1, amount: amount1 }.into(),
		);

		// Deposit of an extra amount that is more than the customer's total balance fails
		let extra_amount1 = 2_000_000_u128;
		assert_noop!(
			DdcCustomers::deposit_for(
				RuntimeOrigin::signed(funder_account),
				account1,
				cluster_id,
				extra_amount1
			),
			Error::<Test>::NotEnoughBalance
		);

		let extra_amount2 = 10_000_u128;

		// Deposited extra
		assert_ok!(DdcCustomers::deposit_for(
			RuntimeOrigin::signed(funder_account),
			account1,
			cluster_id,
			extra_amount2
		));

		// Check storage
		assert_eq!(
			ClusterLedger::<Test>::get(cluster_id, account1),
			Some(CustomerLedger {
				owner: account1,
				total: amount1 + extra_amount2,
				active: amount1 + extra_amount2,
				unlocking: Default::default(),
			})
		);

		// Checking that event was emitted
		System::assert_last_event(
			Event::Deposited { cluster_id, owner_id: account1, amount: extra_amount2 }.into(),
		);
	})
}

#[test]
fn charge_bucket_owner_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let bucket_1_params = BucketParams { is_public: false };
		let account2: u128 = 2;
		let account3: u128 = 3;
		let vault: u128 = 4;
		let deposit = 100_u128;

		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(account3),
			cluster_id,
			bucket_1_params.clone()
		));

		let balance_before_deposit = Balances::free_balance(account3);
		// Deposited
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(account3), cluster_id, deposit));
		let balance_after_deposit = Balances::free_balance(account3);
		assert_eq!(balance_before_deposit - deposit, balance_after_deposit);

		let pallet_balance = Balances::free_balance(DdcCustomers::cluster_vault_id(&cluster_id));
		assert_eq!(deposit, pallet_balance);

		// Check storage
		assert_eq!(
			ClusterLedger::<Test>::get(cluster_id, account3),
			Some(CustomerLedger {
				owner: account3,
				total: deposit,
				active: deposit,
				unlocking: Default::default()
			})
		);

		// Checking that event was emitted
		System::assert_last_event(
			Event::Deposited { cluster_id, owner_id: account3, amount: deposit }.into(),
		);

		// successful transfer
		let charge1 = 10;
		let charged = DdcCustomers::charge_customer(account3, vault, cluster_id, charge1).unwrap();
		assert_eq!(charge1, charged);

		let vault_balance = Balances::free_balance(vault);
		assert_eq!(charged, vault_balance);

		let account_balance = Balances::free_balance(account3);
		assert_eq!(balance_after_deposit, account_balance);

		let pallet_balance_after_charge =
			Balances::free_balance(DdcCustomers::cluster_vault_id(&cluster_id));
		assert_eq!(pallet_balance - charged, pallet_balance_after_charge);

		// Check storage
		assert_eq!(
			ClusterLedger::<Test>::get(cluster_id, account3),
			Some(CustomerLedger {
				owner: account3,
				total: deposit - charge1,
				active: deposit - charge1,
				unlocking: Default::default(),
			})
		);

		// Checking that event was emitted
		System::assert_last_event(
			Event::Charged { cluster_id, owner_id: account3, charged, expected: charged }
				.into(),
		);

		// failed transfer
		let charge2 = 100u128;
		let charge_result =
			DdcCustomers::charge_customer(account3, vault, cluster_id, charge2).unwrap();
		assert_eq!(
			ClusterLedger::<Test>::get(cluster_id, account3),
			Some(CustomerLedger {
				owner: account3,
				total: 0,
				active: 0,
				unlocking: Default::default()
			})
		);

		// Checking that event was emitted
		System::assert_last_event(
			Event::Charged {
				cluster_id,
				owner_id: account3,
				charged: deposit - charge1,
				expected: charge2,
			}
			.into(),
		);

		assert_eq!(0, Balances::free_balance(DdcCustomers::cluster_vault_id(&cluster_id)));
		assert_eq!(charge_result, deposit - charge1);

		assert_ok!(DdcCustomers::deposit_extra(
			RuntimeOrigin::signed(account3),
			cluster_id,
			deposit
		));
		assert_eq!(
			ClusterLedger::<Test>::get(cluster_id, account3),
			Some(CustomerLedger {
				owner: account3,
				total: deposit,
				active: deposit,
				unlocking: Default::default()
			})
		);

		assert_eq!(deposit, Balances::free_balance(DdcCustomers::cluster_vault_id(&cluster_id)));

		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(account2), cluster_id, 50_u128));
	})
}

#[test]
fn unlock_and_withdraw_deposit_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let account1 = 1;
		let account2 = 2;

		// Deposited
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(account1), cluster_id, 35_u128));
		// So there is always positive balance within pallet
		assert_ok!(DdcCustomers::deposit(RuntimeOrigin::signed(account2), cluster_id, 10_u128));

		// Unlock chunk
		assert_ok!(DdcCustomers::unlock_deposit(
			RuntimeOrigin::signed(account1),
			cluster_id,
			1_u128
		));
		System::set_block_number(2);

		let unlocking_chunks = vec![UnlockChunk { value: 1, block: 11 }];
		// Check storage
		assert_eq!(
			ClusterLedger::<Test>::get(cluster_id, account1),
			Some(CustomerLedger {
				owner: account1,
				total: 35_u128,
				active: 34_u128,
				unlocking: BoundedVec::try_from(unlocking_chunks).unwrap(),
			})
		);

		// Reach max unlock chunks
		for i in 1..32 {
			assert_ok!(DdcCustomers::unlock_deposit(
				RuntimeOrigin::signed(account1),
				cluster_id,
				1_u128
			));
			System::set_block_number(i + 2);
		}

		// No more chunks can be added
		assert_noop!(
			DdcCustomers::unlock_deposit(RuntimeOrigin::signed(account1), cluster_id, 1_u128),
			Error::<Test>::NoMoreChunks
		);

		// Set the block to withdraw all unlocked chunks
		System::set_block_number(42);

		assert_ok!(DdcCustomers::withdraw_unlocked_deposit(
			RuntimeOrigin::signed(account1),
			cluster_id
		));
		// Check storage
		assert_eq!(
			ClusterLedger::<Test>::get(cluster_id, account1),
			Some(CustomerLedger {
				owner: account1,
				total: 3_u128,
				active: 3_u128,
				unlocking: Default::default()
			})
		);

		// Unlock remaining chunks & withdraw
		assert_ok!(DdcCustomers::unlock_deposit(
			RuntimeOrigin::signed(account1),
			cluster_id,
			3_u128
		));
		System::set_block_number(52);
		assert_ok!(DdcCustomers::withdraw_unlocked_deposit(
			RuntimeOrigin::signed(account1),
			cluster_id
		));

		// Check storage
		assert_eq!(ClusterLedger::<Test>::get(cluster_id, account1), None);
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
		System::assert_last_event(Event::BucketCreated { cluster_id, bucket_id: 1u64 }.into());

		let bucket_id = 1;
		let update_bucket_params = BucketParams { is_public: true };
		assert_ok!(DdcCustomers::set_bucket_params(
			RuntimeOrigin::signed(bucket_owner),
			bucket_id,
			update_bucket_params.clone()
		));

		assert_eq!(BucketsCount::<Test>::get(), 1);
		assert_eq!(
			Buckets::<Test>::get(1),
			Some(Bucket {
				bucket_id,
				owner_id: bucket_owner,
				cluster_id,
				is_public: update_bucket_params.is_public,
				is_removed: false,
			})
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(Event::BucketUpdated { cluster_id, bucket_id }.into());
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
		System::assert_last_event(Event::BucketCreated { cluster_id, bucket_id: 1u64 }.into());
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

#[test]
fn remove_bucket_works() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		let cluster_id = ClusterId::from([1; 20]);
		let account1 = 1;
		let account2 = 2;
		let bucket_id_1 = 1;
		let bucket_id_2 = 2;
		let bucket_params = BucketParams { is_public: false };

		// Bucket created
		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(account1),
			cluster_id,
			bucket_params.clone()
		));

		// Cannot remove someone else's bucket
		assert_noop!(
			DdcCustomers::remove_bucket(RuntimeOrigin::signed(account2), bucket_id_1),
			Error::<Test>::NotBucketOwner
		);

		// Cannot remove non existing bucket
		assert_noop!(
			DdcCustomers::remove_bucket(RuntimeOrigin::signed(account1), bucket_id_2),
			Error::<Test>::NoBucketWithId
		);

		// Check storage bucket is not removed
		assert_eq!(BucketsCount::<Test>::get(), 1);
		assert_eq!(
			Buckets::<Test>::get(1),
			Some(Bucket {
				bucket_id: 1,
				owner_id: account1,
				cluster_id,
				is_public: bucket_params.is_public,
				is_removed: false,
			})
		);

		// Bucket removed
		assert_ok!(DdcCustomers::remove_bucket(RuntimeOrigin::signed(account1), bucket_id_1));

		// Check storage bucket is removed
		assert_eq!(BucketsCount::<Test>::get(), 1);
		assert_eq!(
			Buckets::<Test>::get(1),
			Some(Bucket {
				bucket_id: 1,
				owner_id: account1,
				cluster_id,
				is_public: bucket_params.is_public,
				is_removed: true,
			})
		);

		// Checking that event was emitted
		assert_eq!(System::events().len(), 2);
		System::assert_last_event(Event::BucketRemoved { cluster_id, bucket_id: 1u64 }.into());

		// Cannot remove bucket twice
		assert_noop!(
			DdcCustomers::remove_bucket(RuntimeOrigin::signed(account1), bucket_id_1),
			Error::<Test>::AlreadyRemoved
		);
	})
}

#[test]
fn remove_bucket_checks_with_multiple_buckets_works() {
	ExtBuilder.build_and_execute(|| {
		let cluster_id = ClusterId::from([1; 20]);
		let account1 = 1;
		let account2 = 2;
		let bucket_id_1 = 1;
		let bucket_id_2 = 2;
		let private_bucket_params = BucketParams { is_public: false };
		let public_bucket_params = BucketParams { is_public: true };

		// Fail to remove non-existing buckets
		assert_noop!(
			DdcCustomers::remove_bucket(RuntimeOrigin::signed(account1), bucket_id_1),
			Error::<Test>::NoBucketWithId
		);

		assert_noop!(
			DdcCustomers::remove_bucket(RuntimeOrigin::signed(account1), bucket_id_2),
			Error::<Test>::NoBucketWithId
		);

		// Bucket created
		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(account1),
			cluster_id,
			private_bucket_params.clone()
		));

		// Bucket created
		assert_ok!(DdcCustomers::create_bucket(
			RuntimeOrigin::signed(account2),
			cluster_id,
			public_bucket_params.clone()
		));

		// Fail to remove bucket with different owner
		assert_noop!(
			DdcCustomers::remove_bucket(RuntimeOrigin::signed(account1), bucket_id_2),
			Error::<Test>::NotBucketOwner
		);

		assert_noop!(
			DdcCustomers::remove_bucket(RuntimeOrigin::signed(account2), bucket_id_1),
			Error::<Test>::NotBucketOwner
		);

		// Remove bucket with correct owner
		assert_ok!(DdcCustomers::remove_bucket(RuntimeOrigin::signed(account1), bucket_id_1));

		// Verify whether bucket has been removed
		assert_eq!(
			Buckets::<Test>::get(bucket_id_1),
			Some(Bucket {
				bucket_id: bucket_id_1,
				owner_id: account1,
				cluster_id,
				is_public: private_bucket_params.is_public,
				is_removed: true,
			})
		);

		assert_eq!(
			Buckets::<Test>::get(bucket_id_2),
			Some(Bucket {
				bucket_id: bucket_id_2,
				owner_id: account2,
				cluster_id,
				is_public: public_bucket_params.is_public,
				is_removed: false,
			})
		);

		// Fail to remove already removed bucket
		assert_noop!(
			DdcCustomers::remove_bucket(RuntimeOrigin::signed(account1), bucket_id_1),
			Error::<Test>::AlreadyRemoved
		);
	})
}
