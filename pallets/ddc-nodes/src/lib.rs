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

// #[cfg(feature = "runtime-benchmarks")]
// pub mod benchmarking;
// #[cfg(any(feature = "runtime-benchmarks", test))]
// pub mod testing_utils;

// #[cfg(test)]
// pub(crate) mod mock;
// #[cfg(test)]
// mod tests;

// pub mod weights;
// use crate::weights::WeightInfo;

use codec::{Decode, Encode, HasCompact};
use frame_support::{pallet_prelude::*, BoundedVec, PalletId};
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
	pub trait Config: frame_system::Config {}

	// #[pallet::genesis_config]
	// pub struct GenesisConfig<T: Config> {

	// }

	// #[cfg(feature = "std")]
	// impl<T: Config> Default for GenesisConfig<T> {
	// 	fn default() -> Self {
	// 		GenesisConfig {}
	// 	}
	// }

	// #[pallet::genesis_build]
	// impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
	// 	fn build(&self) {}
	// }

	// #[pallet::event]
	// #[pallet::generate_deposit(pub(crate) fn deposit_event)]
	// pub enum Event<T: Config> {

	// }

	// #[pallet::error]
	// pub enum Error<T> {

	// }

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
		props: StorageProps,
	}

	impl StorageNode {
		fn from_params(params: StorageParams) -> StorageNode {
			StorageNode {
				key: params.pub_key,
				status: 1,
				props: StorageProps { capacity: params.capacity },
			}
		}
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct StorageProps {
		capacity: u32,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct StorageParams {
		pub_key: StorageNodePubKey,
		capacity: u32,
	}

	type CDNNodePubKey = sp_runtime::AccountId32;
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct CDNNode {
		key: CDNNodePubKey,
		status: u8,
		props: CDNProps,
	}

	impl CDNNode {
		fn from_params(params: CDNParams) -> CDNNode {
			CDNNode {
				key: params.pub_key,
				status: 1,
				props: CDNProps { url: params.url, location: params.location },
			}
		}
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct CDNProps {
		url: Vec<u8>,
		location: [u8; 2],
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct CDNParams {
		pub_key: CDNNodePubKey,
		url: Vec<u8>,
		location: [u8; 2],
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub enum NodeParams {
		StorageParams(StorageParams),
		CDNParams(CDNParams),
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub enum Node {
		Storage(StorageNode),
		CDN(CDNNode),
	}

	#[derive(Debug, PartialEq)]
	pub enum NodePubKey<'a> {
		StoragePubKey(&'a StorageNodePubKey),
		CDNPubKey(&'a CDNNodePubKey),
	}

	#[derive(Debug, PartialEq)]
	pub enum NodeProps<'a> {
		StorageProps(&'a StorageProps),
		CDNProps(&'a CDNProps),
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub enum NodeType {
		Storage = 1,
		CDN = 2,
	}

	pub trait NodeTrait {
		fn get_key<'a>(&'a self) -> NodePubKey<'a>;
		fn get_props<'a>(&'a self) -> NodeProps<'a>;
	}

	impl NodeTrait for StorageNode {
		fn get_key<'a>(&'a self) -> NodePubKey<'a> {
			NodePubKey::StoragePubKey(&self.key)
		}
		fn get_props<'a>(&'a self) -> NodeProps<'a> {
			NodeProps::StorageProps(&self.props)
		}
	}

	impl NodeTrait for CDNNode {
		fn get_key<'a>(&'a self) -> NodePubKey<'a> {
			NodePubKey::CDNPubKey(&self.key)
		}
		fn get_props<'a>(&'a self) -> NodeProps<'a> {
			NodeProps::CDNProps(&self.props)
		}
	}

	impl NodeTrait for Node {
		fn get_key<'a>(&'a self) -> NodePubKey<'a> {
			match &self {
				Node::Storage(node) => node.get_key(),
				Node::CDN(node) => node.get_key(),
			}
		}

		fn get_props<'a>(&'a self) -> NodeProps<'a> {
			match &self {
				Node::Storage(node) => node.get_props(),
				Node::CDN(node) => node.get_props(),
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

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create_node(origin: OriginFor<T>, node_params: NodeParams) -> DispatchResult {
			let _node_provider = ensure_signed(origin)?;

			match node_params {
				NodeParams::StorageParams(storage_params) => {
					let storage_node = StorageNode::from_params(storage_params);
					StorageNodes::<T>::insert(storage_node.key.clone(), storage_node);
				},
				NodeParams::CDNParams(cdn_params) => {
					let cdn_node = CDNNode::from_params(cdn_params);
					CDNNodes::<T>::insert(cdn_node.key.clone(), cdn_node);
				},
			}

			Ok(())
		}
	}
}
