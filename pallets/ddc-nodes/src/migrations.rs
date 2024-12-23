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
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 0 && current_version == 1 {
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
			assert_eq!(StorageNodes::<T>::get(node_pub_key1).unwrap().total_usage, None);
			assert_eq!(StorageNodes::<T>::get(node_pub_key2).unwrap().total_usage, None);
		});
	}
}
