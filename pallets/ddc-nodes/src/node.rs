#![allow(clippy::needless_lifetimes)] // ToDo

use crate::{
	cdn_node::{CDNNode, CDNNodeParams, CDNNodeProps},
	pallet::Error,
	storage_node::{StorageNode, StorageNodeParams, StorageNodeProps},
	ClusterId,
};
use codec::{Decode, Encode};
use ddc_primitives::{NodePubKey, NodeType};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum Node<T: frame_system::Config> {
	Storage(StorageNode<T>),
	CDN(CDNNode<T>),
}

// Params fields are always coming from extrinsic input
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodeParams {
	StorageParams(StorageNodeParams),
	CDNParams(CDNNodeParams),
}

// Props fields may include internal protocol properties
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodeProps {
	StorageProps(StorageNodeProps),
	CDNProps(CDNNodeProps),
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
			NodePubKey::CDNPubKey(_) =>
				CDNNode::new(node_pub_key, provider_id, node_params).map(|n| Node::CDN(n)),
		}
	}
}

impl<T: frame_system::Config> NodeTrait<T> for Node<T> {
	fn get_pub_key(&self) -> NodePubKey {
		match &self {
			Node::Storage(node) => node.get_pub_key(),
			Node::CDN(node) => node.get_pub_key(),
		}
	}
	fn get_provider_id(&self) -> &T::AccountId {
		match &self {
			Node::Storage(node) => node.get_provider_id(),
			Node::CDN(node) => node.get_provider_id(),
		}
	}
	fn get_props(&self) -> NodeProps {
		match &self {
			Node::Storage(node) => node.get_props(),
			Node::CDN(node) => node.get_props(),
		}
	}
	fn set_props(&mut self, props: NodeProps) -> Result<(), NodeError> {
		match self {
			Node::Storage(node) => node.set_props(props),
			Node::CDN(node) => node.set_props(props),
		}
	}
	fn set_params(&mut self, params: NodeParams) -> Result<(), NodeError> {
		match self {
			Node::Storage(node) => node.set_params(params),
			Node::CDN(node) => node.set_params(params),
		}
	}
	fn get_cluster_id(&self) -> &Option<ClusterId> {
		match &self {
			Node::Storage(node) => node.get_cluster_id(),
			Node::CDN(node) => node.get_cluster_id(),
		}
	}
	fn set_cluster_id(&mut self, cluster_id: Option<ClusterId>) {
		match self {
			Node::Storage(node) => node.set_cluster_id(cluster_id),
			Node::CDN(node) => node.set_cluster_id(cluster_id),
		}
	}
	fn get_type(&self) -> NodeType {
		match &self {
			Node::Storage(node) => node.get_type(),
			Node::CDN(node) => node.get_type(),
		}
	}
}

pub enum NodeError {
	InvalidStorageNodePubKey,
	InvalidCDNNodePubKey,
	InvalidStorageNodeParams,
	InvalidCDNNodeParams,
	StorageHostLenExceedsLimit,
	CDNHostLenExceedsLimit,
	InvalidCDNNodeProps,
	InvalidStorageNodeProps,
}

impl<T> From<NodeError> for Error<T> {
	fn from(error: NodeError) -> Self {
		match error {
			NodeError::InvalidStorageNodePubKey => Error::<T>::InvalidNodePubKey,
			NodeError::InvalidCDNNodePubKey => Error::<T>::InvalidNodePubKey,
			NodeError::InvalidStorageNodeParams => Error::<T>::InvalidNodeParams,
			NodeError::InvalidCDNNodeParams => Error::<T>::InvalidNodeParams,
			NodeError::StorageHostLenExceedsLimit => Error::<T>::HostLenExceedsLimit,
			NodeError::CDNHostLenExceedsLimit => Error::<T>::HostLenExceedsLimit,
			NodeError::InvalidStorageNodeProps => Error::<T>::InvalidNodeParams,
			NodeError::InvalidCDNNodeProps => Error::<T>::InvalidNodeParams,
		}
	}
}
