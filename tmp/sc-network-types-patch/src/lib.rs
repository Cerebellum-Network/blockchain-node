// Re-export essential types from dependencies
pub use libp2p_identity::PeerId;
pub use multiaddr::Multiaddr;

// Kad module with required types for sc-network compatibility
pub mod kad {
    use super::*;
    
    /// Kademlia key type - simplified implementation
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Key(pub Vec<u8>);
    
    impl Key {
        pub fn new(data: Vec<u8>) -> Self {
            Self(data)
        }
        
        pub fn to_vec(&self) -> Vec<u8> {
            self.0.clone()
        }
        
        pub fn as_bytes(&self) -> &[u8] {
            &self.0
        }
    }
    
    impl From<Vec<u8>> for Key {
        fn from(data: Vec<u8>) -> Self {
            Self(data)
        }
    }
    
    impl From<Key> for Vec<u8> {
        fn from(key: Key) -> Self {
            key.0
        }
    }
    
    /// Kademlia record type - simplified implementation
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Record {
        pub key: Key,
        pub value: Vec<u8>,
        pub publisher: Option<Vec<u8>>, // Use Vec<u8> instead of PeerId to avoid encode/decode issues
    }
    
    impl Record {
        pub fn new(key: Key, value: Vec<u8>) -> Self {
            Self {
                key,
                value,
                publisher: None,
            }
        }
    }
    
    /// Peer record type - simplified implementation
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PeerRecord {
        pub peer_id: Vec<u8>, // Use Vec<u8> instead of PeerId to avoid encode/decode issues
        pub addresses: Vec<Vec<u8>>, // Use Vec<Vec<u8>> instead of Vec<Multiaddr> to avoid encode/decode issues
    }
    
    impl PeerRecord {
        pub fn new(peer_id: Vec<u8>, addresses: Vec<Vec<u8>>) -> Self {
            Self {
                peer_id,
                addresses,
            }
        }
    }
} 
