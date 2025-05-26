//! DdcCustomers pallet benchmarking.

use ddc_primitives::{BucketParams, ClusterId, ClusterParams, ClusterProtocolParams};
use frame_support::traits::Currency;
use sp_runtime::Perquintill;
use sp_std::prelude::*;

use super::*;
use crate::Pallet as DdcCustomers;

pub type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

use frame_system::{Pallet as System, RawOrigin};

const USER_SEED: u32 = 999666;

use frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {

	use super::*;

	#[benchmark]
	fn create_bucket() {
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let cluster_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>> =
			ClusterProtocolParams {
				treasury_share: Perquintill::default(),
				validators_share: Perquintill::default(),
				cluster_reserve_share: Perquintill::default(),
				storage_bond_size: 100u32.into(),
				storage_chill_delay: 50u32.into(),
				storage_unbonding_delay: 50u32.into(),
				unit_per_mb_stored: 10,
				unit_per_mb_streamed: 10,
				unit_per_put_request: 10,
				unit_per_get_request: 10,
			};

		let _ = <T as pallet::Config>::ClusterCreator::create_cluster(
			ClusterId::from([1; 20]),
			user.clone(),
			user.clone(),
			ClusterParams {
				node_provider_auth_contract: Some(user.clone()),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3,
			},
			cluster_protocol_params,
		);

		let bucket_params = BucketParams { is_public: false };

		whitelist_account!(user);
		#[extrinsic_call]
		create_bucket(RawOrigin::Signed(user), cluster_id, bucket_params);

		assert_eq!(BucketsCount::<T>::get(), 1);
	}

	#[benchmark]
	fn deposit() {
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 100u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let amount = <T as pallet::Config>::Currency::minimum_balance() * 50u32.into();

		whitelist_account!(user);

		#[extrinsic_call]
		deposit::<T>(RawOrigin::Signed(user.clone()), cluster_id, amount);

		assert!(ClusterLedger::<T>::contains_key(cluster_id, &user));
	}

	#[benchmark]
	fn deposit_extra() {
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 200u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let amount = <T as pallet::Config>::Currency::minimum_balance() * 50u32.into();

		let _ =
			DdcCustomers::<T>::deposit(RawOrigin::Signed(user.clone()).into(), cluster_id, amount);

		whitelist_account!(user);

		#[extrinsic_call]
		deposit_extra::<T>(RawOrigin::Signed(user.clone()), cluster_id, amount);

		assert!(ClusterLedger::<T>::contains_key(cluster_id, &user));
	}

	#[benchmark]
	fn deposit_for() {
		let cluster_id = ClusterId::from([1; 20]);
		let funder = account::<T::AccountId>("funder", USER_SEED, 0u32);
		let user = account::<T::AccountId>("user", USER_SEED, 1u32);

		let balance_1 = <T as pallet::Config>::Currency::minimum_balance() * 200u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&funder, balance_1);

		let balance_2 = <T as pallet::Config>::Currency::minimum_balance() * 100u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance_2);

		let fund_amount = <T as pallet::Config>::Currency::minimum_balance() * 50u32.into();

		whitelist_account!(funder);

		#[extrinsic_call]
		deposit_for::<T>(RawOrigin::Signed(funder.clone()), user.clone(), cluster_id, fund_amount);

		assert!(ClusterLedger::<T>::contains_key(cluster_id, &user));
	}

	#[benchmark]
	fn unlock_deposit() {
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 200u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let amount = <T as pallet::Config>::Currency::minimum_balance() * 50u32.into();

		let _ =
			DdcCustomers::<T>::deposit(RawOrigin::Signed(user.clone()).into(), cluster_id, amount);

		whitelist_account!(user);

		#[extrinsic_call]
		unlock_deposit::<T>(RawOrigin::Signed(user.clone()), cluster_id, amount);

		assert!(ClusterLedger::<T>::contains_key(cluster_id, user));
	}

	#[benchmark]
	fn withdraw_unlocked_deposit_update() {
		System::<T>::set_block_number(1u32.into());

		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 2000u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let amount = <T as pallet::Config>::Currency::minimum_balance() * 32u32.into();

		let _ =
			DdcCustomers::<T>::deposit(RawOrigin::Signed(user.clone()).into(), cluster_id, amount);

		for _k in 1..32 {
			let _ = DdcCustomers::<T>::unlock_deposit(
				RawOrigin::Signed(user.clone()).into(),
				cluster_id,
				<T as pallet::Config>::Currency::minimum_balance() * 1u32.into(),
			);
		}

		System::<T>::set_block_number(5256001u32.into());

		whitelist_account!(user);

		#[extrinsic_call]
		withdraw_unlocked_deposit::<T>(RawOrigin::Signed(user.clone()), cluster_id);

		let _ledger = ClusterLedger::<T>::try_get(cluster_id, &user).unwrap();
		assert_eq!(
			ClusterLedger::<T>::try_get(cluster_id, &user).unwrap().active,
			amount / 32u32.into()
		);
	}

	#[benchmark]
	fn withdraw_unlocked_deposit_kill() {
		System::<T>::set_block_number(1u32.into());
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let user2 = account::<T::AccountId>("user", USER_SEED, 1u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 2000u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user2, balance);
		let amount = <T as pallet::Config>::Currency::minimum_balance() * 32u32.into();

		let _ =
			DdcCustomers::<T>::deposit(RawOrigin::Signed(user.clone()).into(), cluster_id, amount);
		// To keep the balance of pallet positive
		let _ = DdcCustomers::<T>::deposit(RawOrigin::Signed(user2).into(), cluster_id, amount);

		for _k in 1..33 {
			let _ = DdcCustomers::<T>::unlock_deposit(
				RawOrigin::Signed(user.clone()).into(),
				cluster_id,
				<T as pallet::Config>::Currency::minimum_balance() * 1u32.into(),
			);
		}

		System::<T>::set_block_number(5256001u32.into());

		whitelist_account!(user);

		#[extrinsic_call]
		withdraw_unlocked_deposit::<T>(RawOrigin::Signed(user.clone()), cluster_id);

		assert!(!ClusterLedger::<T>::contains_key(cluster_id, user));
	}

	#[benchmark]
	fn set_bucket_params() {
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);

		let bucket_id = 1;
		let bucket = Bucket {
			bucket_id,
			owner_id: user.clone(),
			cluster_id,
			is_public: false,
			is_removed: false,
		};

		<BucketsCount<T>>::set(bucket_id);
		<Buckets<T>>::insert(bucket_id, bucket);

		whitelist_account!(user);

		let bucket_params = BucketParams { is_public: true };

		#[extrinsic_call]
		set_bucket_params::<T>(RawOrigin::Signed(user), bucket_id, bucket_params);

		let bucket = <Buckets<T>>::get(bucket_id).unwrap();
		assert!(bucket.is_public);
	}

	#[benchmark]
	fn remove_bucket() {
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);

		let bucket_id = 1;
		let bucket = Bucket {
			bucket_id,
			owner_id: user.clone(),
			cluster_id,
			is_public: false,
			is_removed: false,
		};

		<BucketsCount<T>>::set(bucket_id);
		<Buckets<T>>::insert(bucket_id, bucket);

		whitelist_account!(user);

		#[extrinsic_call]
		remove_bucket::<T>(RawOrigin::Signed(user), bucket_id);

		let bucket = <Buckets<T>>::get(bucket_id).unwrap();
		assert!(bucket.is_removed);
	}

	#[benchmark]
	fn migration_v3_buckets_step() -> Result<(), BenchmarkError> {
		use crate::migrations::{
			v2::Buckets as V2Buckets, v3::Buckets as V3Buckets, v3_mbm::LazyMigrationV2ToV3,
		};

		let setup = LazyMigrationV2ToV3::<T>::setup_benchmark_env_for_migration();
		assert_eq!(V2Buckets::<T>::iter().count(), 1);

		#[block]
		{
			LazyMigrationV2ToV3::<T>::buckets_step(None);
		}

		assert_eq!(V3Buckets::<T>::iter().count(), 1);
		let bucket = V3Buckets::<T>::get(&setup.bucket_id);
		assert!(bucket.is_some());

		Ok(())
	}
}
