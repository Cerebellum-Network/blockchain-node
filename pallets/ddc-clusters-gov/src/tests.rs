//! Tests for the module.

use ddc_primitives::{ClusterGovParams, ClusterNodeKind, ClusterParams, StorageNodeParams};
use frame_support::{assert_noop, assert_ok};
use pallet_conviction_voting::{AccountVote, Conviction, Vote};
use pallet_ddc_clusters::Event::{ClusterActivated, ClusterGovParamsSet};
use pallet_referenda::ReferendumInfo;
use sp_runtime::Perquintill;

use super::{mock::*, *};
use crate::SubmissionDeposit as ReferendaSubmissionDeposit;

fn next_block() {
	System::set_block_number(System::block_number() + 1);
	Scheduler::on_initialize(System::block_number());
	Referenda::on_initialize(System::block_number());
}

fn fast_forward_to(n: u64) {
	while System::block_number() < n {
		next_block();
	}
}

#[test]
fn cluster_activation_proposal_initiated() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let not_cluster_manager = AccountId::from([0; 32]);
		let cluster_gov_params = ClusterGovParams::default();
		assert_noop!(
			DdcClustersGov::propose_activate_cluster(
				RuntimeOrigin::signed(not_cluster_manager.clone()),
				cluster_id,
				cluster_gov_params.clone()
			),
			Error::<Test>::NotClusterManager
		);

		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let not_cluster_id = ClusterId::from([0; 20]);
		assert_noop!(
			DdcClustersGov::propose_activate_cluster(
				RuntimeOrigin::signed(cluster_manager.clone()),
				not_cluster_id,
				cluster_gov_params.clone()
			),
			Error::<Test>::NoCluster
		);

		assert_ok!(DdcClustersGov::propose_activate_cluster(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone()
		));

		let proposal = ClusterProposal::<Test>::get(cluster_id);
		assert_eq!(
			proposal,
			Some(Proposal {
				author: cluster_manager.clone(),
				kind: ProposalKind::ActivateCluster,
				call: <Test as pallet::Config>::ClusterProposalCall::from(
					Call::<Test>::activate_cluster {
						cluster_id,
						cluster_gov_params: cluster_gov_params.clone(),
					}
				)
			})
		);

		let votes = ClusterProposalVoting::<Test>::get(cluster_id);
		let start = BlockNumber::from(1_u64);
		let end = start + <Test as pallet::Config>::ClusterProposalDuration::get();
		let threshold = 4; // 3 nodes + 1 cluster manager
		assert_eq!(votes, Some(Votes { threshold, ayes: vec![], nays: vec![], start, end }));
		System::assert_last_event(
			Event::Proposed { account: cluster_manager, cluster_id, threshold }.into(),
		)
	})
}

#[test]
fn cluster_activation_proposal_fails_on_unexpected_state() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Unbonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		assert_noop!(
			DdcClustersGov::propose_activate_cluster(
				RuntimeOrigin::signed(cluster_manager.clone()),
				cluster_id,
				cluster_gov_params.clone()
			),
			Error::<Test>::UnexpectedState
		);
	})
}

#[test]
fn cluster_activation_proposal_fails_if_there_are_not_enough_validated_nodes() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		assert_noop!(
			DdcClustersGov::propose_activate_cluster(
				RuntimeOrigin::signed(cluster_manager.clone()),
				cluster_id,
				cluster_gov_params.clone()
			),
			Error::<Test>::NotEnoughValidatedNodes
		);
	})
}

#[test]
fn cluster_activation_proposal_fails_if_there_is_active_proposal() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		assert_ok!(DdcClustersGov::propose_activate_cluster(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone()
		));

		assert_noop!(
			DdcClustersGov::propose_activate_cluster(
				RuntimeOrigin::signed(cluster_manager.clone()),
				cluster_id,
				cluster_gov_params.clone()
			),
			Error::<Test>::ActiveProposal
		);
	})
}

#[test]
fn cluster_activation_is_restricted_for_system_origins() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	ExtBuilder.build_and_execute(cluster, vec![], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		assert_noop!(
			DdcClustersGov::activate_cluster(
				RuntimeOrigin::signed(cluster_manager.clone()),
				cluster_id,
				cluster_gov_params.clone()
			),
			DispatchError::BadOrigin
		);

		assert_noop!(
			DdcClustersGov::activate_cluster(
				RuntimeOrigin::root(),
				cluster_id,
				cluster_gov_params.clone()
			),
			DispatchError::BadOrigin
		);
	})
}

#[test]
fn cluster_activation_is_allowed_for_referenda_activator_track_origin() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	ExtBuilder.build_and_execute(cluster, vec![], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_gov_params = ClusterGovParams {
			treasury_share: Perquintill::from_float(5.0),
			validators_share: Perquintill::from_float(10.0),
			cluster_reserve_share: Perquintill::from_float(15.0),
			storage_bond_size: 10 * CERE,
			storage_chill_delay: 20,
			storage_unbonding_delay: 20,
			unit_per_mb_stored: 97656,
			unit_per_mb_streamed: 48828,
			unit_per_put_request: 10,
			unit_per_get_request: 5,
		};
		let open_gov_activator = <Test as pallet::Config>::OpenGovActivatorTrackOrigin::get();
		assert_ok!(DdcClustersGov::activate_cluster(
			open_gov_activator,
			cluster_id,
			cluster_gov_params.clone()
		));

		let cluster = pallet_ddc_clusters::Clusters::<Test>::get(cluster_id).unwrap();
		assert_eq!(cluster.status, ClusterStatus::Activated);

		let updated_cluster_gov_params =
			pallet_ddc_clusters::ClustersGovParams::<Test>::get(cluster_id).unwrap();
		assert_eq!(cluster_gov_params, updated_cluster_gov_params);
		assert_eq!(System::events().len(), 2);
		System::assert_has_event(ClusterActivated { cluster_id }.into());
		System::assert_last_event(ClusterGovParamsSet { cluster_id }.into());
	})
}

#[test]
fn cluster_activation_proposal_can_be_retracted_by_its_author() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();
		let not_cluster_id = ClusterId::from([0; 20]);

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);

		assert_ok!(DdcClustersGov::propose_activate_cluster(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone()
		));

		assert!(ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(ClusterProposalVoting::<Test>::contains_key(cluster_id));

		assert_noop!(
			DdcClustersGov::retract_proposal(
				RuntimeOrigin::signed(cluster_manager.clone()),
				not_cluster_id,
			),
			Error::<Test>::ProposalMissing
		);

		assert_noop!(
			DdcClustersGov::retract_proposal(
				RuntimeOrigin::signed(cluster_node_1_provider.clone()),
				cluster_id,
			),
			Error::<Test>::NotProposalAuthor
		);

		assert_ok!(DdcClustersGov::retract_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
		));

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));
		System::assert_last_event(Event::Removed { cluster_id }.into());
	})
}

#[test]
fn cluster_activation_proposal_cannot_be_initated_for_active_cluster() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();
		assert_noop!(
			DdcClustersGov::propose_activate_cluster(
				RuntimeOrigin::signed(cluster_manager.clone()),
				cluster_id,
				cluster_gov_params.clone()
			),
			Error::<Test>::UnexpectedState
		);
	})
}

#[test]
fn cluster_economics_update_proposal_initiated() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));
		let cluster_gov_params = ClusterGovParams {
			treasury_share: Perquintill::from_float(5.0),
			validators_share: Perquintill::from_float(10.0),
			cluster_reserve_share: Perquintill::from_float(15.0),
			storage_bond_size: 10 * CERE,
			storage_chill_delay: 20,
			storage_unbonding_delay: 20,
			unit_per_mb_stored: 97656,
			unit_per_mb_streamed: 48828,
			unit_per_put_request: 10,
			unit_per_get_request: 5,
		};

		let not_cluster_member = AccountId::from([0; 32]);
		let not_cluster_node_key =
			NodePubKey::StoragePubKey(AccountId::from(AccountId::from([128; 32])));
		assert_noop!(
			DdcClustersGov::propose_update_cluster_economics(
				RuntimeOrigin::signed(not_cluster_member.clone()),
				cluster_id,
				cluster_gov_params.clone(),
				ClusterMember::NodeProvider(not_cluster_node_key.clone())
			),
			Error::<Test>::NotValidatedNode
		);

		let not_cluster_id = ClusterId::from([0; 20]);
		assert_noop!(
			DdcClustersGov::propose_update_cluster_economics(
				RuntimeOrigin::signed(cluster_node_1_provider.clone()),
				not_cluster_id,
				cluster_gov_params.clone(),
				ClusterMember::NodeProvider(cluster_node_1_key.clone())
			),
			Error::<Test>::NotValidatedNode
		);

		assert_ok!(DdcClustersGov::propose_update_cluster_economics(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			cluster_gov_params.clone(),
			ClusterMember::NodeProvider(cluster_node_1_key)
		));

		let proposal = ClusterProposal::<Test>::get(cluster_id);
		assert_eq!(
			proposal,
			Some(Proposal {
				author: cluster_node_1_provider.clone(),
				kind: ProposalKind::UpdateClusterEconomics,
				call: <Test as pallet::Config>::ClusterProposalCall::from(
					Call::<Test>::update_cluster_economics {
						cluster_id,
						cluster_gov_params: cluster_gov_params.clone(),
					}
				)
			})
		);

		let votes = ClusterProposalVoting::<Test>::get(cluster_id);
		let start = BlockNumber::from(1_u64);
		let end = start + <Test as pallet::Config>::ClusterProposalDuration::get();
		let threshold = 4; // 3 nodes + 1 cluster manager
		assert_eq!(votes, Some(Votes { threshold, ayes: vec![], nays: vec![], start, end }));
		System::assert_last_event(
			Event::Proposed { account: cluster_node_1_provider, cluster_id, threshold }.into(),
		)
	})
}

#[test]
fn cluster_economics_update_proposal_fails_on_unexpected_state() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Unbonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		assert_noop!(
			DdcClustersGov::propose_update_cluster_economics(
				RuntimeOrigin::signed(cluster_manager.clone()),
				cluster_id,
				cluster_gov_params.clone(),
				ClusterMember::ClusterManager
			),
			Error::<Test>::UnexpectedState
		);
	})
}

#[test]
fn cluster_economics_update_proposal_fails_if_there_are_not_enough_validated_nodes() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		assert_noop!(
			DdcClustersGov::propose_update_cluster_economics(
				RuntimeOrigin::signed(cluster_manager.clone()),
				cluster_id,
				cluster_gov_params.clone(),
				ClusterMember::ClusterManager
			),
			Error::<Test>::NotEnoughValidatedNodes
		);
	})
}

#[test]
fn cluster_economics_update_proposal_fails_if_there_is_active_proposal() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		assert_ok!(DdcClustersGov::propose_update_cluster_economics(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone(),
			ClusterMember::ClusterManager
		));

		assert_noop!(
			DdcClustersGov::propose_update_cluster_economics(
				RuntimeOrigin::signed(cluster_manager.clone()),
				cluster_id,
				cluster_gov_params.clone(),
				ClusterMember::ClusterManager
			),
			Error::<Test>::ActiveProposal
		);
	})
}

#[test]
fn cluster_economics_update_is_restricted_for_system_origins() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_gov_params = ClusterGovParams::default();

		assert_noop!(
			DdcClustersGov::update_cluster_economics(
				RuntimeOrigin::signed(cluster_node_1_provider.clone()),
				cluster_id,
				cluster_gov_params.clone(),
			),
			DispatchError::BadOrigin
		);

		assert_noop!(
			DdcClustersGov::update_cluster_economics(
				RuntimeOrigin::root(),
				cluster_id,
				cluster_gov_params.clone(),
			),
			DispatchError::BadOrigin
		);
	})
}

#[test]
fn cluster_economics_update_is_allowed_for_referenda_economics_updater_track_origin() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	ExtBuilder.build_and_execute(cluster, vec![], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_gov_params = ClusterGovParams {
			treasury_share: Perquintill::from_float(5.0),
			validators_share: Perquintill::from_float(10.0),
			cluster_reserve_share: Perquintill::from_float(15.0),
			storage_bond_size: 10 * CERE,
			storage_chill_delay: 20,
			storage_unbonding_delay: 20,
			unit_per_mb_stored: 97656,
			unit_per_mb_streamed: 48828,
			unit_per_put_request: 10,
			unit_per_get_request: 5,
		};
		let open_gov_updater = <Test as pallet::Config>::OpenGovUpdaterTrackOrigin::get();
		assert_ok!(DdcClustersGov::update_cluster_economics(
			open_gov_updater,
			cluster_id,
			cluster_gov_params.clone()
		));

		let updated_cluster_gov_params =
			pallet_ddc_clusters::ClustersGovParams::<Test>::get(cluster_id).unwrap();
		assert_eq!(cluster_gov_params, updated_cluster_gov_params);
		assert_eq!(System::events().len(), 1);
		System::assert_last_event(ClusterGovParamsSet { cluster_id }.into());
	})
}

#[test]
fn cluster_activation_proposal_initiates_referendum_on_close_if_there_are_no_abstentions_and_voting_period_continues(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));

		let cluster_node_2_provider = AccountId::from(NODE_PROVIDER_ID_2);
		let cluster_node_2_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_2));

		let cluster_node_3_provider = AccountId::from(NODE_PROVIDER_ID_3);
		let cluster_node_3_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_3));

		assert_ok!(DdcClustersGov::propose_activate_cluster(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone()
		));
		let threshold = 4; // 3 nodes + 1 cluster manager
		System::assert_last_event(
			Event::Proposed { account: cluster_manager.clone(), cluster_id, threshold }.into(),
		);

		let not_cluster_manager = AccountId::from([0; 32]);
		assert_noop!(
			DdcClustersGov::vote_proposal(
				RuntimeOrigin::signed(not_cluster_manager.clone()),
				cluster_id,
				true,
				ClusterMember::ClusterManager,
			),
			Error::<Test>::NotClusterManager
		);

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			true,
			ClusterMember::ClusterManager,
		));

		let votes = ClusterProposalVoting::<Test>::get(cluster_id).unwrap();
		assert_eq!(votes.ayes, vec![cluster_manager.clone()]);
		System::assert_last_event(
			Event::Voted {
				account: cluster_manager.clone(),
				cluster_id,
				voted: true,
				yes: 1,
				no: 0,
			}
			.into(),
		);

		let not_node_provider = AccountId::from([128; 32]);
		let not_cluster_node_key = NodePubKey::StoragePubKey(AccountId::from([128; 32]));
		assert_noop!(
			DdcClustersGov::vote_proposal(
				RuntimeOrigin::signed(cluster_node_1_provider.clone()),
				cluster_id,
				true,
				ClusterMember::NodeProvider(not_cluster_node_key.clone()),
			),
			Error::<Test>::NoClusterNode
		);

		assert_noop!(
			DdcClustersGov::vote_proposal(
				RuntimeOrigin::signed(not_node_provider.clone()),
				cluster_id,
				true,
				ClusterMember::NodeProvider(cluster_node_1_key.clone()),
			),
			Error::<Test>::NotNodeProvider
		);

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_1_key.clone()),
		));
		let votes = ClusterProposalVoting::<Test>::get(cluster_id).unwrap();
		assert_eq!(votes.ayes, vec![cluster_manager.clone(), cluster_node_1_provider.clone()]);
		System::assert_last_event(
			Event::Voted {
				account: cluster_node_1_provider.clone(),
				cluster_id,
				voted: true,
				yes: 2,
				no: 0,
			}
			.into(),
		);

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_2_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_2_key),
		));
		let votes = ClusterProposalVoting::<Test>::get(cluster_id).unwrap();
		assert_eq!(
			votes.ayes,
			vec![
				cluster_manager.clone(),
				cluster_node_1_provider.clone(),
				cluster_node_2_provider.clone()
			]
		);
		System::assert_last_event(
			Event::Voted {
				account: cluster_node_2_provider.clone(),
				cluster_id,
				voted: true,
				yes: 3,
				no: 0,
			}
			.into(),
		);

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_3_key),
		));
		let votes = ClusterProposalVoting::<Test>::get(cluster_id).unwrap();
		assert_eq!(
			votes.ayes,
			vec![
				cluster_manager.clone(),
				cluster_node_1_provider.clone(),
				cluster_node_2_provider.clone(),
				cluster_node_3_provider.clone()
			]
		);
		System::assert_last_event(
			Event::Voted {
				account: cluster_node_3_provider.clone(),
				cluster_id,
				voted: true,
				yes: 4,
				no: 0,
			}
			.into(),
		);

		assert_noop!(
			DdcClustersGov::close_proposal(
				RuntimeOrigin::signed(not_cluster_manager.clone()),
				cluster_id,
				ClusterMember::ClusterManager,
			),
			Error::<Test>::NotClusterManager
		);

		assert_noop!(
			DdcClustersGov::close_proposal(
				RuntimeOrigin::signed(cluster_node_1_provider.clone()),
				cluster_id,
				ClusterMember::NodeProvider(not_cluster_node_key.clone()),
			),
			Error::<Test>::NotValidatedNode
		);

		assert_noop!(
			DdcClustersGov::close_proposal(
				RuntimeOrigin::signed(not_node_provider.clone()),
				cluster_id,
				ClusterMember::NodeProvider(cluster_node_1_key.clone()),
			),
			Error::<Test>::NotNodeProvider
		);

		let balance_before_submission_deposit = Balances::free_balance(cluster_manager.clone());
		assert_ok!(DdcClustersGov::close_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			ClusterMember::ClusterManager,
		));

		let referenda_count = pallet_referenda::ReferendumCount::<Test>::get();
		assert_eq!(referenda_count, 1);
		let referenda_index = referenda_count - 1;

		let submission_deposit_amount =
			<Test as pallet_referenda::Config>::SubmissionDeposit::get().saturated_into::<u128>();

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));

		assert_eq!(
			SubmissionDeposits::<Test>::get(referenda_index),
			Some(ReferendaSubmissionDeposit {
				depositor: cluster_manager.clone(),
				amount: submission_deposit_amount
			})
		);

		let balance_after_submission_deposit = Balances::free_balance(cluster_manager.clone());
		assert_eq!(
			balance_before_submission_deposit.saturating_sub(submission_deposit_amount),
			balance_after_submission_deposit
		);

		System::assert_has_event(Event::Closed { cluster_id, yes: 4, no: 0 }.into());
		System::assert_has_event(Event::Approved { cluster_id }.into());
		System::assert_has_event(Event::ReferendumSubmitted { cluster_id }.into());
		System::assert_has_event(
			Event::SubmissionDepositRetained {
				referenda_index,
				depositor: cluster_manager.clone(),
				amount: submission_deposit_amount,
			}
			.into(),
		);
		System::assert_has_event(Event::Removed { cluster_id }.into());

		// OpenGov

		let balance_before_decision_deposit = Balances::free_balance(cluster_manager.clone());
		assert_ok!(Referenda::place_decision_deposit(
			RuntimeOrigin::signed(cluster_manager.clone()),
			referenda_index,
		));
		let balance_after_decision_deposit = Balances::free_balance(cluster_manager.clone());
		assert_eq!(
			balance_before_decision_deposit.saturating_sub(CLUSTER_ACTIVATOR_DECISION_DEPOSIT),
			balance_after_decision_deposit
		);

		let referendum = pallet_referenda::ReferendumInfoFor::<Test>::get(referenda_index).unwrap();
		assert!(matches!(referendum, ReferendumInfo::Ongoing(..)));

		assert_ok!(ConvictionVoting::vote(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			referenda_index,
			AccountVote::Standard {
				vote: Vote { aye: true, conviction: Conviction::Locked6x },
				balance: Balances::free_balance(cluster_node_1_provider.clone())
			}
		));

		assert_ok!(Referenda::nudge_referendum(RuntimeOrigin::root(), referenda_index));
		fast_forward_to(3);
		assert_ok!(Referenda::nudge_referendum(RuntimeOrigin::root(), referenda_index));

		let referendum = pallet_referenda::ReferendumInfoFor::<Test>::get(referenda_index).unwrap();
		assert!(matches!(referendum, ReferendumInfo::Approved(..)));

		let balance_before_submission_deposit_refund =
			Balances::free_balance(cluster_manager.clone());
		assert_ok!(DdcClustersGov::refund_submission_deposit(
			RuntimeOrigin::signed(cluster_manager.clone()),
			referenda_index,
		));
		let balance_after_submission_deposit_refund =
			Balances::free_balance(cluster_manager.clone());

		assert_eq!(
			balance_before_submission_deposit_refund.saturating_add(submission_deposit_amount),
			balance_after_submission_deposit_refund
		);
		System::assert_last_event(
			Event::SubmissionDepositRefunded {
				referenda_index,
				depositor: cluster_manager.clone(),
				amount: submission_deposit_amount,
			}
			.into(),
		);
	})
}

#[test]
fn cluster_activation_proposal_initiates_referendum_on_close_if_there_are_no_abstentions_and_voting_period_ended(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));

		let cluster_node_2_provider = AccountId::from(NODE_PROVIDER_ID_2);
		let cluster_node_2_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_2));

		let cluster_node_3_provider = AccountId::from(NODE_PROVIDER_ID_3);
		let cluster_node_3_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_3));

		assert_ok!(DdcClustersGov::propose_activate_cluster(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone()
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			true,
			ClusterMember::ClusterManager,
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_1_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_2_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_2_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_3_key),
		));

		let start = BlockNumber::from(1_u64);
		let end = start + <Test as pallet::Config>::ClusterProposalDuration::get();
		fast_forward_to(end + 1);

		let balance_before_submission_deposit =
			Balances::free_balance(cluster_node_2_provider.clone());
		assert_ok!(DdcClustersGov::close_proposal(
			RuntimeOrigin::signed(cluster_node_2_provider.clone()),
			cluster_id,
			ClusterMember::NodeProvider(cluster_node_2_key.clone()),
		));

		let referenda_count = pallet_referenda::ReferendumCount::<Test>::get();
		assert_eq!(referenda_count, 1);
		let referenda_index = referenda_count - 1;

		let submission_deposit_amount =
			<Test as pallet_referenda::Config>::SubmissionDeposit::get().saturated_into::<u128>();

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));

		assert_eq!(
			SubmissionDeposits::<Test>::get(referenda_index),
			Some(ReferendaSubmissionDeposit {
				depositor: cluster_node_2_provider.clone(),
				amount: submission_deposit_amount
			})
		);

		let balance_after_submission_deposit =
			Balances::free_balance(cluster_node_2_provider.clone());
		assert_eq!(
			balance_before_submission_deposit.saturating_sub(submission_deposit_amount),
			balance_after_submission_deposit
		);

		System::assert_has_event(Event::Closed { cluster_id, yes: 4, no: 0 }.into());
		System::assert_has_event(Event::Approved { cluster_id }.into());
		System::assert_has_event(Event::ReferendumSubmitted { cluster_id }.into());
		System::assert_has_event(
			Event::SubmissionDepositRetained {
				referenda_index,
				depositor: cluster_node_2_provider.clone(),
				amount: submission_deposit_amount,
			}
			.into(),
		);
		System::assert_has_event(Event::Removed { cluster_id }.into());
	})
}

#[test]
fn cluster_activation_proposal_does_not_initiate_referendum_on_close_if_there_are_abstentions_and_voting_period_ended(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		assert_ok!(DdcClustersGov::propose_activate_cluster(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone()
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			true,
			ClusterMember::ClusterManager,
		));

		let start = BlockNumber::from(1_u64);
		let end = start + <Test as pallet::Config>::ClusterProposalDuration::get();
		fast_forward_to(end + 1);

		assert_ok!(DdcClustersGov::close_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			ClusterMember::ClusterManager,
		));

		// As the quorum is not reached, the proposal gets rejected and no referenda is created
		assert_eq!(pallet_referenda::ReferendumCount::<Test>::get(), 0);

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));

		System::assert_has_event(Event::Closed { cluster_id, yes: 1, no: 3 }.into());
		System::assert_has_event(Event::Disapproved { cluster_id }.into());
		System::assert_has_event(Event::Removed { cluster_id }.into());
	})
}

#[test]
fn cluster_activation_proposal_does_not_initiate_referendum_on_close_if_there_no_abstentions_and_voting_period_continues(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));

		let cluster_node_2_provider = AccountId::from(NODE_PROVIDER_ID_2);
		let cluster_node_2_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_2));

		let cluster_node_3_provider = AccountId::from(NODE_PROVIDER_ID_3);
		let cluster_node_3_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_3));

		assert_ok!(DdcClustersGov::propose_activate_cluster(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone()
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			true,
			ClusterMember::ClusterManager,
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			false,
			ClusterMember::NodeProvider(cluster_node_1_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_2_provider.clone()),
			cluster_id,
			false,
			ClusterMember::NodeProvider(cluster_node_2_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			false,
			ClusterMember::NodeProvider(cluster_node_3_key),
		));

		assert_ok!(DdcClustersGov::close_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			ClusterMember::ClusterManager,
		));

		// As the quorum is not reached, the proposal gets rejected and no referenda is created
		assert_eq!(pallet_referenda::ReferendumCount::<Test>::get(), 0);

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));

		System::assert_has_event(Event::Closed { cluster_id, yes: 1, no: 3 }.into());
		System::assert_has_event(Event::Disapproved { cluster_id }.into());
		System::assert_has_event(Event::Removed { cluster_id }.into());
	})
}

#[test]
fn cluster_activation_proposal_does_not_initiate_referendum_on_close_if_there_are_no_abstentions_and_voting_period_ended(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));

		let cluster_node_2_provider = AccountId::from(NODE_PROVIDER_ID_2);
		let cluster_node_2_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_2));

		let cluster_node_3_provider = AccountId::from(NODE_PROVIDER_ID_3);
		let cluster_node_3_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_3));

		assert_ok!(DdcClustersGov::propose_activate_cluster(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone()
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			true,
			ClusterMember::ClusterManager,
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			false,
			ClusterMember::NodeProvider(cluster_node_1_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_2_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_2_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_3_key),
		));

		let start = BlockNumber::from(1_u64);
		let end = start + <Test as pallet::Config>::ClusterProposalDuration::get();
		fast_forward_to(end + 1);

		assert_ok!(DdcClustersGov::close_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			ClusterMember::ClusterManager,
		));

		// As the quorum is not reached, the proposal gets rejected and no referenda is created
		assert_eq!(pallet_referenda::ReferendumCount::<Test>::get(), 0);

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));

		System::assert_has_event(Event::Closed { cluster_id, yes: 3, no: 1 }.into());
		System::assert_has_event(Event::Disapproved { cluster_id }.into());
		System::assert_has_event(Event::Removed { cluster_id }.into());
	})
}

#[test]
fn cluster_activation_proposal_cannot_be_closed_if_there_are_abstentions_and_voting_period_continues(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Bonded,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		assert_ok!(DdcClustersGov::propose_activate_cluster(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone()
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			true,
			ClusterMember::ClusterManager,
		));

		assert_noop!(
			DdcClustersGov::close_proposal(
				RuntimeOrigin::signed(cluster_manager.clone()),
				cluster_id,
				ClusterMember::ClusterManager,
			),
			Error::<Test>::TooEarly
		);
	})
}

#[test]
fn cluster_economics_update_proposal_initiates_referendum_on_close_if_there_are_no_abstentions_and_voting_period_continues(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));

		let cluster_node_2_provider = AccountId::from(NODE_PROVIDER_ID_2);
		let cluster_node_2_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_2));

		let cluster_node_3_provider = AccountId::from(NODE_PROVIDER_ID_3);
		let cluster_node_3_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_3));

		assert_ok!(DdcClustersGov::propose_update_cluster_economics(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			cluster_gov_params.clone(),
			ClusterMember::NodeProvider(cluster_node_3_key.clone()),
		));
		let threshold = 4; // 3 nodes + 1 cluster manager
		System::assert_last_event(
			Event::Proposed { account: cluster_node_3_provider.clone(), cluster_id, threshold }
				.into(),
		);

		let not_cluster_manager = AccountId::from([0; 32]);
		assert_noop!(
			DdcClustersGov::vote_proposal(
				RuntimeOrigin::signed(not_cluster_manager.clone()),
				cluster_id,
				true,
				ClusterMember::ClusterManager,
			),
			Error::<Test>::NotClusterManager
		);

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			true,
			ClusterMember::ClusterManager,
		));

		let votes = ClusterProposalVoting::<Test>::get(cluster_id).unwrap();
		assert_eq!(votes.ayes, vec![cluster_manager.clone()]);
		System::assert_last_event(
			Event::Voted {
				account: cluster_manager.clone(),
				cluster_id,
				voted: true,
				yes: 1,
				no: 0,
			}
			.into(),
		);

		let not_node_provider = AccountId::from([128; 32]);
		let not_cluster_node_key = NodePubKey::StoragePubKey(AccountId::from([128; 32]));
		assert_noop!(
			DdcClustersGov::vote_proposal(
				RuntimeOrigin::signed(cluster_node_1_provider.clone()),
				cluster_id,
				true,
				ClusterMember::NodeProvider(not_cluster_node_key.clone()),
			),
			Error::<Test>::NoClusterNode
		);

		assert_noop!(
			DdcClustersGov::vote_proposal(
				RuntimeOrigin::signed(not_node_provider.clone()),
				cluster_id,
				true,
				ClusterMember::NodeProvider(cluster_node_1_key.clone()),
			),
			Error::<Test>::NotNodeProvider
		);

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_1_key.clone()),
		));
		let votes = ClusterProposalVoting::<Test>::get(cluster_id).unwrap();
		assert_eq!(votes.ayes, vec![cluster_manager.clone(), cluster_node_1_provider.clone()]);
		System::assert_last_event(
			Event::Voted {
				account: cluster_node_1_provider.clone(),
				cluster_id,
				voted: true,
				yes: 2,
				no: 0,
			}
			.into(),
		);

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_2_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_2_key),
		));
		let votes = ClusterProposalVoting::<Test>::get(cluster_id).unwrap();
		assert_eq!(
			votes.ayes,
			vec![
				cluster_manager.clone(),
				cluster_node_1_provider.clone(),
				cluster_node_2_provider.clone()
			]
		);
		System::assert_last_event(
			Event::Voted {
				account: cluster_node_2_provider.clone(),
				cluster_id,
				voted: true,
				yes: 3,
				no: 0,
			}
			.into(),
		);

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_3_key.clone()),
		));
		let votes = ClusterProposalVoting::<Test>::get(cluster_id).unwrap();
		assert_eq!(
			votes.ayes,
			vec![
				cluster_manager.clone(),
				cluster_node_1_provider.clone(),
				cluster_node_2_provider.clone(),
				cluster_node_3_provider.clone()
			]
		);
		System::assert_last_event(
			Event::Voted {
				account: cluster_node_3_provider.clone(),
				cluster_id,
				voted: true,
				yes: 4,
				no: 0,
			}
			.into(),
		);

		assert_noop!(
			DdcClustersGov::close_proposal(
				RuntimeOrigin::signed(not_cluster_manager.clone()),
				cluster_id,
				ClusterMember::ClusterManager,
			),
			Error::<Test>::NotClusterManager
		);

		assert_noop!(
			DdcClustersGov::close_proposal(
				RuntimeOrigin::signed(cluster_node_1_provider.clone()),
				cluster_id,
				ClusterMember::NodeProvider(not_cluster_node_key.clone()),
			),
			Error::<Test>::NotValidatedNode
		);

		assert_noop!(
			DdcClustersGov::close_proposal(
				RuntimeOrigin::signed(not_node_provider.clone()),
				cluster_id,
				ClusterMember::NodeProvider(cluster_node_1_key.clone()),
			),
			Error::<Test>::NotNodeProvider
		);

		let balance_before_submission_deposit = Balances::free_balance(cluster_manager.clone());
		assert_ok!(DdcClustersGov::close_proposal(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			ClusterMember::NodeProvider(cluster_node_3_key.clone()),
		));

		let referenda_count = pallet_referenda::ReferendumCount::<Test>::get();
		assert_eq!(referenda_count, 1);
		let referenda_index = referenda_count - 1;

		let submission_deposit_amount =
			<Test as pallet_referenda::Config>::SubmissionDeposit::get().saturated_into::<u128>();

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));

		assert_eq!(
			SubmissionDeposits::<Test>::get(referenda_index),
			Some(ReferendaSubmissionDeposit {
				depositor: cluster_node_3_provider.clone(),
				amount: submission_deposit_amount
			})
		);

		let balance_after_submission_deposit =
			Balances::free_balance(cluster_node_3_provider.clone());
		assert_eq!(
			balance_before_submission_deposit.saturating_sub(submission_deposit_amount),
			balance_after_submission_deposit
		);

		System::assert_has_event(Event::Closed { cluster_id, yes: 4, no: 0 }.into());
		System::assert_has_event(Event::Approved { cluster_id }.into());
		System::assert_has_event(Event::ReferendumSubmitted { cluster_id }.into());
		System::assert_has_event(
			Event::SubmissionDepositRetained {
				referenda_index,
				depositor: cluster_node_3_provider.clone(),
				amount: submission_deposit_amount,
			}
			.into(),
		);
		System::assert_has_event(Event::Removed { cluster_id }.into());

		// OpenGov

		let balance_before_decision_deposit =
			Balances::free_balance(cluster_node_3_provider.clone());
		assert_ok!(Referenda::place_decision_deposit(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			referenda_index,
		));
		let balance_after_decision_deposit =
			Balances::free_balance(cluster_node_3_provider.clone());
		assert_eq!(
			balance_before_decision_deposit
				.saturating_sub(CLUSTER_ECONOMICS_UPDATE_DECISION_DEPOSIT),
			balance_after_decision_deposit
		);

		let referendum = pallet_referenda::ReferendumInfoFor::<Test>::get(referenda_index).unwrap();
		assert!(matches!(referendum, ReferendumInfo::Ongoing(..)));

		assert_ok!(ConvictionVoting::vote(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			referenda_index,
			AccountVote::Standard {
				vote: Vote { aye: true, conviction: Conviction::Locked6x },
				balance: Balances::free_balance(cluster_node_1_provider.clone())
			}
		));

		assert_ok!(Referenda::nudge_referendum(RuntimeOrigin::root(), referenda_index));
		fast_forward_to(3);
		assert_ok!(Referenda::nudge_referendum(RuntimeOrigin::root(), referenda_index));

		let referendum = pallet_referenda::ReferendumInfoFor::<Test>::get(referenda_index).unwrap();
		assert!(matches!(referendum, ReferendumInfo::Approved(..)));

		let balance_before_submission_deposit_refund =
			Balances::free_balance(cluster_node_3_provider.clone());
		assert_ok!(DdcClustersGov::refund_submission_deposit(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			referenda_index,
		));
		let balance_after_submission_deposit_refund =
			Balances::free_balance(cluster_node_3_provider.clone());

		assert_eq!(
			balance_before_submission_deposit_refund.saturating_add(submission_deposit_amount),
			balance_after_submission_deposit_refund
		);
		System::assert_last_event(
			Event::SubmissionDepositRefunded {
				referenda_index,
				depositor: cluster_node_3_provider.clone(),
				amount: submission_deposit_amount,
			}
			.into(),
		);
	})
}

#[test]
fn cluster_economics_update_proposal_initiates_referendum_on_close_if_there_are_no_abstentions_and_voting_period_ended(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));

		let cluster_node_2_provider = AccountId::from(NODE_PROVIDER_ID_2);
		let cluster_node_2_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_2));

		let cluster_node_3_provider = AccountId::from(NODE_PROVIDER_ID_3);
		let cluster_node_3_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_3));

		assert_ok!(DdcClustersGov::propose_update_cluster_economics(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			cluster_gov_params.clone(),
			ClusterMember::NodeProvider(cluster_node_3_key.clone())
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			true,
			ClusterMember::ClusterManager,
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_1_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_2_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_2_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_3_key.clone()),
		));

		let start = BlockNumber::from(1_u64);
		let end = start + <Test as pallet::Config>::ClusterProposalDuration::get();
		fast_forward_to(end + 1);

		let balance_before_submission_deposit =
			Balances::free_balance(cluster_node_3_provider.clone());
		assert_ok!(DdcClustersGov::close_proposal(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			ClusterMember::NodeProvider(cluster_node_3_key.clone()),
		));

		let referenda_count = pallet_referenda::ReferendumCount::<Test>::get();
		assert_eq!(referenda_count, 1);
		let referenda_index = referenda_count - 1;

		let submission_deposit_amount =
			<Test as pallet_referenda::Config>::SubmissionDeposit::get().saturated_into::<u128>();

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));

		assert_eq!(
			SubmissionDeposits::<Test>::get(referenda_index),
			Some(ReferendaSubmissionDeposit {
				depositor: cluster_node_3_provider.clone(),
				amount: submission_deposit_amount
			})
		);

		let balance_after_submission_deposit =
			Balances::free_balance(cluster_node_3_provider.clone());
		assert_eq!(
			balance_before_submission_deposit.saturating_sub(submission_deposit_amount),
			balance_after_submission_deposit
		);

		System::assert_has_event(Event::Closed { cluster_id, yes: 4, no: 0 }.into());
		System::assert_has_event(Event::Approved { cluster_id }.into());
		System::assert_has_event(Event::ReferendumSubmitted { cluster_id }.into());
		System::assert_has_event(
			Event::SubmissionDepositRetained {
				referenda_index,
				depositor: cluster_node_3_provider.clone(),
				amount: submission_deposit_amount,
			}
			.into(),
		);
		System::assert_has_event(Event::Removed { cluster_id }.into());
	})
}

#[test]
fn cluster_economics_update_proposal_does_not_initiate_referendum_on_close_if_there_are_abstentions_and_voting_period_ended(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));

		assert_ok!(DdcClustersGov::propose_update_cluster_economics(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			cluster_gov_params.clone(),
			ClusterMember::NodeProvider(cluster_node_1_key.clone())
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_1_key.clone())
		));

		let start = BlockNumber::from(1_u64);
		let end = start + <Test as pallet::Config>::ClusterProposalDuration::get();
		fast_forward_to(end + 1);

		assert_ok!(DdcClustersGov::close_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			ClusterMember::NodeProvider(cluster_node_1_key.clone())
		));

		// As the quorum is not reached, the proposal gets rejected and no referenda is created
		assert_eq!(pallet_referenda::ReferendumCount::<Test>::get(), 0);

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));

		System::assert_has_event(Event::Closed { cluster_id, yes: 1, no: 3 }.into());
		System::assert_has_event(Event::Disapproved { cluster_id }.into());
		System::assert_has_event(Event::Removed { cluster_id }.into());
	})
}

#[test]
fn cluster_economics_update_proposal_does_not_initiate_referendum_on_close_if_there_are_abstentions_and_voting_period_continues(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));

		let cluster_node_2_provider = AccountId::from(NODE_PROVIDER_ID_2);
		let cluster_node_2_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_2));

		let cluster_node_3_provider = AccountId::from(NODE_PROVIDER_ID_3);
		let cluster_node_3_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_3));

		assert_ok!(DdcClustersGov::propose_update_cluster_economics(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone(),
			ClusterMember::ClusterManager
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			true,
			ClusterMember::ClusterManager,
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			false,
			ClusterMember::NodeProvider(cluster_node_1_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_2_provider.clone()),
			cluster_id,
			false,
			ClusterMember::NodeProvider(cluster_node_2_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			false,
			ClusterMember::NodeProvider(cluster_node_3_key),
		));

		assert_ok!(DdcClustersGov::close_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			ClusterMember::ClusterManager,
		));

		// As the quorum is not reached, the proposal gets rejected and no referenda is created
		assert_eq!(pallet_referenda::ReferendumCount::<Test>::get(), 0);

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));

		System::assert_has_event(Event::Closed { cluster_id, yes: 1, no: 3 }.into());
		System::assert_has_event(Event::Disapproved { cluster_id }.into());
		System::assert_has_event(Event::Removed { cluster_id }.into());
	})
}

#[test]
fn cluster_economics_update_proposal_does_not_initiate_referendum_on_close_if_there_are_no_abstentions_and_voting_period_ended(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));

		let cluster_node_2_provider = AccountId::from(NODE_PROVIDER_ID_2);
		let cluster_node_2_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_2));

		let cluster_node_3_provider = AccountId::from(NODE_PROVIDER_ID_3);
		let cluster_node_3_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_3));

		assert_ok!(DdcClustersGov::propose_update_cluster_economics(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			cluster_gov_params.clone(),
			ClusterMember::ClusterManager
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			true,
			ClusterMember::ClusterManager,
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			false,
			ClusterMember::NodeProvider(cluster_node_1_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_2_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_2_key.clone()),
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_3_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_3_key),
		));

		let start = BlockNumber::from(1_u64);
		let end = start + <Test as pallet::Config>::ClusterProposalDuration::get();
		fast_forward_to(end + 1);

		assert_ok!(DdcClustersGov::close_proposal(
			RuntimeOrigin::signed(cluster_manager.clone()),
			cluster_id,
			ClusterMember::ClusterManager,
		));

		// As the quorum is not reached, the proposal gets rejected and no referenda is created
		assert_eq!(pallet_referenda::ReferendumCount::<Test>::get(), 0);

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));

		System::assert_has_event(Event::Closed { cluster_id, yes: 3, no: 1 }.into());
		System::assert_has_event(Event::Disapproved { cluster_id }.into());
		System::assert_has_event(Event::Removed { cluster_id }.into());
	})
}

#[test]
fn cluster_economics_update_proposal_cannot_be_closed_if_there_are_abstentions_and_voting_period_continues(
) {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_gov_params = ClusterGovParams::default();

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));

		assert_ok!(DdcClustersGov::propose_update_cluster_economics(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			cluster_gov_params.clone(),
			ClusterMember::NodeProvider(cluster_node_1_key.clone())
		));

		assert_ok!(DdcClustersGov::vote_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			true,
			ClusterMember::NodeProvider(cluster_node_1_key.clone())
		));

		assert_noop!(
			DdcClustersGov::close_proposal(
				RuntimeOrigin::signed(cluster_node_1_provider.clone()),
				cluster_id,
				ClusterMember::NodeProvider(cluster_node_1_key)
			),
			Error::<Test>::TooEarly
		);
	})
}

#[test]
fn cluster_economics_update_proposal_can_be_retracted_by_its_author() {
	let cluster = build_cluster(
		CLUSTER_ID,
		CLUSTER_MANAGER_ID,
		CLUSTER_RESERVE_ID,
		ClusterParams::default(),
		ClusterGovParams::default(),
		ClusterStatus::Activated,
	);

	let node_1 = build_cluster_node(
		NODE_PUB_KEY_1,
		NODE_PROVIDER_ID_1,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_2 = build_cluster_node(
		NODE_PUB_KEY_2,
		NODE_PROVIDER_ID_2,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	let node_3 = build_cluster_node(
		NODE_PUB_KEY_3,
		NODE_PROVIDER_ID_3,
		StorageNodeParams::default(),
		CLUSTER_ID,
		ClusterNodeStatus::ValidationSucceeded,
		ClusterNodeKind::Genesis,
	);

	ExtBuilder.build_and_execute(cluster, vec![node_1, node_2, node_3], || {
		fast_forward_to(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_manager = AccountId::from(CLUSTER_MANAGER_ID);
		let cluster_gov_params = ClusterGovParams::default();
		let not_cluster_id = ClusterId::from([0; 20]);

		let cluster_node_1_provider = AccountId::from(NODE_PROVIDER_ID_1);
		let cluster_node_1_key = NodePubKey::StoragePubKey(AccountId::from(NODE_PUB_KEY_1));

		assert_ok!(DdcClustersGov::propose_update_cluster_economics(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
			cluster_gov_params.clone(),
			ClusterMember::NodeProvider(cluster_node_1_key)
		));

		assert!(ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(ClusterProposalVoting::<Test>::contains_key(cluster_id));

		assert_noop!(
			DdcClustersGov::retract_proposal(
				RuntimeOrigin::signed(cluster_node_1_provider.clone()),
				not_cluster_id,
			),
			Error::<Test>::ProposalMissing
		);

		assert_noop!(
			DdcClustersGov::retract_proposal(
				RuntimeOrigin::signed(cluster_manager.clone()),
				cluster_id,
			),
			Error::<Test>::NotProposalAuthor
		);

		assert_ok!(DdcClustersGov::retract_proposal(
			RuntimeOrigin::signed(cluster_node_1_provider.clone()),
			cluster_id,
		));

		assert!(!ClusterProposal::<Test>::contains_key(cluster_id));
		assert!(!ClusterProposalVoting::<Test>::contains_key(cluster_id));
		System::assert_last_event(Event::Removed { cluster_id }.into());
	})
}
