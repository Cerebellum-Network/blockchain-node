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
	pub trait Config:
		frame_system::Config + pallet_contracts::Config + pallet_ddc_staking::Config
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type NodeRepository: NodeRepository<Self>;
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
		NotAuthorized,
		NoStake,
		/// Conditions for fast chill are not met, try the regular `chill` from
		/// `pallet-ddc-staking`.
		FastChillProhibited,
		/// Cluster candidate should not plan to chill.
		ChillingProhibited,
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
		ValueQuery,
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
			let mut node = T::NodeRepository::get(node_pub_key.clone())
				.map_err(|_| Error::<T>::AttemptToAddNonExistentNode)?;
			ensure!(node.get_cluster_id().is_none(), Error::<T>::NodeIsAlreadyAssigned);

			let is_authorized: bool = pallet_contracts::Pallet::<T>::bare_call(
				caller_id,
				cluster.props.node_provider_auth_contract,
				Default::default(),
				EXTENSION_CALL_GAS_LIMIT,
				None,
				Vec::from(INK_SELECTOR_IS_AUTHORIZED),
				false,
			)
			.result?
			.data
			.first()
			.is_some_and(|x| *x == 1);
			ensure!(is_authorized, Error::<T>::NotAuthorized);

			let node_provider_stash =
				<pallet_ddc_staking::Pallet<T>>::nodes(&node_pub_key).ok_or(Error::<T>::NoStake)?;
			let maybe_edge_in_cluster =
				<pallet_ddc_staking::Pallet<T>>::edges(&node_provider_stash);
			let maybe_storage_in_cluster =
				<pallet_ddc_staking::Pallet<T>>::storages(&node_provider_stash);
			let has_stake = maybe_edge_in_cluster
				.or(maybe_storage_in_cluster)
				.is_some_and(|staking_cluster| staking_cluster == cluster_id);
			ensure!(has_stake, Error::<T>::NoStake);

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

		/// Allow cluster node candidate to chill in the next DDC era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller.
		#[pallet::weight(10_000)]
		pub fn fast_chill(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			let stash = <pallet_ddc_staking::Pallet<T>>::ledger(&controller)
				.ok_or(<pallet_ddc_staking::Error<T>>::NotController)?
				.stash;
			let node = <pallet_ddc_staking::pallet::Nodes<T>>::iter()
				.find(|(_, v)| *v == stash)
				.ok_or(<pallet_ddc_staking::Error<T>>::BadState)?
				.0;
			let cluster = <pallet_ddc_staking::Pallet<T>>::edges(&stash)
				.or(<pallet_ddc_staking::Pallet<T>>::storages(&stash))
				.ok_or(Error::<T>::NoStake)?;
			let is_cluster_node = ClustersNodes::<T>::get(cluster, node);
			ensure!(!is_cluster_node, Error::<T>::FastChillProhibited);

			let can_chill_from = <pallet_ddc_staking::Pallet<T>>::current_era().unwrap_or(0) + 1;
			<pallet_ddc_staking::Pallet<T>>::chill_stash_soon(
				&stash,
				&controller,
				cluster,
				can_chill_from,
			);

			Ok(())
		}
	}
}
