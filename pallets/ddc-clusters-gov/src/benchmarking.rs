//! DdcClustersGov pallet benchmarking.
#![cfg(feature = "runtime-benchmarks")]

use ddc_primitives::{
	ClusterGovParams, ClusterId, ClusterNodeKind, ClusterParams, NodeParams, StorageNodeMode,
	StorageNodeParams, StorageNodePubKey,
};
use frame_benchmarking::{account, benchmarks, benchmarks_instance_pallet, whitelist_account};
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
	nodes_keys: Vec<(NodePubKey, T::AccountId)>,
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

	for (node_pub_key, node_provider) in nodes_keys.iter() {
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
	}

	if is_activated {
		T::ClusterCreator::activate_cluster(&cluster_id).expect("Could not activate cluster");
	}
}

fn next_block<T: Config>() {
	frame_system::Pallet::<T>::set_block_number(
		frame_system::Pallet::<T>::block_number() + T::BlockNumber::from(1_u32),
	);
}

fn fast_forward_to<T: Config>(n: T::BlockNumber) {
	while frame_system::Pallet::<T>::block_number() < n {
		next_block::<T>();
	}
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

fn assert_has_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_has_event(generic_event.into());
}

benchmarks! {

	propose_activate_cluster {

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);

		let node_provider_1 = create_funded_user_with_balance::<T>("provider", 0, 5);
		let node_pub_key_1 = NodePubKey::StoragePubKey(StorageNodePubKey::new([1; 32]));

		let node_provider_2 = create_funded_user_with_balance::<T>("provider", 1, 5);
		let node_pub_key_2 = NodePubKey::StoragePubKey(StorageNodePubKey::new([2; 32]));

		let node_provider_3 = create_funded_user_with_balance::<T>("provider", 2, 5);
		let node_pub_key_3 = NodePubKey::StoragePubKey(StorageNodePubKey::new([3; 32]));

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), vec![(node_pub_key_1, node_provider_1), (node_pub_key_2, node_provider_2), (node_pub_key_3, node_provider_3)], false);

	}: propose_activate_cluster(RawOrigin::Signed(cluster_manager_id), cluster_id, ClusterGovParams::default())
	verify {
		assert!(ClusterProposal::<T>::contains_key(cluster_id));
		assert!(ClusterProposalVoting::<T>::contains_key(cluster_id));
	}

	propose_update_cluster_economics {

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();
		for i in 0 .. 3 {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes, true);

	}: propose_update_cluster_economics(RawOrigin::Signed(cluster_manager_id), cluster_id, ClusterGovParams::default(), ClusterMember::ClusterManager)
	verify {
		assert!(ClusterProposal::<T>::contains_key(cluster_id));
		assert!(ClusterProposalVoting::<T>::contains_key(cluster_id));
	}

	vote_proposal {

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();
		for i in 0 .. 3 {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), true);
		next_block::<T>();

		let seats = 4;
		let threshold = seats;
		let votes = {
			let start = frame_system::Pallet::<T>::block_number();
			let end = start + T::ClusterProposalDuration::get();

			Votes { seats, threshold, ayes: vec![], nays: vec![], start, end }
		};
		let call: <T as Config>::ClusterProposalCall =
			T::ClusterProposalCall::from(Call::<T>::activate_cluster {
				cluster_id,
				cluster_gov_params: ClusterGovParams::default(),
			});
		let proposal =
			Proposal { call, author: cluster_manager_id.clone(), kind: ProposalKind::ActivateCluster };

		ClusterProposal::<T>::insert(cluster_id, proposal);
		ClusterProposalVoting::<T>::insert(cluster_id, votes);

		let (node_pub_key_1, node_provider_1) = cluster_nodes.get(0)
			.map(|(key, provider)| (key.clone(), provider.clone()))
			.unwrap();

	}: vote_proposal(RawOrigin::Signed(node_provider_1.clone()), cluster_id, true, ClusterMember::NodeProvider(node_pub_key_1.clone()))
	verify {
		let votes = ClusterProposalVoting::<T>::get(cluster_id).unwrap();
		assert_eq!(votes.ayes, vec![node_provider_1.clone()]);
	}

	close_early_approved {
		let m in 4 .. 64; // nodes range

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();

		for i in 0 .. m {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false);
		next_block::<T>();

		let seats = m + 1; // 64 nodes + 1 cluster manager at max
		let threshold = seats;
		let votes = {
			let start = frame_system::Pallet::<T>::block_number();
			let end = start + T::ClusterProposalDuration::get();

			Votes { seats, threshold, ayes: vec![], nays: vec![], start, end }
		};
		let call: <T as Config>::ClusterProposalCall =
			T::ClusterProposalCall::from(Call::<T>::activate_cluster {
				cluster_id,
				cluster_gov_params: ClusterGovParams::default(),
			});
		let proposal =
			Proposal { call, author: cluster_manager_id.clone(), kind: ProposalKind::ActivateCluster };

		ClusterProposal::<T>::insert(cluster_id, proposal);
		ClusterProposalVoting::<T>::insert(cluster_id, votes);

		for j in 0 .. m {
			let (node_pub_key, node_provider) = &cluster_nodes.get(j as usize).unwrap();
			DdcClustersGov::<T>::vote_proposal(
				RawOrigin::Signed(node_provider.clone()).into(),
				cluster_id,
				true,
				ClusterMember::NodeProvider(node_pub_key.clone()),
			)?;
		}

		DdcClustersGov::<T>::vote_proposal(
			RawOrigin::Signed(cluster_manager_id.clone()).into(),
			cluster_id,
			true,
			ClusterMember::ClusterManager,
		)?;

	}: close_proposal(RawOrigin::Signed(cluster_manager_id.clone()), cluster_id, ClusterMember::ClusterManager)
	verify {
		let cluster_id = ClusterId::from([1; 20]);
		assert!(!ClusterProposal::<T>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<T>::contains_key(cluster_id));
		assert_has_event::<T>(Event::Approved { cluster_id }.into());
		assert_last_event::<T>(Event::Removed { cluster_id }.into());
	}

	close_approved {
		let m in 4 .. 64; // nodes range

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();

		for i in 0 .. m {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false);
		next_block::<T>();

		let seats = m + 1; // 64 nodes + 1 cluster manager at max
		let threshold = seats;
		let votes = {
			let start = frame_system::Pallet::<T>::block_number();
			let end = start + T::ClusterProposalDuration::get();

			Votes { seats, threshold, ayes: vec![], nays: vec![], start, end }
		};
		let call: <T as Config>::ClusterProposalCall =
			T::ClusterProposalCall::from(Call::<T>::activate_cluster {
				cluster_id,
				cluster_gov_params: ClusterGovParams::default(),
			});
		let proposal =
			Proposal { call, author: cluster_manager_id.clone(), kind: ProposalKind::ActivateCluster };

		ClusterProposal::<T>::insert(cluster_id, proposal);
		ClusterProposalVoting::<T>::insert(cluster_id, votes.clone());

		fn is_unanimous<T: Config>() -> bool {
			let max_seats = 100;
			max_seats == T::SeatsConsensus::get_threshold(max_seats)
		}

		fn is_prime_vote<T: Config>() -> bool {
			T::DefaultVote::default_vote(Some(true), 0, 0, 0)
		}

		if is_prime_vote::<T>() && !is_unanimous::<T>() {

			DdcClustersGov::<T>::vote_proposal(
				RawOrigin::Signed(cluster_manager_id.clone()).into(),
				cluster_id,
				true,
				ClusterMember::ClusterManager,
			)?;

		} else {

			DdcClustersGov::<T>::vote_proposal(
				RawOrigin::Signed(cluster_manager_id.clone()).into(),
				cluster_id,
				true,
				ClusterMember::ClusterManager,
			)?;

			for j in 0 .. m {
				let (node_pub_key, node_provider) = &cluster_nodes.get(j as usize).unwrap();
				DdcClustersGov::<T>::vote_proposal(
					RawOrigin::Signed(node_provider.clone()).into(),
					cluster_id,
					true,
					ClusterMember::NodeProvider(node_pub_key.clone()),
				)?;
			}
		}

		fast_forward_to::<T>(votes.end + T::BlockNumber::from(1_u32));

	}: close_proposal(RawOrigin::Signed(cluster_manager_id.clone()), cluster_id, ClusterMember::ClusterManager)
	verify {
		let cluster_id = ClusterId::from([1; 20]);
		assert!(!ClusterProposal::<T>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<T>::contains_key(cluster_id));
		assert_has_event::<T>(Event::Approved { cluster_id }.into());
		assert_last_event::<T>(Event::Removed { cluster_id }.into());
	}

	close_early_disapproved {
		let m in 4 .. 64; // nodes range

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();

		for i in 0 .. m {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false);
		next_block::<T>();

		let seats = m + 1; // 64 nodes + 1 cluster manager at max
		let threshold = seats;
		let votes = {
			let start = frame_system::Pallet::<T>::block_number();
			let end = start + T::ClusterProposalDuration::get();

			Votes { seats, threshold, ayes: vec![], nays: vec![], start, end }
		};
		let call: <T as Config>::ClusterProposalCall =
			T::ClusterProposalCall::from(Call::<T>::activate_cluster {
				cluster_id,
				cluster_gov_params: ClusterGovParams::default(),
			});
		let proposal =
			Proposal { call, author: cluster_manager_id.clone(), kind: ProposalKind::ActivateCluster };

		ClusterProposal::<T>::insert(cluster_id, proposal);
		ClusterProposalVoting::<T>::insert(cluster_id, votes);

		for j in 0 .. m {
			let (node_pub_key, node_provider) = &cluster_nodes.get(j as usize).unwrap();
			DdcClustersGov::<T>::vote_proposal(
				RawOrigin::Signed(node_provider.clone()).into(),
				cluster_id,
				false,
				ClusterMember::NodeProvider(node_pub_key.clone()),
			)?;
		}

		DdcClustersGov::<T>::vote_proposal(
			RawOrigin::Signed(cluster_manager_id.clone()).into(),
			cluster_id,
			false,
			ClusterMember::ClusterManager,
		)?;

	}: close_proposal(RawOrigin::Signed(cluster_manager_id.clone()), cluster_id, ClusterMember::ClusterManager)
	verify {
		let cluster_id = ClusterId::from([1; 20]);
		assert!(!ClusterProposal::<T>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<T>::contains_key(cluster_id));
		assert_has_event::<T>(Event::Disapproved { cluster_id }.into());
		assert_last_event::<T>(Event::Removed { cluster_id }.into());
	}

	close_disapproved {
		let m in 4 .. 64; // nodes range

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();

		for i in 0 .. m {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false);
		next_block::<T>();

		let seats = m + 1; // 64 nodes + 1 cluster manager at max
		let threshold = seats;
		let votes = {
			let start = frame_system::Pallet::<T>::block_number();
			let end = start + T::ClusterProposalDuration::get();

			Votes { seats, threshold, ayes: vec![], nays: vec![], start, end }
		};
		let call: <T as Config>::ClusterProposalCall =
			T::ClusterProposalCall::from(Call::<T>::activate_cluster {
				cluster_id,
				cluster_gov_params: ClusterGovParams::default(),
			});
		let proposal =
			Proposal { call, author: cluster_manager_id.clone(), kind: ProposalKind::ActivateCluster };

		ClusterProposal::<T>::insert(cluster_id, proposal);
		ClusterProposalVoting::<T>::insert(cluster_id, votes.clone());

		fast_forward_to::<T>(votes.end + T::BlockNumber::from(1_u32));

	}: close_proposal(RawOrigin::Signed(cluster_manager_id.clone()), cluster_id, ClusterMember::ClusterManager)
	verify {
		let cluster_id = ClusterId::from([1; 20]);
		assert!(!ClusterProposal::<T>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<T>::contains_key(cluster_id));
		assert_has_event::<T>(Event::Disapproved { cluster_id }.into());
		assert_last_event::<T>(Event::Removed { cluster_id }.into());
	}

}
