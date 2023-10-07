use crate::{
	node::{Node, NodeError, NodeParams, NodePropsRef, NodePubKeyRef, NodeTrait, NodeType},
	pallet::Error,
	ClusterId,
};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

pub type CDNNodePubKey = sp_runtime::AccountId32;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct CDNNode<ProviderId> {
	pub pub_key: CDNNodePubKey,
	pub provider_id: ProviderId,
	pub cluster_id: Option<ClusterId>,
	pub props: CDNNodeProps,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct CDNNodeProps {
	pub url: Vec<u8>,
	pub location: [u8; 2],
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct CDNNodeParams {
	pub pub_key: CDNNodePubKey,
	pub url: Vec<u8>,
	pub location: [u8; 2],
}

impl<ProviderId> NodeTrait<ProviderId> for CDNNode<ProviderId> {
	fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a> {
		NodePubKeyRef::CDNPubKeyRef(&self.pub_key)
	}
	fn get_props<'a>(&'a self) -> NodePropsRef<'a> {
		NodePropsRef::CDNPropsRef(&self.props)
	}
	fn get_type(&self) -> NodeType {
		NodeType::CDN
	}
	fn from_params(
		provider_id: ProviderId,
		params: NodeParams,
	) -> Result<Node<ProviderId>, NodeError> {
		match params {
			NodeParams::CDNParams(params) => Ok(Node::CDN(CDNNode::<ProviderId> {
				provider_id,
				pub_key: params.pub_key,
				cluster_id: None,
				props: CDNNodeProps { url: params.url, location: params.location },
			})),
			_ => Err(NodeError::InvalidCDNNodeParams),
		}
	}
}
