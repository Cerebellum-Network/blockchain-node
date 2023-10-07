use crate::{
	node::{Node, NodeError, NodeParams, NodePropsRef, NodePubKeyRef, NodeTrait, NodeType},
	ClusterId,
};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{AccountId32, RuntimeDebug};

pub type StorageNodePubKey = AccountId32;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct StorageNode<ProviderId> {
	pub pub_key: StorageNodePubKey,
	pub provider_id: ProviderId,
	pub cluster_id: Option<ClusterId>,
	pub props: StorageNodeProps,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct StorageNodeProps {
	pub capacity: u32,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct StorageNodeParams {
	pub pub_key: StorageNodePubKey,
	pub capacity: u32,
}

impl<ProviderId> NodeTrait<ProviderId> for StorageNode<ProviderId> {
	fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a> {
		NodePubKeyRef::StoragePubKeyRef(&self.pub_key)
	}
	fn get_props<'a>(&'a self) -> NodePropsRef<'a> {
		NodePropsRef::StoragePropsRef(&self.props)
	}
	fn get_type(&self) -> NodeType {
		NodeType::Storage
	}
	fn from_params(
		provider_id: ProviderId,
		params: NodeParams,
	) -> Result<Node<ProviderId>, NodeError> {
		match params {
			NodeParams::StorageParams(params) => Ok(Node::Storage(StorageNode::<ProviderId> {
				provider_id,
				pub_key: params.pub_key,
				cluster_id: None,
				props: StorageNodeProps { capacity: params.capacity },
			})),
			_ => Err(NodeError::InvalidStorageNodeParams),
		}
	}
}
