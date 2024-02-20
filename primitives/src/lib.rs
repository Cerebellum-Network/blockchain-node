#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{parameter_types, BoundedVec};
use scale_info::{prelude::vec::Vec, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::hash::H160;
use sp_runtime::{AccountId32, Perquintill, RuntimeDebug};

pub mod traits;

parameter_types! {
	pub MaxHostLen: u8 = 255;
	pub MaxDomainLen: u8 = 255;
}
pub const MILLICENTS: u128 = 100_000;
pub const CENTS: u128 = 1_000 * MILLICENTS; // assume this is worth about a cent.
pub const DOLLARS: u128 = 100 * CENTS;
pub type ClusterId = H160;
pub type DdcEra = u32;
pub type BucketId = u64;
pub type StorageNodePubKey = AccountId32; // todo! to move to runtime

// ClusterParams includes Governance non-sensetive parameters only
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterParams<AccountId> {
	pub node_provider_auth_contract: Option<AccountId>,
}

// ClusterGovParams includes Governance sensitive parameters
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Default)]
#[scale_info(skip_type_params(Balance, BlockNumber, T))]
pub struct ClusterGovParams<Balance, BlockNumber> {
	pub treasury_share: Perquintill,
	pub validators_share: Perquintill,
	pub cluster_reserve_share: Perquintill,
	pub storage_bond_size: Balance,
	pub storage_chill_delay: BlockNumber,
	pub storage_unbonding_delay: BlockNumber,
	pub unit_per_mb_stored: u128,
	pub unit_per_mb_streamed: u128,
	pub unit_per_put_request: u128,
	pub unit_per_get_request: u128,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterPricingParams {
	pub unit_per_mb_stored: u128,
	pub unit_per_mb_streamed: u128,
	pub unit_per_put_request: u128,
	pub unit_per_get_request: u128,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterFeesParams {
	pub treasury_share: Perquintill,
	pub validators_share: Perquintill,
	pub cluster_reserve_share: Perquintill,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterBondingParams<BlockNumber> {
	pub storage_bond_size: u128,
	pub storage_chill_delay: BlockNumber,
	pub storage_unbonding_delay: BlockNumber,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodePubKey {
	StoragePubKey(StorageNodePubKey),
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodeType {
	Storage = 1,
	DAC = 4,
}

impl From<NodeType> for u16 {
	fn from(node_type: NodeType) -> Self {
		match node_type {
			NodeType::Storage => 1,
			NodeType::DAC => 4,
		}
	}
}

impl TryFrom<u16> for NodeType {
	type Error = ();
	fn try_from(value: u16) -> Result<Self, Self::Error> {
		match value {
			1 => Ok(NodeType::Storage),
			4 => Ok(NodeType::DAC),
			_ => Err(()),
		}
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodeMode {
	/// DDC Storage node operates with disabled caching in RAM and stores data in Hard Drive
	Storage = 1,
	/// DDC Storage node operates with enabled caching in RAM and doesn't store data in Hard Drive
	Cache = 2,
	/// DDC DAC node operates as aggregator of activity events
	DAC = 4,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Default, Clone, Copy, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct NodeModeFlags {
	bits: u16,
}

impl From<NodeMode> for NodeModeFlags {
	fn from(mode: NodeMode) -> Self {
		NodeModeFlags { bits: mode as u16 }
	}
}

impl NodeModeFlags {
	pub fn new() -> Self {
		Default::default()
	}

	pub fn from_modes(modes: &Vec<NodeMode>) -> Self {
		let mut bits = 0u16; // Initialize bits as u16
		for &mode in modes {
			// Directly use the copied enum variant, `mode` is now a copy of the enum variant
			bits |= mode as u16;
		}
		Self { bits }
	}

	pub fn add_mode(&mut self, mode: NodeMode) {
		self.bits |= mode as u16;
	}

	pub fn has_mode(&self, mode: NodeMode) -> bool {
		(self.bits & (mode as u16)) != 0
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
#[scale_info(skip_type_params(T))]
pub struct StorageNode<T: frame_system::Config> {
	pub pub_key: StorageNodePubKey,
	pub provider_id: T::AccountId,
	pub cluster_id: Option<ClusterId>,
	pub props: StorageNodeProps,
}

impl<T: frame_system::Config> StorageNode<T> {
	pub fn new(
		node_pub_key: NodePubKey,
		provider_id: T::AccountId,
		node_params: NodeParams,
	) -> Result<Self, NodeError> {
		match node_pub_key {
			NodePubKey::StoragePubKey(pub_key) => match node_params {
				NodeParams::StorageParams(node_params) => Ok(StorageNode::<T> {
					provider_id,
					pub_key,
					cluster_id: None,
					props: StorageNodeProps {
						mode: node_params.mode,
						host: match node_params.host.try_into() {
							Ok(vec) => vec,
							Err(_) => return Err(NodeError::StorageHostLenExceedsLimit),
						},
						domain: match node_params.domain.try_into() {
							Ok(vec) => vec,
							Err(_) => return Err(NodeError::StorageDomainLenExceedsLimit),
						},
						ssl: node_params.ssl,
						http_port: node_params.http_port,
						grpc_port: node_params.grpc_port,
						p2p_port: node_params.p2p_port,
					},
				}),
			},
		}
	}
}

#[derive(Debug, PartialEq)]
pub enum NodeError {
	StorageHostLenExceedsLimit,
	StorageDomainLenExceedsLimit,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct StorageNodeProps {
	pub host: BoundedVec<u8, MaxHostLen>,
	pub domain: BoundedVec<u8, MaxDomainLen>,
	pub ssl: bool,
	pub http_port: u16,
	pub grpc_port: u16,
	pub p2p_port: u16,
	pub mode: NodeModeFlags,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct DDCNodeParams {
	pub mode: NodeModeFlags,
	pub host: Vec<u8>,
	pub domain: Vec<u8>,
	pub ssl: bool,
	pub http_port: u16,
	pub grpc_port: u16,
	pub p2p_port: u16,
}

// Params fields are always coming from extrinsic input
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodeParams {
	StorageParams(DDCNodeParams),
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct StakingLedger<T: frame_system::Config, Balance> {
	/// The stash account whose balance is actually locked and at stake.
	pub stash: T::AccountId,

	/// The total amount of the stash's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	pub total: Balance,

	/// The total amount of the stash's balance that will be at stake in any forthcoming
	/// rounds.
	pub active: Balance,
}
