#[cfg(feature = "try-runtime")]
use frame_support::ensure;
use frame_support::{
	storage_alias,
	traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
	weights::Weight,
};
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
	pub(super) type BucketsCount<T: Config> = StorageValue<crate::Pallet<T>, BucketId, ValueQuery>;

	#[storage_alias]
	pub(super) type Buckets<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		BucketId,
		Bucket<<T as frame_system::Config>::AccountId>,
		OptionQuery,
	>;
}

// Migrate to removable buckets
pub fn migrate_to_v1<T: Config>() -> Weight {
	let on_chain_version = Pallet::<T>::on_chain_storage_version();
	if on_chain_version == 0 {
		let count = v0::BucketsCount::<T>::get();
		info!(
			target: LOG_TARGET,
			" >>> Updating DDC Customers storage. Migrating {} buckets...", count
		);

		Buckets::<T>::translate::<v0::Bucket<T::AccountId>, _>(
			|bucket_id: BucketId, bucket: v0::Bucket<T::AccountId>| {
				info!(target: LOG_TARGET, "     Migrating bucket for bucket ID {:?}...", bucket_id);

				Some(Bucket {
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
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		let prev_bucket_id = v0::BucketsCount::<T>::get();
		let prev_count = v0::Buckets::<T>::iter().count();

		Ok((prev_bucket_id as u64, prev_count as u64).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(prev_state: Vec<u8>) -> Result<(), &'static str> {
		let (prev_bucket_id, prev_count): (u64, u64) =
			Decode::decode(&mut &prev_state[..]).expect("pre_upgrade provides a valid state; qed");

		let post_bucket_id = Pallet::<T>::buckets_count() as u64;
		ensure!(
			prev_bucket_id == post_bucket_id,
			"the last bucket ID before and after the migration should be the same"
		);

		let post_count = Buckets::<T>::iter().count() as u64;
		ensure!(
			prev_count == post_count,
			"the bucket count before and after the migration should be the same"
		);

		let current_version = Pallet::<T>::current_storage_version();
		let on_chain_version = Pallet::<T>::on_chain_storage_version();

		frame_support::ensure!(current_version == 1, "must_upgrade");
		ensure!(
			current_version == on_chain_version,
			"after migration, the current_version and on_chain_version should be the same"
		);

		Buckets::<T>::iter().try_for_each(|(_id, bucket)| -> Result<(), &'static str> {
			ensure!(
				bucket.is_removed == false,
				"At this point all the bucket should have is_removed set to false"
			);
			Ok(())
		})?;
		Ok(())
	}
}
