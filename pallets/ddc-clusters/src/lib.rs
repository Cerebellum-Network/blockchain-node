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
#![feature(is_some_and)] // ToDo: delete at rustc > 1.70

use crate::{
	cluster::{Cluster, ClusterGovParams, ClusterParams},
	node_provider_auth::{NodeProviderAuthContract, NodeProviderAuthContractError},
};
use ddc_primitives::{ClusterId, NodePubKey, NodeType};
use ddc_traits::{
	cluster::{ClusterVisitor, ClusterVisitorError},
	staking::{StakingVisitor, StakingVisitorError},
};
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, LockableCurrency},
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use pallet_ddc_nodes::{NodeRepository, NodeTrait};
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

mod cluster;
mod node_provider_auth;

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use pallet_contracts::chain_extension::UncheckedFrom;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_contracts::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type NodeRepository: NodeRepository<Self>; // todo: get rid of tight coupling with nodes-pallet
		type StakingVisitor: StakingVisitor<Self>;
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		ClusterCreated { cluster_id: ClusterId },
		ClusterNodeAdded { cluster_id: ClusterId, node_pub_key: NodePubKey },
		ClusterNodeRemoved { cluster_id: ClusterId, node_pub_key: NodePubKey },
		ClusterParamsSet { cluster_id: ClusterId },
		ClusterGovParamsSet { cluster_id: ClusterId },
	}

	#[pallet::error]
	pub enum Error<T> {
		ClusterAlreadyExists,
		ClusterDoesNotExist,
		ClusterParamsExceedsLimit,
		AttemptToAddNonExistentNode,
		AttemptToRemoveNonExistentNode,
		NodeIsAlreadyAssigned,
		NodeIsNotAssigned,
		OnlyClusterManager,
		NodeIsNotAuthorized,
		NodeHasNoStake,
		NodeStakeIsInvalid,
		/// Cluster candidate should not plan to chill.
		NodeChillingIsProhibited,
		NodeAuthContractCallFailed,
	}

	#[pallet::storage]
	#[pallet::getter(fn clusters)]
	pub type Clusters<T: Config> =
		StorageMap<_, Blake2_128Concat, ClusterId, Cluster<T::AccountId>>;

	#[pallet::storage]
	#[pallet::getter(fn clusters_gov_params)]
	pub type ClustersGovParams<T: Config> =
		StorageMap<_, Twox64Concat, ClusterId, ClusterGovParams<BalanceOf<T>, T::BlockNumber>>;

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

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		#[pallet::weight(10_000)]
		pub fn create_cluster(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			cluster_manager_id: T::AccountId,
			cluster_reserve_id: T::AccountId,
			cluster_params: ClusterParams<T::AccountId>,
			cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			ensure_root(origin)?; // requires Governance approval
			let cluster =
				Cluster::new(cluster_id, cluster_manager_id, cluster_reserve_id, cluster_params)
					.map_err(Into::<Error<T>>::into)?;
			ensure!(!Clusters::<T>::contains_key(cluster_id), Error::<T>::ClusterAlreadyExists);

			Clusters::<T>::insert(cluster_id, cluster);
			ClustersGovParams::<T>::insert(cluster_id, cluster_gov_params);
			Self::deposit_event(Event::<T>::ClusterCreated { cluster_id });

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn add_node(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			ensure!(cluster.manager_id == caller_id, Error::<T>::OnlyClusterManager);

			// Node with this node with this public key exists.
			let mut node = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;
			ensure!(node.get_cluster_id().is_none(), Error::<T>::NodeIsAlreadyAssigned);

			// Sufficient funds are locked at the DDC Staking module.
			let has_stake = T::StakingVisitor::node_has_stake(&node_pub_key, &cluster_id)
				.map_err(Into::<Error<T>>::into)?;
			ensure!(has_stake, Error::<T>::NodeHasNoStake);

			// Candidate is not planning to pause operations any time soon.
			let is_chilling = T::StakingVisitor::node_is_chilling(&node_pub_key)
				.map_err(Into::<Error<T>>::into)?;
			ensure!(!is_chilling, Error::<T>::NodeChillingIsProhibited);

			// Cluster extension smart contract allows joining.
			let auth_contract = NodeProviderAuthContract::<T>::new(
				cluster.props.node_provider_auth_contract,
				caller_id,
			);
			let is_authorized = auth_contract
				.is_authorized(
					node.get_provider_id().to_owned(),
					node.get_pub_key(),
					node.get_type(),
				)
				.map_err(Into::<Error<T>>::into)?;
			ensure!(is_authorized, Error::<T>::NodeIsNotAuthorized);

			// Add node to the cluster.
			node.set_cluster_id(Some(cluster_id));
			T::NodeRepository::update(node).map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;
			ClustersNodes::<T>::insert(cluster_id, node_pub_key.clone(), true);
			Self::deposit_event(Event::<T>::ClusterNodeAdded { cluster_id, node_pub_key });

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn remove_node(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			node_pub_key: NodePubKey,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			let cluster =
				Clusters::<T>::try_get(cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			ensure!(cluster.manager_id == caller_id, Error::<T>::OnlyClusterManager);
			let mut node = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| Error::<T>::AttemptToRemoveNonExistentNode)?;
			ensure!(node.get_cluster_id() == &Some(cluster_id), Error::<T>::NodeIsNotAssigned);
			node.set_cluster_id(None);
			T::NodeRepository::update(node)
				.map_err(|_| Error::<T>::AttemptToRemoveNonExistentNode)?;
			ClustersNodes::<T>::remove(cluster_id, node_pub_key.clone());
			Self::deposit_event(Event::<T>::ClusterNodeRemoved { cluster_id, node_pub_key });

			Ok(())
		}

		// Sets Governance non-sensetive parameters only
		#[pallet::weight(10_000)]
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

		// Requires Governance approval
		#[pallet::weight(10_000)]
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

	impl<T: Config> ClusterVisitor<T> for Pallet<T> {
		fn cluster_has_node(cluster_id: &ClusterId, node_pub_key: &NodePubKey) -> bool {
			ClustersNodes::<T>::get(cluster_id, node_pub_key).is_some()
		}

		fn ensure_cluster(cluster_id: &ClusterId) -> Result<(), ClusterVisitorError> {
			Clusters::<T>::get(cluster_id)
				.map(|_| ())
				.ok_or(ClusterVisitorError::ClusterDoesNotExist)
		}

		fn get_bond_size(
			cluster_id: &ClusterId,
			node_type: NodeType,
		) -> Result<u128, ClusterVisitorError> {
			let cluster_gov_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| ClusterVisitorError::ClusterGovParamsNotSet)?;
			match node_type {
				NodeType::Storage =>
					Ok(cluster_gov_params.storage_bond_size.saturated_into::<u128>()),
				NodeType::CDN => Ok(cluster_gov_params.cdn_bond_size.saturated_into::<u128>()),
			}
		}

		fn get_chill_delay(
			cluster_id: &ClusterId,
			node_type: NodeType,
		) -> Result<T::BlockNumber, ClusterVisitorError> {
			let cluster_gov_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| ClusterVisitorError::ClusterGovParamsNotSet)?;
			match node_type {
				NodeType::Storage => Ok(cluster_gov_params.storage_chill_delay),
				NodeType::CDN => Ok(cluster_gov_params.cdn_chill_delay),
			}
		}

		fn get_unbonding_delay(
			cluster_id: &ClusterId,
			node_type: NodeType,
		) -> Result<T::BlockNumber, ClusterVisitorError> {
			let cluster_gov_params = ClustersGovParams::<T>::try_get(cluster_id)
				.map_err(|_| ClusterVisitorError::ClusterGovParamsNotSet)?;
			match node_type {
				NodeType::Storage => Ok(cluster_gov_params.storage_unbonding_delay),
				NodeType::CDN => Ok(cluster_gov_params.cdn_unbonding_delay),
			}
		}
	}

	impl<T> From<StakingVisitorError> for Error<T> {
		fn from(error: StakingVisitorError) -> Self {
			match error {
				StakingVisitorError::NodeStakeDoesNotExist => Error::<T>::NodeHasNoStake,
				StakingVisitorError::NodeStakeIsInBadState => Error::<T>::NodeStakeIsInvalid,
			}
		}
	}

	impl<T> From<NodeProviderAuthContractError> for Error<T> {
		fn from(error: NodeProviderAuthContractError) -> Self {
			match error {
				NodeProviderAuthContractError::ContractCallFailed =>
					Error::<T>::NodeAuthContractCallFailed,
			}
		}
	}
}
