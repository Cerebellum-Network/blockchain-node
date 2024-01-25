use frame_support::dispatch::DispatchResult;
use frame_system::Config;

use crate::{ClusterId, NodeParams, NodePubKey};

pub trait NodeVisitor<T: Config> {
	fn get_cluster_id(node_pub_key: &NodePubKey) -> Result<Option<ClusterId>, NodeVisitorError>;
	fn exists(node_pub_key: &NodePubKey) -> bool;
}

pub trait NodeCreator<T: Config> {
	fn create_node(
		node_pub_key: NodePubKey,
		provider_id: T::AccountId,
		node_params: NodeParams,
	) -> DispatchResult;
}

pub enum NodeVisitorError {
	NodeDoesNotExist,
}
