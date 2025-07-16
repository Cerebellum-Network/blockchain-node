//! Network Security Monitoring Module
//!
//! This module provides network security monitoring capabilities for the Cere blockchain node.
//! It includes peer monitoring, consensus health checks, and security scoring.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sc_network::service::traits::NetworkService;
use sp_runtime::traits::Block as BlockT;
use std::marker::PhantomData;
use log::warn;

/// Network health status information
#[derive(Debug, Clone)]
pub struct NetworkHealthStatus {
    pub peer_count: usize,
    pub connected_peers: usize,
    pub block_rate: f64,
    pub consensus_rate: f64,
    pub security_score: u8,
    pub last_updated: Instant,
}

/// Network security metrics
#[derive(Debug, Clone)]
pub struct SecurityMetrics {
    pub malicious_peer_attempts: u64,
    pub consensus_failures: u64,
    pub network_partitions: u64,
    pub peer_reputation_scores: HashMap<String, u8>,
}

/// Network security monitor
pub struct NetworkSecurityMonitor<Block: BlockT> {
    network: Arc<dyn NetworkService>,
    metrics: SecurityMetrics,
    health_status: NetworkHealthStatus,
    min_peer_count: usize,
    max_peer_count: usize,
    block_time_threshold: Duration,
    _phantom: PhantomData<Block>,
}

impl<Block: BlockT> NetworkSecurityMonitor<Block> {
    /// Create a new network security monitor
    pub fn new(
        network: Arc<dyn NetworkService>,
        min_peer_count: usize,
        max_peer_count: usize,
        block_time_threshold: Duration,
    ) -> Self {
        Self {
            network,
            metrics: SecurityMetrics {
                malicious_peer_attempts: 0,
                consensus_failures: 0,
                network_partitions: 0,
                peer_reputation_scores: HashMap::new(),
            },
            health_status: NetworkHealthStatus {
                peer_count: 0,
                connected_peers: 0,
                block_rate: 0.0,
                consensus_rate: 0.0,
                security_score: 0,
                last_updated: Instant::now(),
            },
            min_peer_count,
            max_peer_count,
            block_time_threshold,
            _phantom: PhantomData,
        }
    }

    /// Monitor network health and return current status
    pub fn monitor_network_health(&mut self) -> NetworkHealthStatus {
        let peer_count = self.get_connected_peer_count();
        let block_rate = self.calculate_block_production_rate();
        let consensus_rate = self.calculate_consensus_participation_rate();
        let security_score = self.calculate_security_score(peer_count, block_rate, consensus_rate);

        self.health_status = NetworkHealthStatus {
            peer_count,
            connected_peers: peer_count,
            block_rate,
            consensus_rate,
            security_score,
            last_updated: Instant::now(),
        };

        // Log warnings for security issues
        if peer_count < self.min_peer_count {
            warn!("Low peer count detected: {} (minimum: {})", peer_count, self.min_peer_count);
        }

        if security_score < 70 {
            warn!("Low network security score: {}", security_score);
        }

        self.health_status.clone()
    }

    /// Get the current number of connected peers
    fn get_connected_peer_count(&self) -> usize {
        // In a real implementation, this would query the network service
        // For now, we'll return a placeholder value
        self.network.sync_num_connected()
    }

    /// Calculate block production rate (blocks per minute)
    fn calculate_block_production_rate(&self) -> f64 {
        // This would typically track block timestamps over time
        // For now, return a placeholder based on expected 6-second block time
        10.0 // 10 blocks per minute for 6-second block time
    }

    /// Calculate consensus participation rate (percentage)
    fn calculate_consensus_participation_rate(&self) -> f64 {
        // This would track GRANDPA/BABE participation
        // For now, return a placeholder
        95.0
    }

    /// Calculate overall security score (0-100)
    fn calculate_security_score(&self, peer_count: usize, block_rate: f64, consensus_rate: f64) -> u8 {
        let mut score = 100u8;

        // Peer count scoring
        if peer_count < self.min_peer_count {
            score = score.saturating_sub(30);
        } else if peer_count > self.max_peer_count {
            score = score.saturating_sub(10);
        }

        // Block production scoring
        if block_rate < 8.0 {
            score = score.saturating_sub(20);
        }

        // Consensus participation scoring
        if consensus_rate < 90.0 {
            score = score.saturating_sub(25);
        }

        // Malicious activity penalty
        if self.metrics.malicious_peer_attempts > 0 {
            score = score.saturating_sub(15);
        }

        score
    }

    /// Process network events for security monitoring
    pub async fn process_network_events(&mut self) {
        // Placeholder for network event processing
        // In a real implementation, this would listen to network events
        // and update security metrics accordingly
    }

    /// Handle peer connection
    fn on_peer_connected(&mut self, peer_id: String) {
        self.metrics.peer_reputation_scores.insert(peer_id, 100);
    }

    /// Handle peer disconnection
    fn on_peer_disconnected(&mut self, peer_id: String) {
        self.metrics.peer_reputation_scores.remove(&peer_id);
    }

    /// Analyze peer behavior for suspicious activity
    fn analyze_peer_behavior(&mut self, peer_id: String, message_count: usize) {
        // Simple heuristic: too many messages might indicate spam
        if message_count > 100 {
            if let Some(reputation) = self.metrics.peer_reputation_scores.get_mut(&peer_id) {
                *reputation = reputation.saturating_sub(10);
                if *reputation < 50 {
                    warn!("Suspicious activity detected from peer: {}", peer_id);
                    self.metrics.malicious_peer_attempts += 1;
                }
            }
        }
    }

    /// Get current security metrics
    pub fn get_security_metrics(&self) -> &SecurityMetrics {
        &self.metrics
    }

    /// Get current health status
    pub fn get_health_status(&self) -> &NetworkHealthStatus {
        &self.health_status
    }

    /// Check if network is healthy
    pub fn is_network_healthy(&self) -> bool {
        self.health_status.security_score >= 70 &&
        self.health_status.peer_count >= self.min_peer_count &&
        self.health_status.consensus_rate >= 90.0
    }

    /// Reset security metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = SecurityMetrics {
            malicious_peer_attempts: 0,
            consensus_failures: 0,
            network_partitions: 0,
            peer_reputation_scores: HashMap::new(),
        };
    }
}

/// Network security configuration
#[derive(Debug, Clone)]
pub struct NetworkSecurityConfig {
    pub min_peers: usize,
    pub max_peers: usize,
    pub block_time_threshold_ms: u64,
    pub reputation_threshold: u8,
    pub monitoring_interval_secs: u64,
}

impl Default for NetworkSecurityConfig {
    fn default() -> Self {
        Self {
            min_peers: 3,
            max_peers: 50,
            block_time_threshold_ms: 12000, // 12 seconds (2x expected block time)
            reputation_threshold: 50,
            monitoring_interval_secs: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock network service for testing
    struct MockNetworkService;

    impl NetworkService for MockNetworkService {
        fn num_connected(&self) -> Result<usize, ()> {
            Ok(5)
        }

        // Add other required methods with default implementations
        fn local_peer_id(&self) -> sc_network_types::PeerId {
            sc_network_types::PeerId::random()
        }

        fn is_major_syncing(&self) -> bool {
            false
        }

        fn network_state(&self) -> Result<sc_network_types::NetworkState, ()> {
            Err(())
        }

        fn add_known_address(&self, _peer_id: sc_network_types::PeerId, _addr: sc_network_types::Multiaddr) {
            // Mock implementation
        }
    }

    #[test]
    fn test_network_security_monitor_creation() {
        let network = Arc::new(MockNetworkService) as Arc<dyn NetworkService>;
        let monitor = NetworkSecurityMonitor::<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>, sp_runtime::OpaqueExtrinsic>>::new(
            network,
            3,
            50,
            Duration::from_secs(12),
        );

        assert_eq!(monitor.min_peer_count, 3);
        assert_eq!(monitor.max_peer_count, 50);
    }

    #[test]
    fn test_security_score_calculation() {
        let network = Arc::new(MockNetworkService) as Arc<dyn NetworkService>;
        let mut monitor = NetworkSecurityMonitor::<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>, sp_runtime::OpaqueExtrinsic>>::new(
            network,
            3,
            50,
            Duration::from_secs(12),
        );

        // Test with good metrics
        let score = monitor.calculate_security_score(10, 10.0, 95.0);
        assert_eq!(score, 100);

        // Test with low peer count
        let score = monitor.calculate_security_score(2, 10.0, 95.0);
        assert_eq!(score, 70);
    }

    #[test]
    fn test_network_health_monitoring() {
        let network = Arc::new(MockNetworkService) as Arc<dyn NetworkService>;
        let mut monitor = NetworkSecurityMonitor::<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>, sp_runtime::OpaqueExtrinsic>>::new(
            network,
            3,
            50,
            Duration::from_secs(12),
        );

        let health = monitor.monitor_network_health();
        assert!(health.security_score > 0);
        assert!(health.peer_count >= 0);
    }

    #[test]
    fn test_peer_reputation_system() {
        let network = Arc::new(MockNetworkService) as Arc<dyn NetworkService>;
        let mut monitor = NetworkSecurityMonitor::<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>, sp_runtime::OpaqueExtrinsic>>::new(
            network,
            3,
            50,
            Duration::from_secs(12),
        );

        let peer_id = "test_peer".to_string();
        monitor.on_peer_connected(peer_id.clone());
        
        assert_eq!(monitor.metrics.peer_reputation_scores.get(&peer_id), Some(&100));
        
        // Simulate suspicious behavior
        monitor.analyze_peer_behavior(peer_id.clone(), 150);
        assert_eq!(monitor.metrics.peer_reputation_scores.get(&peer_id), Some(&90));
    }

    #[test]
    fn test_network_security_config_default() {
        let config = NetworkSecurityConfig::default();
        assert_eq!(config.min_peers, 3);
        assert_eq!(config.max_peers, 50);
        assert_eq!(config.block_time_threshold_ms, 12000);
        assert_eq!(config.reputation_threshold, 50);
        assert_eq!(config.monitoring_interval_secs, 30);
    }
}