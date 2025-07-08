use sc_network::{NetworkService, PeerId, NetworkStatus};
use sp_runtime::traits::Block as BlockT;
use std::{
    sync::Arc,
    time::{Duration, Instant},
    collections::{HashMap, HashSet},
};
use tokio::time::{interval, sleep};
use log::{info, warn, error, debug};

/// Network health status information
#[derive(Debug, Clone)]
pub struct NetworkHealthStatus {
    pub peer_count: u32,
    pub block_rate: u32,
    pub consensus_rate: u32,
    pub security_score: u32,
    pub last_updated: u64,
}

impl Default for NetworkHealthStatus {
    fn default() -> Self {
        Self {
            peer_count: 0,
            block_rate: 0,
            consensus_rate: 95,
            security_score: 100,
            last_updated: 0,
        }
    }
}

/// Security threat levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreatLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// Types of network attacks that can be detected
#[derive(Debug, Clone)]
pub enum AttackType {
    DDoS {
        peer_count: u32,
        connection_rate: f64,
    },
    Eclipse {
        peer_count: u32,
        isolation_duration: Duration,
    },
    Sybil {
        suspicious_peers: Vec<PeerId>,
        pattern: String,
    },
    LongRange {
        peer_id: PeerId,
        block_range: (u32, u32),
    },
    Spam {
        peer_id: PeerId,
        message_count: u32,
        time_window: Duration,
    },
}

/// Peer security information
#[derive(Debug, Clone)]
pub struct PeerSecurityInfo {
    pub peer_id: PeerId,
    pub reputation: f64,
    pub connection_time: Instant,
    pub message_count: u32,
    pub last_message: Instant,
    pub suspicious_activities: Vec<String>,
    pub threat_level: ThreatLevel,
}

impl PeerSecurityInfo {
    pub fn new(peer_id: PeerId) -> Self {
        Self {
            peer_id,
            reputation: 50.0, // Start with neutral reputation
            connection_time: Instant::now(),
            message_count: 0,
            last_message: Instant::now(),
            suspicious_activities: Vec::new(),
            threat_level: ThreatLevel::None,
        }
    }
    
    pub fn update_reputation(&mut self, delta: f64) {
        self.reputation = (self.reputation + delta).clamp(0.0, 100.0);
        
        // Update threat level based on reputation
        self.threat_level = match self.reputation {
            r if r < 20.0 => ThreatLevel::Critical,
            r if r < 40.0 => ThreatLevel::High,
            r if r < 60.0 => ThreatLevel::Medium,
            r if r < 80.0 => ThreatLevel::Low,
            _ => ThreatLevel::None,
        };
    }
    
    pub fn add_suspicious_activity(&mut self, activity: String) {
        self.suspicious_activities.push(activity);
        
        // Decrease reputation for suspicious activity
        self.update_reputation(-5.0);
        
        // Limit the number of stored activities
        if self.suspicious_activities.len() > 10 {
            self.suspicious_activities.remove(0);
        }
    }
}

/// Network security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum number of peers before DDoS alert
    pub max_peer_count: u32,
    /// Minimum number of peers before eclipse alert
    pub min_peer_count: u32,
    /// Maximum messages per peer per minute
    pub max_messages_per_minute: u32,
    /// Time window for monitoring (seconds)
    pub monitoring_window: u64,
    /// Reputation threshold for peer blocking
    pub reputation_threshold: f64,
    /// Enable automatic peer blocking
    pub auto_block_enabled: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_peer_count: 1000,
            min_peer_count: 3,
            max_messages_per_minute: 100,
            monitoring_window: 300, // 5 minutes
            reputation_threshold: 20.0,
            auto_block_enabled: true,
        }
    }
}

/// Network security service for threat detection and monitoring
pub struct NetworkSecurityService<Block: BlockT> {
    network: Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
    health_status: NetworkHealthStatus,
    peer_security: HashMap<PeerId, PeerSecurityInfo>,
    blocked_peers: HashSet<PeerId>,
    security_config: SecurityConfig,
    attack_detection_enabled: bool,
    monitoring_start_time: Instant,
    block_production_times: Vec<(u64, Instant)>, // (block_number, timestamp)
}

impl<Block: BlockT> NetworkSecurityService<Block> {
    /// Create a new network security service
    pub fn new(
        network: Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
        config: SecurityConfig,
    ) -> Self {
        Self {
            network,
            health_status: NetworkHealthStatus::default(),
            peer_security: HashMap::new(),
            blocked_peers: HashSet::new(),
            security_config: config,
            attack_detection_enabled: true,
            monitoring_start_time: Instant::now(),
            block_production_times: Vec::new(),
        }
    }
    
    /// Start the security monitoring service
    pub async fn start_monitoring(&mut self) {
        info!("Starting network security monitoring service");
        
        // Monitor network health every 30 seconds
        let mut health_interval = interval(Duration::from_secs(30));
        
        // Check for attacks every 10 seconds
        let mut attack_interval = interval(Duration::from_secs(10));
        
        // Update peer reputations every minute
        let mut reputation_interval = interval(Duration::from_secs(60));
        
        loop {
            tokio::select! {
                _ = health_interval.tick() => {
                    self.update_network_health().await;
                }
                _ = attack_interval.tick() => {
                    if self.attack_detection_enabled {
                        self.detect_network_attacks().await;
                    }
                }
                _ = reputation_interval.tick() => {
                    self.update_peer_reputations().await;
                }
            }
        }
    }
    
    /// Update network health status
    async fn update_network_health(&mut self) {
        let network_status = self.network.status();
        let connected_peers: Vec<PeerId> = self.network.connected_peers().collect();
        
        self.health_status = NetworkHealthStatus {
            peer_count: connected_peers.len() as u32,
            block_rate: self.calculate_block_rate(),
            consensus_rate: self.calculate_consensus_rate(),
            security_score: self.calculate_security_score(),
            last_updated: chrono::Utc::now().timestamp() as u64,
        };
        
        // Log health status
        debug!(
            "Network health: {} peers, block rate: {}/min, consensus: {}%, security: {}",
            self.health_status.peer_count,
            self.health_status.block_rate,
            self.health_status.consensus_rate,
            self.health_status.security_score
        );
        
        // Check if health is degraded
        if self.health_status.security_score < 70 {
            warn!(
                "Network security score low: {} (peers: {}, block rate: {})",
                self.health_status.security_score,
                self.health_status.peer_count,
                self.health_status.block_rate
            );
        }
        
        // Update peer security information
        for peer_id in connected_peers {
            if !self.peer_security.contains_key(&peer_id) {
                self.peer_security.insert(peer_id, PeerSecurityInfo::new(peer_id));
                info!("New peer connected: {:?}", peer_id);
            }
        }
        
        // Remove disconnected peers
        let current_peers: HashSet<PeerId> = self.network.connected_peers().collect();
        self.peer_security.retain(|peer_id, _| current_peers.contains(peer_id));
    }
    
    /// Detect various types of network attacks
    async fn detect_network_attacks(&mut self) {
        let connected_peers: Vec<PeerId> = self.network.connected_peers().collect();
        let peer_count = connected_peers.len() as u32;
        
        // DDoS detection
        if peer_count > self.security_config.max_peer_count {
            let attack = AttackType::DDoS {
                peer_count,
                connection_rate: self.calculate_connection_rate(),
            };
            
            self.handle_attack_detected(attack).await;
        }
        
        // Eclipse attack detection
        if peer_count < self.security_config.min_peer_count && peer_count > 0 {
            let isolation_duration = self.monitoring_start_time.elapsed();
            
            let attack = AttackType::Eclipse {
                peer_count,
                isolation_duration,
            };
            
            self.handle_attack_detected(attack).await;
        }
        
        // Sybil attack detection
        self.detect_sybil_attacks(&connected_peers).await;
        
        // Spam detection
        self.detect_spam_attacks().await;
    }
    
    /// Detect Sybil attacks by analyzing peer behavior patterns
    async fn detect_sybil_attacks(&mut self, peers: &[PeerId]) {
        let mut suspicious_groups: HashMap<String, Vec<PeerId>> = HashMap::new();
        
        for peer_id in peers {
            if let Some(peer_info) = self.peer_security.get(peer_id) {
                // Group peers by connection time patterns
                let connection_time_bucket = peer_info.connection_time.elapsed().as_secs() / 60; // 1-minute buckets
                let pattern = format!("time_bucket_{}", connection_time_bucket);
                
                suspicious_groups.entry(pattern).or_default().push(*peer_id);
            }
        }
        
        // Check for suspicious groups
        for (pattern, group_peers) in suspicious_groups {
            if group_peers.len() > 5 {
                let attack = AttackType::Sybil {
                    suspicious_peers: group_peers.clone(),
                    pattern: pattern.clone(),
                };
                
                warn!("Potential Sybil attack detected: {} peers with pattern {}", group_peers.len(), pattern);
                self.handle_attack_detected(attack).await;
            }
        }
    }
    
    /// Detect spam attacks by monitoring message rates
    async fn detect_spam_attacks(&mut self) {
        let threshold = self.security_config.max_messages_per_minute;
        let window = Duration::from_secs(60);
        
        for (peer_id, peer_info) in &mut self.peer_security {
            let messages_per_minute = if peer_info.last_message.elapsed() < window {
                peer_info.message_count
            } else {
                0
            };
            
            if messages_per_minute > threshold {
                let attack = AttackType::Spam {
                    peer_id: *peer_id,
                    message_count: messages_per_minute,
                    time_window: window,
                };
                
                warn!("Spam attack detected from peer {:?}: {} messages/min", peer_id, messages_per_minute);
                self.handle_attack_detected(attack).await;
                
                // Add to suspicious activities
                peer_info.add_suspicious_activity(format!("Spam: {} msg/min", messages_per_minute));
            }
        }
    }
    
    /// Handle detected attacks
    async fn handle_attack_detected(&mut self, attack: AttackType) {
        match &attack {
            AttackType::DDoS { peer_count, connection_rate } => {
                error!("DDoS attack detected: {} peers, {:.2} connections/sec", peer_count, connection_rate);
                
                // Block recent connections if auto-blocking is enabled
                if self.security_config.auto_block_enabled {
                    self.block_recent_connections().await;
                }
            },
            AttackType::Eclipse { peer_count, isolation_duration } => {
                error!("Eclipse attack detected: only {} peers, isolated for {:?}", peer_count, isolation_duration);
                // In a real implementation, try to connect to more peers
            },
            AttackType::Sybil { suspicious_peers, pattern } => {
                error!("Sybil attack detected: {} suspicious peers with pattern {}", suspicious_peers.len(), pattern);
                
                // Block suspicious peers
                if self.security_config.auto_block_enabled {
                    for peer_id in suspicious_peers {
                        self.block_peer(*peer_id, "Sybil attack participant".to_string()).await;
                    }
                }
            },
            AttackType::Spam { peer_id, message_count, .. } => {
                error!("Spam attack from {:?}: {} messages", peer_id, message_count);
                
                if self.security_config.auto_block_enabled {
                    self.block_peer(*peer_id, "Spam attack".to_string()).await;
                }
            },
            AttackType::LongRange { peer_id, block_range } => {
                error!("Long range attack from {:?}: blocks {:?}", peer_id, block_range);
                
                if self.security_config.auto_block_enabled {
                    self.block_peer(*peer_id, "Long range attack".to_string()).await;
                }
            },
        }
    }
    
    /// Block a peer
    async fn block_peer(&mut self, peer_id: PeerId, reason: String) {
        warn!("Blocking peer {:?}: {}", peer_id, reason);
        
        self.blocked_peers.insert(peer_id);
        
        // Disconnect from the peer
        self.network.disconnect_peer(peer_id, sc_network::DisconnectionReason::BadProtocol);
        
        // Update peer reputation
        if let Some(peer_info) = self.peer_security.get_mut(&peer_id) {
            peer_info.update_reputation(-50.0);
            peer_info.add_suspicious_activity(format!("Blocked: {}", reason));
        }
    }
    
    /// Block recent connections (for DDoS mitigation)
    async fn block_recent_connections(&mut self) {
        let recent_threshold = Duration::from_secs(60); // Block peers connected in last minute
        let now = Instant::now();
        
        let recent_peers: Vec<PeerId> = self.peer_security
            .iter()
            .filter(|(_, info)| now.duration_since(info.connection_time) < recent_threshold)
            .map(|(peer_id, _)| *peer_id)
            .collect();
        
        for peer_id in recent_peers {
            self.block_peer(peer_id, "DDoS mitigation".to_string()).await;
        }
    }
    
    /// Update peer reputations based on behavior
    async fn update_peer_reputations(&mut self) {
        for (peer_id, peer_info) in &mut self.peer_security {
            // Slowly increase reputation for good behavior
            let time_connected = peer_info.connection_time.elapsed().as_secs();
            if time_connected > 300 && peer_info.suspicious_activities.is_empty() {
                peer_info.update_reputation(1.0); // Small increase for good behavior
            }
            
            // Decrease reputation for inactive peers
            if peer_info.last_message.elapsed() > Duration::from_secs(300) {
                peer_info.update_reputation(-0.5);
            }
            
            // Block peers with very low reputation
            if self.security_config.auto_block_enabled 
               && peer_info.reputation < self.security_config.reputation_threshold 
               && !self.blocked_peers.contains(peer_id) {
                self.block_peer(*peer_id, "Low reputation".to_string()).await;
            }
        }
    }
    
    /// Calculate block production rate
    fn calculate_block_rate(&self) -> u32 {
        if self.block_production_times.len() < 2 {
            return 60; // Default: 1 block per second = 60 per minute
        }
        
        let recent_blocks: Vec<_> = self.block_production_times
            .iter()
            .filter(|(_, timestamp)| timestamp.elapsed() < Duration::from_secs(60))
            .collect();
        
        recent_blocks.len() as u32
    }
    
    /// Calculate consensus participation rate
    fn calculate_consensus_rate(&self) -> u32 {
        // In a real implementation, this would check actual consensus participation
        // For now, return based on peer count and security
        let peer_count = self.health_status.peer_count;
        
        if peer_count < self.security_config.min_peer_count {
            50 // Low participation due to few peers
        } else if peer_count > self.security_config.max_peer_count {
            75 // Reduced due to potential DDoS
        } else {
            95 // Normal participation
        }
    }
    
    /// Calculate overall security score
    fn calculate_security_score(&self) -> u32 {
        let mut score = 100u32;
        let peer_count = self.health_status.peer_count;
        
        // Penalize for too few peers (eclipse risk)
        if peer_count < self.security_config.min_peer_count {
            score = score.saturating_sub(30);
        }
        
        // Penalize for too many peers (DDoS risk)
        if peer_count > self.security_config.max_peer_count {
            score = score.saturating_sub(20);
        }
        
        // Factor in peer reputations
        if !self.peer_security.is_empty() {
            let avg_reputation: f64 = self.peer_security
                .values()
                .map(|info| info.reputation)
                .sum::<f64>() / self.peer_security.len() as f64;
            
            if avg_reputation < 50.0 {
                score = score.saturating_sub(((50.0 - avg_reputation) as u32).min(30));
            }
        }
        
        // Penalize for blocked peers
        if !self.blocked_peers.is_empty() {
            score = score.saturating_sub((self.blocked_peers.len() as u32).min(15));
        }
        
        score
    }
    
    /// Calculate connection rate for DDoS detection
    fn calculate_connection_rate(&self) -> f64 {
        let window = Duration::from_secs(60);
        let recent_connections = self.peer_security
            .values()
            .filter(|info| info.connection_time.elapsed() < window)
            .count();
        
        recent_connections as f64 / 60.0 // connections per second
    }
    
    /// Record block production for rate calculation
    pub fn record_block_production(&mut self, block_number: u64) {
        self.block_production_times.push((block_number, Instant::now()));
        
        // Keep only recent blocks
        let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes
        self.block_production_times.retain(|(_, timestamp)| *timestamp > cutoff);
    }
    
    /// Get current network health status
    pub fn get_health_status(&self) -> NetworkHealthStatus {
        self.health_status.clone()
    }
    
    /// Get peer security information
    pub fn get_peer_security_info(&self, peer_id: &PeerId) -> Option<&PeerSecurityInfo> {
        self.peer_security.get(peer_id)
    }
    
    /// Check if a peer is blocked
    pub fn is_peer_blocked(&self, peer_id: &PeerId) -> bool {
        self.blocked_peers.contains(peer_id)
    }
    
    /// Enable or disable attack detection
    pub fn set_attack_detection(&mut self, enabled: bool) {
        self.attack_detection_enabled = enabled;
        info!("Attack detection {}", if enabled { "enabled" } else { "disabled" });
    }
    
    /// Update security configuration
    pub fn update_config(&mut self, config: SecurityConfig) {
        self.security_config = config;
        info!("Security configuration updated");
    }
} 
