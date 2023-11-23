use codec::{Decode, Encode};
use ddc_primitives::{
	ClusterBondingParams, ClusterFeesParams, ClusterGovParams, ClusterId, ClusterParams,
	ClusterPricingParams, NodePubKey, NodeType,
};
use frame_support::dispatch::DispatchResult;
use frame_system::Config;
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

pub trait ClusterVisitor<T: Config> {
	fn ensure_cluster(cluster_id: &ClusterId) -> Result<(), ClusterVisitorError>;

	fn get_bond_size(
		cluster_id: &ClusterId,
		node_type: NodeType,
	) -> Result<u128, ClusterVisitorError>;

	fn get_pricing_params(
		cluster_id: &ClusterId,
	) -> Result<ClusterPricingParams, ClusterVisitorError>;

	fn get_fees_params(cluster_id: &ClusterId) -> Result<ClusterFeesParams, ClusterVisitorError>;

	fn get_reserve_account_id(cluster_id: &ClusterId) -> Result<T::AccountId, ClusterVisitorError>;

	fn get_chill_delay(
		cluster_id: &ClusterId,
		node_type: NodeType,
	) -> Result<T::BlockNumber, ClusterVisitorError>;

	fn get_unbonding_delay(
		cluster_id: &ClusterId,
		node_type: NodeType,
	) -> Result<T::BlockNumber, ClusterVisitorError>;

	fn get_bonding_params(
		cluster_id: &ClusterId,
	) -> Result<ClusterBondingParams<T::BlockNumber>, ClusterVisitorError>;
}

pub trait ClusterCreator<T: Config, Balance> {
	fn create_new_cluster(
		cluster_id: ClusterId,
		cluster_manager_id: T::AccountId,
		cluster_reserve_id: T::AccountId,
		cluster_params: ClusterParams<T::AccountId>,
		cluster_gov_params: ClusterGovParams<Balance, T::BlockNumber>,
	) -> DispatchResult;
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum ClusterVisitorError {
	ClusterDoesNotExist,
	ClusterGovParamsNotSet,
}

pub trait ClusterManager<T: Config> {
	fn contains_node(cluster_id: &ClusterId, node_pub_key: &NodePubKey) -> bool;
	fn add_node(
		cluster_id: &ClusterId,
		node_pub_key: &NodePubKey,
	) -> Result<(), ClusterManagerError>;
	fn remove_node(
		cluster_id: &ClusterId,
		node_pub_key: &NodePubKey,
	) -> Result<(), ClusterManagerError>;
}

pub enum ClusterManagerError {
	AttemptToAddNonExistentNode,
	AttemptToAddAlreadyAssignedNode,
	AttemptToRemoveNotAssignedNode,
	AttemptToRemoveNonExistentNode,
}
