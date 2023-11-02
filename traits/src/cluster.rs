use ddc_primitives::{ClusterId, NodePubKey};
use frame_system::Config;

pub trait ClusterVisitor<T: Config> {
	fn cluster_has_node(cluster_id: &ClusterId, node_pub_key: &NodePubKey) -> bool;

	fn ensure_cluster(cluster_id: &ClusterId) -> Result<(), ClusterVisitorError>;
}

pub enum ClusterVisitorError {
	ClusterDoesNotExist,
}
