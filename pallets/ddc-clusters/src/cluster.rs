use codec::{Decode, Encode};
use ddc_primitives::{ClusterId, ClusterParams, ClusterStatus};
use frame_support::{pallet_prelude::*, parameter_types};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

use crate::pallet::Error;

parameter_types! {
	pub MaxClusterParamsLen: u16 = 2048;
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
pub struct Cluster<AccountId> {
	pub cluster_id: ClusterId,
	pub manager_id: AccountId,
	pub reserve_id: AccountId,
	pub props: ClusterProps<AccountId>,
	pub status: ClusterStatus, // todo: provide migration
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
pub struct ClusterProps<AccountId> {
	pub node_provider_auth_contract: Option<AccountId>,
	pub erasure_coding_required: u32,
	pub erasure_coding_total: u32,
	pub replication_total: u32,
}

impl<AccountId> Cluster<AccountId> {
	pub fn new(
		cluster_id: ClusterId,
		manager_id: AccountId,
		reserve_id: AccountId,
		cluster_params: ClusterParams<AccountId>,
	) -> Result<Cluster<AccountId>, ClusterError> {
		Ok(Cluster {
			cluster_id,
			manager_id,
			reserve_id,
			props: ClusterProps {
				node_provider_auth_contract: cluster_params.node_provider_auth_contract,
				erasure_coding_required: cluster_params.erasure_coding_required,
				erasure_coding_total: cluster_params.erasure_coding_total,
				replication_total: cluster_params.replication_total,
			},
			status: ClusterStatus::Inactive,
		})
	}

	pub fn set_params(&mut self, cluster_params: ClusterParams<AccountId>) {
		self.props = ClusterProps {
			node_provider_auth_contract: cluster_params.node_provider_auth_contract,
			erasure_coding_required: cluster_params.erasure_coding_required,
			erasure_coding_total: cluster_params.erasure_coding_total,
			replication_total: cluster_params.replication_total,
		};
	}

	pub fn set_status(&mut self, status: ClusterStatus) {
		self.status = status;
	}
}

pub enum ClusterError {
	ClusterParamsExceedsLimit,
}

impl<T> From<ClusterError> for Error<T> {
	fn from(error: ClusterError) -> Self {
		match error {
			ClusterError::ClusterParamsExceedsLimit => Error::<T>::ClusterParamsExceedsLimit,
		}
	}
}
