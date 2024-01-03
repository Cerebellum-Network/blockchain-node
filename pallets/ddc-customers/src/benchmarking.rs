//! DdcStaking pallet benchmarking.
#![cfg(feature = "runtime-benchmarks")]

use ddc_primitives::{ClusterGovParams, ClusterId, ClusterParams};
use frame_benchmarking::{account, benchmarks, whitelist_account};
use frame_support::traits::Currency;
use sp_runtime::Perquintill;
use sp_std::prelude::*;

use super::*;
use crate::Pallet as DdcCustomers;

pub type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

use frame_system::{Pallet as System, RawOrigin};

const USER_SEED: u32 = 999666;

benchmarks! {
	create_bucket {
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber> = ClusterGovParams {
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

		let _ = <T as pallet::Config>::ClusterCreator::create_new_cluster(
			ClusterId::from([1; 20]),
			user.clone(),
			user.clone(),
			ClusterParams { node_provider_auth_contract: Some(user.clone()) },
			cluster_gov_params
		);

		let bucket_params = BucketParams {
			is_public: false
		};

		whitelist_account!(user);
	}: _(RawOrigin::Signed(user), cluster_id, bucket_params)
	verify {
		assert_eq!(Pallet::<T>::buckets_count(), 1);
	}

	deposit {
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 100u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let amount = <T as pallet::Config>::Currency::minimum_balance() * 50u32.into();

		whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), amount)
	verify {
		assert!(Ledger::<T>::contains_key(user));
	}

	deposit_extra {
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 200u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let amount = <T as pallet::Config>::Currency::minimum_balance() * 50u32.into();

		let _ = DdcCustomers::<T>::deposit(RawOrigin::Signed(user.clone()).into(), amount);

		whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), amount)
	verify {
		assert!(Ledger::<T>::contains_key(user));
	}

	unlock_deposit {
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 200u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let amount = <T as pallet::Config>::Currency::minimum_balance() * 50u32.into();

		let _ = DdcCustomers::<T>::deposit(RawOrigin::Signed(user.clone()).into(), amount);

		whitelist_account!(user);
	}: unlock_deposit(RawOrigin::Signed(user.clone()), amount)
	verify {
		assert!(Ledger::<T>::contains_key(user));
	}

	// Worst case scenario, 31/32 chunks unlocked after the unlocking duration
	withdraw_unlocked_deposit_update {

		System::<T>::set_block_number(1u32.into());

		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 2000u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let amount = <T as pallet::Config>::Currency::minimum_balance() * 32u32.into();

		let _ = DdcCustomers::<T>::deposit(RawOrigin::Signed(user.clone()).into(), amount);

		for _k in 1 .. 32 {
			let _ = DdcCustomers::<T>::unlock_deposit(RawOrigin::Signed(user.clone()).into(), <T as pallet::Config>::Currency::minimum_balance() * 1u32.into());
		}

		System::<T>::set_block_number(5256001u32.into());

		whitelist_account!(user);
	}: withdraw_unlocked_deposit(RawOrigin::Signed(user.clone()))
	verify {
		let ledger = Ledger::<T>::try_get(user).unwrap();
		assert_eq!(ledger.active, amount / 32u32.into());
	}

	// Worst case scenario, everything is removed after the unlocking duration
	withdraw_unlocked_deposit_kill {

		System::<T>::set_block_number(1u32.into());

		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let user2 = account::<T::AccountId>("user", USER_SEED, 1u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 2000u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user2, balance);
		let amount = <T as pallet::Config>::Currency::minimum_balance() * 32u32.into();

		let _ = DdcCustomers::<T>::deposit(RawOrigin::Signed(user.clone()).into(), amount);
		// To keep the balance of pallet positive
		let _ = DdcCustomers::<T>::deposit(RawOrigin::Signed(user2).into(), amount);


		for _k in 1 .. 33 {
			let _ = DdcCustomers::<T>::unlock_deposit(RawOrigin::Signed(user.clone()).into(), <T as pallet::Config>::Currency::minimum_balance() * 1u32.into());
		}

		System::<T>::set_block_number(5256001u32.into());

		whitelist_account!(user);
	}: withdraw_unlocked_deposit(RawOrigin::Signed(user.clone()))
	verify {
		assert!(!Ledger::<T>::contains_key(user));
	}

	set_bucket_params {
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);

		let bucket_id = 1;
		let bucket = Bucket {
			bucket_id,
			owner_id: user.clone(),
			cluster_id,
			is_public: false,
		};

		<BucketsCount<T>>::set(bucket_id);
		<Buckets<T>>::insert(bucket_id, bucket);

		whitelist_account!(user);

		let bucket_params = BucketParams {
			is_public: true
		};

	}: _(RawOrigin::Signed(user), bucket_id, bucket_params)
	verify {
		let bucket = <Buckets<T>>::get(bucket_id).unwrap();
		assert!(bucket.is_public);
	}

	impl_benchmark_test_suite!(
		DdcCustomers,
		crate::mock::ExtBuilder.build(),
		crate::mock::Test,
	);
}
