//! DdcClustersGov pallet benchmarking.
#![cfg(feature = "runtime-benchmarks")]

use ddc_primitives::{
	ClusterGovParams, ClusterId, ClusterNodeKind, ClusterParams, NodeParams, StorageNodeMode,
	StorageNodeParams, StorageNodePubKey,
};
use frame_benchmarking::{account, benchmarks, whitelist_account};
use frame_system::RawOrigin;
use sp_runtime::Perquintill;
use sp_std::prelude::*;

use super::*;
use crate::Pallet as DdcClustersGov;

/// Grab a funded user with max Balance.
pub fn create_funded_user_with_balance<T: Config>(
	name: &'static str,
	n: u32,
	balance_factor: u128,
) -> T::AccountId {
	let user = account(name, n, 0);
	let balance = <T as pallet::Config>::Currency::minimum_balance() *
		balance_factor.saturated_into::<BalanceOf<T>>();
	let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
	user
}

pub fn create_cluster_with_nodes<T: Config>(
	cluster_id: ClusterId,
	cluster_manager_id: T::AccountId,
	cluster_reserve_id: T::AccountId,
	nodes_keys: Vec<NodePubKey>,
	is_activated: bool,
) {
	let bond_size: BalanceOf<T> = 10000_u32.saturated_into::<BalanceOf<T>>();
	let cluster_gov_params = ClusterGovParams {
		treasury_share: Perquintill::from_percent(10),
		validators_share: Perquintill::from_percent(20),
		cluster_reserve_share: Perquintill::from_percent(30),
		storage_bond_size: bond_size,
		storage_chill_delay: T::BlockNumber::from(20_u32),
		storage_unbonding_delay: T::BlockNumber::from(20_u32),
		unit_per_mb_stored: 97656,
		unit_per_mb_streamed: 48828,
		unit_per_put_request: 10,
		unit_per_get_request: 5,
	};

	let cluster_params = ClusterParams { node_provider_auth_contract: None };

	T::ClusterCreator::create_cluster(
		cluster_id,
		cluster_manager_id.clone(),
		cluster_reserve_id.clone(),
		cluster_params,
		cluster_gov_params.clone(),
	)
	.expect("Cluster is not created");

	T::StakerCreator::bond_cluster(
		cluster_reserve_id.clone(),
		cluster_manager_id.clone(),
		cluster_id,
	)
	.expect("Cluster could not be bonded");

	let mut i = 0;
	for node_pub_key in nodes_keys.iter() {
		let node_provider = create_funded_user_with_balance::<T>("provider", i, 5);
		let node_params = NodeParams::StorageParams(StorageNodeParams {
			mode: StorageNodeMode::Storage,
			host: vec![1u8; 255],
			domain: vec![2u8; 255],
			ssl: true,
			http_port: 8080_u16,
			grpc_port: 9090_u16,
			p2p_port: 9070_u16,
		});

		T::NodeCreator::create_node(node_pub_key.clone(), node_provider.clone(), node_params)
			.expect("Node is not created");

		T::StakerCreator::bond_stake_and_participate(
			node_provider.clone(),
			node_provider.clone(),
			node_pub_key.clone(),
			bond_size.into(),
			cluster_id,
		)
		.expect("Staking is not created");

		T::ClusterManager::add_node(&cluster_id, &node_pub_key, &ClusterNodeKind::Genesis)
			.expect("Node could not be added to the cluster");

		T::ClusterManager::validate_node(&cluster_id, &node_pub_key, true)
			.expect("Node could not be validated in the cluster");

		i = i + 1;
	}

	if is_activated {
		T::ClusterCreator::activate_cluster(&cluster_id);
	}
}

benchmarks! {
	propose_activate_cluster {

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);

		let node_pub_key_1 = NodePubKey::StoragePubKey(StorageNodePubKey::new([1; 32]));
		let node_pub_key_2 = NodePubKey::StoragePubKey(StorageNodePubKey::new([2; 32]));
		let node_pub_key_3 = NodePubKey::StoragePubKey(StorageNodePubKey::new([3; 32]));

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), vec![node_pub_key_1, node_pub_key_2, node_pub_key_3], false);

	}: propose_activate_cluster(RawOrigin::Signed(cluster_manager_id), cluster_id, ClusterGovParams::default())
	verify {
		assert!(ClusterProposal::<T>::contains_key(cluster_id));
		assert!(ClusterProposalVoting::<T>::contains_key(cluster_id));
	}

	propose_update_cluster_economics {

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);

		let node_pub_key_1 = NodePubKey::StoragePubKey(StorageNodePubKey::new([1; 32]));
		let node_pub_key_2 = NodePubKey::StoragePubKey(StorageNodePubKey::new([2; 32]));
		let node_pub_key_3 = NodePubKey::StoragePubKey(StorageNodePubKey::new([3; 32]));

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), vec![node_pub_key_1, node_pub_key_2, node_pub_key_3], true);

	}: propose_update_cluster_economics(RawOrigin::Signed(cluster_manager_id), cluster_id, ClusterGovParams::default(), ClusterMember::ClusterManager)
	verify {
		assert!(ClusterProposal::<T>::contains_key(cluster_id));
		assert!(ClusterProposalVoting::<T>::contains_key(cluster_id));
	}
}
