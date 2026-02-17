//! DdcClustersGov pallet benchmarking.
#![allow(clippy::extra_unused_type_parameters)]

use ddc_primitives::{
	ClusterBondingParams, ClusterId, ClusterNodeKind, ClusterParams, ClusterProtocolParams,
	NodeParams, StorageNodeMode, StorageNodeParams, StorageNodePubKey,
};
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use pallet_referenda::Pallet as Referenda;
use sp_runtime::{Perquintill, SaturatedConversion};
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
	let balance = <T as pallet::Config>::Currency::minimum_balance()
		* balance_factor.saturated_into::<BalanceOf<T>>();
	let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
	user
}

pub fn fund_user<T: Config>(user: T::AccountId, balance_factor: u128) -> T::AccountId {
	let balance = <T as pallet::Config>::Currency::minimum_balance()
		* balance_factor.saturated_into::<BalanceOf<T>>();
	let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
	user
}

pub fn create_account<T: Config>(name: &'static str) -> T::AccountId {
	account(name, 0, 0)
}

pub fn create_cluster_with_nodes<T: Config>(
	cluster_id: ClusterId,
	cluster_manager_id: T::AccountId,
	cluster_reserve_id: T::AccountId,
	nodes_keys: Vec<(NodePubKey, T::AccountId)>,
	is_activated: bool,
	customer_deposit_contract: T::AccountId,
) {
	let bond_size: BalanceOf<T> = 10000_u32.saturated_into::<BalanceOf<T>>();
	let cluster_protocol_params = ClusterProtocolParams {
		treasury_share: Perquintill::from_percent(10),
		validators_share: Perquintill::from_percent(20),
		cluster_reserve_share: Perquintill::from_percent(30),
		storage_bond_size: bond_size,
		storage_chill_delay: BlockNumberFor::<T>::from(20_u32),
		storage_unbonding_delay: BlockNumberFor::<T>::from(20_u32),
		cost_per_mb_stored: 97656,
		cost_per_mb_streamed: 48828,
		cost_per_put_request: 10,
		cost_per_get_request: 5,
		cost_per_gpu_unit: 0,
		cost_per_cpu_unit: 0,
		cost_per_ram_unit: 0,
		customer_deposit_contract,
	};

	let cluster_params = ClusterParams {
		node_provider_auth_contract: None,
		erasure_coding_required: 0,
		erasure_coding_total: 0,
		replication_total: 0,
		inspection_dry_run_params: None,
	};

	T::ClusterCreator::create_cluster(
		cluster_id,
		cluster_manager_id.clone(),
		cluster_reserve_id.clone(),
		cluster_params,
		cluster_protocol_params.clone(),
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

		T::NodeManager::create_node(node_pub_key.clone(), node_provider.clone(), node_params)
			.expect("Node is not created");

		T::StakerCreator::bond_stake_and_participate(
			node_provider.clone(),
			node_provider.clone(),
			node_pub_key.clone(),
			bond_size,
			cluster_id,
		)
		.expect("Staking is not created");

		T::ClusterManager::add_node(&cluster_id, node_pub_key, &ClusterNodeKind::Genesis)
			.expect("Node could not be added to the cluster");

		T::ClusterManager::validate_node(&cluster_id, node_pub_key, true)
			.expect("Node could not be validated in the cluster");
	}

	if is_activated {
		T::ClusterProtocol::activate_cluster_protocol(&cluster_id)
			.expect("Could not activate cluster");
	}
}

fn next_block<T: Config>() {
	frame_system::Pallet::<T>::set_block_number(
		frame_system::Pallet::<T>::block_number() + BlockNumberFor::<T>::from(1_u32),
	);
}

fn fast_forward_to<T: Config>(n: BlockNumberFor<T>) {
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

fn default_cluster_protocol_params<T: Config>(
) -> ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>, T::AccountId> {
	let customer_deposit_contract = create_account::<T>("customer-deposit-contract");
	ClusterProtocolParams {
		customer_deposit_contract,
		treasury_share: Default::default(),
		validators_share: Default::default(),
		cluster_reserve_share: Default::default(),
		storage_bond_size: Default::default(),
		storage_chill_delay: Default::default(),
		storage_unbonding_delay: Default::default(),
		cost_per_mb_stored: Default::default(),
		cost_per_mb_streamed: Default::default(),
		cost_per_put_request: Default::default(),
		cost_per_get_request: Default::default(),
		cost_per_gpu_unit: 0,
		cost_per_cpu_unit: 0,
		cost_per_ram_unit: 0,
	}
}

benchmarks! {

	propose_activate_cluster_protocol {

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);
		let customer_deposit_contract = create_account::<T>("customer-deposit-contract");

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();
		for i in 0 .. 3 {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false, customer_deposit_contract.clone());

	}: propose_activate_cluster_protocol(RawOrigin::Signed(cluster_manager_id), cluster_id, default_cluster_protocol_params::<T>())
	verify {
		assert!(ClusterProposal::<T>::contains_key(cluster_id));
		assert!(ClusterProposalVoting::<T>::contains_key(cluster_id));
	}

	propose_update_cluster_protocol {

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);
		let customer_deposit_contract = create_account::<T>("customer-deposit-contract");

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();
		for i in 0 .. 3 {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes, true, customer_deposit_contract.clone());

	}: propose_update_cluster_protocol(RawOrigin::Signed(cluster_manager_id), cluster_id, default_cluster_protocol_params::<T>(), ClusterMember::ClusterManager)
	verify {
		assert!(ClusterProposal::<T>::contains_key(cluster_id));
		assert!(ClusterProposalVoting::<T>::contains_key(cluster_id));
	}

	vote_proposal {

		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);
		let customer_deposit_contract = create_account::<T>("customer-deposit-contract");

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();
		for i in 0 .. 3 {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false, customer_deposit_contract.clone());
		next_block::<T>();

		DdcClustersGov::<T>::propose_activate_cluster_protocol(RawOrigin::Signed(cluster_manager_id.clone()).into(), cluster_id, default_cluster_protocol_params::<T>())?;

		let (node_pub_key_1, node_provider_1) = cluster_nodes.first()
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
		let customer_deposit_contract = create_account::<T>("customer-deposit-contract");
		let _ = fund_user::<T>(DdcClustersGov::<T>::account_id(), 1000);

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();

		for i in 0 .. m {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false, customer_deposit_contract.clone());
		next_block::<T>();

		DdcClustersGov::<T>::propose_activate_cluster_protocol(RawOrigin::Signed(cluster_manager_id.clone()).into(), cluster_id, default_cluster_protocol_params::<T>())?;

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
		let customer_deposit_contract = create_account::<T>("customer-deposit-contract");
		let _ = fund_user::<T>(DdcClustersGov::<T>::account_id(), 1000);

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();

		for i in 0 .. m {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false, customer_deposit_contract.clone());
		next_block::<T>();

		DdcClustersGov::<T>::propose_activate_cluster_protocol(RawOrigin::Signed(cluster_manager_id.clone()).into(), cluster_id, default_cluster_protocol_params::<T>())?;

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

		let votes = ClusterProposalVoting::<T>::get(cluster_id).unwrap();
		fast_forward_to::<T>(votes.end + BlockNumberFor::<T>::from(1_u32));

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
		let customer_deposit_contract = create_account::<T>("customer-deposit-contract");

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();

		for i in 0 .. m {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false, customer_deposit_contract.clone());
		next_block::<T>();

		DdcClustersGov::<T>::propose_activate_cluster_protocol(RawOrigin::Signed(cluster_manager_id.clone()).into(), cluster_id, default_cluster_protocol_params::<T>())?;

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
		let customer_deposit_contract = create_account::<T>("customer-deposit-contract");

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();

		for i in 0 .. m {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false, customer_deposit_contract.clone());
		next_block::<T>();

		DdcClustersGov::<T>::propose_activate_cluster_protocol(RawOrigin::Signed(cluster_manager_id.clone()).into(), cluster_id, default_cluster_protocol_params::<T>())?;

		let votes = ClusterProposalVoting::<T>::get(cluster_id).unwrap();
		fast_forward_to::<T>(votes.end + BlockNumberFor::<T>::from(1_u32));

	}: close_proposal(RawOrigin::Signed(cluster_manager_id.clone()), cluster_id, ClusterMember::ClusterManager)
	verify {
		let cluster_id = ClusterId::from([1; 20]);
		assert!(!ClusterProposal::<T>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<T>::contains_key(cluster_id));
		assert_has_event::<T>(Event::Disapproved { cluster_id }.into());
	}

	retract_proposal {
		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);
		let customer_deposit_contract = create_account::<T>("customer-deposit-contract");

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();
		for i in 0 .. 3 {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false, customer_deposit_contract.clone());
		next_block::<T>();

		DdcClustersGov::<T>::propose_activate_cluster_protocol(RawOrigin::Signed(cluster_manager_id.clone()).into(), cluster_id, default_cluster_protocol_params::<T>())?;

		assert!(ClusterProposal::<T>::contains_key(cluster_id));
		assert!(ClusterProposalVoting::<T>::contains_key(cluster_id));

	}: retract_proposal(RawOrigin::Signed(cluster_manager_id.clone()), cluster_id)
	verify {
		let cluster_id = ClusterId::from([1; 20]);
		assert!(!ClusterProposal::<T>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<T>::contains_key(cluster_id));
		assert_last_event::<T>(Event::Removed { cluster_id }.into());
	}

	refund_submission_deposit {
		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);
		let customer_deposit_contract = create_account::<T>("customer-deposit-contract");
		let _ = fund_user::<T>(DdcClustersGov::<T>::account_id(), 1000);

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();
		for i in 0 .. 3 {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false, customer_deposit_contract.clone());
		next_block::<T>();

		DdcClustersGov::<T>::propose_activate_cluster_protocol(RawOrigin::Signed(cluster_manager_id.clone()).into(), cluster_id, default_cluster_protocol_params::<T>())?;

		for i in 0 .. 3 {
			let (node_pub_key, node_provider) = &cluster_nodes.get(i as usize).unwrap();
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

		DdcClustersGov::<T>::close_proposal(RawOrigin::Signed(cluster_manager_id.clone()).into(), cluster_id, ClusterMember::ClusterManager).expect("Could not close proposal");

		let referenda_index = pallet_referenda::ReferendumCount::<T>::get() - 1;

		assert!(SubmissionDeposits::<T>::contains_key(referenda_index));

		Referenda::<T>::place_decision_deposit(RawOrigin::Signed(cluster_manager_id.clone()).into(), referenda_index).expect("Could not place decision deposit");
		Referenda::<T>::cancel(RawOrigin::Root.into(), referenda_index).expect("Could not cancel referendum");

	}: refund_submission_deposit(RawOrigin::Signed(cluster_manager_id.clone()), referenda_index)
	verify {
		assert!(!SubmissionDeposits::<T>::contains_key(referenda_index));
	}

	activate_cluster_protocol {
		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);
		let customer_deposit_contract = create_account::<T>("customer-deposit-contract");

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();
		for i in 0 .. 3 {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), false, customer_deposit_contract.clone());
		next_block::<T>();

	}: activate_cluster_protocol(RawOrigin::Root, cluster_id, default_cluster_protocol_params::<T>())
	verify {
		let cluster_id = ClusterId::from([1; 20]);
		let cluster_status = <T::ClusterProtocol as ClusterQuery<T::AccountId>>::get_cluster_status(&cluster_id).unwrap();
		assert_eq!(cluster_status, ClusterStatus::Activated);
	}

	update_cluster_protocol {
		let cluster_id = ClusterId::from([1; 20]);
		let cluster_manager_id = create_funded_user_with_balance::<T>("cluster-controller", 0, 5);
		let cluster_reserve_id = create_funded_user_with_balance::<T>("cluster-stash", 0, 5);
		let customer_deposit_contract = create_account::<T>("customer-deposit-contract");

		let mut cluster_nodes: Vec<(NodePubKey, T::AccountId)> = Vec::new();
		for i in 0 .. 3 {
			let node_provider = create_funded_user_with_balance::<T>("node-provider", i, 5);
			let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([i as u8; 32]));
			cluster_nodes.push((node_pub_key.clone(), node_provider.clone()));
		}

		create_cluster_with_nodes::<T>(cluster_id, cluster_manager_id.clone(), cluster_reserve_id.clone(), cluster_nodes.clone(), true, customer_deposit_contract.clone());
		next_block::<T>();

	}: update_cluster_protocol(RawOrigin::Root, cluster_id, ClusterProtocolParams {
		treasury_share: Perquintill::from_percent(5),
		validators_share: Perquintill::from_percent(10),
		cluster_reserve_share: Perquintill::from_percent(15),
		storage_bond_size: 10000_u128.saturated_into::<BalanceOf<T>>(),
		storage_chill_delay: BlockNumberFor::<T>::from(20_u32),
		storage_unbonding_delay: BlockNumberFor::<T>::from(20_u32),
		cost_per_mb_stored: 97656,
		cost_per_mb_streamed: 48828,
		cost_per_put_request: 10,
		cost_per_get_request: 5,
		cost_per_gpu_unit: 0,
		cost_per_cpu_unit: 0,
		cost_per_ram_unit: 0,
		customer_deposit_contract,
	})
	verify {
		let cluster_id = ClusterId::from([1; 20]);
		let bonding_params = T::ClusterProtocol::get_bonding_params(&cluster_id).unwrap();
		let updated_bonding = ClusterBondingParams {
			storage_bond_size: 10000_u128,
			storage_chill_delay: BlockNumberFor::<T>::from(20_u32),
			storage_unbonding_delay: BlockNumberFor::<T>::from(20_u32)
		};
		assert_eq!(bonding_params, updated_bonding);
	}

}
