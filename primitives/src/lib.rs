#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::hash::H160;
use sp_runtime::{AccountId32, RuntimeDebug};

pub type ClusterId = H160;
pub type DdcEra = u32;
pub type BucketId = u64;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterPricingParams {
	pub unit_per_mb_stored: u128,
	pub unit_per_mb_streamed: u128,
	pub unit_per_put_request: u128,
	pub unit_per_get_request: u128,
}
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodePubKey {
	StoragePubKey(StorageNodePubKey),
	CDNPubKey(CDNNodePubKey),
}

pub type StorageNodePubKey = AccountId32;
pub type CDNNodePubKey = AccountId32;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodeType {
	Storage = 1,
	CDN = 2,
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
