#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::{prelude::vec::Vec, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::hash::H160;
use sp_runtime::{AccountId32, Perquintill, RuntimeDebug};

pub const MILLICENTS: u128 = 100_000;
pub const CENTS: u128 = 1_000 * MILLICENTS; // assume this is worth about a cent.
pub const DOLLARS: u128 = 100 * CENTS;
/// DDC Cluster identifier.
pub type ClusterId = H160;
pub type DdcEra = u32;
pub type BucketId = u64;
pub type StorageNodePubKey = AccountId32;

// ClusterParams includes Governance non-sensetive parameters only
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterParams<AccountId> {
	pub node_provider_auth_contract: Option<AccountId>,
}

/// List of all economic parameters for DDC Cluster locked by Governance.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Default)]
#[scale_info(skip_type_params(Balance, BlockNumber, T))]
pub struct ClusterGovParams<Balance, BlockNumber> {
	/// Share from DAC payout cycles that goes to the Treasury Fund.
	pub treasury_share: Perquintill,
	/// Share from DAC payout cycles that goes to Validators.
	pub validators_share: Perquintill,
	/// Share from DAC payout cycles that goes to the cluster reserve account.
	pub cluster_reserve_share: Perquintill,
	/// Minimum amount of CERE tokens to bond by a DDC node to serve the cluster.
	pub storage_bond_size: Balance,
	/// Minimum delay before a serving DDC node can be stopped for maintenance without a risk of
	/// being slashed.
	pub storage_chill_delay: BlockNumber,
	/// Minimum delay before a DDC node provider can fully unbond tokens to leave the serving
	/// cluster.
	pub storage_unbonding_delay: BlockNumber,
	/// Amount of tokens to pay for 1 MB of stored data.
	pub unit_per_mb_stored: u128,
	/// Amount of tokens to pay for 1 MB of streamed data.
	pub unit_per_mb_streamed: u128,
	/// Amount of tokens to pay for 1 PUT request.
	pub unit_per_put_request: u128,
	/// Amount of tokens to pay for 1 GET request.
	pub unit_per_get_request: u128,
}

/// DDC cluster pricing parameters for data storing and data streaming
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterPricingParams {
	/// Amount of tokens to pay for 1 MB of stored data.
	pub unit_per_mb_stored: u128,
	/// Amount of tokens to pay for 1 MB of streamed data.
	pub unit_per_mb_streamed: u128,
	/// Amount of tokens to pay for 1 PUT request.
	pub unit_per_put_request: u128,
	/// Amount of tokens to pay for 1 GET request.
	pub unit_per_get_request: u128,
}

/// DDC cluster fee parameters charged in DAC payout cycles
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterFeesParams {
	/// Share from DAC payout cycles that goes to the Treasury Fund.
	pub treasury_share: Perquintill,
	/// Share from DAC payout cycles that goes to Validators.
	pub validators_share: Perquintill,
	/// Share from DAC payout cycles that goes to the cluster reserve account.
	pub cluster_reserve_share: Perquintill,
}

/// DDC cluster bonding parameters for serving DDC nodes
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct ClusterBondingParams<BlockNumber> {
	/// Minimum amount of CERE tokens to bond by a DDC node to serve the cluster.
	pub storage_bond_size: u128,
	/// Minimum delay before a serving DDC node can be stopped for maintenance without a risk of
	/// being slashed.
	pub storage_chill_delay: BlockNumber,
	/// Minimum delay before a DDC node provider can fully unbond tokens to leave the serving
	/// cluster.
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
}

impl From<NodeType> for u8 {
	fn from(node_type: NodeType) -> Self {
		match node_type {
			NodeType::Storage => 1,
		}
	}
}

impl TryFrom<u8> for NodeType {
	type Error = ();
	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			1 => Ok(NodeType::Storage),
			_ => Err(()),
		}
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum StorageNodeMode {
	/// DDC Storage node operates with enabled caching in RAM and stores data in Hard Drive
	Full = 1,
	/// DDC Storage node operates with disabled caching in RAM and stores data in Hard Drive
	Storage = 2,
	/// DDC Storage node operates with enabled caching in RAM and doesn't store data in Hard Drive
	Cache = 3,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub struct StorageNodeParams {
	pub mode: StorageNodeMode,
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
	StorageParams(StorageNodeParams),
}
