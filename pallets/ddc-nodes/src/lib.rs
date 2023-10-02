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
		NodeCreated(NodeType),
	}

	#[pallet::error]
	pub enum Error<T> {
		NodeAlreadyExists,
		InvalidNodeParams,
	}

	/// Map from all (unlocked) "controller" accounts to the info regarding the staking.
	#[pallet::storage]
	#[pallet::getter(fn storage_nodes)]
	pub type StorageNodes<T: Config> =
		StorageMap<_, Blake2_128Concat, StorageNodePubKey, StorageNode>;

	#[pallet::storage]
	#[pallet::getter(fn cdn_nodes)]
	pub type CDNNodes<T: Config> = StorageMap<_, Blake2_128Concat, CDNNodePubKey, CDNNode>;

	type StorageNodePubKey = sp_runtime::AccountId32;
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct StorageNode {
		key: StorageNodePubKey,
		status: u8,
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
		key: CDNNodePubKey,
		status: u8,
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

	#[derive(Clone, RuntimeDebug, PartialEq)]
	pub enum NodePubKey<'a> {
		StoragePubKey(&'a StorageNodePubKey),
		CDNPubKey(&'a CDNNodePubKey),
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub enum OwnableNodePubKey {
		StoragePubKey(StorageNodePubKey),
		CDNPubKey(CDNNodePubKey),
	}

	#[derive(Clone, RuntimeDebug, PartialEq)]
	pub enum NodeProps<'a> {
		StorageProps(&'a StorageNodeProps),
		CDNProps(&'a CDNNodeProps),
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub enum NodeType {
		Storage = 1,
		CDN = 2,
	}

	pub trait NodeTrait {
		fn get_pub_key<'a>(&'a self) -> NodePubKey<'a>;
		fn get_props<'a>(&'a self) -> NodeProps<'a>;
		fn get_type(&self) -> NodeType;
		fn from_params<T: Config>(params: NodeParams) -> Result<Node, pallet::Error<T>>;
	}

	impl NodeTrait for StorageNode {
		fn get_pub_key<'a>(&'a self) -> NodePubKey<'a> {
			NodePubKey::StoragePubKey(&self.key)
		}
		fn get_props<'a>(&'a self) -> NodeProps<'a> {
			NodeProps::StorageProps(&self.props)
		}
		fn get_type(&self) -> NodeType {
			NodeType::Storage
		}
		fn from_params<T: Config>(params: NodeParams) -> Result<Node, pallet::Error<T>> {
			match params {
				NodeParams::StorageParams(params) => Ok(Node::Storage(StorageNode {
					key: params.pub_key,
					status: 1,
					props: StorageNodeProps { capacity: params.capacity },
				})),
				_ => Err(Error::<T>::NodeAlreadyExists),
			}
		}
	}

	impl NodeTrait for CDNNode {
		fn get_pub_key<'a>(&'a self) -> NodePubKey<'a> {
			NodePubKey::CDNPubKey(&self.key)
		}
		fn get_props<'a>(&'a self) -> NodeProps<'a> {
			NodeProps::CDNProps(&self.props)
		}
		fn get_type(&self) -> NodeType {
			NodeType::CDN
		}
		fn from_params<T: Config>(params: NodeParams) -> Result<Node, pallet::Error<T>> {
			match params {
				NodeParams::CDNParams(params) => Ok(Node::CDN(CDNNode {
					key: params.pub_key,
					status: 1,
					props: CDNNodeProps { url: params.url, location: params.location },
				})),
				_ => Err(Error::<T>::NodeAlreadyExists),
			}
		}
	}

	impl NodeTrait for Node {
		fn get_pub_key<'a>(&'a self) -> NodePubKey<'a> {
			match &self {
				Node::Storage(node) => node.get_pub_key(),
				Node::CDN(node) => node.get_pub_key(),
			}
		}
		fn get_props<'a>(&'a self) -> NodeProps<'a> {
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
		fn save<T: Config>(node: Node) -> Result<(), pallet::Error<T>>;
	}

	struct NodeRepository;
	impl NodeRepositoryTrait for NodeRepository {
		fn save<T: Config>(node: Node) -> Result<(), pallet::Error<T>> {
			match node {
				Node::Storage(storage_node) => {
					if StorageNodes::<T>::contains_key(&storage_node.key) {
						return Err(Error::<T>::NodeAlreadyExists)
					}
					StorageNodes::<T>::insert(storage_node.key.clone(), storage_node);
					Ok(())
				},
				Node::CDN(cdn_node) => {
					if CDNNodes::<T>::contains_key(&cdn_node.key) {
						return Err(Error::<T>::NodeAlreadyExists)
					}
					CDNNodes::<T>::insert(cdn_node.key.clone(), cdn_node);
					Ok(())
				},
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create_node(origin: OriginFor<T>, node_params: NodeParams) -> DispatchResult {
			let node_provider = ensure_signed(origin)?;
			let node = Node::from_params::<T>(node_params)?;
			let node_type = node.get_type();
			NodeRepository::save::<T>(node)?;
			Self::deposit_event(Event::<T>::NodeCreated(node_type));
			Ok(())
		}
	}
}
