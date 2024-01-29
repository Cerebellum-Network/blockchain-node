use codec::{Decode, Encode};
use frame_support::dispatch::{DispatchError, DispatchResult};
use frame_system::{pallet_prelude::BlockNumberFor, Config};

use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

use crate::{
	ClusterBondingParams, ClusterFeesParams, ClusterGovParams, ClusterId, ClusterNodeKind,
	ClusterNodeStatus, ClusterNodesStats, ClusterParams, ClusterPricingParams, ClusterStatus,
	NodePubKey, NodeType,
};

// todo: This trait becomes too overloaded and needs to be revised.
// One of possible solutions is to define a trait for basic storage queries and use it to extend
// more specific traits, i.e. ClusterQuery + ClusterEconomy, ClusterQuery + ClusteManager, etc.
pub trait ClusterVisitor<T: Config> {
	fn cluster_exists(cluster_id: &ClusterId) -> bool;

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

	fn get_nodes_stats(cluster_id: &ClusterId) -> Result<ClusterNodesStats, DispatchError>;

	fn get_cluster_status(cluster_id: &ClusterId) -> Result<ClusterStatus, DispatchError>;
}

pub trait ClusterCreator<T: Config, Balance> {
	fn create_cluster(
		cluster_id: ClusterId,
		cluster_manager_id: T::AccountId,
		cluster_reserve_id: T::AccountId,
		cluster_params: ClusterParams<T::AccountId>,
		cluster_gov_params: ClusterGovParams<Balance, BlockNumberFor<T>>,
	) -> DispatchResult;
}

pub trait ClusterManager<T: Config> {
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
}

pub trait ClusterAdministrator<T: Config, Balance> {
	fn activate_cluster(cluster_id: ClusterId) -> DispatchResult;

	fn update_cluster_gov_params(
		cluster_id: ClusterId,
		cluster_gov_params: ClusterGovParams<Balance, T::BlockNumber>,
	) -> DispatchResult;
}
