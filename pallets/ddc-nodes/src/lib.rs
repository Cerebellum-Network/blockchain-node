//! # DDC Nodes Pallet
//!
//! The DDC Nodes pallet is used to manage nodes in DDC Cluster
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## GenesisConfig
//!
//! The DDC Nodes pallet depends on the [`GenesisConfig`]. The
//! `GenesisConfig` is optional and allow to set some initial nodes in DDC.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::hash::H160;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

pub use pallet::*;
mod cdn_node;
mod node;
mod storage_node;

pub use crate::{
	cdn_node::{CDNNode, CDNNodePubKey},
	node::{Node, NodeError, NodeParams, NodePubKey, NodeTrait},
	storage_node::{StorageNode, StorageNodePubKey},
};

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		NodeCreated { node_type: u8, node_pub_key: NodePubKey },
	}

	#[pallet::error]
	pub enum Error<T> {
		NodeAlreadyExists,
		NodeDoesNotExist,
		InvalidNodeParams,
	}

	#[pallet::storage]
	#[pallet::getter(fn storage_nodes)]
	pub type StorageNodes<T: Config> =
		StorageMap<_, Blake2_128Concat, StorageNodePubKey, StorageNode<T::AccountId>>;

	#[pallet::storage]
	#[pallet::getter(fn cdn_nodes)]
	pub type CDNNodes<T: Config> =
		StorageMap<_, Blake2_128Concat, CDNNodePubKey, CDNNode<T::AccountId>>;

	// todo: add the type to the Config
	pub type ClusterId = H160;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create_node(origin: OriginFor<T>, node_params: NodeParams) -> DispatchResult {
			let provider_id = ensure_signed(origin)?;
			let node = Node::<T::AccountId>::from_params(provider_id, node_params)
				.map_err(|e| Into::<Error<T>>::into(NodeError::from(e)))?;
			let node_type = node.get_type();
			let node_pub_key = node.get_pub_key().to_owned();
			Self::create(node)?;
			Self::deposit_event(Event::<T>::NodeCreated {
				node_type: node_type.into(),
				node_pub_key,
			});
			Ok(())
		}
	}

	pub trait NodeRepository<T: frame_system::Config> {
		fn create(node: Node<T::AccountId>) -> Result<(), &'static str>;
		fn get(pub_key: NodePubKey) -> Result<Node<T::AccountId>, &'static str>;
		fn add_to_cluster(pub_key: NodePubKey, cluster_id: ClusterId) -> Result<(), &'static str>;
	}

	impl<T: Config> NodeRepository<T> for Pallet<T> {
		fn create(node: Node<T::AccountId>) -> Result<(), &'static str> {
			match node {
				Node::Storage(storage_node) => {
					if StorageNodes::<T>::contains_key(&storage_node.pub_key) {
						return Err("Node already exists")
					}
					StorageNodes::<T>::insert(storage_node.pub_key.clone(), storage_node);
					Ok(())
				},
				Node::CDN(cdn_node) => {
					if CDNNodes::<T>::contains_key(&cdn_node.pub_key) {
						return Err("Node already exists")
					}
					CDNNodes::<T>::insert(cdn_node.pub_key.clone(), cdn_node);
					Ok(())
				},
			}
		}

		fn get(node_pub_key: NodePubKey) -> Result<Node<T::AccountId>, &'static str> {
			match node_pub_key {
				NodePubKey::StoragePubKey(pub_key) => match StorageNodes::<T>::try_get(pub_key) {
					Ok(node) => Ok(Node::Storage(node)),
					Err(_) => Err("Node does not exist"),
				},
				NodePubKey::CDNPubKey(pub_key) => match CDNNodes::<T>::try_get(pub_key) {
					Ok(node) => Ok(Node::CDN(node)),
					Err(_) => Err("Node does not exist"),
				},
			}
		}

		fn add_to_cluster(pub_key: NodePubKey, cluster_id: ClusterId) -> Result<(), &'static str> {
			let node = Self::get(pub_key)?;
			match node {
				Node::Storage(mut storage_node) => {
					storage_node.cluster_id = Some(cluster_id);
					StorageNodes::<T>::insert(storage_node.pub_key.clone(), storage_node);
				},
				Node::CDN(mut cdn_node) => {
					cdn_node.cluster_id = Some(cluster_id);
					CDNNodes::<T>::insert(cdn_node.pub_key.clone(), cdn_node);
				},
			}
			Ok(())
		}
	}
}
