use codec::{Decode, Encode};
<<<<<<< HEAD
<<<<<<< HEAD
use ddc_primitives::{ClusterId, ClusterParams, ClusterStatus, DdcEra};
=======
use ddc_primitives::{ClusterId, ClusterParams, ClusterStatus};
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
=======
use ddc_primitives::{ClusterId, ClusterParams, ClusterStatus, DdcEra};
>>>>>>> 99095ecd (verified copy of PR#393 (#402))
use frame_support::{pallet_prelude::*, parameter_types};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

parameter_types! {
	pub MaxClusterParamsLen: u16 = 2048;
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Serialize, Deserialize)]
pub struct Cluster<AccountId> {
	pub cluster_id: ClusterId,
	pub manager_id: AccountId,
	pub reserve_id: AccountId,
	pub props: ClusterProps<AccountId>,
	pub status: ClusterStatus,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
	pub last_paid_era: DdcEra,
=======
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
=======
=======
	// todo(yahortsaryk): this should be renamed to `last_paid_era` to eliminate ambiguity,
	// as the validation step is decoupled from payout step.
>>>>>>> 342b5f27 (chore: methods for last paid removed to eliminate ambiguity)
=======
	// todo(yahortsaryk): `last_validated_era_id` should be renamed to `last_paid_era` to eliminate
	// ambiguity, as the validation step is decoupled from payout step.
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
	pub last_validated_era_id: DdcEra,
>>>>>>> 99095ecd (verified copy of PR#393 (#402))
=======
	pub last_paid_era: DdcEra,
>>>>>>> beaea12a (chore: addressing PR comments)
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
	) -> Cluster<AccountId> {
		Cluster {
			cluster_id,
			manager_id,
			reserve_id,
			props: ClusterProps {
				node_provider_auth_contract: cluster_params.node_provider_auth_contract,
				erasure_coding_required: cluster_params.erasure_coding_required,
				erasure_coding_total: cluster_params.erasure_coding_total,
				replication_total: cluster_params.replication_total,
			},
<<<<<<< HEAD
<<<<<<< HEAD
			status: ClusterStatus::Unbonded,
			last_paid_era: DdcEra::default(),
=======
		})
=======
			status: ClusterStatus::Unbonded,
			last_paid_era: DdcEra::default(),
		}
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
	}

	pub fn set_params(&mut self, cluster_params: ClusterParams<AccountId>) {
		self.props = ClusterProps {
			node_provider_auth_contract: cluster_params.node_provider_auth_contract,
			erasure_coding_required: cluster_params.erasure_coding_required,
			erasure_coding_total: cluster_params.erasure_coding_total,
			replication_total: cluster_params.replication_total,
		};
<<<<<<< HEAD
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
>>>>>>> b1afc1d4 (Extended Cluster pallet by Cluster Configuration parameters (#332))
		}
=======
	}

	pub fn set_status(&mut self, status: ClusterStatus) {
		self.status = status;
	}

	pub fn can_manage_nodes(&self) -> bool {
		matches!(self.status, ClusterStatus::Bonded | ClusterStatus::Activated)
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
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

	pub fn can_manage_nodes(&self) -> bool {
		matches!(self.status, ClusterStatus::Bonded | ClusterStatus::Activated)
	}
}
