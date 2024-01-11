//! # DDC Nodes Pallet
//!
//! The DDC Clusters pallet is used to manage DDC clusters.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## GenesisConfig
//!
//! The DDC Clusters pallet depends on the [`GenesisConfig`]. The
//! `GenesisConfig` is optional and allow to set some initial clusters in DDC.
#![warn(clippy::missing_docs_in_private_items)]
#![feature(is_some_and)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

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

use ddc_primitives::{
	ClusterBondingParams, ClusterFeesParams, ClusterGovParams, ClusterId, ClusterParams,
	ClusterPricingParams, NodePubKey, NodeType,
};
use ddc_traits::{
	cluster::{ClusterCreator, ClusterVisitor, ClusterVisitorError},
	staking::{StakerCreator, StakingVisitor, StakingVisitorError},
};
use frame_support::{
	assert_ok,
	pallet_prelude::*,
	traits::{Currency, LockableCurrency},
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use pallet_ddc_nodes::{NodeRepository, NodeTrait};
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

use crate::{
	cluster::Cluster,
	node_provider_auth::{NodeProviderAuthContract, NodeProviderAuthContractError},
};

/// DDC cluster data structures.
pub mod cluster;
/// DDC cluster additional authorization smart contract.
mod node_provider_auth;

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use ddc_traits::cluster::{ClusterManager, ClusterManagerError};
	use pallet_contracts::chain_extension::UncheckedFrom;

	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_contracts::Config {
		/// Runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// DDC nodes registry.
		type NodeRepository: NodeRepository<Self>; // todo: get rid of tight coupling with nodes-pallet
		/// DDC nodes staking read-only registry.
		type StakingVisitor: StakingVisitor<Self>;
		/// DDC nodes staking creator.
		/// NOTE: Required for the benchmarking only.
		type StakerCreator: StakerCreator<Self, BalanceOf<Self>>;
		/// Accounts balances registry.
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		/// Weight info type.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New DDC cluster was created in the network.
		ClusterCreated {
			/// DDC cluster identifier.
			cluster_id: ClusterId,
		},
		/// DDC node was added to DDC cluster.
		ClusterNodeAdded {
			/// DDC cluster identifier.
			cluster_id: ClusterId,
			/// DDC node public key.
			node_pub_key: NodePubKey,
		},
		/// DDC node was removed from DDC cluster.
		ClusterNodeRemoved {
			/// DDC cluster identifier.
			cluster_id: ClusterId,
			/// DDC node public key.
			node_pub_key: NodePubKey,
		},
		/// Operational parameters for a DDC cluster were set.
		ClusterParamsSet {
			/// DDC cluster identifier.
			cluster_id: ClusterId,
		},
		/// Economic parameters for a DDC cluster were set.
		ClusterGovParamsSet {
			/// DDC cluster identifier.
			cluster_id: ClusterId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// DDC cluster with such `cluster_id` already exists.
		ClusterAlreadyExists,
		/// DDC cluster with such `cluster_id` does not exist.
		ClusterDoesNotExist,
		/// DDC cluster parameters size exceeds the limit.
		ClusterParamsExceedsLimit,
		/// Attempt to add a DDC node that does not exist.
		AttemptToAddNonExistentNode,
		/// Attempt to add a DDC node that is already added to another DDC Cluster.
		AttemptToAddAlreadyAssignedNode,
		/// Attempt to remove a DDC node that does not exist.
		AttemptToRemoveNonExistentNode,
		/// Attempt to remove a DDC node that is not added to the DDC Cluster.
		AttemptToRemoveNotAssignedNode,
		/// Operation is restricted to DDC cluster manager role.
		OnlyClusterManager,
		/// DDC node is not authorized by additional DDC cluster authorization smart contract.
		NodeIsNotAuthorized,
		/// DDC node has not activated the bond for a DDC cluster before adding to its topology.
		NodeHasNoActivatedStake,
		/// Tokens bonded for the DDC node are in an inconsistent state.
		NodeStakeIsInvalid,
		/// DDC node selected as a candidate for DDC cluster should not plan to chill.
		NodeChillingIsProhibited,
		/// Call to additional DDC cluster authorization smart contract failed.
		NodeAuthContractCallFailed,
		/// Deployment of additional DDC cluster authorization smart contract failed.
		NodeAuthContractDeployFailed,
		/// DDC node is not authorized by authorization smart contract of the DDC cluster.
		NodeAuthNodeAuthorizationNotSuccessful,
	}

	/// Map of DDC Clusters.
	#[pallet::storage]
	#[pallet::getter(fn clusters)]
	pub type Clusters<T: Config> =
		StorageMap<_, Blake2_128Concat, ClusterId, Cluster<T::AccountId>>;

	/// Map of economic parameters for all DDC Clusters.
	#[pallet::storage]
	#[pallet::getter(fn clusters_gov_params)]
	pub type ClustersGovParams<T: Config> =
		StorageMap<_, Twox64Concat, ClusterId, ClusterGovParams<BalanceOf<T>, T::BlockNumber>>;

	/// Map of DDC nodes assigned to DDC Clusters.
	#[pallet::storage]
	#[pallet::getter(fn clusters_nodes)]
	pub type ClustersNodes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		NodePubKey,
		bool,
		OptionQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		/// List of initial DDC Clusters.
		pub clusters: Vec<Cluster<T::AccountId>>,
		#[allow(clippy::type_complexity)]
		/// List of economic parameters for initial DDC Clusters.
		pub clusters_gov_params: Vec<(ClusterId, ClusterGovParams<BalanceOf<T>, T::BlockNumber>)>,
		/// List of DDC nodes assigned to initial DDC Clusters.
		pub clusters_nodes: Vec<(ClusterId, Vec<NodePubKey>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				clusters: Default::default(),
				clusters_gov_params: Default::default(),
				clusters_nodes: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		fn build(&self) {
			for cluster in &self.clusters {
				assert_ok!(Pallet::<T>::create_cluster(
					frame_system::Origin::<T>::Root.into(),
					cluster.cluster_id,
					cluster.manager_id.clone(),
					cluster.reserve_id.clone(),
					ClusterParams::<T::AccountId> {
						node_provider_auth_contract: cluster
							.props
							.node_provider_auth_contract
							.clone(),
					},
					self.clusters_gov_params
						.iter()
						.find(|(id, _)| id == &cluster.cluster_id)
						.unwrap()
						.1
						.clone(),
				));

				for (cluster_id, nodes) in &self.clusters_nodes {
					for node_pub_key in nodes {
						<ClustersNodes<T>>::insert(cluster_id, node_pub_key, true);
					}
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		/// Creates a new DDC Cluster in the network.
		///
		/// The dispatch origin of this call must be _Root_.
		///
		/// Parameters:
		/// - `cluster_id`: Hash-based identifier of the targeting DDC cluster.
		/// - `cluster_manager_id`: The account of the cluster manager responsible for executing
		///   operational actions.
		/// - `cluster_reserve_id`: The account of the cluster reserve responsible for holding
		///   rewards.
		/// - `cluster_params`: Set of operational parameters for the cluster.
		/// - `cluster_gov_params`: Set of economic parameters for the cluster locked by the
		///   Governance.
		///
		/// Emits: `ClusterCreated`.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_cluster())]
		pub fn create_cluster(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			cluster_manager_id: T::AccountId,
			cluster_reserve_id: T::AccountId,
			cluster_params: ClusterParams<T::AccountId>,
			cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			ensure_root(origin)?; // requires Governance approval
			Self::do_create_cluster(
				cluster_id,
				cluster_manager_id,
				cluster_reserve_id,
				cluster_params,
				cluster_gov_params,
			)
		}

		/// Adds a DDC node to a DDC Cluster.
		///
		/// The dispatch origin of this call must be _Signed_, and the signing account must be the
		/// cluster manager.
		///
		/// Parameters:
		/// - `cluster_id`: Hash-based identifier of the targeting DDC cluster.
		/// - `node_pub_key`: Public key of the targeting DDC node to add.
		///
		/// Emits: `ClusterNodeAdded`.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::add_node())]
		pub fn add_node(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
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
			let node = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;

			// Cluster extension smart contract allows joining.
			if let Some(address) = cluster.props.node_provider_auth_contract {
				let auth_contract = NodeProviderAuthContract::<T>::new(address, caller_id);

				let is_authorized = auth_contract
					.is_authorized(
						node.get_provider_id().to_owned(),
						node.get_pub_key(),
						node.get_type(),
					)
					.map_err(Into::<Error<T>>::into)?;
				ensure!(is_authorized, Error::<T>::NodeIsNotAuthorized);
			};

			// Add node to the cluster.
			<Self as ClusterManager<T>>::add_node(&cluster_id, &node_pub_key)
				.map_err(Into::<Error<T>>::into)?;
			Self::deposit_event(Event::<T>::ClusterNodeAdded { cluster_id, node_pub_key });

			Ok(())
		}

		/// Removes a DDC node from a DDC cluster.
		///
		/// The dispatch origin of this call must be _Signed_, and the signing account must be the
		/// cluster manager.
		///
		/// Parameters:
		/// - `cluster_id`: Hash-based identifier of the targeting DDC cluster.
		/// - `node_pub_key`: Public key of the targeting DDC node to remove.
		///
		/// Emits: `ClusterNodeRemoved`.
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

			// Remove node from the cluster.
			<Self as ClusterManager<T>>::remove_node(&cluster_id, &node_pub_key)
				.map_err(Into::<Error<T>>::into)?;
			Self::deposit_event(Event::<T>::ClusterNodeRemoved { cluster_id, node_pub_key });

			Ok(())
		}

		/// Sets operational parameters for a DDC cluster.
		///
		/// The dispatch origin of this call must be _Signed_, and the signing account must be the
		/// cluster manager.
		///
		/// Parameters:
		/// - `cluster_id`: Hash-based identifier of the targeting DDC cluster.
		/// - `cluster_params`: Set of operational parameters for the cluster.
		///
		/// Emits: `ClusterParamsSet`.
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
			cluster.set_params(cluster_params).map_err(Into::<Error<T>>::into)?;
			Clusters::<T>::insert(cluster_id, cluster);
			Self::deposit_event(Event::<T>::ClusterParamsSet { cluster_id });

			Ok(())
		}

		/// Sets operational parameters for a DDC cluster.
		///
		/// The dispatch origin of this call must be _Root_.
		///
		/// Parameters:
		/// - `cluster_id`: Hash-based identifier of the targeting DDC cluster.
		/// - `cluster_gov_params`: Set of economic parameters for the cluster locked by the
		///   Governance.
		///
		/// Emits: `ClusterGovParamsSet`.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_cluster_gov_params())]
		pub fn set_cluster_gov_params(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			ensure_root(origin)?; // requires Governance approval
			let _cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			ClustersGovParams::<T>::insert(cluster_id, cluster_gov_params);
			Self::deposit_event(Event::<T>::ClusterGovParamsSet { cluster_id });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Creates a new DDC cluster and emits appropriate events.
		fn do_create_cluster(
			cluster_id: ClusterId,
			cluster_manager_id: T::AccountId,
			cluster_reserve_id: T::AccountId,
			cluster_params: ClusterParams<T::AccountId>,
			cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			let cluster =
				Cluster::new(cluster_id, cluster_manager_id, cluster_reserve_id, cluster_params)
					.map_err(Into::<Error<T>>::into)?;
			ensure!(!Clusters::<T>::contains_key(cluster_id), Error::<T>::ClusterAlreadyExists);

			Clusters::<T>::insert(cluster_id, cluster);
			ClustersGovParams::<T>::insert(cluster_id, cluster_gov_params);
			Self::deposit_event(Event::<T>::ClusterCreated { cluster_id });

			Ok(())
		}
	}

	impl<T: Config> ClusterVisitor<T> for Pallet<T> {
		/// Checks whether a DDC Cluster exists.
		fn ensure_cluster(cluster_id: &ClusterId) -> Result<(), ClusterVisitorError> {
			Clusters::<T>::get(cluster_id)
				.map(|_| ())
				.ok_or(ClusterVisitorError::ClusterDoesNotExist)
		}

		/// Gets minimal bonding size set for a DDC cluster.
		fn get_bond_size(
			cluster_id: &ClusterId,
			node_type: NodeType,
		) -> Result<u128, ClusterVisitorError> {
			let cluster_gov_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| ClusterVisitorError::ClusterGovParamsNotSet)?;
			match node_type {
				NodeType::Storage =>
					Ok(cluster_gov_params.storage_bond_size.saturated_into::<u128>()),
			}
		}

		/// Gets pricing-related parameters for data storing and data streaming set for a
		/// DDC cluster.
		fn get_pricing_params(
			cluster_id: &ClusterId,
		) -> Result<ClusterPricingParams, ClusterVisitorError> {
			let cluster_gov_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| ClusterVisitorError::ClusterGovParamsNotSet)?;
			Ok(ClusterPricingParams {
				unit_per_mb_stored: cluster_gov_params.unit_per_mb_stored,
				unit_per_mb_streamed: cluster_gov_params.unit_per_mb_streamed,
				unit_per_put_request: cluster_gov_params.unit_per_put_request,
				unit_per_get_request: cluster_gov_params.unit_per_get_request,
			})
		}

		/// Gets fee-related parameters set for a DDC cluster.
		fn get_fees_params(
			cluster_id: &ClusterId,
		) -> Result<ClusterFeesParams, ClusterVisitorError> {
			let cluster_gov_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| ClusterVisitorError::ClusterGovParamsNotSet)?;

			Ok(ClusterFeesParams {
				treasury_share: cluster_gov_params.treasury_share,
				validators_share: cluster_gov_params.validators_share,
				cluster_reserve_share: cluster_gov_params.cluster_reserve_share,
			})
		}

		/// Gets reserve account set for a DDC cluster.
		fn get_reserve_account_id(
			cluster_id: &ClusterId,
		) -> Result<T::AccountId, ClusterVisitorError> {
			let cluster = Clusters::<T>::try_get(cluster_id)
				.map_err(|_| ClusterVisitorError::ClusterDoesNotExist)?;
			Ok(cluster.reserve_id)
		}

		/// Gets minimum chilling delay for DDC nodes set for a DDC cluster.
		fn get_chill_delay(
			cluster_id: &ClusterId,
			node_type: NodeType,
		) -> Result<T::BlockNumber, ClusterVisitorError> {
			let cluster_gov_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| ClusterVisitorError::ClusterGovParamsNotSet)?;
			match node_type {
				NodeType::Storage => Ok(cluster_gov_params.storage_chill_delay),
			}
		}

		/// Gets minimum unbonding delay for DDC nodes set for a DDC cluster.
		fn get_unbonding_delay(
			cluster_id: &ClusterId,
			node_type: NodeType,
		) -> Result<T::BlockNumber, ClusterVisitorError> {
			let cluster_gov_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| ClusterVisitorError::ClusterGovParamsNotSet)?;
			match node_type {
				NodeType::Storage => Ok(cluster_gov_params.storage_unbonding_delay),
			}
		}

		/// Gets bonding-related parameters set for a DDC cluster.
		fn get_bonding_params(
			cluster_id: &ClusterId,
		) -> Result<ClusterBondingParams<T::BlockNumber>, ClusterVisitorError> {
			let cluster_gov_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| ClusterVisitorError::ClusterGovParamsNotSet)?;
			Ok(ClusterBondingParams {
				storage_bond_size: cluster_gov_params.storage_bond_size.saturated_into::<u128>(),
				storage_chill_delay: cluster_gov_params.storage_chill_delay,
				storage_unbonding_delay: cluster_gov_params.storage_unbonding_delay,
			})
		}
	}

	impl<T: Config> ClusterManager<T> for Pallet<T> {
		/// Checks whether a DDC node is added to a DDC Cluster.
		fn contains_node(cluster_id: &ClusterId, node_pub_key: &NodePubKey) -> bool {
			ClustersNodes::<T>::get(cluster_id, node_pub_key).is_some()
		}

		/// Adds a DDC to a DDC Cluster and emits appropriate events.
		fn add_node(
			cluster_id: &ClusterId,
			node_pub_key: &NodePubKey,
		) -> Result<(), ClusterManagerError> {
			let mut node = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| ClusterManagerError::AttemptToAddNonExistentNode)?;

			ensure!(
				node.get_cluster_id().is_none(),
				ClusterManagerError::AttemptToAddAlreadyAssignedNode
			);

			node.set_cluster_id(Some(*cluster_id));
			T::NodeRepository::update(node)
				.map_err(|_| ClusterManagerError::AttemptToAddNonExistentNode)?;

			ClustersNodes::<T>::insert(cluster_id, node_pub_key.clone(), true);

			Ok(())
		}

		/// Removes a DDC from a DDC Cluster and emits appropriate events.
		fn remove_node(
			cluster_id: &ClusterId,
			node_pub_key: &NodePubKey,
		) -> Result<(), ClusterManagerError> {
			let mut node = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| ClusterManagerError::AttemptToRemoveNonExistentNode)?;

			ensure!(
				node.get_cluster_id() == &Some(*cluster_id),
				ClusterManagerError::AttemptToRemoveNotAssignedNode
			);

			node.set_cluster_id(None);
			T::NodeRepository::update(node)
				.map_err(|_| ClusterManagerError::AttemptToRemoveNonExistentNode)?;

			ClustersNodes::<T>::remove(cluster_id, node_pub_key.clone());

			Ok(())
		}
	}

	impl<T: Config> ClusterCreator<T, BalanceOf<T>> for Pallet<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		/// Creates a DDC cluster.
		/// NOTE: Required for the benchmarking only.
		fn create_new_cluster(
			cluster_id: ClusterId,
			cluster_manager_id: T::AccountId,
			cluster_reserve_id: T::AccountId,
			cluster_params: ClusterParams<T::AccountId>,
			cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			Self::do_create_cluster(
				cluster_id,
				cluster_manager_id,
				cluster_reserve_id,
				cluster_params,
				cluster_gov_params,
			)
		}
	}

	impl<T> From<StakingVisitorError> for Error<T> {
		fn from(error: StakingVisitorError) -> Self {
			match error {
				StakingVisitorError::NodeStakeDoesNotExist => Error::<T>::NodeHasNoActivatedStake,
				StakingVisitorError::NodeStakeIsInBadState => Error::<T>::NodeStakeIsInvalid,
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

	impl<T> From<ClusterManagerError> for Error<T> {
		fn from(error: ClusterManagerError) -> Self {
			match error {
				ClusterManagerError::AttemptToRemoveNotAssignedNode =>
					Error::<T>::AttemptToRemoveNotAssignedNode,
				ClusterManagerError::AttemptToRemoveNonExistentNode =>
					Error::<T>::AttemptToRemoveNonExistentNode,
				ClusterManagerError::AttemptToAddNonExistentNode =>
					Error::<T>::AttemptToAddNonExistentNode,
				ClusterManagerError::AttemptToAddAlreadyAssignedNode =>
					Error::<T>::AttemptToAddAlreadyAssignedNode,
			}
		}
	}
}
