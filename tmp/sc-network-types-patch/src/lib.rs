// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

//! Substrate network types.

pub mod kad;
pub use multiaddr;

pub use multiaddr::{Multiaddr, PeerId};

// Helper function for PeerId conversions to avoid orphan rule violations
pub fn peer_id_from_bytes(bytes: &[u8]) -> Option<PeerId> {
    PeerId::from_bytes(bytes).ok()
}

pub fn peer_id_to_bytes(peer_id: &PeerId) -> Vec<u8> {
    peer_id.to_bytes()
}

/// Build multiaddr from components
pub fn build_multiaddr(peer_id: PeerId, port: u16) -> Multiaddr {
    format!("/ip4/127.0.0.1/tcp/{}/p2p/{}", port, peer_id)
        .parse()
        .expect("Valid multiaddr")
}

/// Ed25519 public key utilities
pub mod ed25519 {
    use codec::{Decode, Encode};
    use scale_info::TypeInfo;
    
    /// Ed25519 public key
    #[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
    pub struct Public(pub [u8; 32]);
    
    impl From<[u8; 32]> for Public {
        fn from(bytes: [u8; 32]) -> Self {
            Public(bytes)
        }
    }
    
    impl AsRef<[u8]> for Public {
        fn as_ref(&self) -> &[u8] {
            &self.0
        }
    }
}

/// DHT record
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DhtRecord {
    /// Key of the record.
    pub key: Vec<u8>,
    /// Value of the record.
    pub value: Vec<u8>,
    /// Publisher of the record.
    pub publisher: Option<Vec<u8>>,
    /// Time-to-live of the record.
    pub expires: Option<std::time::Instant>,
}

/// The parameters to pass to `Discovery::new`.
#[derive(Clone, Debug)]
pub struct DiscoveryConfig {
    /// Identity of the local node.
    pub local_peer_id: Vec<u8>,
    /// TTL of the records put in the DHT.
    pub ttl: std::time::Duration,
    /// Interval to publish our identification.
    pub publication_interval: std::time::Duration,
} 
