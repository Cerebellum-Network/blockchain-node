#![allow(clippy::needless_lifetimes)] // ToDo

use codec::{Decode, Encode};
use ddc_primitives::{NodeParams, NodePubKey, NodeType};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

use crate::{
	pallet::Error,
	storage_node::{StorageNode, StorageNodeProps},
	ClusterId,
};

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum Node<T: frame_system::Config> {
	Storage(StorageNode<T>),
}

// Props fields may include internal protocol properties
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodeProps {
	StorageProps(StorageNodeProps),
}

pub trait NodeTrait<T: frame_system::Config> {
	fn get_pub_key(&self) -> NodePubKey;
	fn get_provider_id(&self) -> &T::AccountId;
	fn get_props(&self) -> NodeProps;
	fn set_props(&mut self, props: NodeProps) -> Result<(), NodeError>;
	fn set_params(&mut self, props: NodeParams) -> Result<(), NodeError>;
	fn get_cluster_id(&self) -> &Option<ClusterId>;
	fn set_cluster_id(&mut self, cluster_id: Option<ClusterId>);
	fn get_type(&self) -> NodeType;
}

impl<T: frame_system::Config> Node<T> {
	pub fn new(
		node_pub_key: NodePubKey,
		provider_id: T::AccountId,
		node_params: NodeParams,
	) -> Result<Self, NodeError> {
		match node_pub_key {
			NodePubKey::StoragePubKey(_) =>
				StorageNode::new(node_pub_key, provider_id, node_params).map(|n| Node::Storage(n)),
		}
	}
}

impl<T: frame_system::Config> NodeTrait<T> for Node<T> {
	fn get_pub_key(&self) -> NodePubKey {
		match &self {
			Node::Storage(node) => node.get_pub_key(),
		}
	}
	fn get_provider_id(&self) -> &T::AccountId {
		match &self {
			Node::Storage(node) => node.get_provider_id(),
		}
	}
	fn get_props(&self) -> NodeProps {
		match &self {
			Node::Storage(node) => node.get_props(),
		}
	}
	fn set_props(&mut self, props: NodeProps) -> Result<(), NodeError> {
		match self {
			Node::Storage(node) => node.set_props(props),
		}
	}
	fn set_params(&mut self, params: NodeParams) -> Result<(), NodeError> {
		match self {
			Node::Storage(node) => node.set_params(params),
		}
	}
	fn get_cluster_id(&self) -> &Option<ClusterId> {
		match &self {
			Node::Storage(node) => node.get_cluster_id(),
		}
	}
	fn set_cluster_id(&mut self, cluster_id: Option<ClusterId>) {
		match self {
			Node::Storage(node) => node.set_cluster_id(cluster_id),
		}
	}
	fn get_type(&self) -> NodeType {
		match &self {
			Node::Storage(node) => node.get_type(),
		}
	}
}

#[derive(Debug, PartialEq)]
pub enum NodeError {
	StorageHostLenExceedsLimit,
	StorageDomainLenExceedsLimit,
}

impl<T> From<NodeError> for Error<T> {
	fn from(error: NodeError) -> Self {
		match error {
			NodeError::StorageHostLenExceedsLimit => Error::<T>::HostLenExceedsLimit,
			NodeError::StorageDomainLenExceedsLimit => Error::<T>::DomainLenExceedsLimit,
		}
	}
}
