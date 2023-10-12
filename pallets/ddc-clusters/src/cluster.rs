use crate::pallet::Error;
use codec::{Decode, Encode};
use frame_support::{pallet_prelude::*, parameter_types, BoundedVec};
use scale_info::TypeInfo;
use sp_core::hash::H160;
use sp_std::vec::Vec;

pub type ClusterId = H160;
parameter_types! {
	pub MaxClusterParamsLen: u16 = 2048;
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct Cluster<ManagerId> {
	pub cluster_id: ClusterId,
	pub manager_id: ManagerId,
	pub props: ClusterProps,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterProps {
	// this is a temporal way of storing cluster parameters as a stringified json,
	// should be replaced with specific properties for cluster
	pub params: BoundedVec<u8, MaxClusterParamsLen>,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterParams {
	pub params: Vec<u8>,
}

impl<ManagerId> Cluster<ManagerId> {
	pub fn new(
		cluster_id: ClusterId,
		manager_id: ManagerId,
		cluster_params: ClusterParams,
	) -> Result<Cluster<ManagerId>, ClusterError> {
		Ok(Cluster {
			cluster_id,
			manager_id,
			props: ClusterProps {
				params: match cluster_params.params.try_into() {
					Ok(vec) => vec,
					Err(_) => return Err(ClusterError::ClusterParamsExceedsLimit),
				},
			},
		})
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
