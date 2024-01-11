use codec::{Decode, Encode};
use ddc_primitives::{ClusterId, ClusterParams};
use frame_support::{pallet_prelude::*, parameter_types};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use crate::pallet::Error;

parameter_types! {
	pub MaxClusterParamsLen: u16 = 2048;
}

/// DDC cluster data structure.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct Cluster<AccountId> {
	/// Hash-based cluster identifier.
	pub cluster_id: ClusterId,
	/// Cluster manager account.
	pub manager_id: AccountId,
	/// Cluster reserve account.
	pub reserve_id: AccountId,
	/// Cluster operational parameters.
	pub props: ClusterProps<AccountId>,
}

/// DDC cluster operational parameters.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterProps<AccountId> {
	/// Additional authorization contract for DDC nodes.
	pub node_provider_auth_contract: Option<AccountId>,
}

impl<AccountId> Cluster<AccountId> {
	/// DDC cluster constructor.
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
			},
		})
	}
	/// DDC cluster operational parameters setter.
	pub fn set_params(
		&mut self,
		cluster_params: ClusterParams<AccountId>,
	) -> Result<(), ClusterError> {
		self.props = ClusterProps {
			node_provider_auth_contract: cluster_params.node_provider_auth_contract,
		};
		Ok(())
	}
}

/// DDC cluster error.
pub enum ClusterError {
	/// Cluster operational parameters size exceeds the limit.
	ClusterParamsExceedsLimit,
}

impl<T> From<ClusterError> for Error<T> {
	fn from(error: ClusterError) -> Self {
		match error {
			ClusterError::ClusterParamsExceedsLimit => Error::<T>::ClusterParamsExceedsLimit,
		}
	}
}
