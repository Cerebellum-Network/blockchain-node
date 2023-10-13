use crate::{
	node::{
		Node, NodeError, NodeParams, NodeProps, NodePropsRef, NodePubKey, NodePubKeyRef, NodeTrait,
		NodeType,
	},
	ClusterId,
};
use codec::{Decode, Encode};
use frame_support::{parameter_types, BoundedVec};
use scale_info::TypeInfo;
use sp_runtime::{AccountId32, RuntimeDebug};
use sp_std::prelude::Vec;

pub type StorageNodePubKey = AccountId32;
parameter_types! {
	pub MaxStorageNodeParamsLen: u16 = 2048;
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct StorageNode<ProviderId> {
	pub pub_key: StorageNodePubKey,
	pub provider_id: ProviderId,
	pub cluster_id: Option<ClusterId>,
	pub props: StorageNodeProps,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct StorageNodeProps {
	// this is a temporal way of storing node parameters as a stringified json,
	// should be replaced with specific properties for this type of node once they are defined
	pub params: BoundedVec<u8, MaxStorageNodeParamsLen>,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct StorageNodeParams {
	pub params: Vec<u8>, // should be replaced with specific parameters for this type of node
}

impl<ProviderId> NodeTrait<ProviderId> for StorageNode<ProviderId> {
	fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a> {
		NodePubKeyRef::StoragePubKeyRef(&self.pub_key)
	}
	fn get_provider_id(&self) -> &ProviderId {
		&self.provider_id
	}
	fn get_props<'a>(&'a self) -> NodePropsRef<'a> {
		NodePropsRef::StoragePropsRef(&self.props)
	}
	fn set_props(&mut self, props: NodeProps) -> Result<(), NodeError> {
		self.props = match props {
			NodeProps::StorageProps(props) => props,
			_ => return Err(NodeError::InvalidStorageNodeProps),
		};
		Ok(())
	}
	fn set_params(&mut self, node_params: NodeParams) -> Result<(), NodeError> {
		self.props.params = match node_params {
			NodeParams::StorageParams(cdn_params) => match cdn_params.params.try_into() {
				Ok(vec) => vec,
				Err(_) => return Err(NodeError::StorageNodeParamsExceedsLimit),
			},
			_ => return Err(NodeError::InvalidStorageNodeParams),
		};
		Ok(())
	}
	fn get_cluster_id(&self) -> &Option<ClusterId> {
		&self.cluster_id
	}
	fn set_cluster_id(&mut self, cluster_id: ClusterId) {
		self.cluster_id = Some(cluster_id);
	}
	fn get_type(&self) -> NodeType {
		NodeType::Storage
	}
	fn new(
		node_pub_key: NodePubKey,
		provider_id: ProviderId,
		node_params: NodeParams,
	) -> Result<Node<ProviderId>, NodeError> {
		match node_pub_key {
			NodePubKey::StoragePubKey(pub_key) => match node_params {
				NodeParams::StorageParams(node_params) =>
					Ok(Node::Storage(StorageNode::<ProviderId> {
						provider_id,
						pub_key,
						cluster_id: None,
						props: StorageNodeProps {
							params: match node_params.params.try_into() {
								Ok(vec) => vec,
								Err(_) => return Err(NodeError::StorageNodeParamsExceedsLimit),
							},
						},
					})),
				_ => Err(NodeError::InvalidStorageNodeParams),
			},
			_ => Err(NodeError::InvalidStorageNodePubKey),
		}
	}
}
