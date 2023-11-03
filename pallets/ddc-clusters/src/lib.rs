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

use ddc_primitives::{ClusterId, NodePubKey};
use ddc_traits::{
	cluster::{ClusterVisitor, ClusterVisitorError},
	staking::{StakingVisitor, StakingVisitorError},
};
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
pub use pallet::*;
use pallet_ddc_nodes::{NodeRepository, NodeTrait};
use sp_std::prelude::*;
mod cluster;

pub use crate::cluster::{Cluster, ClusterError, ClusterParams};

/// ink! 4.x selector for the "is_authorized" message, equals to the first four bytes of the
/// blake2("is_authorized"). See also: https://use.ink/basics/selectors#selector-calculation/,
/// https://use.ink/macros-attributes/selector/.
const INK_SELECTOR_IS_AUTHORIZED: [u8; 4] = [0x96, 0xb0, 0x45, 0x3e];

/// The maximum amount of weight that the cluster extension contract call is allowed to consume.
/// See also https://github.com/paritytech/substrate/blob/a3ed0119c45cdd0d571ad34e5b3ee7518c8cef8d/frame/contracts/rpc/src/lib.rs#L63.
const EXTENSION_CALL_GAS_LIMIT: Weight = Weight::from_ref_time(5_000_000_000_000);

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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		ClusterCreated { cluster_id: ClusterId },
		ClusterNodeAdded { cluster_id: ClusterId, node_pub_key: NodePubKey },
		ClusterNodeRemoved { cluster_id: ClusterId, node_pub_key: NodePubKey },
		ClusterParamsSet { cluster_id: ClusterId },
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
	}

	#[pallet::storage]
	#[pallet::getter(fn clusters)]
	pub type Clusters<T: Config> =
		StorageMap<_, Blake2_128Concat, ClusterId, Cluster<T::AccountId>>;

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
			cluster_params: ClusterParams<T::AccountId>,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			let cluster = Cluster::new(cluster_id.clone(), caller_id, cluster_params)
				.map_err(|e: ClusterError| Into::<Error<T>>::into(ClusterError::from(e)))?;
			ensure!(!Clusters::<T>::contains_key(&cluster_id), Error::<T>::ClusterAlreadyExists);
			Clusters::<T>::insert(cluster_id.clone(), cluster);
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
				Clusters::<T>::try_get(&cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			ensure!(cluster.manager_id == caller_id, Error::<T>::OnlyClusterManager);

			// Node with this node with this public key exists
			let mut node = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;
			ensure!(node.get_cluster_id().is_none(), Error::<T>::NodeIsAlreadyAssigned);

			// Sufficient funds are locked at the DDC Staking module.
			let has_stake = T::StakingVisitor::node_has_stake(&node_pub_key, &cluster_id)
				.map_err(|e| Into::<Error<T>>::into(StakingVisitorError::from(e)))?;
			ensure!(has_stake, Error::<T>::NodeHasNoStake);

			// Candidate is not planning to pause operations any time soon.
			let is_chilling = T::StakingVisitor::node_is_chilling(&node_pub_key)
				.map_err(|e| Into::<Error<T>>::into(StakingVisitorError::from(e)))?;
			ensure!(!is_chilling, Error::<T>::NodeChillingIsProhibited);

			// Cluster extension smart contract allows joining.
			let call_data = {
				// is_authorized(node_provider: AccountId, node: Vec<u8>, node_variant: u8) -> bool
				let args: ([u8; 4], T::AccountId, Vec<u8>, u8) = (
					INK_SELECTOR_IS_AUTHORIZED,
					node.get_provider_id().to_owned(),
					/* remove the first byte* added by SCALE */
					node.get_pub_key().to_owned().encode()[1..].to_vec(),
					node.get_type().into(),
				);
				args.encode()
			};
			let is_authorized = pallet_contracts::Pallet::<T>::bare_call(
				caller_id,
				cluster.props.node_provider_auth_contract,
				Default::default(),
				EXTENSION_CALL_GAS_LIMIT,
				None,
				call_data,
				false,
			)
			.result?
			.data
			.first()
			.is_some_and(|x| *x == 1);
			ensure!(is_authorized, Error::<T>::NodeIsNotAuthorized);

			node.set_cluster_id(Some(cluster_id.clone()));
			T::NodeRepository::update(node).map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;
			ClustersNodes::<T>::insert(cluster_id.clone(), node_pub_key.clone(), true);
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
				Clusters::<T>::try_get(&cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			ensure!(cluster.manager_id == caller_id, Error::<T>::OnlyClusterManager);
			let mut node = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| Error::<T>::AttemptToRemoveNonExistentNode)?;
			ensure!(node.get_cluster_id() == &Some(cluster_id), Error::<T>::NodeIsNotAssigned);
			node.set_cluster_id(None);
			T::NodeRepository::update(node)
				.map_err(|_| Error::<T>::AttemptToRemoveNonExistentNode)?;
			ClustersNodes::<T>::remove(cluster_id.clone(), node_pub_key.clone());
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
				Clusters::<T>::try_get(&cluster_id).map_err(|_| Error::<T>::ClusterDoesNotExist)?;
			ensure!(cluster.manager_id == caller_id, Error::<T>::OnlyClusterManager);
			cluster
				.set_params(cluster_params)
				.map_err(|e: ClusterError| Into::<Error<T>>::into(ClusterError::from(e)))?;
			Clusters::<T>::insert(cluster_id.clone(), cluster);
			Self::deposit_event(Event::<T>::ClusterParamsSet { cluster_id });

			Ok(())
		}
	}

	impl<T: Config> ClusterVisitor<T> for Pallet<T> {
		fn cluster_has_node(cluster_id: &ClusterId, node_pub_key: &NodePubKey) -> bool {
			ClustersNodes::<T>::get(cluster_id, node_pub_key).is_some()
		}

		fn ensure_cluster(cluster_id: &ClusterId) -> Result<(), ClusterVisitorError> {
			Clusters::<T>::get(&cluster_id)
				.map(|_| ())
				.ok_or(ClusterVisitorError::ClusterDoesNotExist)
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
}
