use frame_support::dispatch::DispatchResult;
use frame_system::Config;
use scale_info::{prelude::vec::Vec, TypeInfo};
use crate::{ClusterId, NodeMode, NodeParams, NodePubKey, StorageNode};

pub trait NodeVisitor<T: Config> {
	fn get_cluster_id(node_pub_key: &NodePubKey) -> Result<Option<ClusterId>, NodeVisitorError>;
	fn exists(node_pub_key: &NodePubKey) -> bool;
	fn get_nodes(mode: NodeMode) -> Vec<StorageNode<T>>;
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
