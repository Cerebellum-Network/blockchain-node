#![allow(clippy::needless_lifetimes)] // ToDo

use crate::{
	cdn_node::{CDNNode, CDNNodeParams, CDNNodeProps},
	pallet::Error,
	storage_node::{StorageNode, StorageNodeParams, StorageNodeProps},
	ClusterId,
};
use codec::{Decode, Encode};
use ddc_primitives::{CDNNodePubKey, NodePubKey, StorageNodePubKey};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum Node<AccountId> {
	Storage(StorageNode<AccountId>),
	CDN(CDNNode<AccountId>),
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

#[derive(Clone, RuntimeDebug, PartialEq)]
pub enum NodePubKeyRef<'a> {
	StoragePubKeyRef(&'a StorageNodePubKey),
	CDNPubKeyRef(&'a CDNNodePubKey),
}

impl<'a> NodePubKeyRef<'a> {
	pub fn to_owned(&self) -> NodePubKey {
		match &self {
			NodePubKeyRef::StoragePubKeyRef(pub_key_ref) =>
				NodePubKey::StoragePubKey((**pub_key_ref).clone()),
			NodePubKeyRef::CDNPubKeyRef(pub_key_ref) =>
				NodePubKey::CDNPubKey((**pub_key_ref).clone()),
		}
	}
}

#[derive(Clone, RuntimeDebug, PartialEq)]
pub enum NodePropsRef<'a> {
	StoragePropsRef(&'a StorageNodeProps),
	CDNPropsRef(&'a CDNNodeProps),
}

pub trait NodeTrait<AccountId> {
	fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a>;
	fn get_provider_id(&self) -> &AccountId;
	fn get_props<'a>(&'a self) -> NodePropsRef<'a>;
	fn set_props(&mut self, props: NodeProps) -> Result<(), NodeError>;
	fn set_params(&mut self, props: NodeParams) -> Result<(), NodeError>;
	fn get_cluster_id(&self) -> &Option<ClusterId>;
	fn set_cluster_id(&mut self, cluster_id: Option<ClusterId>);
	fn get_type(&self) -> NodeType;
	fn new(
		node_pub_key: NodePubKey,
		provider_id: AccountId,
		params: NodeParams,
	) -> Result<Node<AccountId>, NodeError>;
}

impl<AccountId> NodeTrait<AccountId> for Node<AccountId> {
	fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a> {
		match &self {
			Node::Storage(node) => node.get_pub_key(),
			Node::CDN(node) => node.get_pub_key(),
		}
	}
	fn get_provider_id(&self) -> &AccountId {
		match &self {
			Node::Storage(node) => node.get_provider_id(),
			Node::CDN(node) => node.get_provider_id(),
		}
	}
	fn get_props<'a>(&'a self) -> NodePropsRef<'a> {
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
	fn new(
		node_pub_key: NodePubKey,
		provider_id: AccountId,
		node_params: NodeParams,
	) -> Result<Node<AccountId>, NodeError> {
		match node_pub_key {
			NodePubKey::StoragePubKey(_) =>
				StorageNode::new(node_pub_key, provider_id, node_params),
			NodePubKey::CDNPubKey(_) => CDNNode::new(node_pub_key, provider_id, node_params),
		}
	}
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodeType {
	Storage = 1,
	CDN = 2,
}

impl From<NodeType> for u8 {
	fn from(node_type: NodeType) -> Self {
		match node_type {
			NodeType::Storage => 1,
			NodeType::CDN => 2,
		}
	}
}

impl TryFrom<u8> for NodeType {
	type Error = ();
	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			1 => Ok(NodeType::Storage),
			2 => Ok(NodeType::CDN),
			_ => Err(()),
		}
	}
}

pub enum NodeError {
	InvalidStorageNodePubKey,
	InvalidCDNNodePubKey,
	InvalidStorageNodeParams,
	InvalidCDNNodeParams,
	StorageNodeParamsExceedsLimit,
	CDNNodeParamsExceedsLimit,
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
			NodeError::StorageNodeParamsExceedsLimit => Error::<T>::NodeParamsExceedsLimit,
			NodeError::CDNNodeParamsExceedsLimit => Error::<T>::InvalidNodeParams,
			NodeError::InvalidStorageNodeProps => Error::<T>::InvalidNodeParams,
			NodeError::InvalidCDNNodeProps => Error::<T>::InvalidNodeParams,
		}
	}
}
