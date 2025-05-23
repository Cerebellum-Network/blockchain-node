use frame_support::{storage_alias, traits::OnRuntimeUpgrade};
use log::info;

use super::*;

const LOG_TARGET: &str = "ddc-customers";

pub mod v0 {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct Bucket<AccountId> {
		pub bucket_id: BucketId,
		pub owner_id: AccountId,
		pub cluster_id: ClusterId,
		pub is_public: bool,
	}

	#[storage_alias]
	pub type BucketsCount<T: Config> = StorageValue<crate::Pallet<T>, BucketId, ValueQuery>;

	#[storage_alias]
	pub type Buckets<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		BucketId,
		Bucket<<T as frame_system::Config>::AccountId>,
		OptionQuery,
	>;
}

pub mod v1 {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[storage_alias]
	pub type BucketsCount<T: Config> = StorageValue<crate::Pallet<T>, BucketId, ValueQuery>;

	#[storage_alias]
	pub type Buckets<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		BucketId,
		Bucket<<T as frame_system::Config>::AccountId>,
		OptionQuery,
	>;

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct Bucket<AccountId> {
		pub bucket_id: BucketId,
		pub owner_id: AccountId,
		pub cluster_id: ClusterId,
		pub is_public: bool,
		pub is_removed: bool, // new field
	}

	// Migrate to removable buckets
	pub fn migrate_to_v1<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		info!(
			target: LOG_TARGET,
			"Running v1 migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 0 && current_version >= 1 {
			let count = v0::BucketsCount::<T>::get();
			info!(
				target: LOG_TARGET,
				" >>> Updating DDC Customers storage. Migrating {} buckets...", count
			);

			v1::Buckets::<T>::translate::<v0::Bucket<T::AccountId>, _>(
				|bucket_id: BucketId, bucket: v0::Bucket<T::AccountId>| {
					info!(target: LOG_TARGET, "     Migrating bucket for bucket ID {:?}...", bucket_id);

					Some(v1::Bucket {
						bucket_id: bucket.bucket_id,
						owner_id: bucket.owner_id,
						cluster_id: bucket.cluster_id,
						is_public: bucket.is_public,
						is_removed: false,
					})
				},
			);

			// Update storage version.
			StorageVersion::new(1).put::<Pallet<T>>();
			let count = v0::BucketsCount::<T>::get();
			info!(
				target: LOG_TARGET,
				" <<< DDC Customers storage updated! Migrated {} buckets ✅", count
			);

			T::DbWeight::get().reads_writes(count + 2, count + 1)
		} else {
			info!(target: LOG_TARGET, " >>> Unused migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v1::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
			let prev_bucket_id = v0::BucketsCount::<T>::get();
			let prev_count = v0::Buckets::<T>::iter().count();

			Ok((prev_bucket_id, prev_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
			let (prev_bucket_id, prev_count): (u64, u64) = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			let post_bucket_id = BucketsCount::<T>::get();
			ensure!(
				prev_bucket_id == post_bucket_id,
				"the last bucket ID before and after the migration should be the same"
			);

			let post_count = BucketsCount::<T>::get();
			ensure!(
				prev_count == post_count,
				"the bucket count before and after the migration should be the same"
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			frame_support::ensure!(current_version == 1, "must_upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same"
			);

			v1::Buckets::<T>::iter().try_for_each(|(_id, bucket)| -> Result<(), &'static str> {
				ensure!(
					!bucket.is_removed,
					"At this point all the bucket should have is_removed set to false"
				);
				Ok(())
			})?;
			Ok(())
		}
	}
}

pub mod v2 {

	use frame_support::pallet_prelude::*;

	use super::*;

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Bucket<AccountId> {
		pub bucket_id: BucketId,
		pub owner_id: AccountId,
		pub cluster_id: ClusterId,
		pub is_public: bool,
		pub is_removed: bool,
		pub total_customers_usage: Option<BucketUsage>, // new field
	}

	#[storage_alias]
	pub type BucketsCount<T: Config> = StorageValue<crate::Pallet<T>, BucketId, ValueQuery>;

	#[storage_alias]
	pub type Buckets<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		BucketId,
		Bucket<<T as frame_system::Config>::AccountId>,
		OptionQuery,
	>;

	// New migration to add total_customers_usage field
	pub fn migrate_to_v2<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		info!(
			target: LOG_TARGET,
			"Running v2 migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 1 && current_version >= 2 {
			let count = v1::BucketsCount::<T>::get();
			info!(
				target: LOG_TARGET,
				" >>> Updating DDC Customers storage to v2. Migrating {} buckets...", count
			);

			v2::Buckets::<T>::translate::<v1::Bucket<T::AccountId>, _>(
				|bucket_id: BucketId, bucket: v1::Bucket<T::AccountId>| {
					info!(target: LOG_TARGET, "Migrating bucket for bucket ID {:?}...", bucket_id);

					Some(v2::Bucket {
						bucket_id: bucket.bucket_id,
						owner_id: bucket.owner_id,
						cluster_id: bucket.cluster_id,
						is_public: bucket.is_public,
						is_removed: bucket.is_removed,
						total_customers_usage: None,
					})
				},
			);

			// Update storage version.
			StorageVersion::new(2).put::<Pallet<T>>();
			let count = v2::BucketsCount::<T>::get();
			info!(
				target: LOG_TARGET,
				"<<< DDC Customers storage updated to v2! Migrated {} buckets ✅", count
			);

			T::DbWeight::get().reads_writes(count + 2, count + 1)
		} else {
			info!(target: LOG_TARGET, " >>> Unused migration to v2!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v2::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
			let prev_bucket_id = v1::BucketsCount::<T>::get();
			let prev_count = v1::Buckets::<T>::iter().count();

			Ok((prev_bucket_id, prev_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
			let (prev_bucket_id, prev_count): (u64, u64) = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			let post_bucket_id = v2::BucketsCount::<T>::get();
			ensure!(
				prev_bucket_id == post_bucket_id,
				"the last bucket ID before and after the migration should be the same"
			);

			let post_count = v2::Buckets::<T>::iter().count() as u64;
			ensure!(
				prev_count == post_count,
				"the bucket count before and after the migration should be the same"
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			frame_support::ensure!(current_version == 2, "must_upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same"
			);

			v2::Buckets::<T>::iter().try_for_each(|(_id, bucket)| -> Result<(), &'static str> {
				ensure!(
					bucket.total_customers_usage.is_none(),
					"At this point all the bucket should have total_customers_usage set to None"
				);
				Ok(())
			})?;

			Ok(())
		}
	}
}

pub mod v3 {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[storage_alias]
	pub type Buckets<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		BucketId,
		Bucket<<T as frame_system::Config>::AccountId>,
		OptionQuery,
	>;

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Bucket<AccountId> {
		pub bucket_id: BucketId,
		pub owner_id: AccountId,
		pub cluster_id: ClusterId,
		pub is_public: bool,
		pub is_removed: bool,
	}

	// New migration to remove total_customers_usage field
	pub fn migrate_to_v3<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		info!(
			target: LOG_TARGET,
			"Running v3 migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 2 && current_version >= 3 {
			let count = BucketsCount::<T>::get();
			info!(
				target: LOG_TARGET,
				" >>> Updating DDC Customers storage to v3. Migrating {} buckets...", count
			);

			v3::Buckets::<T>::translate::<v2::Bucket<T::AccountId>, _>(
				|bucket_id: BucketId, bucket: v2::Bucket<T::AccountId>| {
					info!(target: LOG_TARGET, "Migrating bucket for bucket ID {:?}...", bucket_id);

					Some(v3::Bucket {
						bucket_id: bucket.bucket_id,
						owner_id: bucket.owner_id,
						cluster_id: bucket.cluster_id,
						is_public: bucket.is_public,
						is_removed: bucket.is_removed,
					})
				},
			);

			// Update storage version.
			StorageVersion::new(3).put::<Pallet<T>>();
			let count = BucketsCount::<T>::get();
			info!(
				target: LOG_TARGET,
				" <<< DDC Customers storage updated to v3! Migrated {} buckets ✅", count
			);

			T::DbWeight::get().reads_writes(count + 2, count + 1)
		} else {
			info!(target: LOG_TARGET, " >>> Unused migration to v3!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV3<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v3::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
			let on_chain_version = Pallet::<T>::on_chain_storage_version();
			let current_version = Pallet::<T>::in_code_storage_version();

			info!(
				target: LOG_TARGET,
				"Executing `pre_upgrade` check of v3 migration with current storage version {:?} / onchain {:?}",
				current_version,
				on_chain_version
			);

			let prev_bucket_id = v3::BucketsCount::<T>::get();
			let prev_count = v2::Buckets::<T>::iter().count();

			Ok((prev_bucket_id as u64, prev_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
			let (prev_bucket_id, prev_count): (u64, u64) = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			info!(
				target: LOG_TARGET,
				"Executing post check of v3 migration prev_count={:?}, prev_bucket_id={:?} ...", prev_count, prev_bucket_id
			);

			let post_bucket_id = BucketsCount::<T>::get();
			ensure!(
				prev_bucket_id == post_bucket_id,
				"the last bucket ID before and after the v3 migration should be the same"
			);

			let post_count = v3::Buckets::<T>::iter().count() as u64;
			ensure!(
				prev_count == post_count,
				"the bucket count before and after the v3 migration should be the same"
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			frame_support::ensure!(current_version == 3, "must_upgrade");

			ensure!(
				current_version == on_chain_version,
				"the current_version and on_chain_version should be the same after the v3 migration"
			);

			Ok(())
		}
	}
}
