//! DdcCustomers pallet benchmarking.

use polkadot_sdk::*;

use ddc_primitives::{BucketParams, ClusterId, ClusterParams, ClusterProtocolParams};
use polkadot_sdk::frame_support::traits::Currency;
use polkadot_sdk::sp_runtime::Perquintill;
use polkadot_sdk::sp_std::prelude::*;

use super::*;

pub type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<
	<T as polkadot_sdk::frame_system::Config>::AccountId,
>>::Balance;

use polkadot_sdk::frame_system::RawOrigin;

const USER_SEED: u32 = 999666;

use polkadot_sdk::frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {

	use super::*;

	fn create_dafault_cluster<T: Config>(cluster_owner: T::AccountId) -> ClusterId {
		let customer_deposit_contract =
			account::<T::AccountId>("customer_deposit_contract", USER_SEED, 0u32);
		let cluster_protocol_params: ClusterProtocolParams<
			BalanceOf<T>,
			BlockNumberFor<T>,
			T::AccountId,
		> = ClusterProtocolParams {
			treasury_share: Perquintill::default(),
			validators_share: Perquintill::default(),
			cluster_reserve_share: Perquintill::default(),
			storage_bond_size: 100u32.into(),
			storage_chill_delay: 50u32.into(),
			storage_unbonding_delay: 50u32.into(),
			cost_per_mb_stored: 10,
			cost_per_mb_streamed: 10,
			cost_per_put_request: 10,
			cost_per_get_request: 10,
			cost_per_gpu_unit: 0,
			cost_per_cpu_unit: 0,
			cost_per_ram_unit: 0,
			customer_deposit_contract,
		};

		let cluster_id = ClusterId::from([1; 20]);
		let _ = <T as pallet::Config>::ClusterCreator::create_cluster(
			cluster_id,
			cluster_owner.clone(),
			cluster_owner.clone(),
			ClusterParams {
				node_provider_auth_contract: Some(cluster_owner.clone()),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3,
			},
			cluster_protocol_params,
		);

		cluster_id
	}

	#[benchmark]
	fn create_bucket() {
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let cluster_id = create_dafault_cluster::<T>(user.clone());

		let bucket_params = BucketParams { is_public: false };

		whitelist_account!(user);
		#[extrinsic_call]
		create_bucket(RawOrigin::Signed(user), cluster_id, bucket_params);

		assert_eq!(BucketsCount::<T>::get(), 1);
	}

	#[benchmark]
	fn set_bucket_params() {
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let cluster_id = create_dafault_cluster::<T>(user.clone());

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
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let cluster_id = create_dafault_cluster::<T>(user.clone());

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

		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let _ = create_dafault_cluster::<T>(user.clone());

		let setup = LazyMigrationV2ToV3::<T>::setup_benchmark_env_for_migration();
		assert_eq!(V2Buckets::<T>::iter().count(), 1);

		#[block]
		{
			LazyMigrationV2ToV3::<T>::buckets_step(None);
		}

		assert_eq!(V3Buckets::<T>::iter().count(), 1);
		let bucket = V3Buckets::<T>::get(setup.bucket_id);
		assert!(bucket.is_some());

		Ok(())
	}

	#[benchmark]
	fn migration_v4_ledgers_step() -> Result<(), BenchmarkError> {
		use crate::{
			migrations::v4_mbm::{LazyMigrationV3ToV4, Ledger as V3Ledgers},
			ClusterLedger as V4Ledgers,
		};

		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let _ = create_dafault_cluster::<T>(user.clone());

		let setup = LazyMigrationV3ToV4::<T>::setup_benchmark_env_for_migration();
		assert_eq!(V3Ledgers::<T>::iter().count(), 1);

		#[block]
		{
			LazyMigrationV3ToV4::<T>::ledgers_step(None, None);
		}

		assert_eq!(V4Ledgers::<T>::iter_values().count(), 1);
		let ledger = V4Ledgers::<T>::get(setup.cluster_id, &setup.ledger_owner);

		assert!(ledger.is_some());

		Ok(())
	}

	#[benchmark]
	fn migration_v5_ledgers_step() -> Result<(), BenchmarkError> {
		use crate::{migrations::v5_mbm::LazyMigrationV4ToV5, ClusterLedger as V4Ledgers};

		let _setup = LazyMigrationV4ToV5::<T>::setup_benchmark_env_for_migration();
		assert_eq!(V4Ledgers::<T>::iter_values().count(), 1);

		#[block]
		{
			LazyMigrationV4ToV5::<T>::ledgers_step(None, None);
		}

		assert_eq!(V4Ledgers::<T>::iter_values().count(), 0);

		Ok(())
	}
}
