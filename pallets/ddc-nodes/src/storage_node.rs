use ddc_primitives::{
	ClusterId, NodeError, NodeParams, NodePubKey, NodeType, StorageNode,
};

use crate::node::{NodeProps, NodeTrait};

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
