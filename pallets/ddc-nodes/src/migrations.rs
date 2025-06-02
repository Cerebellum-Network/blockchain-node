#[cfg(feature = "try-runtime")]
use ddc_primitives::StorageNodePubKey;
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
use frame_support::{
	storage_alias,
	traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
	weights::Weight,
};
use log::info;
use serde::{Deserialize, Serialize};
use sp_runtime::Saturating;

use super::*;
use crate::{storage_node::StorageNodeProps, ClusterId};

const LOG_TARGET: &str = "ddc-customers";
pub const PALLET_MIGRATIONS_ID: &[u8; 16] = b"pallet-ddc-nodes";

pub mod v0 {
	use frame_support::pallet_prelude::*;

	use super::*;

	// Define the old storage node structure
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
	#[scale_info(skip_type_params(T))]
	pub struct StorageNode<T: frame_system::Config> {
		pub pub_key: StorageNodePubKey,
		pub provider_id: <T as frame_system::Config>::AccountId,
		pub cluster_id: Option<ClusterId>,
		pub props: StorageNodeProps,
	}

	#[storage_alias]
	pub type StorageNodes<T: Config> =
		StorageMap<crate::Pallet<T>, Blake2_128Concat, StorageNodePubKey, StorageNode<T>>;
}

pub mod v1 {
	use ddc_primitives::NodeUsage;

	use super::*;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
	#[scale_info(skip_type_params(T))]
	pub struct StorageNode<T: frame_system::Config> {
		pub pub_key: StorageNodePubKey,
		pub provider_id: T::AccountId,
		pub cluster_id: Option<ClusterId>,
		pub props: StorageNodeProps,
		pub total_usage: Option<NodeUsage>, // new field
	}

	#[storage_alias]
	pub type StorageNodes<T: Config> =
		StorageMap<crate::Pallet<T>, Blake2_128Concat, StorageNodePubKey, StorageNode<T>>;

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
			let weight = T::DbWeight::get().reads(1);

			let mut translated = 0u64;
			let count = v0::StorageNodes::<T>::iter().count();
			info!(
				target: LOG_TARGET,
				" >>> Updating DDC Storage Nodes. Migrating {} nodes...", count
			);
			v1::StorageNodes::<T>::translate::<v0::StorageNode<T>, _>(
				|_, old: v0::StorageNode<T>| {
					let node_pub_key_ref: &[u8; 32] = old.pub_key.as_ref();
					let node_pub_key_string = hex::encode(node_pub_key_ref);
					info!(target: LOG_TARGET, "     Migrating node for node ID {:?}...", node_pub_key_string);
					translated.saturating_inc();

					Some(StorageNode {
						pub_key: old.pub_key,
						provider_id: old.provider_id,
						cluster_id: old.cluster_id,
						props: old.props,
						total_usage: None, // Default value for the new field
					})
				},
			);

			// Update storage version.
			StorageVersion::new(1).put::<Pallet<T>>();
			let count = v1::StorageNodes::<T>::iter().count();
			info!(
				target: LOG_TARGET,
				"Upgraded {} records, storage to version {:?}",
				count,
				current_version
			);

			weight.saturating_add(T::DbWeight::get().reads_writes(translated + 1, translated + 1))
		} else {
			info!(target: LOG_TARGET, " >>> Unused migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV1<T>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v1::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
			let prev_count = v0::StorageNodes::<T>::iter().count();

			Ok((prev_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
			let prev_count: u64 = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			let post_count = v1::StorageNodes::<T>::iter().count() as u64;
			ensure!(
				prev_count == post_count,
				"the storage node count before and after the migration should be the same"
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			ensure!(current_version == 1, "must_upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same"
			);

			// Ensure all nodes have total_usage set to None
			for (_key, node) in v1::StorageNodes::<T>::iter() {
				ensure!(node.total_usage.is_none(), "total_usage should be None");
			}

			Ok(())
		}
	}
}

pub mod v2 {
	use super::*;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
	#[scale_info(skip_type_params(T))]
	pub struct StorageNode<T: frame_system::Config> {
		pub pub_key: StorageNodePubKey,
		pub provider_id: T::AccountId,
		pub cluster_id: Option<ClusterId>,
		pub props: StorageNodeProps,
	}

	#[storage_alias]
	pub type StorageNodes<T: Config> =
		StorageMap<crate::Pallet<T>, Blake2_128Concat, StorageNodePubKey, StorageNode<T>>;

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
			let weight = T::DbWeight::get().reads(1);

			let mut translated = 0u64;
			let count = v1::StorageNodes::<T>::iter().count();
			info!(
				target: LOG_TARGET,
				">>> Updating DDC Storage Nodes. Migrating {} nodes in v2 migration ...", count
			);
			v2::StorageNodes::<T>::translate::<v1::StorageNode<T>, _>(
				|_, old: v1::StorageNode<T>| {
					let node_pub_key_ref: &[u8; 32] = old.pub_key.as_ref();
					let node_pub_key_string = hex::encode(node_pub_key_ref);
					info!(target: LOG_TARGET, "Migrating node for node ID {:?} in v2 migration ...", node_pub_key_string);
					translated.saturating_inc();

					Some(v2::StorageNode {
						pub_key: old.pub_key,
						provider_id: old.provider_id,
						cluster_id: old.cluster_id,
						props: old.props,
					})
				},
			);

			// Update storage version.
			StorageVersion::new(2).put::<Pallet<T>>();
			info!(
				target: LOG_TARGET,
				"Upgraded {} records, storage to version {:?} in v2 migration ...",
				count,
				current_version
			);

			weight.saturating_add(T::DbWeight::get().reads_writes(translated + 1, translated + 1))
		} else {
			info!(target: LOG_TARGET, " >>> Unused v2 migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV2<T>(PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v2::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
			let prev_count = v1::StorageNodes::<T>::iter().count();
			Ok((prev_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
			let prev_count: u64 = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			info!(
				target: LOG_TARGET,
				"Executing post check of v2 migration prev_count={:?} ...", prev_count
			);

			let post_count = v2::StorageNodes::<T>::iter().count() as u64;
			ensure!(
				prev_count == post_count,
				"the storage node count before and after the v2 migration should be the same"
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			ensure!(current_version == 2, "must_upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same in v2 migration"
			);

			Ok(())
		}
	}
}

pub mod v2_mbm {
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
		MigratingNodes(StorageNodePubKey),
		Finished,
	}

	pub struct LazyMigrationV1ToV2<T: Config>(PhantomData<T>);

	impl<T: Config> SteppedMigration for LazyMigrationV1ToV2<T> {
		type Cursor = MigrationState;
		type Identifier = MigrationId<16>;

		fn id() -> Self::Identifier {
			MigrationId { pallet_id: *PALLET_MIGRATIONS_ID, version_from: 1, version_to: 2 }
		}

		fn step(
			mut cursor: Option<Self::Cursor>,
			meter: &mut WeightMeter,
		) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
			info!(
				target: LOG_TARGET,
				"Step in v2 migration cursor={:?}", cursor
			);

			if Pallet::<T>::on_chain_storage_version() != Self::id().version_from as u16 {
				return Ok(None);
			}

			// Check that we have enough weight for at least the next step. If we don't, then the
			// migration cannot be complete.
			let required = match &cursor {
				Some(state) => Self::required_weight(state),
				None => T::WeightInfo::migration_v2_nodes_step(),
			};
			if meter.remaining().any_lt(required) {
				return Err(SteppedMigrationError::InsufficientWeight { required });
			}

			loop {
				// Check that we would have enough weight to perform this step in the worst case
				// scenario.
				let required_weight = match &cursor {
					Some(state) => Self::required_weight(state),
					None => T::WeightInfo::migration_v2_nodes_step(),
				};
				if !meter.can_consume(required_weight) {
					break;
				}

				let next = match &cursor {
					None => Self::nodes_step(None),
					Some(MigrationState::MigratingNodes(maybe_last_node)) =>
						Self::nodes_step(Some(maybe_last_node)),
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
				"PRE-CHECK Step in v2 migration"
			);

			let prev_count = v1::StorageNodes::<T>::iter().count();
			Ok((prev_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
			info!(
				target: LOG_TARGET,
				"POST-CHECK Step in v2 migration"
			);

			let prev_count: u64 = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			info!(
				target: LOG_TARGET,
				"Executing post check of v2 migration prev_count={:?} ...", prev_count
			);

			let post_count = v2::StorageNodes::<T>::iter().count() as u64;
			ensure!(
				prev_count == post_count,
				"the storage node count before and after the v2 migration should be the same"
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			ensure!(current_version == 2, "must_upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same in v2 migration"
			);

			Ok(())
		}
	}

	impl<T: Config> LazyMigrationV1ToV2<T> {
		// Migrate one entry from `UsernameAuthorities` to `AuthorityOf`.
		pub(crate) fn nodes_step(maybe_last_key: Option<&StorageNodePubKey>) -> MigrationState {
			let mut iter = if let Some(last_key) = maybe_last_key {
				v1::StorageNodes::<T>::iter_from(v1::StorageNodes::<T>::hashed_key_for(last_key))
			} else {
				v1::StorageNodes::<T>::iter()
			};
			if let Some((node_key, node_v1)) = iter.next() {
				let node_v2 = v2::StorageNode {
					pub_key: node_v1.pub_key,
					provider_id: node_v1.provider_id,
					cluster_id: node_v1.cluster_id,
					props: node_v1.props,
				};
				v2::StorageNodes::<T>::insert(&node_key, node_v2);
				MigrationState::MigratingNodes(node_key)
			} else {
				MigrationState::Finished
			}
		}

		pub(crate) fn required_weight(step: &MigrationState) -> Weight {
			match step {
				MigrationState::MigratingNodes(_) => T::WeightInfo::migration_v2_nodes_step(),
				MigrationState::Finished => Weight::zero(),
			}
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub(crate) struct BenchmarkingSetupV1ToV2 {
		pub(crate) node_key: StorageNodePubKey,
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl<T: Config> LazyMigrationV1ToV2<T> {
		pub(crate) fn setup_benchmark_env_for_migration() -> BenchmarkingSetupV1ToV2 {
			use ddc_primitives::StorageNodeMode;

			let node_key = StorageNodePubKey::from([0; 32]);

			let provider_id: T::AccountId = frame_benchmarking::account("account", 1, 0);
			let cluster_id = ClusterId::from([0; 20]);
			let props = StorageNodeProps {
				mode: StorageNodeMode::Storage,
				host: vec![3u8; 255].try_into().unwrap(),
				domain: vec![4u8; 255].try_into().unwrap(),
				ssl: true,
				http_port: 45000u16,
				grpc_port: 55000u16,
				p2p_port: 65000u16,
			};

			let node = v1::StorageNode {
				pub_key: node_key.clone(),
				provider_id,
				cluster_id: Some(cluster_id),
				props,
				total_usage: None,
			};

			v1::StorageNodes::<T>::insert(&node_key, &node);

			BenchmarkingSetupV1ToV2 { node_key }
		}
	}
}

#[cfg(test)]
#[cfg(feature = "try-runtime")]
mod test {
	use ddc_primitives::StorageNodeMode;
	use frame_support::pallet_prelude::StorageVersion;

	use super::*;
	use crate::mock::{Test as T, *};

	#[test]
	fn storage_node_migration_works() {
		ExtBuilder.build_and_execute(|| {
			let node_pub_key0 = StorageNodePubKey::from([0; 32]);
			let node_pub_key1 = StorageNodePubKey::from([1; 32]);
			let node_pub_key2 = StorageNodePubKey::from([2; 32]);
			let provider_id = AccountId::from([1; 32]);
			let cluster_id = Some(ClusterId::from([1; 20]));

			assert_eq!(StorageVersion::get::<Pallet<T>>(), 0);

			let node1 = v0::StorageNode {
				pub_key: node_pub_key1.clone(),
				provider_id: provider_id.clone(),
				cluster_id,
				props: StorageNodeProps {
					mode: StorageNodeMode::Storage,
					host: vec![3u8; 255].try_into().unwrap(),
					domain: vec![4u8; 255].try_into().unwrap(),
					ssl: true,
					http_port: 45000u16,
					grpc_port: 55000u16,
					p2p_port: 65000u16,
				},
			};

			v0::StorageNodes::<T>::insert(node_pub_key1.clone(), node1);
			let node2 = v0::StorageNode {
				pub_key: node_pub_key2.clone(),
				provider_id: provider_id.clone(),
				cluster_id,
				props: StorageNodeProps {
					mode: StorageNodeMode::Storage,
					host: vec![3u8; 255].try_into().unwrap(),
					domain: vec![4u8; 255].try_into().unwrap(),
					ssl: true,
					http_port: 45000u16,
					grpc_port: 55000u16,
					p2p_port: 65000u16,
				},
			};

			v0::StorageNodes::<T>::insert(node_pub_key2.clone(), node2);
			let node_count = v0::StorageNodes::<T>::iter_values().count() as u32;

			assert_eq!(node_count, 2);
			let state = v1::MigrateToV1::<T>::pre_upgrade().unwrap();
			let _weight = v1::MigrateToV1::<T>::on_runtime_upgrade();
			v1::MigrateToV1::<T>::post_upgrade(state).unwrap();

			let node_count_after_upgrade = v1::StorageNodes::<T>::iter_values().count() as u32;

			assert_eq!(StorageVersion::get::<Pallet<T>>(), 1);
			assert_eq!(node_count_after_upgrade, 2);
			assert_eq!(StorageNodes::<T>::get(node_pub_key0), None);
			assert!(StorageNodes::<T>::get(node_pub_key1.clone()).is_some());
			assert!(StorageNodes::<T>::get(node_pub_key2.clone()).is_some());
		});
	}
}
