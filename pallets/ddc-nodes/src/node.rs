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

/// DDC node type enum.
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum Node<T: frame_system::Config> {
	/// DDC Storage node variant.
	Storage(StorageNode<T>),
}

/// DDC Storage node properties that include non-input fields.
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodeProps {
	/// DDC Storage node properties.
	StorageProps(StorageNodeProps),
}

/// DDC node trait.
pub trait NodeTrait<T: frame_system::Config> {
	/// DDC node public key getter.
	fn get_pub_key(&self) -> NodePubKey;
	/// DDC node provider account getter.
	fn get_provider_id(&self) -> &T::AccountId;
	/// DDC node properties getter.
	fn get_props(&self) -> NodeProps;
	/// DDC node properties setter.
	fn set_props(&mut self, props: NodeProps) -> Result<(), NodeError>;
	/// DDC node parameters setter.
	fn set_params(&mut self, props: NodeParams) -> Result<(), NodeError>;
	/// Assigned cluster getter.
	fn get_cluster_id(&self) -> &Option<ClusterId>;
	/// Assigned cluster setter.
	fn set_cluster_id(&mut self, cluster_id: Option<ClusterId>);
	/// DDC node type getter.
	fn get_type(&self) -> NodeType;
}

impl<T: frame_system::Config> Node<T> {
	/// DDC node constructor.
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

/// DDC node error.
#[derive(Debug, PartialEq)]
pub enum NodeError {
	/// DDC node host length exceeds the limit.
	StorageHostLenExceedsLimit,
	/// DDC node domain length exceeds the limit.
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
