//! # DDC Nodes Pallet
//!
//! The DDC Clusters pallet is used to manage DDC Clusters
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## GenesisConfig
//!
//! The DDC Clusters pallet depends on the [`GenesisConfig`]. The
//! `GenesisConfig` is optional and allow to set some initial nodes in DDC.
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]
#![allow(clippy::manual_inspect)]
pub mod weights;
use crate::weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod testing_utils;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

pub mod migrations;
const LOG_TARGET: &str = "runtime::ddc-clusters";

use ddc_primitives::{
	traits::{
		cluster::{ClusterCreator, ClusterProtocol, ClusterQuery, ClusterValidator},
		staking::{StakerCreator, StakingVisitor, StakingVisitorError},
	},
	ClusterBondingParams, ClusterFeesParams, ClusterId, ClusterNodeKind, ClusterNodeState,
	ClusterNodeStatus, ClusterNodesStats, ClusterParams, ClusterPricingParams,
	ClusterProtocolParams, ClusterStatus, DdcEra, NodePubKey, NodeType,
};
use frame_support::{
	assert_ok,
	pallet_prelude::*,
	traits::{Currency, LockableCurrency},
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use pallet_ddc_nodes::{NodeRepository, NodeTrait};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

use crate::{
	cluster::Cluster,
	node_provider_auth::{NodeProviderAuthContract, NodeProviderAuthContractError},
};

pub mod cluster;
mod node_provider_auth;

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use ddc_primitives::traits::cluster::ClusterManager;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
		frame_support::traits::StorageVersion::new(3);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_contracts::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type NodeRepository: NodeRepository<Self>; // todo: get rid of tight coupling with nodes-pallet
		type StakingVisitor: StakingVisitor<Self>;
		type StakerCreator: StakerCreator<Self, BalanceOf<Self>>;
		type Currency: LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>;
		type WeightInfo: WeightInfo;
		#[pallet::constant]
		type MinErasureCodingRequiredLimit: Get<u32>;
		#[pallet::constant]
		type MinErasureCodingTotalLimit: Get<u32>;
		#[pallet::constant]
		type MinReplicationTotalLimit: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		ClusterCreated { cluster_id: ClusterId },
		ClusterNodeAdded { cluster_id: ClusterId, node_pub_key: NodePubKey },
		ClusterNodeRemoved { cluster_id: ClusterId, node_pub_key: NodePubKey },
		ClusterParamsSet { cluster_id: ClusterId },
		ClusterProtocolParamsSet { cluster_id: ClusterId },
		ClusterActivated { cluster_id: ClusterId },
		ClusterBonded { cluster_id: ClusterId },
		ClusterUnbonding { cluster_id: ClusterId },
		ClusterUnbonded { cluster_id: ClusterId },
		ClusterNodeValidated { cluster_id: ClusterId, node_pub_key: NodePubKey, succeeded: bool },
		ClusterEraPaid { cluster_id: ClusterId, era_id: DdcEra },
	}

	#[pallet::error]
	pub enum Error<T> {
		ClusterAlreadyExists,
		ClusterDoesNotExist,
		ClusterParamsExceedsLimit,
		AttemptToAddNonExistentNode,
		AttemptToAddAlreadyAssignedNode,
		AttemptToRemoveNonExistentNode,
		AttemptToRemoveNotAssignedNode,
		OnlyClusterManager,
		OnlyNodeProvider,
		NodeIsNotAuthorized,
		NodeHasNoActivatedStake,
		NodeStakeIsInvalid,
		/// Cluster candidate should not plan to chill.
		NodeChillingIsProhibited,
		NodeAuthContractCallFailed,
		NodeAuthContractDeployFailed,
		NodeAuthNodeAuthorizationNotSuccessful,
		ErasureCodingRequiredDidNotMeetMinimum,
		ErasureCodingTotalNotMeetMinimum,
		ReplicationTotalDidNotMeetMinimum,
		ClusterAlreadyActivated,
		UnexpectedClusterStatus,
		AttemptToValidateNotAssignedNode,
		ClusterProtocolParamsNotSet,
		ArithmeticOverflow,
		NodeIsNotAssignedToCluster,
		ControllerDoesNotExist,
	}

	#[pallet::storage]
	pub type Clusters<T: Config> =
		StorageMap<_, Blake2_128Concat, ClusterId, Cluster<T::AccountId>>;

	#[pallet::storage]
	pub type ClustersGovParams<T: Config> = StorageMap<
		_,
		Twox64Concat,
		ClusterId,
		ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>,
	>;

	#[pallet::storage]
	pub type ClustersNodes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		NodePubKey,
		ClusterNodeState<BlockNumberFor<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	pub type ClustersNodesStats<T: Config> =
		StorageMap<_, Twox64Concat, ClusterId, ClusterNodesStats>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub clusters: Vec<Cluster<T::AccountId>>,
		#[allow(clippy::type_complexity)]
		pub clusters_protocol_params:
			Vec<(ClusterId, ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>)>,
		#[allow(clippy::type_complexity)]
		pub clusters_nodes: Vec<(ClusterId, Vec<(NodePubKey, ClusterNodeKind, ClusterNodeStatus)>)>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				clusters: Default::default(),
				clusters_protocol_params: Default::default(),
				clusters_nodes: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		fn build(&self) {
			for cluster in &self.clusters {
				assert_ok!(Pallet::<T>::create_cluster(
					frame_system::Origin::<T>::Signed(cluster.manager_id.clone()).into(),
					cluster.cluster_id,
					cluster.reserve_id.clone(),
					ClusterParams::<T::AccountId> {
						node_provider_auth_contract: cluster
							.props
							.node_provider_auth_contract
							.clone(),
						erasure_coding_required: cluster.props.erasure_coding_required,
						erasure_coding_total: cluster.props.erasure_coding_total,
						replication_total: cluster.props.replication_total,
					},
					self.clusters_protocol_params
						.iter()
						.find(|(id, _)| id == &cluster.cluster_id)
						.unwrap()
						.1
						.clone(),
				));

				Clusters::<T>::mutate(cluster.cluster_id, |value| {
					if let Some(clust) = value {
						clust.status = cluster.status.clone();
					};
				});

				for (cluster_id, nodes) in &self.clusters_nodes {
					let mut stats = ClusterNodesStats {
						await_validation: 0,
						validation_succeeded: 0,
						validation_failed: 0,
					};

					for (node_pub_key, kind, status) in nodes {
						<ClustersNodes<T>>::insert(
							cluster_id,
							node_pub_key,
							ClusterNodeState {
								kind: kind.clone(),
								status: status.clone(),
								added_at: frame_system::Pallet::<T>::block_number(),
							},
						);
						match status {
							ClusterNodeStatus::AwaitsValidation => {
								stats.await_validation = stats.await_validation.saturating_add(1);
							},
							ClusterNodeStatus::ValidationSucceeded => {
								stats.validation_succeeded =
									stats.validation_succeeded.saturating_add(1);
							},
							ClusterNodeStatus::ValidationFailed => {
								stats.validation_failed = stats.validation_failed.saturating_add(1);
							},
						}
					}

					<ClustersNodesStats<T>>::insert(cluster.cluster_id, stats);
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_cluster())]
		pub fn create_cluster(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			cluster_reserve_id: T::AccountId,
			cluster_params: ClusterParams<T::AccountId>,
			initial_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			let cluster_manager_id = ensure_signed(origin)?;
			Self::do_create_cluster(
				cluster_id,
				cluster_manager_id,
				cluster_reserve_id,
				cluster_params,
				initial_protocol_params,
			)
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::add_node())]
		pub fn add_node(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
			node_kind: ClusterNodeKind,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;

			ensure!(cluster.manager_id == caller_id, Error::<T>::OnlyClusterManager);

			// Sufficient funds are locked at the DDC Staking module.
			let has_activated_stake =
				T::StakingVisitor::has_activated_stake(&node_pub_key, &cluster_id)
					.map_err(Into::<Error<T>>::into)?;
			ensure!(has_activated_stake, Error::<T>::NodeHasNoActivatedStake);

			// Candidate is not planning to pause operations any time soon.
			let has_chilling_attempt = T::StakingVisitor::has_chilling_attempt(&node_pub_key)
				.map_err(Into::<Error<T>>::into)?;
			ensure!(!has_chilling_attempt, Error::<T>::NodeChillingIsProhibited);

			// Node with this node with this public key exists.
			T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;

			Self::do_add_node(cluster, node_pub_key, node_kind)
		}

		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::remove_node())]
		pub fn remove_node(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;

			ensure!(cluster.manager_id == caller_id, Error::<T>::OnlyClusterManager);

			Self::do_remove_node(cluster, node_pub_key)
		}

		// Sets Governance non-sensetive parameters only
		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_cluster_params())]
		pub fn set_cluster_params(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			cluster_params: ClusterParams<T::AccountId>,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			let mut cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			ensure!(cluster.manager_id == caller_id, Error::<T>::OnlyClusterManager);
			ensure!(
				cluster_params.erasure_coding_required >= T::MinErasureCodingRequiredLimit::get(),
				Error::<T>::ErasureCodingRequiredDidNotMeetMinimum
			);
			ensure!(
				cluster_params.erasure_coding_total >= T::MinErasureCodingTotalLimit::get(),
				Error::<T>::ErasureCodingTotalNotMeetMinimum
			);
			ensure!(
				cluster_params.replication_total >= T::MinReplicationTotalLimit::get(),
				Error::<T>::ReplicationTotalDidNotMeetMinimum
			);
			cluster.set_params(cluster_params);
			Clusters::<T>::insert(cluster_id, cluster);
			Self::deposit_event(Event::<T>::ClusterParamsSet { cluster_id });

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::validate_node())]
		pub fn validate_node(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
			succeeded: bool,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			// todo: allow to execute this extrinsic to Validator's manager only
			ensure!(cluster.manager_id == caller_id, Error::<T>::OnlyClusterManager);

			Self::do_validate_node(cluster_id, node_pub_key, succeeded)
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::join_cluster())]
		pub fn join_cluster(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;

			// Cluster with a given id exists and has an auth smart contract.
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			let node_provider_auth_contract_address = cluster
				.props
				.node_provider_auth_contract
				.clone()
				.ok_or(Error::<T>::NodeIsNotAuthorized)?;

			// Node with this public key exists and belongs to the caller.
			let node = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;
			ensure!(*node.get_provider_id() == caller_id, Error::<T>::OnlyNodeProvider);

			// Sufficient funds are locked at the DDC Staking module.
			let has_activated_stake =
				T::StakingVisitor::has_activated_stake(&node_pub_key, &cluster_id)
					.map_err(Into::<Error<T>>::into)?;
			ensure!(has_activated_stake, Error::<T>::NodeHasNoActivatedStake);

			// Candidate is not planning to pause operations any time soon.
			let has_chilling_attempt = T::StakingVisitor::has_chilling_attempt(&node_pub_key)
				.map_err(Into::<Error<T>>::into)?;
			ensure!(!has_chilling_attempt, Error::<T>::NodeChillingIsProhibited);

			// Cluster auth smart contract allows joining.
			let auth_contract =
				NodeProviderAuthContract::<T>::new(node_provider_auth_contract_address, caller_id);
			let is_authorized = auth_contract
				.is_authorized(
					node.get_provider_id().to_owned(),
					node.get_pub_key(),
					node.get_type(),
				)
				.map_err(Into::<Error<T>>::into)?;
			ensure!(is_authorized, Error::<T>::NodeIsNotAuthorized);

			Self::do_join_cluster(cluster, node_pub_key)
		}
	}

	impl<T: Config> Pallet<T> {
		fn do_create_cluster(
			cluster_id: ClusterId,
			cluster_manager_id: T::AccountId,
			cluster_reserve_id: T::AccountId,
			cluster_params: ClusterParams<T::AccountId>,
			initial_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			ensure!(!Clusters::<T>::contains_key(cluster_id), Error::<T>::ClusterAlreadyExists);

			ensure!(
				cluster_params.erasure_coding_required >= T::MinErasureCodingRequiredLimit::get(),
				Error::<T>::ErasureCodingRequiredDidNotMeetMinimum
			);
			ensure!(
				cluster_params.erasure_coding_total >= T::MinErasureCodingTotalLimit::get(),
				Error::<T>::ErasureCodingTotalNotMeetMinimum
			);
			ensure!(
				cluster_params.replication_total >= T::MinReplicationTotalLimit::get(),
				Error::<T>::ReplicationTotalDidNotMeetMinimum
			);

			ensure!(!Clusters::<T>::contains_key(cluster_id), Error::<T>::ClusterAlreadyExists);
			let cluster =
				Cluster::new(cluster_id, cluster_manager_id, cluster_reserve_id, cluster_params);

			Clusters::<T>::insert(cluster_id, cluster);
			Self::deposit_event(Event::<T>::ClusterCreated { cluster_id });
			ClustersGovParams::<T>::insert(cluster_id, initial_protocol_params);
			Self::deposit_event(Event::<T>::ClusterProtocolParamsSet { cluster_id });
			ClustersNodesStats::<T>::insert(cluster_id, ClusterNodesStats::default());

			Ok(())
		}

		fn do_bond_cluster(cluster_id: &ClusterId) -> DispatchResult {
			let mut cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			ensure!(cluster.status == ClusterStatus::Unbonded, Error::<T>::UnexpectedClusterStatus);

			cluster.set_status(ClusterStatus::Bonded);
			Clusters::<T>::insert(cluster_id, cluster);
			Self::deposit_event(Event::<T>::ClusterBonded { cluster_id: *cluster_id });

			Ok(())
		}

		fn do_activate_cluster_protocol(cluster_id: &ClusterId) -> DispatchResult {
			let mut cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			ensure!(cluster.status == ClusterStatus::Bonded, Error::<T>::UnexpectedClusterStatus);

			cluster.set_status(ClusterStatus::Activated);
			Clusters::<T>::insert(cluster_id, cluster);
			Self::deposit_event(Event::<T>::ClusterActivated { cluster_id: *cluster_id });

			Ok(())
		}

		fn do_start_unbond_cluster(cluster_id: &ClusterId) -> DispatchResult {
			let mut cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			ensure!(Self::can_unbond_cluster(cluster_id), Error::<T>::UnexpectedClusterStatus);

			cluster.set_status(ClusterStatus::Unbonding);
			Clusters::<T>::insert(cluster_id, cluster);
			Self::deposit_event(Event::<T>::ClusterUnbonding { cluster_id: *cluster_id });

			Ok(())
		}

		fn do_end_unbond_cluster(cluster_id: &ClusterId) -> DispatchResult {
			let mut cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			ensure!(
				cluster.status == ClusterStatus::Unbonding,
				Error::<T>::UnexpectedClusterStatus
			);

			cluster.set_status(ClusterStatus::Unbonded);
			Clusters::<T>::insert(cluster_id, cluster);
			Self::deposit_event(Event::<T>::ClusterUnbonded { cluster_id: *cluster_id });

			Ok(())
		}

		fn do_update_cluster_protocol(
			cluster_id: &ClusterId,
			cluster_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			ensure!(
				ClustersGovParams::<T>::contains_key(cluster_id),
				Error::<T>::ClusterProtocolParamsNotSet
			);

			ClustersGovParams::<T>::insert(cluster_id, cluster_protocol_params);
			Self::deposit_event(Event::<T>::ClusterProtocolParamsSet { cluster_id: *cluster_id });

			Ok(())
		}

		fn do_add_node(
			cluster: Cluster<T::AccountId>,
			node_pub_key: NodePubKey,
			node_kind: ClusterNodeKind,
		) -> DispatchResult {
			ensure!(cluster.can_manage_nodes(), Error::<T>::UnexpectedClusterStatus);

			let mut node: pallet_ddc_nodes::Node<T> = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;

			ensure!(node.get_cluster_id().is_none(), Error::<T>::AttemptToAddAlreadyAssignedNode);
			node.set_cluster_id(Some(cluster.cluster_id));
			T::NodeRepository::update(node).map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;

			ClustersNodes::<T>::insert(
				cluster.cluster_id,
				node_pub_key.clone(),
				ClusterNodeState {
					kind: node_kind.clone(),
					status: ClusterNodeStatus::AwaitsValidation,
					added_at: frame_system::Pallet::<T>::block_number(),
				},
			);
			Self::deposit_event(Event::<T>::ClusterNodeAdded {
				cluster_id: cluster.cluster_id,
				node_pub_key,
			});

			let mut current_stats = ClustersNodesStats::<T>::try_get(cluster.cluster_id)
				.map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			current_stats.await_validation = current_stats
				.await_validation
				.checked_add(1)
				.ok_or(Error::<T>::ArithmeticOverflow)?;

			ClustersNodesStats::<T>::insert(cluster.cluster_id, current_stats);

			Ok(())
		}

		fn do_join_cluster(
			cluster: Cluster<T::AccountId>,
			node_pub_key: NodePubKey,
		) -> DispatchResult {
			ensure!(cluster.can_manage_nodes(), Error::<T>::UnexpectedClusterStatus);

			let mut node: pallet_ddc_nodes::Node<T> = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;
			ensure!(node.get_cluster_id().is_none(), Error::<T>::AttemptToAddAlreadyAssignedNode);

			node.set_cluster_id(Some(cluster.cluster_id));
			T::NodeRepository::update(node).map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;

			ClustersNodes::<T>::insert(
				cluster.cluster_id,
				node_pub_key.clone(),
				ClusterNodeState {
					kind: ClusterNodeKind::External,
					status: ClusterNodeStatus::ValidationSucceeded,
					added_at: frame_system::Pallet::<T>::block_number(),
				},
			);
			Self::deposit_event(Event::<T>::ClusterNodeAdded {
				cluster_id: cluster.cluster_id,
				node_pub_key,
			});

			let mut current_stats = ClustersNodesStats::<T>::try_get(cluster.cluster_id)
				.map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			current_stats.validation_succeeded = current_stats
				.validation_succeeded
				.checked_add(1)
				.ok_or(Error::<T>::ArithmeticOverflow)?;
			ClustersNodesStats::<T>::insert(cluster.cluster_id, current_stats);

			Ok(())
		}

		fn do_remove_node(
			cluster: Cluster<T::AccountId>,
			node_pub_key: NodePubKey,
		) -> DispatchResult {
			ensure!(cluster.can_manage_nodes(), Error::<T>::UnexpectedClusterStatus);

			let mut node = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| Error::<T>::AttemptToRemoveNonExistentNode)?;

			ensure!(
				node.get_cluster_id() == &Some(cluster.cluster_id),
				Error::<T>::AttemptToRemoveNotAssignedNode
			);

			node.set_cluster_id(None);
			T::NodeRepository::update(node)
				.map_err(|_| Error::<T>::AttemptToRemoveNonExistentNode)?;

			let current_node_state =
				ClustersNodes::<T>::take(cluster.cluster_id, node_pub_key.clone())
					.ok_or(Error::<T>::AttemptToRemoveNotAssignedNode)?;
			Self::deposit_event(Event::<T>::ClusterNodeRemoved {
				cluster_id: cluster.cluster_id,
				node_pub_key,
			});

			let mut current_stats = ClustersNodesStats::<T>::try_get(cluster.cluster_id)
				.map_err(|_| Error::<T>::ClusterDoesNotExist)?;

			let updated_stats = match current_node_state.status {
				ClusterNodeStatus::AwaitsValidation => {
					current_stats.await_validation = current_stats
						.await_validation
						.checked_sub(1)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					current_stats
				},
				ClusterNodeStatus::ValidationSucceeded => {
					current_stats.validation_succeeded = current_stats
						.validation_succeeded
						.checked_sub(1)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					current_stats
				},
				ClusterNodeStatus::ValidationFailed => {
					current_stats.validation_failed = current_stats
						.validation_failed
						.checked_sub(1)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					current_stats
				},
			};

			ClustersNodesStats::<T>::insert(cluster.cluster_id, updated_stats);

			Ok(())
		}

		fn do_validate_node(
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
			succeeded: bool,
		) -> DispatchResult {
			let mut current_stats = ClustersNodesStats::<T>::try_get(cluster_id)
				.map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			let mut current_node_state =
				ClustersNodes::<T>::try_get(cluster_id, node_pub_key.clone())
					.map_err(|_| Error::<T>::AttemptToValidateNotAssignedNode)?;

			let updated_stats = match current_node_state.status {
				ClusterNodeStatus::AwaitsValidation if succeeded => {
					current_stats.await_validation = current_stats
						.await_validation
						.checked_sub(1)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					current_stats.validation_succeeded = current_stats
						.validation_succeeded
						.checked_add(1)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					current_stats
				},
				ClusterNodeStatus::AwaitsValidation if !succeeded => {
					current_stats.await_validation = current_stats
						.await_validation
						.checked_sub(1)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					current_stats.validation_failed = current_stats
						.validation_failed
						.checked_add(1)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					current_stats
				},
				ClusterNodeStatus::ValidationSucceeded if !succeeded => {
					current_stats.validation_succeeded = current_stats
						.validation_succeeded
						.checked_sub(1)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					current_stats.validation_failed = current_stats
						.validation_failed
						.checked_add(1)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					current_stats
				},
				ClusterNodeStatus::ValidationFailed if succeeded => {
					current_stats.validation_failed = current_stats
						.validation_failed
						.checked_sub(1)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					current_stats.validation_succeeded = current_stats
						.validation_succeeded
						.checked_add(1)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					current_stats
				},
				_ => current_stats,
			};

			current_node_state.status = if succeeded {
				ClusterNodeStatus::ValidationSucceeded
			} else {
				ClusterNodeStatus::ValidationFailed
			};

			ClustersNodes::<T>::insert(cluster_id, node_pub_key.clone(), current_node_state);
			Self::deposit_event(Event::<T>::ClusterNodeValidated {
				cluster_id,
				node_pub_key,
				succeeded,
			});
			ClustersNodesStats::<T>::insert(cluster_id, updated_stats);

			Ok(())
		}

		fn can_unbond_cluster(cluster_id: &ClusterId) -> bool {
			let cluster = match Clusters::<T>::get(cluster_id) {
				Some(cluster) => cluster,
				None => return false,
			};

			let is_empty_nodes = ClustersNodesStats::<T>::try_get(cluster_id)
				.map(|status| {
					status.await_validation + status.validation_succeeded + status.validation_failed ==
						0
				})
				.unwrap_or(false);

			is_empty_nodes &&
				matches!(cluster.status, ClusterStatus::Bonded | ClusterStatus::Activated)
		}
	}

	impl<T: Config> ClusterQuery<T> for Pallet<T> {
		fn cluster_exists(cluster_id: &ClusterId) -> bool {
			Clusters::<T>::contains_key(cluster_id)
		}

		fn get_cluster_status(cluster_id: &ClusterId) -> Result<ClusterStatus, DispatchError> {
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			Ok(cluster.status)
		}

		fn get_manager_and_reserve_id(
			cluster_id: &ClusterId,
		) -> Result<(T::AccountId, T::AccountId), DispatchError> {
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			Ok((cluster.manager_id, cluster.reserve_id))
		}
	}

	impl<T: Config> ClusterProtocol<T, BalanceOf<T>> for Pallet<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		fn get_bond_size(
			cluster_id: &ClusterId,
			node_type: NodeType,
		) -> Result<u128, DispatchError> {
			let cluster_protocol_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| Error::<T>::ClusterProtocolParamsNotSet)?;
			match node_type {
				NodeType::Storage =>
					Ok(cluster_protocol_params.storage_bond_size.saturated_into::<u128>()),
			}
		}

		fn get_pricing_params(
			cluster_id: &ClusterId,
		) -> Result<ClusterPricingParams, DispatchError> {
			let cluster_protocol_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| Error::<T>::ClusterProtocolParamsNotSet)?;
			Ok(ClusterPricingParams {
				unit_per_mb_stored: cluster_protocol_params.unit_per_mb_stored,
				unit_per_mb_streamed: cluster_protocol_params.unit_per_mb_streamed,
				unit_per_put_request: cluster_protocol_params.unit_per_put_request,
				unit_per_get_request: cluster_protocol_params.unit_per_get_request,
			})
		}

		fn get_fees_params(cluster_id: &ClusterId) -> Result<ClusterFeesParams, DispatchError> {
			let cluster_protocol_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| Error::<T>::ClusterProtocolParamsNotSet)?;

			Ok(ClusterFeesParams {
				treasury_share: cluster_protocol_params.treasury_share,
				validators_share: cluster_protocol_params.validators_share,
				cluster_reserve_share: cluster_protocol_params.cluster_reserve_share,
			})
		}

		fn get_chill_delay(
			cluster_id: &ClusterId,
			node_type: NodeType,
		) -> Result<BlockNumberFor<T>, DispatchError> {
			let cluster_protocol_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| Error::<T>::ClusterProtocolParamsNotSet)?;
			match node_type {
				NodeType::Storage => Ok(cluster_protocol_params.storage_chill_delay),
			}
		}

		fn get_unbonding_delay(
			cluster_id: &ClusterId,
			node_type: NodeType,
		) -> Result<BlockNumberFor<T>, DispatchError> {
			let cluster_protocol_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| Error::<T>::ClusterProtocolParamsNotSet)?;
			match node_type {
				NodeType::Storage => Ok(cluster_protocol_params.storage_unbonding_delay),
			}
		}

		fn get_bonding_params(
			cluster_id: &ClusterId,
		) -> Result<ClusterBondingParams<BlockNumberFor<T>>, DispatchError> {
			let cluster_protocol_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| Error::<T>::ClusterProtocolParamsNotSet)?;
			Ok(ClusterBondingParams {
				storage_bond_size: cluster_protocol_params
					.storage_bond_size
					.saturated_into::<u128>(),
				storage_chill_delay: cluster_protocol_params.storage_chill_delay,
				storage_unbonding_delay: cluster_protocol_params.storage_unbonding_delay,
			})
		}

		fn get_reserve_account_id(cluster_id: &ClusterId) -> Result<T::AccountId, DispatchError> {
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			Ok(cluster.reserve_id)
		}

		fn activate_cluster_protocol(cluster_id: &ClusterId) -> DispatchResult {
			Self::do_activate_cluster_protocol(cluster_id)
		}

		fn update_cluster_protocol(
			cluster_id: &ClusterId,
			cluster_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			Self::do_update_cluster_protocol(cluster_id, cluster_protocol_params)
		}

		fn bond_cluster(cluster_id: &ClusterId) -> DispatchResult {
			Self::do_bond_cluster(cluster_id)
		}

		fn start_unbond_cluster(cluster_id: &ClusterId) -> DispatchResult {
			Self::do_start_unbond_cluster(cluster_id)
		}

		fn end_unbond_cluster(cluster_id: &ClusterId) -> DispatchResult {
			Self::do_end_unbond_cluster(cluster_id)
		}
	}
	impl<T: Config> ClusterValidator<T> for Pallet<T> {
		fn set_last_paid_era(cluster_id: &ClusterId, era_id: DdcEra) -> Result<(), DispatchError> {
			let mut cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;

			cluster.last_paid_era = era_id;
			Clusters::<T>::insert(cluster_id, cluster);
			Self::deposit_event(Event::<T>::ClusterEraPaid { cluster_id: *cluster_id, era_id });

			Ok(())
		}

		fn get_last_paid_era(cluster_id: &ClusterId) -> Result<DdcEra, DispatchError> {
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;

			Ok(cluster.last_paid_era)
		}
	}

	impl<T: Config> ClusterManager<T> for Pallet<T> {
		fn get_manager_account_id(cluster_id: &ClusterId) -> Result<T::AccountId, DispatchError> {
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			Ok(cluster.manager_id)
		}

		fn get_nodes(cluster_id: &ClusterId) -> Result<Vec<NodePubKey>, DispatchError> {
			let mut nodes = Vec::new();

			// Iterate through all nodes associated with the cluster_id
			for (node_pubkey, _) in ClustersNodes::<T>::iter_prefix(cluster_id) {
				nodes.push(node_pubkey);
			}

			Ok(nodes)
		}

		fn contains_node(
			cluster_id: &ClusterId,
			node_pub_key: &NodePubKey,
			validation_status: Option<ClusterNodeStatus>,
		) -> bool {
			match validation_status {
				Some(status) => ClustersNodes::<T>::try_get(cluster_id, node_pub_key)
					.map(|n| n.status == status)
					.unwrap_or(false),
				None => ClustersNodes::<T>::get(cluster_id, node_pub_key).is_some(),
			}
		}

		fn add_node(
			cluster_id: &ClusterId,
			node_pub_key: &NodePubKey,
			node_kind: &ClusterNodeKind,
		) -> Result<(), DispatchError> {
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			Self::do_add_node(cluster, node_pub_key.clone(), node_kind.clone())
		}

		fn remove_node(
			cluster_id: &ClusterId,
			node_pub_key: &NodePubKey,
		) -> Result<(), DispatchError> {
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			Self::do_remove_node(cluster, node_pub_key.clone())
		}

		fn get_node_state(
			cluster_id: &ClusterId,
			node_pub_key: &NodePubKey,
		) -> Result<ClusterNodeState<BlockNumberFor<T>>, DispatchError> {
			let node_state = ClustersNodes::<T>::try_get(cluster_id, node_pub_key)
				.map_err(|_| Error::<T>::NodeIsNotAssignedToCluster)?;
			Ok(node_state)
		}

		fn get_nodes_stats(cluster_id: &ClusterId) -> Result<ClusterNodesStats, DispatchError> {
			let current_stats = ClustersNodesStats::<T>::try_get(cluster_id)
				.map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			Ok(current_stats)
		}

		fn validate_node(
			cluster_id: &ClusterId,
			node_pub_key: &NodePubKey,
			succeeded: bool,
		) -> Result<(), DispatchError> {
			Self::do_validate_node(*cluster_id, node_pub_key.clone(), succeeded)
		}

		fn get_clusters(status: ClusterStatus) -> Result<Vec<ClusterId>, DispatchError> {
			let mut clusters_ids = Vec::new();
			for (cluster_id, cluster) in <Clusters<T>>::iter() {
				if cluster.status == status {
					clusters_ids.push(cluster_id);
				}
			}
			Ok(clusters_ids)
		}
	}

	impl<T: Config> ClusterCreator<T, BalanceOf<T>> for Pallet<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		fn create_cluster(
			cluster_id: ClusterId,
			cluster_manager_id: T::AccountId,
			cluster_reserve_id: T::AccountId,
			cluster_params: ClusterParams<T::AccountId>,
			initial_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			Self::do_create_cluster(
				cluster_id,
				cluster_manager_id,
				cluster_reserve_id,
				cluster_params,
				initial_protocol_params,
			)
		}
	}

	impl<T> From<StakingVisitorError> for Error<T> {
		fn from(error: StakingVisitorError) -> Self {
			match error {
				StakingVisitorError::NodeStakeDoesNotExist => Error::<T>::NodeHasNoActivatedStake,
				StakingVisitorError::NodeStakeIsInBadState => Error::<T>::NodeStakeIsInvalid,
				StakingVisitorError::ControllerDoesNotExist => Error::<T>::ControllerDoesNotExist,
			}
		}
	}

	impl<T> From<NodeProviderAuthContractError> for Error<T> {
		fn from(error: NodeProviderAuthContractError) -> Self {
			match error {
				NodeProviderAuthContractError::ContractCallFailed =>
					Error::<T>::NodeAuthContractCallFailed,
				NodeProviderAuthContractError::ContractDeployFailed =>
					Error::<T>::NodeAuthContractDeployFailed,
				NodeProviderAuthContractError::NodeAuthorizationNotSuccessful =>
					Error::<T>::NodeAuthNodeAuthorizationNotSuccessful,
			}
		}
	}
}
