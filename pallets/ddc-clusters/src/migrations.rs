use ddc_primitives::InspectionDryRunParams;
use frame_support::{storage_alias, traits::OnRuntimeUpgrade};
use log::info;
use serde::{Deserialize, Serialize};
use sp_runtime::{Perquintill, Saturating};

use super::*;

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

pub mod v1 {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
	pub struct Cluster<AccountId> {
		pub cluster_id: ClusterId,
		pub manager_id: AccountId,
		pub reserve_id: AccountId,
		pub props: ClusterProps<AccountId>,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
	pub struct ClusterProps<AccountId> {
		pub node_provider_auth_contract: Option<AccountId>,
		pub erasure_coding_required: u32, // new field
		pub erasure_coding_total: u32,    // new field
		pub replication_total: u32,       // new field
	}

	#[storage_alias]
	pub(super) type Clusters<T: Config> = StorageMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		ClusterId,
		Cluster<<T as frame_system::Config>::AccountId>,
	>;

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
			let mut translated = 0u64;
			let count = v0::Clusters::<T>::iter().count();
			info!(
				target: LOG_TARGET,
				" >>> Updating DDC Cluster storage. Migrating {} clusters...", count
			);

			v1::Clusters::<T>::translate::<v0::Cluster<T::AccountId>, _>(
				|cluster_id: ClusterId, cluster: v0::Cluster<T::AccountId>| {
					info!(target: LOG_TARGET, "     Migrating cluster for cluster ID {:?}...", cluster_id);
					translated.saturating_inc();
					let props = ClusterProps {
						node_provider_auth_contract: cluster.props.node_provider_auth_contract,
						erasure_coding_required: 16,
						erasure_coding_total: 48,
						replication_total: 20,
					};

					Some(v1::Cluster {
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
			let prev_count: u64 = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			let post_count = Clusters::<T>::iter().count() as u64;
			ensure!(
				prev_count == post_count,
				"the cluster count before and after the migration should be the same"
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			frame_support::ensure!(current_version == 1, "must_upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same"
			);
			Ok(())
		}
	}
}

pub mod v2 {
	use ddc_primitives::{
		ClusterId, ClusterNodeKind, ClusterNodeState, ClusterNodeStatus, ClusterNodesCount,
		ClusterNodesStats, ClusterStatus,
	};
	use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade, weights::Weight};
	use sp_runtime::Saturating;
	use sp_std::collections::btree_map::BTreeMap;
	#[cfg(feature = "try-runtime")]
	use sp_std::vec::Vec;

	use super::*;
	use crate::{ClustersNodes, ClustersNodesStats, Config, Pallet, LOG_TARGET};

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
	pub struct Cluster<AccountId> {
		pub cluster_id: ClusterId,
		pub manager_id: AccountId,
		pub reserve_id: AccountId,
		pub props: ClusterProps<AccountId>,
		pub status: ClusterStatus, // new field
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
	pub struct ClusterProps<AccountId> {
		pub node_provider_auth_contract: Option<AccountId>,
		pub erasure_coding_required: u32,
		pub erasure_coding_total: u32,
		pub replication_total: u32,
	}

	#[storage_alias]
	pub(super) type Clusters<T: Config> = StorageMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		ClusterId,
		Cluster<<T as frame_system::Config>::AccountId>,
	>;

	pub type OldNodeStatus = bool;

	pub fn migrate_to_v2<T: Config>() -> Weight {
		let current_version = Pallet::<T>::in_code_storage_version();
		let onchain_version = Pallet::<T>::on_chain_storage_version();
		let mut weight = T::DbWeight::get().reads(1);

		if onchain_version == 1 && current_version == 2 {
			let mut nodes_count_by_cluster: BTreeMap<ClusterId, ClusterNodesCount> =
				BTreeMap::new();

			let mut translated_clusters = 0u64;
			v2::Clusters::<T>::translate::<v1::Cluster<T::AccountId>, _>(
				|cluster_id, old_cluster| {
					translated_clusters.saturating_inc();
					nodes_count_by_cluster.insert(cluster_id, 0);

					// all clusters are unbonded by default
					let status = ClusterStatus::Unbonded;
					let new_cluster = Cluster {
						cluster_id: old_cluster.cluster_id,
						manager_id: old_cluster.manager_id,
						reserve_id: old_cluster.reserve_id,
						props: ClusterProps {
							node_provider_auth_contract: old_cluster
								.props
								.node_provider_auth_contract,
							erasure_coding_required: old_cluster.props.erasure_coding_required,
							erasure_coding_total: old_cluster.props.erasure_coding_total,
							replication_total: old_cluster.props.replication_total,
						},
						status,
					};

					Some(new_cluster)
				},
			);
			weight.saturating_accrue(
				T::DbWeight::get().reads_writes(translated_clusters, translated_clusters),
			);

			current_version.put::<Pallet<T>>();
			weight.saturating_accrue(T::DbWeight::get().writes(1));
			log::info!(
				target: LOG_TARGET,
				"Upgraded {} clusters, storage to version {:?}",
				translated_clusters,
				current_version
			);

			let mut translated_clusters_nodes = 0u64;
			ClustersNodes::<T>::translate::<OldNodeStatus, _>(
				|cluster_id, _node_pub_key, _old_value| {
					translated_clusters_nodes.saturating_inc();

					nodes_count_by_cluster
						.entry(cluster_id)
						.and_modify(|count| *count = count.saturating_add(1)) // If exists, update
						.or_insert(1); // If not, insert with a count of 1

					Some(ClusterNodeState {
						kind: ClusterNodeKind::External,
						status: ClusterNodeStatus::ValidationSucceeded,
						added_at: <frame_system::Pallet<T>>::block_number(),
					})
				},
			);
			weight.saturating_accrue(
				T::DbWeight::get()
					.reads_writes(translated_clusters_nodes, translated_clusters_nodes),
			);
			log::info!(
				target: LOG_TARGET,
				"Upgraded {} clusters nodes statuses, storage to version {:?}",
				translated_clusters_nodes,
				current_version
			);

			for (cluster_id, nodes_count) in nodes_count_by_cluster.iter() {
				ClustersNodesStats::<T>::insert(
					cluster_id,
					ClusterNodesStats {
						await_validation: 0,
						validation_succeeded: *nodes_count,
						validation_failed: 0,
					},
				);
				weight.saturating_accrue(T::DbWeight::get().writes(1));
			}
			log::info!(
				target: LOG_TARGET,
				"Upgraded {} clusters statistics, storage to version {:?}",
				nodes_count_by_cluster.len(),
				current_version
			);

			// Update storage version.
			StorageVersion::new(2).put::<Pallet<T>>();

			weight
		} else {
			log::info!(
				target: LOG_TARGET,
				"Migration did not execute. This probably should be removed"
			);

			weight
		}
	}

	pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v2::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
			frame_support::ensure!(
				Pallet::<T>::on_chain_storage_version() == 1,
				"must upgrade linearly"
			);
			let pre_clusters_count = Clusters::<T>::iter_keys().count();
			let pre_clusters_nodes_count = ClustersNodes::<T>::iter_keys().count();
			let pre_clusters_nodes_stats_count = ClustersNodesStats::<T>::iter_keys().count();

			assert_eq!(
				pre_clusters_nodes_stats_count, 0,
				"clusters statistics should be empty before the migration"
			);

			Ok((pre_clusters_count as u32, pre_clusters_nodes_count as u32).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(state: Vec<u8>) -> Result<(), DispatchError> {
			let (pre_clusters_count, pre_clusters_nodes_count): (u32, u32) = Decode::decode(
				&mut &state[..],
			)
			.expect("the state parameter should be something that was generated by pre_upgrade");

			let post_clusters_count = Clusters::<T>::iter().count() as u32;
			assert_eq!(
                pre_clusters_count, post_clusters_count,
                "the clusters count before (pre: {}) and after (post: {}) the migration should be the same",
                pre_clusters_count, post_clusters_count
            );
			let post_clusters_nodes_count = ClustersNodes::<T>::iter().count() as u32;
			assert_eq!(
                pre_clusters_nodes_count, post_clusters_nodes_count,
                "the clusters nodes count before (pre: {}) and after (post: {})  the migration should be the same",
                pre_clusters_nodes_count, post_clusters_nodes_count
            );

			let post_clusters_nodes_stats_count = ClustersNodesStats::<T>::iter().count() as u32;
			assert_eq!(
                post_clusters_nodes_stats_count, post_clusters_count,
                "the clusters statistics ({}) should be equal to clusters count ({}) after the migration",
                post_clusters_nodes_stats_count, post_clusters_count
            );

			let current_version = Pallet::<T>::in_code_storage_version();
			let onchain_version = Pallet::<T>::on_chain_storage_version();

			frame_support::ensure!(current_version == 2, "must_upgrade");
			assert_eq!(
				current_version, onchain_version,
				"after migration, the current_version and onchain_version should be the same"
			);

			Clusters::<T>::iter().for_each(|(_id, cluster)| {
				assert!(
					cluster.status == ClusterStatus::Unbonded,
					"cluster status should only be 'Unbonded'."
				)
			});
			Ok(())
		}
	}
}

pub mod v3 {
	use frame_support::{
		pallet_prelude::*,
		traits::{Get, OnRuntimeUpgrade},
		weights::Weight,
	};
	use sp_std::marker::PhantomData;
	#[cfg(feature = "try-runtime")]
	use sp_std::vec::Vec;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
	pub struct Cluster<AccountId> {
		pub cluster_id: ClusterId,
		pub manager_id: AccountId,
		pub reserve_id: AccountId,
		pub props: ClusterProps<AccountId>,
		pub status: ClusterStatus,
		pub last_paid_era: EhdEra, // new field
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
	pub struct ClusterProps<AccountId> {
		pub node_provider_auth_contract: Option<AccountId>,
		pub erasure_coding_required: u32,
		pub erasure_coding_total: u32,
		pub replication_total: u32,
	}

	#[storage_alias]
	pub(super) type Clusters<T: Config> = StorageMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		ClusterId,
		Cluster<<T as frame_system::Config>::AccountId>,
	>;

	#[derive(
		Clone,
		Encode,
		Decode,
		DecodeWithMemTracking,
		RuntimeDebug,
		TypeInfo,
		PartialEq,
		Default,
		Serialize,
		Deserialize,
	)]
	#[scale_info(skip_type_params(Balance, BlockNumber, T))]
	pub struct ClusterProtocolParams<Balance, BlockNumber> {
		pub treasury_share: Perquintill,
		pub validators_share: Perquintill,
		pub cluster_reserve_share: Perquintill,
		pub storage_bond_size: Balance,
		pub storage_chill_delay: BlockNumber,
		pub storage_unbonding_delay: BlockNumber,
		pub unit_per_mb_stored: u128,
		pub unit_per_mb_streamed: u128,
		pub unit_per_put_request: u128,
		pub unit_per_get_request: u128,
	}

	#[storage_alias]
	pub type ClustersGovParams<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		ClusterId,
		ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>,
	>;

	use super::*;
	pub fn migrate_to_v3<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		info!(
			target: LOG_TARGET,
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 2 && current_version == 3 {
			let mut translated = 0u64;
			let count = v2::Clusters::<T>::iter().count();
			info!(
				target: LOG_TARGET,
				" >>> Updating DDC Cluster storage. Migrating {} clusters...", count
			);

			v3::Clusters::<T>::translate::<v2::Cluster<T::AccountId>, _>(
				|cluster_id: ClusterId, old_cluster: v2::Cluster<T::AccountId>| {
					info!(target: LOG_TARGET, "     Migrating cluster for cluster ID {:?}...", cluster_id);
					translated.saturating_inc();
					Some(Cluster {
						cluster_id: old_cluster.cluster_id,
						manager_id: old_cluster.manager_id,
						reserve_id: old_cluster.reserve_id,
						props: ClusterProps {
							node_provider_auth_contract: old_cluster
								.props
								.node_provider_auth_contract,
							erasure_coding_required: old_cluster.props.erasure_coding_required,
							erasure_coding_total: old_cluster.props.erasure_coding_total,
							replication_total: old_cluster.props.replication_total,
						},
						status: old_cluster.status,
						last_paid_era: 0,
					})
				},
			);

			// Update storage version.
			StorageVersion::new(3).put::<Pallet<T>>();
			info!(
				target: LOG_TARGET,
				"Upgraded {} records, storage to version {:?}",
				count,
				current_version
			);

			T::DbWeight::get().reads_writes(translated + 1, translated + 1)
		} else {
			info!(target: LOG_TARGET, " >>> Unused migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV3<T>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v3::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
			let prev_count = v2::Clusters::<T>::iter().count();

			Ok((prev_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
			let prev_count: u64 = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			let post_count = v3::Clusters::<T>::iter().count() as u64;
			ensure!(
				prev_count == post_count,
				"the cluster count before and after the migration should be the same"
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			ensure!(current_version == 3, "must_upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same"
			);
			Ok(())
		}
	}
}

pub mod v4 {
	use super::*;
	use crate::BalanceOf;
	use frame_support::{pallet_prelude::*, traits::Get, weights::Weight};

	#[derive(
		Clone,
		Encode,
		Decode,
		DecodeWithMemTracking,
		RuntimeDebug,
		TypeInfo,
		PartialEq,
		Default,
		Serialize,
		Deserialize,
	)]
	#[scale_info(skip_type_params(Balance, BlockNumber, T))]
	pub struct ClusterProtocolParams<Balance, BlockNumber, AccountId> {
		pub treasury_share: Perquintill,
		pub validators_share: Perquintill,
		pub cluster_reserve_share: Perquintill,
		pub storage_bond_size: Balance,
		pub storage_chill_delay: BlockNumber,
		pub storage_unbonding_delay: BlockNumber,
		pub unit_per_mb_stored: u128,
		pub unit_per_mb_streamed: u128,
		pub unit_per_put_request: u128,
		pub unit_per_get_request: u128,
		pub customer_deposit_contract: AccountId,
	}

	#[storage_alias]
	pub type ClustersGovParams<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		ClusterId,
		ClusterProtocolParams<
			BalanceOf<T>,
			BlockNumberFor<T>,
			<T as frame_system::Config>::AccountId,
		>,
	>;

	pub fn migrate_to_v4<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		info!(
			target: LOG_TARGET,
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		// Allow bundled runtime: when in-code is already 5/6, we still need to run 3->4
		if on_chain_version == 3 && current_version >= 4 {
			let mut translated = 0u64;
			let count = v3::ClustersGovParams::<T>::iter().count();
			info!(
				target: LOG_TARGET,
				" >>> Updating DDC Cluster protocol params storage. Migrating {} cluster protocol params...", count
			);

			v4::ClustersGovParams::<T>::translate::<v3::ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>, _>(
				|cluster_id: ClusterId, old_cluster_params: v3::ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>| {
					info!(target: LOG_TARGET, "     Migrating protocol params for cluster {:?} ...", cluster_id);
					translated.saturating_inc();
					let bytes = [0u8; 32];
					let default_customer_deposit_contract = T::AccountId::decode(&mut &bytes.encode()[..])
						.expect("Customer deposit contract to be parsed");

					Some(v4::ClusterProtocolParams {
						treasury_share: old_cluster_params.treasury_share,
						validators_share: old_cluster_params.validators_share,
						cluster_reserve_share: old_cluster_params.cluster_reserve_share,
						storage_bond_size: old_cluster_params.storage_bond_size,
						storage_chill_delay: old_cluster_params.storage_chill_delay,
						storage_unbonding_delay: old_cluster_params.storage_unbonding_delay,
						unit_per_mb_stored: old_cluster_params.unit_per_mb_stored,
						unit_per_mb_streamed: old_cluster_params.unit_per_mb_streamed,
						unit_per_put_request: old_cluster_params.unit_per_put_request,
						unit_per_get_request: old_cluster_params.unit_per_get_request,
						customer_deposit_contract: default_customer_deposit_contract,
					})
				},
			);

			// Update storage version.
			StorageVersion::new(4).put::<Pallet<T>>();
			info!(
				target: LOG_TARGET,
				"Upgraded {} records, storage to version {:?}",
				count,
				current_version
			);

			T::DbWeight::get().reads_writes(translated + 1, translated + 1)
		} else {
			info!(target: LOG_TARGET, " >>> Unused migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV4<T>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for MigrateToV4<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v4::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
			let on_chain_version = Pallet::<T>::on_chain_storage_version();
			let current_version = Pallet::<T>::in_code_storage_version();
			let will_run = on_chain_version == 3 && current_version >= 4;
			let prev_count = if will_run {
				v3::ClustersGovParams::<T>::iter().count() as u64
			} else {
				0u64
			};
			Ok((will_run, prev_count).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
			let (will_run, prev_count): (bool, u64) = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			if will_run {
				let post_count = v4::ClustersGovParams::<T>::iter().count() as u64;
				ensure!(
					prev_count == post_count,
					"the cluster protocol params count before and after the migration should be the same"
				);

				let on_chain_version = Pallet::<T>::on_chain_storage_version();
				ensure!(
					on_chain_version == 4,
					"after migration, the on_chain_version should be 4"
				);
			}
			Ok(())
		}
	}
}

pub mod v5 {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
	pub struct Cluster<AccountId> {
		pub cluster_id: ClusterId,
		pub manager_id: AccountId,
		pub reserve_id: AccountId,
		pub props: ClusterProps<AccountId>,
		pub status: ClusterStatus,
		pub last_paid_era: EhdEra,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
	pub struct ClusterProps<AccountId> {
		pub node_provider_auth_contract: Option<AccountId>,
		pub erasure_coding_required: u32,
		pub erasure_coding_total: u32,
		pub replication_total: u32,
		pub inspection_dry_run_params: Option<InspectionDryRunParams>, // new field
	}

	#[storage_alias]
	pub(super) type Clusters<T: Config> = StorageMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		ClusterId,
		Cluster<<T as frame_system::Config>::AccountId>,
	>;

	pub fn migrate_to_v5<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		info!(
			target: LOG_TARGET,
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		// Allow bundled runtime: when in-code is already 6, we still need to run 4->5 then 5->6
		if on_chain_version == 4 && current_version >= 5 {
			let mut translated = 0u64;
			let count = v3::Clusters::<T>::iter().count();
			info!(
				target: LOG_TARGET,
				" >>> Updating DDC Cluster storage. Migrating {} clusters...", count
			);

			v5::Clusters::<T>::translate::<v3::Cluster<T::AccountId>, _>(
				|cluster_id: ClusterId, cluster: v3::Cluster<T::AccountId>| {
					info!(target: LOG_TARGET, "     Migrating cluster for cluster ID {:?}...", cluster_id);
					translated.saturating_inc();
					let props = ClusterProps {
						node_provider_auth_contract: cluster.props.node_provider_auth_contract,
						erasure_coding_required: 16,
						erasure_coding_total: 48,
						replication_total: 20,
						inspection_dry_run_params: None,
					};

					Some(v5::Cluster {
						cluster_id: cluster.cluster_id,
						manager_id: cluster.manager_id,
						reserve_id: cluster.reserve_id,
						status: cluster.status,
						last_paid_era: cluster.last_paid_era,
						props,
					})
				},
			);

			// Update storage version.
			StorageVersion::new(5).put::<Pallet<T>>();
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
	pub struct MigrateToV5<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV5<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v5::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
			let on_chain_version = Pallet::<T>::on_chain_storage_version();
			let current_version = Pallet::<T>::in_code_storage_version();
			let will_run = on_chain_version == 4 && current_version >= 5;
			let prev_count = if will_run {
				v3::Clusters::<T>::iter().count() as u64
			} else {
				0u64
			};
			Ok((will_run, prev_count).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
			let (will_run, prev_count): (bool, u64) = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			if will_run {
				let post_count = v5::Clusters::<T>::iter().count() as u64;
				ensure!(
					prev_count == post_count,
					"the cluster count before and after the migration should be the same"
				);

				let on_chain_version = Pallet::<T>::on_chain_storage_version();
				ensure!(
					on_chain_version == 5,
					"after migration, the on_chain_version should be 5"
				);
			}
			Ok(())
		}
	}
}

pub mod v6 {
	use super::*;
	use crate::BalanceOf;
	use frame_support::{pallet_prelude::*, traits::Get, weights::Weight};
	use sp_std::marker::PhantomData;
	#[cfg(feature = "try-runtime")]
	use sp_std::vec::Vec;

	#[derive(
		Clone,
		Encode,
		Decode,
		DecodeWithMemTracking,
		RuntimeDebug,
		TypeInfo,
		PartialEq,
		Default,
		Serialize,
		Deserialize,
	)]
	#[scale_info(skip_type_params(Balance, BlockNumber, T))]
	pub struct ClusterProtocolParams<Balance, BlockNumber, AccountId> {
		pub treasury_share: Perquintill,
		pub validators_share: Perquintill,
		pub cluster_reserve_share: Perquintill,
		pub storage_bond_size: Balance,
		pub storage_chill_delay: BlockNumber,
		pub storage_unbonding_delay: BlockNumber,
		pub cost_per_mb_stored: u128,
		pub cost_per_mb_streamed: u128,
		pub cost_per_put_request: u128,
		pub cost_per_get_request: u128,
		pub cost_per_gpu_unit: u128,
		pub cost_per_cpu_unit: u128,
		pub cost_per_ram_unit: u128,
		pub customer_deposit_contract: AccountId,
	}

	#[storage_alias]
	pub type ClustersGovParams<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		ClusterId,
		ClusterProtocolParams<
			BalanceOf<T>,
			BlockNumberFor<T>,
			<T as frame_system::Config>::AccountId,
		>,
	>;

	pub fn migrate_to_v6<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		info!(
			target: LOG_TARGET,
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		// Allow bundled runtime: when in-code is 6 and chain is 5, run 5->6
		if on_chain_version == 5 && current_version >= 6 {
			let mut translated = 0u64;
			let count = v4::ClustersGovParams::<T>::iter().count();
			info!(
				target: LOG_TARGET,
				" >>> Updating DDC Cluster protocol params storage. Migrating {} cluster protocol params...", count
			);

			v6::ClustersGovParams::<T>::translate::<
				v4::ClusterProtocolParams<
					BalanceOf<T>,
					BlockNumberFor<T>,
					<T as frame_system::Config>::AccountId,
				>,
				_,
			>(
				|cluster_id: ClusterId,
				 old_cluster_params: v4::ClusterProtocolParams<
					BalanceOf<T>,
					BlockNumberFor<T>,
					<T as frame_system::Config>::AccountId,
				>| {
					info!(target: LOG_TARGET, "     Migrating protocol params for cluster {:?} ...", cluster_id);
					translated.saturating_inc();

					Some(v6::ClusterProtocolParams {
						treasury_share: old_cluster_params.treasury_share,
						validators_share: old_cluster_params.validators_share,
						cluster_reserve_share: old_cluster_params.cluster_reserve_share,
						storage_bond_size: old_cluster_params.storage_bond_size,
						storage_chill_delay: old_cluster_params.storage_chill_delay,
						storage_unbonding_delay: old_cluster_params.storage_unbonding_delay,
						cost_per_mb_stored: old_cluster_params.unit_per_mb_stored,
						cost_per_mb_streamed: old_cluster_params.unit_per_mb_streamed,
						cost_per_put_request: old_cluster_params.unit_per_put_request,
						cost_per_get_request: old_cluster_params.unit_per_get_request,
						cost_per_gpu_unit: 0,
						cost_per_cpu_unit: 0,
						cost_per_ram_unit: 0,
						customer_deposit_contract: old_cluster_params.customer_deposit_contract,
					})
				},
			);

			// Update storage version.
			StorageVersion::new(6).put::<Pallet<T>>();
			info!(
				target: LOG_TARGET,
				"Upgraded {} records, storage to version {:?}",
				count,
				current_version
			);

			T::DbWeight::get().reads_writes(translated + 1, translated + 1)
		} else {
			info!(target: LOG_TARGET, " >>> Unused migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV6<T>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for MigrateToV6<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v6::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
			let on_chain_version = Pallet::<T>::on_chain_storage_version();
			let current_version = Pallet::<T>::in_code_storage_version();
			let will_run = on_chain_version == 5 && current_version >= 6;
			let prev_count = if will_run {
				v4::ClustersGovParams::<T>::iter().count() as u64
			} else {
				0u64
			};
			Ok((will_run, prev_count).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
			let (will_run, prev_count): (bool, u64) = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			if will_run {
				let post_count = v6::ClustersGovParams::<T>::iter().count() as u64;
				ensure!(
					prev_count == post_count,
					"the cluster protocol params count before and after the migration should be the same"
				);

				let on_chain_version = Pallet::<T>::on_chain_storage_version();
				ensure!(
					on_chain_version == 6,
					"after migration, the on_chain_version should be 6"
				);
			}
			Ok(())
		}
	}
}
