#![allow(clippy::unnecessary_unwrap)]
#![allow(clippy::collapsible_else_if)]

use frame_support::{storage_alias, traits::OnRuntimeUpgrade};
use hex_literal::hex;
use log::{info, warn};

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

			frame_support::ensure!(current_version == 1, "must_upgrade to 1st version");
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

			frame_support::ensure!(current_version == 2, "must_upgrade to 2nd version");
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

			Ok((prev_bucket_id, prev_count as u64).encode())
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
				Some(state) => Self::required_weight(state),
				None => T::WeightInfo::migration_v3_buckets_step(),
			};
			if meter.remaining().any_lt(required) {
				return Err(SteppedMigrationError::InsufficientWeight { required });
			}

			loop {
				// Check that we would have enough weight to perform this step in the worst case
				// scenario.
				let required_weight = match &cursor {
					Some(state) => Self::required_weight(state),
					None => T::WeightInfo::migration_v3_buckets_step(),
				};
				if !meter.can_consume(required_weight) {
					break;
				}

				let next = match &cursor {
					None => Self::buckets_step(None),
					Some(MigrationState::MigratingBuckets(maybe_last_bucket)) => {
						Self::buckets_step(Some(maybe_last_bucket))
					},
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
				"PRE-CHECK Step in v3 migration"
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

			Ok((prev_bucket_id, prev_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
			info!(
				target: LOG_TARGET,
				"POST-CHECK Step in v3 migration"
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

			frame_support::ensure!(current_version >= 3, "must_upgrade to 3rd version or higher");

			ensure!(
				on_chain_version == 3,
				"the `on_chain_version` version should be the 3rd after the v3 migration"
			);

			Ok(())
		}
	}

	impl<T: Config> LazyMigrationV2ToV3<T> {
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
				v3::Buckets::<T>::insert(bucket_id, bucket_v3);
				MigrationState::MigratingBuckets(bucket_id)
			} else {
				MigrationState::Finished
			}
		}

		pub(crate) fn required_weight(step: &MigrationState) -> Weight {
			match step {
				MigrationState::MigratingBuckets(_) => T::WeightInfo::migration_v3_buckets_step(),
				MigrationState::Finished => Weight::zero(),
			}
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub(crate) struct BenchmarkingSetupV2ToV3 {
		pub(crate) bucket_id: BucketId,
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl<T: Config> LazyMigrationV2ToV3<T> {
		pub(crate) fn setup_benchmark_env_for_migration() -> BenchmarkingSetupV2ToV3 {
			let bucket_id = 0;
			let owner_id: T::AccountId = frame_benchmarking::account("account", 1, 0);
			let cluster_id = ClusterId::from([0; 20]);

			let bucket = v2::Bucket {
				bucket_id,
				owner_id,
				cluster_id,
				is_public: true,
				is_removed: false,
				total_customers_usage: None,
			};

			v2::Buckets::<T>::insert(bucket_id, &bucket);

			BenchmarkingSetupV2ToV3 { bucket_id }
		}
	}
}

pub mod v4_mbm {
	use frame_support::{
		migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
		pallet_prelude::*,
		weights::WeightMeter,
	};

	const DEVNET_CLUSTER: [u8; 20] = hex!("7f82864e4f097e63d04cc279e4d8d2eb45a42ffa");
	const TESTNET_CLUSTER: [u8; 20] = hex!("825c4b2352850de9986d9d28568db6f0c023a1e3");
	const QANET_CLUSTER: [u8; 20] = hex!("b1242a78440e20f50841ffa399fd9d607a2e93b8");
	const MAINNET_CLUSTER: [u8; 20] = hex!("0059f5ada35eee46802d80750d5ca4a490640511");
	const DEFAULT_CLUSTER: [u8; 20] = hex!("0000000000000000000000000000000000000001");

	use super::*;

	#[storage_alias]
	pub type Ledger<T: Config> = StorageMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		<T as frame_system::Config>::AccountId,
		CustomerLedger<T>,
	>;

	/// Progressive states of a migration. The migration starts with the first variant and ends with
	/// the last.
	#[derive(Decode, Encode, MaxEncodedLen, Eq, PartialEq, Debug)]
	pub enum MigrationState<A> {
		MigratingLedgers(A, ClusterId),
		TransferringBalance(ClusterId),
		Finished,
	}

	pub struct LazyMigrationV3ToV4<T: Config>(PhantomData<T>);
	impl<T: Config> SteppedMigration for LazyMigrationV3ToV4<T> {
		type Cursor = MigrationState<T::AccountId>;
		type Identifier = MigrationId<20>;

		fn id() -> Self::Identifier {
			MigrationId { pallet_id: *PALLET_MIGRATIONS_ID, version_from: 3, version_to: 4 }
		}

		fn step(
			mut cursor: Option<Self::Cursor>,
			meter: &mut WeightMeter,
		) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
			info!(
				target: LOG_TARGET,
				"Step in v4 migration cursor={:?}", cursor
			);

			if Pallet::<T>::on_chain_storage_version() != Self::id().version_from as u16 {
				return Ok(None);
			}

			// Check that we have enough weight for at least the next step. If we don't, then the
			// migration cannot be complete.
			let required = match &cursor {
				Some(state) => Self::required_weight(state),
				None => T::WeightInfo::migration_v4_ledgers_step(),
			};
			if meter.remaining().any_lt(required) {
				return Err(SteppedMigrationError::InsufficientWeight { required });
			}

			loop {
				// Check that we would have enough weight to perform this step in the worst case
				// scenario.
				let required_weight = match &cursor {
					Some(state) => Self::required_weight(state),
					None => T::WeightInfo::migration_v4_ledgers_step(),
				};
				if !meter.can_consume(required_weight) {
					break;
				}

				let next = match &cursor {
					None => Self::ledgers_step(None, None),
					Some(MigrationState::MigratingLedgers(maybe_last_ledger, maybe_cluster_id)) => {
						Self::ledgers_step(Some(maybe_last_ledger), Some(maybe_cluster_id))
					},
					Some(MigrationState::TransferringBalance(cluster_id)) => {
						Self::transfer_balance_step(cluster_id)
					},
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
				"PRE-CHECK Step in v4 migration"
			);

			let on_chain_version = Pallet::<T>::on_chain_storage_version();
			let current_version = Pallet::<T>::in_code_storage_version();

			info!(
				target: LOG_TARGET,
				"Executing `pre_upgrade` check of v4 migration with current storage version {:?} / onchain {:?}",
				current_version,
				on_chain_version
			);

			let prev_count = v4_mbm::Ledger::<T>::iter().count();

			Ok((prev_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
			info!(
				target: LOG_TARGET,
				"POST-CHECK Step in v4 migration"
			);

			let prev_count: u64 = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			info!(
				target: LOG_TARGET,
				"Executing post check of v4 migration prev_count={:?} ...", prev_count
			);

			let post_count = v4_mbm::Ledger::<T>::iter().count() as u64;
			ensure!(
				prev_count == post_count,
				"ledger count before and after the v4 migration should be the same"
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			frame_support::ensure!(current_version >= 4, "must_upgrade to 4th version or higher");

			ensure!(
				on_chain_version == 4,
				"the `on_chain_version` version should be the 4th after the v4 migration"
			);

			Ok(())
		}
	}

	impl<T: Config> LazyMigrationV3ToV4<T> {
		pub(crate) fn ledgers_step(
			maybe_last_key: Option<&T::AccountId>,
			maybe_cluster_id: Option<&ClusterId>,
		) -> MigrationState<T::AccountId> {
			let mut iter = if let Some(last_key) = maybe_last_key {
				v4_mbm::Ledger::<T>::iter_from(v4_mbm::Ledger::<T>::hashed_key_for(last_key))
			} else {
				v4_mbm::Ledger::<T>::iter()
			};

			let cluster_id = if maybe_cluster_id.is_some() {
				*maybe_cluster_id.unwrap()
			} else {
				if <T::ClusterProtocol as ClusterQuery<T::AccountId>>::cluster_exists(
					&ClusterId::from(MAINNET_CLUSTER),
				) {
					info!(
						target: LOG_TARGET,
						"Migrating ledgers to MAINNET_CLUSTER"
					);
					ClusterId::from(MAINNET_CLUSTER)
				} else if <T::ClusterProtocol as ClusterQuery<T::AccountId>>::cluster_exists(
					&ClusterId::from(TESTNET_CLUSTER),
				) {
					info!(
						target: LOG_TARGET,
						"Migrating ledgers to TESTNET_CLUSTER"
					);
					ClusterId::from(TESTNET_CLUSTER)
				} else if <T::ClusterProtocol as ClusterQuery<T::AccountId>>::cluster_exists(
					&ClusterId::from(QANET_CLUSTER),
				) {
					info!(
						target: LOG_TARGET,
						"Migrating ledgers to QANET_CLUSTER"
					);
					ClusterId::from(QANET_CLUSTER)
				} else if <T::ClusterProtocol as ClusterQuery<T::AccountId>>::cluster_exists(
					&ClusterId::from(DEVNET_CLUSTER),
				) {
					info!(
						target: LOG_TARGET,
						"Migrating ledgers to DEVNET_CLUSTER"
					);
					ClusterId::from(DEVNET_CLUSTER)
				} else {
					warn!(
						target: LOG_TARGET,
						"Failling back to DEFAULT_CLUSTER"
					);
					ClusterId::from(DEFAULT_CLUSTER)
				}
			};

			if let Some((key, ledger)) = iter.next() {
				ClusterLedger::<T>::insert(cluster_id, key.clone(), ledger);
				MigrationState::MigratingLedgers(key, cluster_id)
			} else {
				MigrationState::TransferringBalance(cluster_id)
			}
		}

		pub(crate) fn transfer_balance_step(
			cluster_id: &ClusterId,
		) -> MigrationState<T::AccountId> {
			let pallet_account_id = crate::Pallet::<T>::pallet_account_id();
			let cluster_vault_id = crate::Pallet::<T>::cluster_vault_id(cluster_id);

			let pallet_balance = <T as pallet::Config>::Currency::free_balance(&pallet_account_id);

			if pallet_balance > <T as pallet::Config>::Currency::minimum_balance() {
				if let Err(e) = <T as pallet::Config>::Currency::transfer(
					&pallet_account_id,
					&cluster_vault_id,
					pallet_balance,
					ExistenceRequirement::AllowDeath,
				) {
					log::error!("❌ Error transferring balance: {:?}. Resolve this issue manually after the migration.", e);
				} else {
					log::info!(
						"✅ Successfully transferred {:?} tokens from pallet {:?} to cluster vault {:?}",
						pallet_balance,
						pallet_account_id,
						cluster_vault_id
					);
				}
			}

			MigrationState::Finished
		}

		pub(crate) fn required_weight(step: &MigrationState<T::AccountId>) -> Weight {
			match step {
				MigrationState::MigratingLedgers(_, _) => {
					T::WeightInfo::migration_v4_ledgers_step()
				},
				MigrationState::TransferringBalance(_) => {
					// This is copied from pallet_balances::WeightInfo::transfer_allow_death()

					// Proof Size summary in bytes:
					//  Measured:  `0`
					//  Estimated: `3593`
					// Minimum execution time: 44_771_000 picoseconds.
					Weight::from_parts(45_635_000, 0)
						.saturating_add(Weight::from_parts(0, 3593))
						.saturating_add(T::DbWeight::get().reads(1))
						.saturating_add(T::DbWeight::get().writes(1))
				},
				MigrationState::Finished => Weight::zero(),
			}
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub(crate) struct BenchmarkingSetupV3ToV4<A> {
		pub(crate) ledger_owner: A,
		pub(crate) cluster_id: ClusterId,
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl<T: Config> LazyMigrationV3ToV4<T> {
		pub(crate) fn setup_benchmark_env_for_migration() -> BenchmarkingSetupV3ToV4<T::AccountId> {
			use ddc_primitives::{ClusterParams, ClusterProtocolParams};
			use sp_runtime::Perquintill;

			let cluster_id = ClusterId::from(DEFAULT_CLUSTER);
			let cluster_owner: T::AccountId = frame_benchmarking::account("account", 1, 0);
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

			let cluster_params = ClusterParams {
				node_provider_auth_contract: None,
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3,
			};

			let _ = <T as pallet::Config>::ClusterCreator::create_cluster(
				cluster_id,
				cluster_owner.clone(),
				cluster_owner.clone(),
				cluster_params,
				cluster_protocol_params,
			);

			let owner: T::AccountId = frame_benchmarking::account("account", 2, 0);
			let ledger = CustomerLedger::default_from(owner.clone());

			v4_mbm::Ledger::<T>::insert(&owner, &ledger);

			BenchmarkingSetupV3ToV4::<T::AccountId> { ledger_owner: owner, cluster_id }
		}
	}
}
