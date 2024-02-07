pub mod v1 {
	#[cfg(feature = "try-runtime")]
	use ddc_primitives::ClusterStatus;
	use ddc_primitives::{
		traits::{ClusterCreator, ClusterEconomics, ClusterQuery, NodeVisitor},
		ClusterId, NodePubKey,
	};
	use frame_support::{
		log,
		pallet_prelude::*,
		traits::{Currency, LockableCurrency, OnRuntimeUpgrade, WithdrawReasons},
		weights::Weight,
	};
	#[cfg(feature = "try-runtime")]
	use hex_literal::hex;
	use sp_runtime::Saturating;
	use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

	use crate::{
		ClusterBonded, ClusterLedger, Config, Nodes, Pallet, StakingLedger, DDC_CLUSTER_STAKING_ID,
		LOG_TARGET,
	};

	#[cfg(feature = "try-runtime")]
	const DEVNET_CLUSTER: [u8; 20] = hex!("7f82864e4f097e63d04cc279e4d8d2eb45a42ffa");
	#[cfg(feature = "try-runtime")]
	const TESTNET_CLUSTER: [u8; 20] = hex!("825c4b2352850de9986d9d28568db6f0c023a1e3");
	#[cfg(feature = "try-runtime")]
	const MAINNET_CLUSTER: [u8; 20] = hex!("0059f5ada35eee46802d80750d5ca4a490640511");
	#[cfg(feature = "try-runtime")]
	const KNOWN_ACTIVE_CLUSTERS: [[u8; 20]; 3] = [DEVNET_CLUSTER, TESTNET_CLUSTER, MAINNET_CLUSTER];

	pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			let current_version = Pallet::<T>::current_storage_version();
			let onchain_version = Pallet::<T>::on_chain_storage_version();
			let mut weight = T::DbWeight::get().reads(1);

			if onchain_version == 0 && current_version == 1 {
				let cluster_bonding_amount = T::ClusterBondingAmount::get();
				let minimum_balance = T::Currency::minimum_balance();
				weight.saturating_accrue(T::DbWeight::get().reads(1));
				if cluster_bonding_amount < minimum_balance {
					return weight
				}

				let mut bonded_nodes: Vec<NodePubKey> = Vec::new();
				let mut bonded_nodes_count = 0u64;
				Nodes::<T>::iter().for_each(|(node_pub_key, _)| {
					bonded_nodes_count.saturating_inc();
					bonded_nodes.push(node_pub_key);
				});
				weight.saturating_accrue(T::DbWeight::get().reads(bonded_nodes_count));

				let mut served_clusters: BTreeMap<ClusterId, ()> = BTreeMap::new();
				for node_pub_key in bonded_nodes.iter() {
					if let Ok(Some(cluster_id)) = T::NodeVisitor::get_cluster_id(node_pub_key) {
						served_clusters.insert(cluster_id, ());
					}
					weight.saturating_accrue(T::DbWeight::get().reads(1));
				}

				for (cluster_id, _) in served_clusters.iter() {
					if let Ok((cluster_controller, cluster_stash)) =
						<T::ClusterEconomics as ClusterQuery<T>>::get_manager_and_reserve_id(
							&cluster_id,
						) {
						let cluster_stash_balance = T::Currency::free_balance(&cluster_stash);
						weight.saturating_accrue(T::DbWeight::get().reads(1));

						if cluster_stash_balance >= cluster_bonding_amount {
							if let Ok(_) = T::ClusterEconomics::bond_cluster(&cluster_id) {
								weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
							} else {
								weight.saturating_accrue(T::DbWeight::get().reads(1));
								continue
							}

							if let Ok(_) = frame_system::Pallet::<T>::inc_consumers(&cluster_stash)
							{
								weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
							} else {
								weight.saturating_accrue(T::DbWeight::get().reads(1));
								continue
							}

							<ClusterBonded<T>>::insert(&cluster_stash, &cluster_controller);
							weight.saturating_accrue(T::DbWeight::get().writes(1));

							let ledger = StakingLedger {
								stash: cluster_stash,
								total: cluster_bonding_amount,
								active: cluster_bonding_amount,
								chilling: Default::default(),
								unlocking: Default::default(),
							};

							T::Currency::set_lock(
								DDC_CLUSTER_STAKING_ID,
								&ledger.stash,
								ledger.total,
								WithdrawReasons::all(),
							);
							weight.saturating_accrue(T::DbWeight::get().writes(1));

							<ClusterLedger<T>>::insert(cluster_controller, ledger);
							weight.saturating_accrue(T::DbWeight::get().writes(1));

							if let Ok(_) = T::ClusterCreator::activate_cluster(&cluster_id) {
								weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
							} else {
								weight.saturating_accrue(T::DbWeight::get().reads(1));
								continue
							}
						}
					}
					weight.saturating_accrue(T::DbWeight::get().reads(1));
				}

				weight
			} else {
				log::info!(
					target: LOG_TARGET,
					"Migration did not execute. This probably should be removed"
				);

				weight
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
			frame_support::ensure!(
				Pallet::<T>::on_chain_storage_version() == 0,
				"must upgrade linearly"
			);
			let pre_clusters_bonded_count = ClusterBonded::<T>::iter().count();
			let pre_clusters_ledgers_count = ClusterLedger::<T>::iter().count();

			assert_eq!(
				pre_clusters_bonded_count, 0,
				"clusters bonds should be empty before the migration"
			);

			assert_eq!(
				pre_clusters_ledgers_count, 0,
				"clusters ledgers should be empty before the migration"
			);

			let mut clusters_to_activate: Vec<ClusterId> = Vec::new();
			for bytes_id in KNOWN_ACTIVE_CLUSTERS.iter() {
				let cluster_id = ClusterId::from(bytes_id);
				if <T::ClusterEconomics as ClusterQuery<T>>::cluster_exists(&cluster_id) {
					clusters_to_activate.push(cluster_id);
				}
			}

			Ok(clusters_to_activate.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
			let clusters_to_activate: Vec<ClusterId> = Decode::decode(&mut &state[..]).expect(
				"the state parameter should be something that was generated by pre_upgrade",
			);

			for cluster_id in clusters_to_activate.iter() {
				let (cluster_controller, cluster_stash) =
					<T::ClusterEconomics as ClusterQuery<T>>::get_manager_and_reserve_id(
						&cluster_id,
					)
					.expect("no controller and stash accounts found for activating cluster");

				assert_eq!(
					<ClusterBonded<T>>::get(cluster_stash)
						.expect("binding is not created for activating cluster"),
					cluster_controller
				);

				let ledger = <ClusterLedger<T>>::get(cluster_controller)
					.expect("staking ledger is not created for activating cluster");

				let bonding_amount = T::ClusterBondingAmount::get();
				assert_eq!(ledger.total, bonding_amount);
				assert_eq!(ledger.active, bonding_amount);

				let cluster_status =
					<T::ClusterEconomics as ClusterQuery<T>>::get_cluster_status(&cluster_id)
						.expect("no activating cluster found");

				assert_eq!(cluster_status, ClusterStatus::Activated);
			}

			Ok(())
		}
	}
}
