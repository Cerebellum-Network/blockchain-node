use log::info;
#[cfg(feature = "try-runtime")]
use polkadot_sdk::frame_support::ensure;
use polkadot_sdk::frame_support::{
	storage_alias,
	traits::{GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
	weights::Weight,
	Identity,
};
use polkadot_sdk::sp_runtime::Saturating;

use super::*;

const LOG_TARGET: &str = "runtime::ddc-clusters-gov";

pub mod v0 {
	use super::*;

	// The layout v0 stored votes in: one vote per account.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct Votes<AccountId, BlockNumber> {
		pub seats: MemberCount,
		pub threshold: MemberCount,
		pub ayes: Vec<AccountId>,
		pub nays: Vec<AccountId>,
		pub start: BlockNumber,
		pub end: BlockNumber,
	}

	#[storage_alias]
	pub type ClusterProposalVoting<T: Config> = StorageMap<
		crate::Pallet<T>,
		Identity,
		ClusterId,
		Votes<<T as polkadot_sdk::frame_system::Config>::AccountId, BlockNumberFor<T>>,
	>;
}

pub mod v1 {
	use super::*;

	/// The seat a v0 vote of `account` is kept as: the cluster manager's, or else
	/// the account's validated node in the cluster with the lowest key. None
	/// when the account holds no seat in the cluster any more.
	pub fn seat_of<T: Config>(
		cluster_id: &ClusterId,
		manager: &T::AccountId,
		account: &T::AccountId,
	) -> Option<ClusterMember> {
		if account == manager {
			return Some(ClusterMember::ClusterManager);
		}
		T::ClusterManager::get_nodes(cluster_id)
			.ok()?
			.into_iter()
			.filter(|node| {
				T::ClusterManager::get_node_state(cluster_id, node)
					.is_ok_and(|state| state.status == ClusterNodeStatus::ValidationSucceeded)
					&& T::NodeManager::get_node_provider_id(node).is_ok_and(|p| &p == account)
			})
			.min()
			.map(ClusterMember::NodeProvider)
	}

	/// Keys every open vote by seat. Each v0 vote is kept as exactly one seat of
	/// its account, so the aye and nay counts of an open ballot are the same
	/// before and after: no ballot changes outcome because of the migration. A
	/// vote whose account holds no seat in the cluster any more is dropped.
	pub fn migrate_to_v1<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		info!(
			target: LOG_TARGET,
			"Running v1 migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 0 && current_version == 1 {
			let mut translated = 0u64;
			let mut dropped = 0u64;
			crate::ClusterProposalVoting::<T>::translate::<
				v0::Votes<T::AccountId, BlockNumberFor<T>>,
				_,
			>(|cluster_id, old| {
				translated.saturating_inc();
				let manager = T::ClusterManager::get_manager_account_id(&cluster_id).ok();
				let mut keep = |accounts: Vec<T::AccountId>| {
					accounts
						.into_iter()
						.filter_map(|account| {
							let seat = manager
								.as_ref()
								.and_then(|m| seat_of::<T>(&cluster_id, m, &account));
							if seat.is_none() {
								dropped.saturating_inc();
							}
							seat.map(|member| (account, member))
						})
						.collect::<Vec<_>>()
				};
				Some(Votes {
					seats: old.seats,
					threshold: old.threshold,
					ayes: keep(old.ayes),
					nays: keep(old.nays),
					start: old.start,
					end: old.end,
				})
			});

			StorageVersion::new(1).put::<Pallet<T>>();
			info!(
				target: LOG_TARGET,
				"Upgraded {} open ballots, dropped {} votes whose account holds no seat, storage to version {:?} in v1 migration",
				translated,
				dropped,
				current_version
			);

			T::DbWeight::get().reads_writes(translated + 1, translated + 1)
		} else {
			info!(target: LOG_TARGET, " >>> Unused v1 migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV1<T>(PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v1::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, polkadot_sdk::sp_runtime::DispatchError> {
			let counts: Vec<(ClusterId, u32, u32)> = if Pallet::<T>::on_chain_storage_version() == 0
			{
				v0::ClusterProposalVoting::<T>::iter()
					.map(|(cluster_id, votes)| {
						(cluster_id, votes.ayes.len() as u32, votes.nays.len() as u32)
					})
					.collect()
			} else {
				crate::ClusterProposalVoting::<T>::iter()
					.map(|(cluster_id, votes)| {
						(cluster_id, votes.ayes.len() as u32, votes.nays.len() as u32)
					})
					.collect()
			};
			Ok(counts.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			prev_state: Vec<u8>,
		) -> Result<(), polkadot_sdk::sp_runtime::DispatchError> {
			let prev: Vec<(ClusterId, u32, u32)> = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			for (cluster_id, ayes, nays) in prev {
				let votes = crate::ClusterProposalVoting::<T>::get(cluster_id)
					.ok_or("an open ballot is missing after the v1 migration")?;
				ensure!(
					votes.ayes.len() as u32 == ayes && votes.nays.len() as u32 == nays,
					"an open ballot's aye or nay count changed in the v1 migration"
				);
			}

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			ensure!(current_version == 1, "must_upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same in v1 migration"
			);

			Ok(())
		}
	}

	#[cfg(test)]
	mod test {
		use super::*;
		use crate::mock_clusters_gov::*;
		use ddc_primitives::{ClusterNodeKind, ClusterParams, StorageNodeParams};

		fn node(key: [u8; 32], provider: [u8; 32]) -> BuiltNode {
			build_cluster_node(
				key,
				provider,
				StorageNodeParams::default(),
				CLUSTER_ID,
				ClusterNodeStatus::ValidationSucceeded,
				ClusterNodeKind::Genesis,
			)
		}

		fn cluster() -> BuiltCluster {
			build_cluster(
				CLUSTER_ID,
				CLUSTER_MANAGER_ID,
				CLUSTER_RESERVE_ID,
				ClusterParams::default(),
				crate::tests::default_cluster_protocol_params::<Test>(),
				ClusterStatus::Bonded,
			)
		}

		fn node_member(key: [u8; 32]) -> ClusterMember {
			ClusterMember::NodeProvider(NodePubKey::StoragePubKey(AccountId::from(key)))
		}

		fn open_v0_ballot(ayes: Vec<[u8; 32]>, nays: Vec<[u8; 32]>) {
			StorageVersion::new(0).put::<Pallet<Test>>();
			v0::ClusterProposalVoting::<Test>::insert(
				ClusterId::from(CLUSTER_ID),
				v0::Votes {
					seats: 4,
					threshold: 4,
					ayes: ayes.into_iter().map(AccountId::from).collect(),
					nays: nays.into_iter().map(AccountId::from).collect(),
					start: 1,
					end: 100,
				},
			);
		}

		#[test]
		fn an_open_ballot_is_keyed_by_seat_with_its_counts_unchanged() {
			let nodes = vec![
				node(NODE_PUB_KEY_2, NODE_PROVIDER_ID_1),
				node(NODE_PUB_KEY_1, NODE_PROVIDER_ID_1),
				node(NODE_PUB_KEY_3, NODE_PROVIDER_ID_3),
			];
			ExtBuilder.build_and_execute(cluster(), nodes, || {
				open_v0_ballot(
					vec![CLUSTER_MANAGER_ID, NODE_PROVIDER_ID_1],
					vec![NODE_PROVIDER_ID_3],
				);

				MigrateToV1::<Test>::on_runtime_upgrade();

				assert_eq!(StorageVersion::get::<Pallet<Test>>(), 1);
				let votes =
					crate::ClusterProposalVoting::<Test>::get(ClusterId::from(CLUSTER_ID)).unwrap();
				assert_eq!(
					votes.ayes,
					vec![
						(AccountId::from(CLUSTER_MANAGER_ID), ClusterMember::ClusterManager),
						(AccountId::from(NODE_PROVIDER_ID_1), node_member(NODE_PUB_KEY_1)),
					]
				);
				assert_eq!(
					votes.nays,
					vec![(AccountId::from(NODE_PROVIDER_ID_3), node_member(NODE_PUB_KEY_3))]
				);
				assert_eq!((votes.seats, votes.threshold, votes.start, votes.end), (4, 4, 1, 100));
			})
		}

		#[test]
		fn a_vote_whose_account_holds_no_seat_is_dropped() {
			let nodes = vec![
				node(NODE_PUB_KEY_1, NODE_PROVIDER_ID_1),
				node(NODE_PUB_KEY_2, NODE_PROVIDER_ID_2),
				node(NODE_PUB_KEY_3, NODE_PROVIDER_ID_3),
			];
			ExtBuilder.build_and_execute(cluster(), nodes, || {
				open_v0_ballot(vec![NODE_PROVIDER_ID_1, [200; 32]], vec![]);

				MigrateToV1::<Test>::on_runtime_upgrade();

				let votes =
					crate::ClusterProposalVoting::<Test>::get(ClusterId::from(CLUSTER_ID)).unwrap();
				assert_eq!(
					votes.ayes,
					vec![(AccountId::from(NODE_PROVIDER_ID_1), node_member(NODE_PUB_KEY_1))]
				);
			})
		}

		#[cfg(feature = "try-runtime")]
		#[test]
		fn the_migration_checks_pass_when_the_migration_runs_twice() {
			let nodes = vec![
				node(NODE_PUB_KEY_1, NODE_PROVIDER_ID_1),
				node(NODE_PUB_KEY_2, NODE_PROVIDER_ID_2),
				node(NODE_PUB_KEY_3, NODE_PROVIDER_ID_3),
			];
			ExtBuilder.build_and_execute(cluster(), nodes, || {
				open_v0_ballot(
					vec![CLUSTER_MANAGER_ID, NODE_PROVIDER_ID_2],
					vec![NODE_PROVIDER_ID_3],
				);

				for _ in 0..2 {
					let state = MigrateToV1::<Test>::pre_upgrade().unwrap();
					MigrateToV1::<Test>::on_runtime_upgrade();
					MigrateToV1::<Test>::post_upgrade(state).unwrap();
				}
			})
		}
	}
}
