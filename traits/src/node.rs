use ddc_primitives::{ClusterId, NodePubKey};
use frame_system::Config;

pub trait NodeVisitor<T: Config> {
	fn get_cluster_id(node_pub_key: &NodePubKey) -> Result<Option<ClusterId>, NodeVisitorError>;
}

pub enum NodeVisitorError {
	NodeDoesNotExist,
}
