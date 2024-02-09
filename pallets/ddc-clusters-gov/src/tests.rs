//! Tests for the module.

use ddc_primitives::{ClusterGovParams, ClusterNodeKind, ClusterParams, StorageNodeParams};
use frame_support::{assert_noop, assert_ok};

use super::{mock::*, *};

#[test]
fn cluster_activation_proposal_works() {
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
		System::set_block_number(1);

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
		assert_eq!(votes, Some(Votes { threshold: 4, ayes: vec![], nays: vec![], start, end }));

		assert_eq!(System::events().len(), 1);
		System::assert_last_event(
			Event::Proposed { account: cluster_manager, cluster_id, threshold: 4 }.into(),
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
		System::set_block_number(1);

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
		System::set_block_number(1);

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
		System::set_block_number(1);

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
		System::set_block_number(1);

		let cluster_id = ClusterId::from(CLUSTER_ID);
		let cluster_gov_params = ClusterGovParams::default();
		let open_gov_activator = <Test as pallet::Config>::OpenGovActivatorTrackOrigin::get();
		assert_ok!(DdcClustersGov::activate_cluster(
			open_gov_activator,
			cluster_id,
			cluster_gov_params.clone()
		));
	})
}
