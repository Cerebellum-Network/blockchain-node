use crate::pallet::Error;
use codec::{Decode, Encode};
use ddc_primitives::ClusterId;
use frame_support::{pallet_prelude::*, parameter_types, BoundedVec};
use scale_info::TypeInfo;
use sp_runtime::Perbill;
use sp_staking::EraIndex;
use sp_std::vec::Vec;

parameter_types! {
	pub MaxClusterParamsLen: u16 = 2048;
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct Cluster<AccountId> {
	pub cluster_id: ClusterId,
	pub manager_id: AccountId,
	pub props: ClusterProps<AccountId>,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterProps<AccountId> {
	// this is a temporal way of storing cluster parameters as a stringified json,
	// should be replaced with specific properties for cluster
	pub params: BoundedVec<u8, MaxClusterParamsLen>,
	pub node_provider_auth_contract: AccountId,
}

// ClusterParams includes Governance non-sensetive parameters only
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterParams<AccountId> {
	pub params: Vec<u8>,
	pub node_provider_auth_contract: AccountId,
}

// ClusterGovParams includes Governance sensetive parameters
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterGovParams<Balance> {
	pub treasury_share: Perbill,
	pub validators_share: Perbill,
	pub cluster_reserve_share: Perbill,
	#[codec(compact)]
	pub edge_bond_size: Balance,
	pub edge_chill_delay: EraIndex,
	pub edge_unbonding_delay: EraIndex,
	#[codec(compact)]
	pub storage_bond_size: Balance,
	pub storage_chill_delay: EraIndex,
	pub storage_unbonding_delay: EraIndex,
	pub unit_per_mb_stored: u128,
	pub unit_per_mb_streamed: u128,
	pub unit_per_put_request: u128,
	pub unit_per_get_request: u128,
}

impl<AccountId> Cluster<AccountId> {
	pub fn new(
		cluster_id: ClusterId,
		manager_id: AccountId,
		cluster_params: ClusterParams<AccountId>,
	) -> Result<Cluster<AccountId>, ClusterError> {
		Ok(Cluster {
			cluster_id,
			manager_id,
			props: ClusterProps {
				params: match cluster_params.params.try_into() {
					Ok(vec) => vec,
					Err(_) => return Err(ClusterError::ClusterParamsExceedsLimit),
				},
				node_provider_auth_contract: cluster_params.node_provider_auth_contract,
			},
		})
	}

	pub fn set_params(
		&mut self,
		cluster_params: ClusterParams<AccountId>,
	) -> Result<(), ClusterError> {
		self.props = ClusterProps {
			params: match cluster_params.params.try_into() {
				Ok(vec) => vec,
				Err(_) => return Err(ClusterError::ClusterParamsExceedsLimit),
			},
			node_provider_auth_contract: cluster_params.node_provider_auth_contract,
		};
		Ok(())
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
