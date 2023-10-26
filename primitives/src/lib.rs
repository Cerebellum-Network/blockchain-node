#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_core::hash::H160;
use sp_runtime::{AccountId32, RuntimeDebug};

pub type ClusterId = H160;
pub type DdcEra = u64;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum NodePubKey {
	StoragePubKey(StorageNodePubKey),
	CDNPubKey(CDNNodePubKey),
}

pub type StorageNodePubKey = AccountId32;
pub type CDNNodePubKey = AccountId32;
