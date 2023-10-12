use crate::{
	node::{Node, NodeError, NodeParams, NodePropsRef, NodePubKeyRef, NodeTrait, NodeType},
	ClusterId,
};
use codec::{Decode, Encode};
use frame_support::{parameter_types, BoundedVec};
use scale_info::TypeInfo;
use sp_runtime::{AccountId32, RuntimeDebug};
use sp_std::prelude::*;

pub type CDNNodePubKey = AccountId32;
parameter_types! {
	pub MaxCDNNodeParamsLen: u16 = 2048;
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct CDNNode<ProviderId> {
	pub pub_key: CDNNodePubKey,
	pub provider_id: ProviderId,
	pub cluster_id: Option<ClusterId>,
	pub props: CDNNodeProps,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct CDNNodeProps {
	// this is a temporal way of storing node parameters as a stringified json,
	// should be replaced with specific properties for this type of node once they are defined
	pub params: BoundedVec<u8, MaxCDNNodeParamsLen>,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct CDNNodeParams {
	pub pub_key: CDNNodePubKey,
	pub params: Vec<u8>, // should be replaced with specific parameters for this type of node
}

impl<ProviderId> NodeTrait<ProviderId> for CDNNode<ProviderId> {
	fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a> {
		NodePubKeyRef::CDNPubKeyRef(&self.pub_key)
	}
	fn get_provider_id(&self) -> &ProviderId {
		&self.provider_id
	}
	fn get_props<'a>(&'a self) -> NodePropsRef<'a> {
		NodePropsRef::CDNPropsRef(&self.props)
	}
	fn get_cluster_id(&self) -> &Option<ClusterId> {
		&self.cluster_id
	}
	fn set_cluster_id(&mut self, cluster_id: ClusterId) {
		self.cluster_id = Some(cluster_id);
	}
	fn get_type(&self) -> NodeType {
		NodeType::CDN
	}
	fn new(
		provider_id: ProviderId,
		node_params: NodeParams,
	) -> Result<Node<ProviderId>, NodeError> {
		match node_params {
			NodeParams::CDNParams(node_params) => Ok(Node::CDN(CDNNode::<ProviderId> {
				provider_id,
				pub_key: node_params.pub_key,
				cluster_id: None,
				props: CDNNodeProps {
					params: match node_params.params.try_into() {
						Ok(vec) => vec,
						Err(_) => return Err(NodeError::CDNNodeParamsExceedsLimit),
					},
				},
			})),
			_ => Err(NodeError::InvalidCDNNodeParams),
		}
	}
}
