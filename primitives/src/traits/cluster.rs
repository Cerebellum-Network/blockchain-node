use frame_system::{pallet_prelude::BlockNumberFor, Config};
use scale_info::TypeInfo;
use sp_runtime::{DispatchError, DispatchResult, RuntimeDebug};

use crate::{
	ClusterBondingParams, ClusterFeesParams, ClusterGovParams, ClusterId, ClusterNodeKind,
	ClusterNodeState, ClusterNodeStatus, ClusterNodesStats, ClusterParams, ClusterPricingParams,
	ClusterStatus, NodePubKey, NodeType,
};

pub trait ClusterQuery<T: Config> {
	fn cluster_exists(cluster_id: &ClusterId) -> bool;
	fn get_cluster_status(cluster_id: &ClusterId) -> Result<ClusterStatus, DispatchError>;
	fn get_manager_and_reserve_id(
		cluster_id: &ClusterId,
	) -> Result<(T::AccountId, T::AccountId), DispatchError>;
}

pub trait ClusterEconomics<T: Config, Balance>: ClusterQuery<T> {
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

	fn update_cluster_economics(
		cluster_id: &ClusterId,
		cluster_gov_params: ClusterGovParams<Balance, BlockNumberFor<T>>,
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
		cluster_gov_params: ClusterGovParams<Balance, BlockNumberFor<T>>,
	) -> DispatchResult;

	fn activate_cluster(cluster_id: &ClusterId) -> DispatchResult;
}

pub trait ClusterManager<T: Config>: ClusterQuery<T> {
	fn get_manager_account_id(cluster_id: &ClusterId) -> Result<T::AccountId, DispatchError>;

	fn contains_node(
		cluster_id: &ClusterId,
		node_pub_key: &NodePubKey,
		validation_status: Option<ClusterNodeStatus>,
	) -> bool;

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
}
