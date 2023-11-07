use ddc_primitives::{ClusterId, NodePubKey, NodeType};
use frame_system::Config;

pub trait ClusterVisitor<T: Config> {
	fn cluster_has_node(cluster_id: &ClusterId, node_pub_key: &NodePubKey) -> bool;

	fn ensure_cluster(cluster_id: &ClusterId) -> Result<(), ClusterVisitorError>;

	fn get_bond_size(
		cluster_id: &ClusterId,
		node_pub_key: NodeType,
	) -> Result<u128, ClusterVisitorError>;
}

pub enum ClusterVisitorError {
	ClusterDoesNotExist,
	ClusterGovParamsNotSet,
}
