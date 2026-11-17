//! Tests for the module.

use polkadot_sdk::frame_support::{assert_noop, assert_ok};

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

#[test]
fn buckets_are_the_only_calls_customers_can_make() {
	use polkadot_sdk::frame_support::traits::{GetCallIndex, GetCallName};

	assert_eq!(
		Call::<Test>::get_call_names(),
		&["create_bucket", "set_bucket_params", "remove_bucket"]
	);
	assert_eq!(Call::<Test>::get_call_indices(), &[0, 6, 7]);
}

#[test]
fn bucket_events_keep_their_encoded_indices() {
	let cluster_id = ClusterId::from([1; 20]);
	let bucket_id = 1;

	assert_eq!(Event::<Test>::BucketCreated { cluster_id, bucket_id }.encode()[0], 4);
	assert_eq!(Event::<Test>::BucketUpdated { cluster_id, bucket_id }.encode()[0], 5);
	assert_eq!(Event::<Test>::BucketRemoved { cluster_id, bucket_id }.encode()[0], 6);
}

#[test]
fn genesis_buckets_carry_no_deposit() {
	use polkadot_sdk::sp_runtime::BuildStorage;

	let cluster_id = ClusterId::from([1; 20]);
	let owner = 1;
	let bucket = Bucket::<Test> {
		bucket_id: 1,
		owner_id: owner,
		cluster_id,
		is_public: true,
		is_removed: false,
	};
	let storage = crate::GenesisConfig::<Test> { buckets: vec![bucket.clone()] }
		.build_storage()
		.unwrap();

	polkadot_sdk::sp_io::TestExternalities::new(storage).execute_with(|| {
		assert_eq!(BucketsCount::<Test>::get(), 1);
		assert_eq!(Buckets::<Test>::get(1), Some(bucket));
		assert_eq!(ClusterLedger::<Test>::get(cluster_id, owner), None);
		assert_eq!(Balances::free_balance(DdcCustomers::cluster_vault_id(&cluster_id)), 0);
	});
}
