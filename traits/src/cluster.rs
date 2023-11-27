use codec::{Decode, Encode};
use ddc_primitives::{ClusterFeesParams, ClusterId, ClusterPricingParams, NodePubKey, NodeType};
use frame_system::Config;
use sp_runtime::RuntimeDebug;
use scale_info::TypeInfo;

pub trait ClusterVisitor<T: Config> {
	fn cluster_has_node(cluster_id: &ClusterId, node_pub_key: &NodePubKey) -> bool;

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
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum ClusterVisitorError {
	ClusterDoesNotExist,
	ClusterGovParamsNotSet,
}
