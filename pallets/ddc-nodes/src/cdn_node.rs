use crate::{
	node::{Node, NodeParams, NodePropsRef, NodePubKeyRef, NodeTrait, NodeType},
	pallet::Error,
	ClusterId, Config,
};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

pub type CDNNodePubKey = sp_runtime::AccountId32;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct CDNNode {
	pub pub_key: CDNNodePubKey,
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

impl NodeTrait for CDNNode {
	fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a> {
		NodePubKeyRef::CDNPubKeyRef(&self.pub_key)
	}
	fn get_props<'a>(&'a self) -> NodePropsRef<'a> {
		NodePropsRef::CDNPropsRef(&self.props)
	}
	fn get_type(&self) -> NodeType {
		NodeType::CDN
	}
	fn from_params<T: Config>(params: NodeParams) -> Result<Node, Error<T>> {
		match params {
			NodeParams::CDNParams(params) => Ok(Node::CDN(CDNNode {
				pub_key: params.pub_key,
				cluster_id: None,
				props: CDNNodeProps { url: params.url, location: params.location },
			})),
			_ => Err(Error::<T>::NodeAlreadyExists),
		}
	}
}
