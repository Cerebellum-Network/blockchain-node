use codec::{Decode, Encode};
use ddc_primitives::{
	ClusterId, NodeParams, NodePubKey, NodeType, StorageNodeMode, StorageNodePubKey,
};
use frame_support::{parameter_types, BoundedVec};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

use crate::node::{NodeError, NodeProps, NodeTrait};

parameter_types! {
	pub MaxHostLen: u8 = 255;
	pub MaxDomainLen: u8 = 255;
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
#[scale_info(skip_type_params(T))]
pub struct StorageNode<T: frame_system::Config> {
	pub pub_key: StorageNodePubKey,
	pub provider_id: T::AccountId,
	pub cluster_id: Option<ClusterId>,
	pub props: StorageNodeProps,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct StorageNodeProps {
	pub host: BoundedVec<u8, MaxHostLen>,
	pub domain: BoundedVec<u8, MaxDomainLen>,
	pub ssl: bool,
	pub http_port: u16,
	pub grpc_port: u16,
	pub p2p_port: u16,
	pub mode: StorageNodeMode,
}

impl<T: frame_system::Config> StorageNode<T> {
	pub fn new(
		node_pub_key: NodePubKey,
		provider_id: T::AccountId,
		node_params: NodeParams,
	) -> Result<Self, NodeError> {
		match node_pub_key {
			NodePubKey::StoragePubKey(pub_key) => match node_params {
				NodeParams::StorageParams(node_params) => Ok(StorageNode::<T> {
					provider_id,
					pub_key,
					cluster_id: None,
					props: StorageNodeProps {
						mode: node_params.mode,
						host: match node_params.host.try_into() {
							Ok(vec) => vec,
							Err(_) => return Err(NodeError::StorageHostLenExceedsLimit),
						},
						domain: match node_params.domain.try_into() {
							Ok(vec) => vec,
							Err(_) => return Err(NodeError::StorageDomainLenExceedsLimit),
						},
						ssl: node_params.ssl,
						http_port: node_params.http_port,
						grpc_port: node_params.grpc_port,
						p2p_port: node_params.p2p_port,
					},
				}),
			},
		}
	}
}

impl<T: frame_system::Config> NodeTrait<T> for StorageNode<T> {
	fn get_pub_key(&self) -> NodePubKey {
		NodePubKey::StoragePubKey(self.pub_key.clone())
	}
	fn get_provider_id(&self) -> &T::AccountId {
		&self.provider_id
	}
	fn get_props(&self) -> NodeProps {
		NodeProps::StorageProps(self.props.clone())
	}
	fn set_props(&mut self, props: NodeProps) -> Result<(), NodeError> {
		self.props = match props {
			NodeProps::StorageProps(props) => props,
		};
		Ok(())
	}
	fn set_params(&mut self, node_params: NodeParams) -> Result<(), NodeError> {
		match node_params {
			NodeParams::StorageParams(storage_params) => {
				self.props.mode = storage_params.mode;
				self.props.host = match storage_params.host.try_into() {
					Ok(vec) => vec,
					Err(_) => return Err(NodeError::StorageHostLenExceedsLimit),
				};
				self.props.domain = match storage_params.domain.try_into() {
					Ok(vec) => vec,
					Err(_) => return Err(NodeError::StorageDomainLenExceedsLimit),
				};
				self.props.ssl = storage_params.ssl;
				self.props.http_port = storage_params.http_port;
				self.props.grpc_port = storage_params.grpc_port;
				self.props.p2p_port = storage_params.p2p_port;
			},
		};
		Ok(())
	}
	fn get_cluster_id(&self) -> &Option<ClusterId> {
		&self.cluster_id
	}
	fn set_cluster_id(&mut self, cluster_id: Option<ClusterId>) {
		self.cluster_id = cluster_id;
	}
	fn get_type(&self) -> NodeType {
		NodeType::Storage
	}
}
