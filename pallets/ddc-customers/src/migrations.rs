use frame_support::{storage_alias, traits::OnRuntimeUpgrade};
use log::info;

use super::*;

const LOG_TARGET: &str = "ddc-customers";
pub const PALLET_MIGRATIONS_ID: &[u8; 20] = b"pallet-ddc-customers";

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

pub mod v3_mbm {
	use frame_support::{
		migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
		pallet_prelude::*,
		weights::WeightMeter,
	};

	use super::*;

	/// Progressive states of a migration. The migration starts with the first variant and ends with
	/// the last.
	#[derive(Decode, Encode, MaxEncodedLen, Eq, PartialEq, Debug)]
	pub enum MigrationState {
		MigratingBuckets(BucketId),
		Finished,
	}

	pub struct LazyMigrationV2ToV3<T: Config>(PhantomData<T>);

	impl<T: Config> SteppedMigration for LazyMigrationV2ToV3<T> {
		type Cursor = MigrationState;
		type Identifier = MigrationId<20>;

		fn id() -> Self::Identifier {
			MigrationId { pallet_id: *PALLET_MIGRATIONS_ID, version_from: 2, version_to: 3 }
		}

		fn step(
			mut cursor: Option<Self::Cursor>,
			meter: &mut WeightMeter,
		) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
			info!(
				target: LOG_TARGET,
				"Step in v3 migration cursor={:?}", cursor
			);

			if Pallet::<T>::on_chain_storage_version() != Self::id().version_from as u16 {
				return Ok(None);
			}

			// Check that we have enough weight for at least the next step. If we don't, then the
			// migration cannot be complete.
			let required = match &cursor {
				Some(state) => Self::required_weight(&state),
				// Worst case weight for `authority_step`.
				// None => T::WeightInfo::migration_v2_authority_step(),
				None => Weight::from_parts(10_000_000_u64, 0),
			};
			if meter.remaining().any_lt(required) {
				return Err(SteppedMigrationError::InsufficientWeight { required });
			}

			loop {
				// Check that we would have enough weight to perform this step in the worst case
				// scenario.
				let required_weight = match &cursor {
					Some(state) => Self::required_weight(&state),
					// Worst case weight for `authority_step`.

					// None => T::WeightInfo::migration_v2_authority_step(),
					None => Weight::from_parts(10_000_000_u64, 0),
				};
				if !meter.can_consume(required_weight) {
					break;
				}

				let next = match &cursor {
					// At first, migrate any authorities.
					None => Self::buckets_step(None),
					// Migrate any remaining authorities.
					Some(MigrationState::MigratingBuckets(maybe_last_bucket)) =>
						Self::buckets_step(Some(maybe_last_bucket)),
					// After the last obsolete username was cleared from storage, the migration is
					// done.
					Some(MigrationState::Finished) => {
						StorageVersion::new(Self::id().version_to as u16).put::<Pallet<T>>();
						return Ok(None);
					},
				};

				cursor = Some(next);
				meter.consume(required_weight);
			}

			Ok(cursor)
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
			info!(
				target: LOG_TARGET,
				"-!--> PRE-CHECK Step in v3 migration <--!-"
			);

			let on_chain_version = Pallet::<T>::on_chain_storage_version();
			let current_version = Pallet::<T>::in_code_storage_version();

			info!(
				target: LOG_TARGET,
				"Executing `pre_upgrade` check of v3 migration with current storage version {:?} / onchain {:?}",
				current_version,
				on_chain_version
			);

			let prev_bucket_id = BucketsCount::<T>::get();
			let prev_count = v2::Buckets::<T>::iter().count();

			Ok((prev_bucket_id as u64, prev_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
			info!(
				target: LOG_TARGET,
				"=!==> POST-CHECK Step in v3 migration <==!="
			);

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

	impl<T: Config> LazyMigrationV2ToV3<T> {
		// Migrate one entry from `UsernameAuthorities` to `AuthorityOf`.
		pub(crate) fn buckets_step(maybe_last_key: Option<&BucketId>) -> MigrationState {
			let mut iter = if let Some(last_key) = maybe_last_key {
				v2::Buckets::<T>::iter_from(v2::Buckets::<T>::hashed_key_for(last_key))
			} else {
				v2::Buckets::<T>::iter()
			};
			if let Some((bucket_id, bucket_v2)) = iter.next() {
				let bucket_v3 = v3::Bucket {
					bucket_id: bucket_v2.bucket_id,
					owner_id: bucket_v2.owner_id,
					cluster_id: bucket_v2.cluster_id,
					is_public: bucket_v2.is_public,
					is_removed: bucket_v2.is_removed,
				};
				v3::Buckets::<T>::insert(&bucket_id, bucket_v3);
				MigrationState::MigratingBuckets(bucket_id)
			} else {
				MigrationState::Finished
			}
		}

		pub(crate) fn required_weight(step: &MigrationState) -> Weight {
			match step {
				MigrationState::MigratingBuckets(_) => Weight::from_parts(10_000_000_u64, 0),
				MigrationState::Finished => Weight::zero(),
			}
		}
	}
}
