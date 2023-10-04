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
		NodeCreated(NodeType, NodePubKey),
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
		StorageMap<_, Blake2_128Concat, StorageNodePubKey, StorageNode>;

	#[pallet::storage]
	#[pallet::getter(fn cdn_nodes)]
	pub type CDNNodes<T: Config> = StorageMap<_, Blake2_128Concat, CDNNodePubKey, CDNNode>;

	// todo: add the type to the Config
	type ClusterId = H160;

	type StorageNodePubKey = sp_runtime::AccountId32;
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct StorageNode {
		pub_key: StorageNodePubKey,
		cluster_id: Option<ClusterId>,
		props: StorageNodeProps,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct StorageNodeProps {
		capacity: u32,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct StorageNodeParams {
		pub_key: StorageNodePubKey,
		capacity: u32,
	}

	type CDNNodePubKey = sp_runtime::AccountId32;
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct CDNNode {
		pub_key: CDNNodePubKey,
		cluster_id: Option<ClusterId>,
		props: CDNNodeProps,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct CDNNodeProps {
		url: Vec<u8>,
		location: [u8; 2],
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct CDNNodeParams {
		pub_key: CDNNodePubKey,
		url: Vec<u8>,
		location: [u8; 2],
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub enum Node {
		Storage(StorageNode),
		CDN(CDNNode),
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub enum NodeParams {
		StorageParams(StorageNodeParams),
		CDNParams(CDNNodeParams),
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub enum NodePubKey {
		StoragePubKey(StorageNodePubKey),
		CDNPubKey(CDNNodePubKey),
	}

	#[derive(Clone, RuntimeDebug, PartialEq)]
	pub enum NodePubKeyRef<'a> {
		StoragePubKeyRef(&'a StorageNodePubKey),
		CDNPubKeyRef(&'a CDNNodePubKey),
	}

	impl<'a> NodePubKeyRef<'a> {
		pub fn to_owned(&self) -> NodePubKey {
			match &self {
				NodePubKeyRef::StoragePubKeyRef(pub_key_ref) =>
					NodePubKey::StoragePubKey((**pub_key_ref).clone()),
				NodePubKeyRef::CDNPubKeyRef(pub_key_ref) =>
					NodePubKey::CDNPubKey((**pub_key_ref).clone()),
			}
		}
	}

	#[derive(Clone, RuntimeDebug, PartialEq)]
	pub enum NodePropsRef<'a> {
		StoragePropsRef(&'a StorageNodeProps),
		CDNPropsRef(&'a CDNNodeProps),
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub enum NodeType {
		Storage = 1,
		CDN = 2,
	}

	pub trait NodeTrait {
		fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a>;
		fn get_props<'a>(&'a self) -> NodePropsRef<'a>;
		fn get_type(&self) -> NodeType;
		fn from_params<T: Config>(params: NodeParams) -> Result<Node, pallet::Error<T>>;
	}

	impl NodeTrait for StorageNode {
		fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a> {
			NodePubKeyRef::StoragePubKeyRef(&self.pub_key)
		}
		fn get_props<'a>(&'a self) -> NodePropsRef<'a> {
			NodePropsRef::StoragePropsRef(&self.props)
		}
		fn get_type(&self) -> NodeType {
			NodeType::Storage
		}
		fn from_params<T: Config>(params: NodeParams) -> Result<Node, pallet::Error<T>> {
			match params {
				NodeParams::StorageParams(params) => Ok(Node::Storage(StorageNode {
					pub_key: params.pub_key,
					cluster_id: None,
					props: StorageNodeProps { capacity: params.capacity },
				})),
				_ => Err(Error::<T>::NodeAlreadyExists),
			}
		}
	}

	impl NodeTrait for CDNNode {
		fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a> {
			NodePubKeyRef::CDNPubKeyRef(&self.pub_key)
		}
		fn get_props<'a>(&'a self) -> NodePropsRef<'a> {
			NodePropsRef::CDNPropsRef(&self.props)
		}
		fn get_type(&self) -> NodeType {
			NodeType::CDN
		}
		fn from_params<T: Config>(params: NodeParams) -> Result<Node, pallet::Error<T>> {
			match params {
				NodeParams::CDNParams(params) => Ok(Node::CDN(CDNNode {
					pub_key: params.pub_key,
					cluster_id: None,
					props: CDNNodeProps { url: params.url, location: params.location },
				})),
				_ => Err(Error::<T>::NodeAlreadyExists),
			}
		}
	}

	impl NodeTrait for Node {
		fn get_pub_key<'a>(&'a self) -> NodePubKeyRef<'a> {
			match &self {
				Node::Storage(node) => node.get_pub_key(),
				Node::CDN(node) => node.get_pub_key(),
			}
		}
		fn get_props<'a>(&'a self) -> NodePropsRef<'a> {
			match &self {
				Node::Storage(node) => node.get_props(),
				Node::CDN(node) => node.get_props(),
			}
		}
		fn get_type(&self) -> NodeType {
			match &self {
				Node::Storage(node) => node.get_type(),
				Node::CDN(node) => node.get_type(),
			}
		}
		fn from_params<T: Config>(params: NodeParams) -> Result<Node, pallet::Error<T>> {
			match params {
				NodeParams::StorageParams(_) => StorageNode::from_params(params),
				NodeParams::CDNParams(_) => CDNNode::from_params(params),
			}
		}
	}

	impl From<NodeType> for u8 {
		fn from(node_type: NodeType) -> Self {
			match node_type {
				NodeType::Storage => 1,
				NodeType::CDN => 2,
			}
		}
	}

	impl TryFrom<u8> for NodeType {
		type Error = ();
		fn try_from(value: u8) -> Result<Self, Self::Error> {
			match value {
				1 => Ok(NodeType::Storage),
				2 => Ok(NodeType::CDN),
				_ => Err(()),
			}
		}
	}

	pub trait NodeRepositoryTrait {
		fn create<T: Config>(node: Node) -> Result<(), pallet::Error<T>>;
		fn get<T: Config>(pub_key: NodePubKey) -> Result<Node, pallet::Error<T>>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create_node(origin: OriginFor<T>, node_params: NodeParams) -> DispatchResult {
			ensure_signed(origin)?;
			let node: Node = Node::from_params::<T>(node_params)?;
			let node_pub_key = node.get_pub_key().to_owned();
			let node_type = node.get_type();
			Self::create(node)?;
			Self::deposit_event(Event::<T>::NodeCreated(node_type, node_pub_key));
			Ok(())
		}
	}

	pub trait NodeRepository {
		fn create(node: Node) -> Result<(), &'static str>;
		fn get(pub_key: NodePubKey) -> Result<Node, &'static str>;
		fn add_to_cluster(pub_key: NodePubKey, cluster_id: ClusterId) -> Result<(), &'static str>;
	}

	impl<T: Config> NodeRepository for Pallet<T> {
		fn create(node: Node) -> Result<(), &'static str> {
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

		fn get(node_pub_key: NodePubKey) -> Result<Node, &'static str> {
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
			let mut node = Self::get(pub_key)?;
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
