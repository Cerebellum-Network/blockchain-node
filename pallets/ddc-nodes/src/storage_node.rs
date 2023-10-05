use crate::{
	node::{Node, NodeParams, NodePropsRef, NodePubKeyRef, NodeTrait, NodeType},
	pallet::Error,
	ClusterId, Config,
};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

pub type StorageNodePubKey = sp_runtime::AccountId32;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct StorageNode {
	pub pub_key: StorageNodePubKey,
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

impl NodeTrait for StorageNode {
	fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a> {
		NodePubKeyRef::StoragePubKeyRef(&self.pub_key)
	}
	fn get_props<'a>(&'a self) -> NodePropsRef<'a> {
		NodePropsRef::StoragePropsRef(&self.props)
	}
	fn get_type(&self) -> NodeType {
		NodeType::Storage
	}
	fn from_params<T: Config>(params: NodeParams) -> Result<Node, Error<T>> {
		match params {
			NodeParams::StorageParams(params) => Ok(Node::Storage(StorageNode {
				pub_key: params.pub_key,
				cluster_id: None,
				props: StorageNodeProps { capacity: params.capacity },
			})),
			_ => Err(Error::<T>::NodeAlreadyExists),
		}
	}
}
