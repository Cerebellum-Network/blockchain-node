// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

//! Kademlia DHT types.

use codec::{Decode, Encode};
use scale_info::TypeInfo;

/// A `Key` in the DHT keyspace with preserved preimage.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Encode, Decode, TypeInfo)]
pub struct Key(pub Vec<u8>);

impl Key {
    /// Returns the raw bytes of the key.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Converts the key into its raw bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl From<Vec<u8>> for Key {
    fn from(bytes: Vec<u8>) -> Self {
        Key(bytes)
    }
}

impl From<&[u8]> for Key {
    fn from(bytes: &[u8]) -> Self {
        Key(bytes.to_vec())
    }
}

/// A record in the DHT.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct Record {
    /// The key of the record.
    pub key: Key,
    /// The value of the record.
    pub value: Vec<u8>,
    /// The publisher of the record.
    pub publisher: Option<Vec<u8>>,
    /// The expiration time of the record.
    pub expires: Option<u64>,
}

impl Record {
    /// Creates a new record.
    pub fn new(key: Key, value: Vec<u8>) -> Self {
        Self {
            key,
            value,
            publisher: None,
            expires: None,
        }
    }

    /// Creates a new record with publisher.
    pub fn with_publisher(key: Key, value: Vec<u8>, publisher: Vec<u8>) -> Self {
        Self {
            key,
            value,
            publisher: Some(publisher),
            expires: None,
        }
    }
}

/// A peer record in the DHT.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct PeerRecord {
    /// The peer ID.
    pub peer_id: Vec<u8>,
    /// The addresses of the peer.
    pub addresses: Vec<Vec<u8>>,
}

impl PeerRecord {
    /// Creates a new peer record.
    pub fn new(peer_id: Vec<u8>, addresses: Vec<Vec<u8>>) -> Self {
        Self { peer_id, addresses }
    }
} 
