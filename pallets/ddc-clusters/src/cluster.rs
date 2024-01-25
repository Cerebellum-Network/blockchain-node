use codec::{Decode, Encode};
use ddc_primitives::{ClusterId, ClusterParams, ClusterStatus};
use frame_support::{pallet_prelude::*, parameter_types};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

parameter_types! {
	pub MaxClusterParamsLen: u16 = 2048;
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct Cluster<AccountId> {
	pub cluster_id: ClusterId,
	pub manager_id: AccountId,
	pub reserve_id: AccountId,
	pub props: ClusterProps<AccountId>,
	pub status: ClusterStatus, // todo: provide migration
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterProps<AccountId> {
	pub node_provider_auth_contract: Option<AccountId>,
}

impl<AccountId> Cluster<AccountId> {
	pub fn new(
		cluster_id: ClusterId,
		manager_id: AccountId,
		reserve_id: AccountId,
		cluster_params: ClusterParams<AccountId>,
	) -> Cluster<AccountId> {
		Cluster {
			cluster_id,
			manager_id,
			reserve_id,
			props: ClusterProps {
				node_provider_auth_contract: cluster_params.node_provider_auth_contract,
			},
			status: ClusterStatus::Inactive,
		}
	}

	pub fn set_params(&mut self, cluster_params: ClusterParams<AccountId>) {
		self.props = ClusterProps {
			node_provider_auth_contract: cluster_params.node_provider_auth_contract,
		};
	}

	pub fn set_status(&mut self, status: ClusterStatus) {
		self.status = status;
	}
}
