use frame_system::{pallet_prelude::BlockNumberFor, Config};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::*;

use crate::{
	ClusterBondingParams, ClusterFeesParams, ClusterId, ClusterNodeKind, ClusterNodeState,
	ClusterNodeStatus, ClusterNodesStats, ClusterParams, ClusterPricingParams,
	ClusterProtocolParams, ClusterStatus, DdcEra, NodePubKey, NodeType,
};

pub trait ClusterQuery<T: Config> {
	fn cluster_exists(cluster_id: &ClusterId) -> bool;
	fn get_cluster_status(cluster_id: &ClusterId) -> Result<ClusterStatus, DispatchError>;
	fn get_manager_and_reserve_id(
		cluster_id: &ClusterId,
	) -> Result<(T::AccountId, T::AccountId), DispatchError>;
}

pub trait ClusterProtocol<T: Config, Balance>: ClusterQuery<T> {
	fn get_bond_size(cluster_id: &ClusterId, node_type: NodeType) -> Result<u128, DispatchError>;

	fn get_pricing_params(cluster_id: &ClusterId) -> Result<ClusterPricingParams, DispatchError>;

	fn get_fees_params(cluster_id: &ClusterId) -> Result<ClusterFeesParams, DispatchError>;

	fn get_chill_delay(
		cluster_id: &ClusterId,
		node_type: NodeType,
	) -> Result<BlockNumberFor<T>, DispatchError>;

	fn get_unbonding_delay(
		cluster_id: &ClusterId,
		node_type: NodeType,
	) -> Result<BlockNumberFor<T>, DispatchError>;

	fn get_bonding_params(
		cluster_id: &ClusterId,
	) -> Result<ClusterBondingParams<BlockNumberFor<T>>, DispatchError>;

	fn get_reserve_account_id(cluster_id: &ClusterId) -> Result<T::AccountId, DispatchError>;

	fn activate_cluster_protocol(cluster_id: &ClusterId) -> DispatchResult;

	fn update_cluster_protocol(
		cluster_id: &ClusterId,
		cluster_protocol_params: ClusterProtocolParams<Balance, BlockNumberFor<T>>,
	) -> DispatchResult;

	fn bond_cluster(cluster_id: &ClusterId) -> DispatchResult;

	fn start_unbond_cluster(cluster_id: &ClusterId) -> DispatchResult;

	fn end_unbond_cluster(cluster_id: &ClusterId) -> DispatchResult;
}

pub trait ClusterCreator<T: Config, Balance> {
	fn create_cluster(
		cluster_id: ClusterId,
		cluster_manager_id: T::AccountId,
		cluster_reserve_id: T::AccountId,
		cluster_params: ClusterParams<T::AccountId>,
		initial_protocol_params: ClusterProtocolParams<Balance, BlockNumberFor<T>>,
	) -> DispatchResult;
}

pub trait ClusterManager<T: Config>: ClusterQuery<T> {
	fn get_manager_account_id(cluster_id: &ClusterId) -> Result<T::AccountId, DispatchError>;

	fn contains_node(
		cluster_id: &ClusterId,
		node_pub_key: &NodePubKey,
		validation_status: Option<ClusterNodeStatus>,
	) -> bool;

	fn get_nodes(cluster_id: &ClusterId) -> Result<Vec<NodePubKey>, DispatchError>;

	fn add_node(
		cluster_id: &ClusterId,
		node_pub_key: &NodePubKey,
		node_kind: &ClusterNodeKind,
	) -> Result<(), DispatchError>;

	fn remove_node(cluster_id: &ClusterId, node_pub_key: &NodePubKey) -> Result<(), DispatchError>;

	fn get_node_state(
		cluster_id: &ClusterId,
		node_pub_key: &NodePubKey,
	) -> Result<ClusterNodeState<BlockNumberFor<T>>, DispatchError>;

	fn get_nodes_stats(cluster_id: &ClusterId) -> Result<ClusterNodesStats, DispatchError>;

	fn validate_node(
		cluster_id: &ClusterId,
		node_pub_key: &NodePubKey,
		succeeded: bool,
	) -> Result<(), DispatchError>;

	fn get_clusters(status: ClusterStatus) -> Result<Vec<ClusterId>, DispatchError>;
}
pub trait ClusterValidator<T: Config> {
	/// Updates the `last_paid_era` for the given cluster and emits an event indicating the
	/// update.
	///
	/// # Parameters
	///
	/// - `cluster_id`: A reference to the unique identifier of the cluster that needs its last paid
	///   era updated.
	/// - `era_id`: The new era identifier to be set as the last paid era for the cluster.
	///
	/// # Returns
	///
	/// Returns `Ok(())` if the operation was successful, otherwise returns a `DispatchError`.
	///
	/// # Events
	///
	/// Emits `ClusterEraPaid` event if the operation is successful.
	fn set_last_paid_era(cluster_id: &ClusterId, era_id: DdcEra) -> Result<(), DispatchError>;

	/// Retrieves the `last_paid_era` for the given cluster
	/// update.
	///
	/// # Parameters
	///
	/// - `cluster_id`: A reference to the unique identifier of the cluster the `last_paid_era` is
	///   being retrieved for
	///
	/// # Returns
	///
	/// Returns `Ok(DdcEra)` identifier of the last validated era in cluster
	fn get_last_paid_era(cluster_id: &ClusterId) -> Result<DdcEra, DispatchError>;
}
