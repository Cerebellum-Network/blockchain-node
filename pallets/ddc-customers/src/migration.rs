use super::*;

use frame_support::{
	storage_alias,
	traits::{Get, GetStorageVersion, StorageVersion},
	weights::Weight,
	BoundedVec, Twox64Concat,
};

// only contains V1 storage format
pub mod v1 {
	use super::*;

	#[storage_alias]
	pub(super) type Buckets<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, u64, Bucket<<T as frame_system::Config>::AccountId>>;
}

// contains checks and transforms storage to V2 format
pub fn migrate_to_v2<T: Config>() -> Weight {
	// We transform the storage values from the old into the new format.
	Buckets::<T>::translate::<Bucket<T::AccountId>, _>(|_k: u64, bucket: Bucket<T::AccountId>| {
		Some(Bucket {
			bucket_id: bucket.bucket_id as u64,
			owner_id: bucket.owner_id,
			cluster_id: bucket.cluster_id,
			public_availability: bucket.public_availability,
			resources_reserved: bucket.resources_reserved,
		})
	});

	// Very inefficient, mostly here for illustration purposes.
	let count = Buckets::<T>::iter().count();
	// Return the weight consumed by the migration.
	T::DbWeight::get().reads_writes(count as u64 + 1, count as u64 + 1)
}
