use super::*;
use crate::cluster::ClusterProps;
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
use frame_support::{
	storage_alias,
	traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
	weights::Weight,
};
use log::info;
#[cfg(feature = "try-runtime")]
use sp_runtime::DispatchError;
use sp_runtime::Saturating;

const LOG_TARGET: &str = "ddc-clusters";

pub mod v0 {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct Cluster<AccountId> {
		pub cluster_id: ClusterId,
		pub manager_id: AccountId,
		pub reserve_id: AccountId,
		pub props: ClusterProps<AccountId>,
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct ClusterProps<AccountId> {
		pub node_provider_auth_contract: Option<AccountId>,
	}

	#[storage_alias]
	pub(super) type Clusters<T: Config> = StorageMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		ClusterId,
		Cluster<<T as frame_system::Config>::AccountId>,
	>;
}

pub fn migrate_to_v1<T: Config>() -> Weight {
	let on_chain_version = Pallet::<T>::on_chain_storage_version();
	let current_version = Pallet::<T>::current_storage_version();

	info!(
		target: LOG_TARGET,
		"Running migration with current storage version {:?} / onchain {:?}",
		current_version,
		on_chain_version
	);

	if on_chain_version == 0 && current_version == 1 {
		let mut translated = 0u64;
		let count = v0::Clusters::<T>::iter().count();
		info!(
			target: LOG_TARGET,
			" >>> Updating DDC Cluster storage. Migrating {} clusters...", count
		);

		Clusters::<T>::translate::<v0::Cluster<T::AccountId>, _>(
			|cluster_id: ClusterId, cluster: v0::Cluster<T::AccountId>| {
				info!(target: LOG_TARGET, "     Migrating cluster for cluster ID {:?}...", cluster_id);
				translated.saturating_inc();
				let props = ClusterProps {
					node_provider_auth_contract: cluster.props.node_provider_auth_contract,
					erasure_coding_required: 16,
					erasure_coding_total: 48,
					replication_total: 20,
				};

				Some(Cluster {
					cluster_id: cluster.cluster_id,
					manager_id: cluster.manager_id,
					reserve_id: cluster.reserve_id,
					props,
				})
			},
		);

		// Update storage version.
		StorageVersion::new(1).put::<Pallet<T>>();
		info!(
			target: LOG_TARGET,
			"Upgraded {} records, storage to version {:?}",
			translated,
			current_version
		);

		T::DbWeight::get().reads_writes(translated + 1, translated + 1)
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
		let prev_count = v0::Clusters::<T>::iter().count();

		Ok((prev_count as u64).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
		let prev_count: u64 =
			Decode::decode(&mut &prev_state[..]).expect("pre_upgrade provides a valid state; qed");

		let post_count = Clusters::<T>::iter().count() as u64;
		ensure!(
			prev_count == post_count,
			"the cluster count before and after the migration should be the same"
		);

		let current_version = Pallet::<T>::current_storage_version();
		let on_chain_version = Pallet::<T>::on_chain_storage_version();

		frame_support::ensure!(current_version == 1, "must_upgrade");
		ensure!(
			current_version == on_chain_version,
			"after migration, the current_version and on_chain_version should be the same"
		);
		Ok(())
	}
}

#[cfg(test)]
#[cfg(feature = "try-runtime")]
mod test {

	use super::*;
	use crate::mock::{Test as T, *};
	use frame_support::pallet_prelude::StorageVersion;

	#[test]
	fn cluster_migration_works() {
		ExtBuilder.build_and_execute(|| {
			let cluster_id0 = ClusterId::from([0; 20]);
			let cluster_id1 = ClusterId::from([1; 20]);
			let cluster_id2 = ClusterId::from([2; 20]);
			let cluster_manager_id = AccountId::from([1; 32]);
			let cluster_reserve_id = AccountId::from([2; 32]);
			let auth_contract = AccountId::from([3; 32]);

			assert_eq!(StorageVersion::get::<Pallet<T>>(), 0);

			let cluster1 = v0::Cluster {
				cluster_id: cluster_id1,
				manager_id: cluster_manager_id.clone(),
				reserve_id: cluster_reserve_id.clone(),
				props: v0::ClusterProps {
					node_provider_auth_contract: Some(auth_contract.clone()),
				},
			};

			v0::Clusters::<T>::insert(cluster_id1, cluster1);
			let cluster2 = v0::Cluster {
				cluster_id: cluster_id2,
				manager_id: cluster_manager_id,
				reserve_id: cluster_reserve_id,
				props: v0::ClusterProps {
					node_provider_auth_contract: Some(auth_contract.clone()),
				},
			};

			v0::Clusters::<T>::insert(cluster_id2, cluster2);
			let cluster_count = v0::Clusters::<T>::iter_values().count() as u32;

			assert_eq!(cluster_count, 3);
			let state = MigrateToV1::<T>::pre_upgrade().unwrap();
			let _weight = MigrateToV1::<T>::on_runtime_upgrade();
			MigrateToV1::<T>::post_upgrade(state).unwrap();

			let cluster_count_after_upgrade = Clusters::<T>::iter_values().count() as u32;

			assert_eq!(StorageVersion::get::<Pallet<T>>(), 1);
			assert_eq!(cluster_count_after_upgrade, 3);
			assert_eq!(Clusters::<T>::get(&cluster_id0).unwrap().props.erasure_coding_required, 16);
			assert_eq!(Clusters::<T>::get(&cluster_id0).unwrap().props.erasure_coding_total, 48);
			assert_eq!(Clusters::<T>::get(&cluster_id0).unwrap().props.replication_total, 20);
			assert_eq!(Clusters::<T>::get(&cluster_id1).unwrap().props.erasure_coding_required, 16);
			assert_eq!(Clusters::<T>::get(&cluster_id1).unwrap().props.erasure_coding_total, 48);
			assert_eq!(Clusters::<T>::get(&cluster_id1).unwrap().props.replication_total, 20);
			assert_eq!(Clusters::<T>::get(&cluster_id2).unwrap().props.erasure_coding_required, 16);
			assert_eq!(Clusters::<T>::get(&cluster_id2).unwrap().props.erasure_coding_total, 48);
			assert_eq!(Clusters::<T>::get(&cluster_id2).unwrap().props.replication_total, 20);
		});
	}
}
