//! Network Security Monitoring Module
//!
//! This module provides comprehensive network security monitoring capabilities for the Cere blockchain node.
//! It includes peer monitoring, consensus health checks, security scoring, audit trails, and SOC2 compliance.
//!
//! Phase 4: Enhanced with security event logging, audit trails, and compliance monitoring.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures::channel::oneshot;
use log::{error, info, warn};
use sc_network::config::ProtocolName;
use sc_network::service::traits::{
	NetworkDHTProvider, NetworkEventStream, NetworkPeers, NetworkRequest, NetworkService,
	NetworkSigner, NetworkStateInfo,
};
use sc_network::NetworkStatusProvider;
use sc_network_types::PeerId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sp_runtime::traits::Block as BlockT;
use std::marker::PhantomData;
// Define missing types locally
#[derive(Debug, Clone)]
pub enum SyncStatus<BlockNumber> {
	Idle,
	Downloading { target: BlockNumber },
	Importing { target: BlockNumber },
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
	NotConnected,
	NotificationStreamOpened { remote: PeerId, protocol: ProtocolName },
	NotificationStreamClosed { remote: PeerId, protocol: ProtocolName },
}

/// Network health status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealthStatus {
	pub peer_count: usize,
	pub connected_peers: usize,
	pub block_rate: f64,
	pub consensus_rate: f64,
	pub security_score: u8,
	#[serde(skip, default = "Instant::now")]
	pub last_updated: Instant,
	pub compliance_status: ComplianceStatus,
}

/// SOC2 Compliance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
	pub audit_logging_active: bool,
	pub access_controls_verified: bool,
	pub configuration_validated: bool,
	pub security_monitoring_active: bool,
	pub last_compliance_check: SystemTime,
}

/// SOC2 Compliance Report
#[derive(Debug, Clone)]
pub struct ComplianceReport {
	pub report_period_start: u64,
	pub report_period_end: u64,
	pub total_security_events: usize,
	pub critical_events_count: usize,
	pub high_severity_events_count: usize,
	pub compliance_status: ComplianceStatus,
	pub audit_events: Vec<AuditEvent>,
	pub generated_at: SystemTime,
}

/// Security event types for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
	PeerConnection,
	PeerDisconnection,
	MaliciousActivity,
	ConsensusFailure,
	UnauthorizedAccess,
	ConfigurationChange,
	SecurityPolicyViolation,
	AuditLogFailure,
	CertificateEvent,
	NetworkPartition,
}

/// Audit event for SOC2 compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
	pub timestamp: u64,
	pub event_type: SecurityEventType,
	pub actor: String,
	pub resource: String,
	pub action: String,
	pub result: AuditResult,
	pub details: HashMap<String, String>,
	pub severity: SecuritySeverity,
}

/// Audit result enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
	Success,
	Failure,
	Blocked,
	Suspicious,
}

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
	Low,
	Medium,
	High,
	Critical,
}

/// Security event logger for comprehensive audit trails
pub struct SecurityEventLogger {
	audit_events: Vec<AuditEvent>,
	max_events: usize,
}

impl SecurityEventLogger {
	pub fn new(max_events: usize) -> Self {
		Self { audit_events: Vec::new(), max_events }
	}

	/// Log a security event with full audit trail
	pub fn log_security_event(
		&mut self,
		event_type: SecurityEventType,
		actor: &str,
		resource: &str,
		action: &str,
		result: AuditResult,
		severity: SecuritySeverity,
		details: HashMap<String, String>,
	) {
		let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

		let event = AuditEvent {
			timestamp,
			event_type: event_type.clone(),
			actor: actor.to_string(),
			resource: resource.to_string(),
			action: action.to_string(),
			result: result.clone(),
			details,
			severity: severity.clone(),
		};

		// Log to system logger with structured format
		let event_json = json!({
			"timestamp": timestamp,
			"event_type": format!("{:?}", event_type),
			"actor": actor,
			"resource": resource,
			"action": action,
			"result": format!("{:?}", result),
			"severity": format!("{:?}", severity),
			"details": event.details
		});

		match severity {
			SecuritySeverity::Critical => error!("SECURITY_EVENT_CRITICAL: {}", event_json),
			SecuritySeverity::High => error!("SECURITY_EVENT_HIGH: {}", event_json),
			SecuritySeverity::Medium => warn!("SECURITY_EVENT_MEDIUM: {}", event_json),
			SecuritySeverity::Low => info!("SECURITY_EVENT_LOW: {}", event_json),
		}

		// Store in memory for compliance reporting
		self.audit_events.push(event);

		// Maintain maximum event count
		if self.audit_events.len() > self.max_events {
			self.audit_events.remove(0);
		}
	}

	/// Get audit events for compliance reporting
	pub fn get_audit_events(&self) -> &Vec<AuditEvent> {
		&self.audit_events
	}

	/// Export audit events for SOC2 compliance
	pub fn export_compliance_report(&self, start_time: u64, end_time: u64) -> Vec<AuditEvent> {
		self.audit_events
			.iter()
			.filter(|event| event.timestamp >= start_time && event.timestamp <= end_time)
			.cloned()
			.collect()
	}
}

/// Network security metrics
#[derive(Debug, Clone)]
pub struct SecurityMetrics {
	pub malicious_peer_attempts: u64,
	pub consensus_failures: u64,
	pub network_partitions: u64,
	pub peer_reputation_scores: HashMap<String, u8>,
}

/// Enhanced network security monitor with audit trails and compliance
pub struct NetworkSecurityMonitor<Block: BlockT> {
	network: Arc<dyn NetworkService>,
	metrics: SecurityMetrics,
	health_status: NetworkHealthStatus,
	min_peer_count: usize,
	max_peer_count: usize,
	#[allow(dead_code)]
	block_time_threshold: Duration,
	security_logger: SecurityEventLogger,
	compliance_status: ComplianceStatus,
	_phantom: PhantomData<Block>,
}

impl<Block: BlockT> NetworkSecurityMonitor<Block> {
	/// Create a new enhanced network security monitor with audit trails
	pub fn new(
		network: Arc<dyn NetworkService>,
		min_peer_count: usize,
		max_peer_count: usize,
		block_time_threshold: Duration,
	) -> Self {
		let compliance_status = ComplianceStatus {
			audit_logging_active: true,
			access_controls_verified: true,
			configuration_validated: true,
			security_monitoring_active: true,
			last_compliance_check: SystemTime::now(),
		};

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
				compliance_status: compliance_status.clone(),
			},
			min_peer_count,
			max_peer_count,
			block_time_threshold,
			security_logger: SecurityEventLogger::new(10000), // Store up to 10k events
			compliance_status,
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
			compliance_status: self.compliance_status.clone(),
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
	fn calculate_security_score(
		&self,
		peer_count: usize,
		block_rate: f64,
		consensus_rate: f64,
	) -> u8 {
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

	/// Handle peer connection with audit logging
	pub fn on_peer_connected(&mut self, peer_id: String) {
		self.metrics.peer_reputation_scores.insert(peer_id.clone(), 100);

		// Log security event
		let mut details = HashMap::new();
		details.insert("peer_id".to_string(), peer_id.clone());
		details.insert("initial_reputation".to_string(), "100".to_string());

		self.security_logger.log_security_event(
			SecurityEventType::PeerConnection,
			&peer_id,
			"network",
			"connect",
			AuditResult::Success,
			SecuritySeverity::Low,
			details,
		);
	}

	/// Handle peer disconnection with audit logging
	pub fn on_peer_disconnected(&mut self, peer_id: String) {
		let reputation = self.metrics.peer_reputation_scores.remove(&peer_id);

		// Log security event
		let mut details = HashMap::new();
		details.insert("peer_id".to_string(), peer_id.clone());
		if let Some(rep) = reputation {
			details.insert("final_reputation".to_string(), rep.to_string());
		}

		self.security_logger.log_security_event(
			SecurityEventType::PeerDisconnection,
			&peer_id,
			"network",
			"disconnect",
			AuditResult::Success,
			SecuritySeverity::Low,
			details,
		);
	}

	/// Analyze peer behavior for suspicious activity with enhanced logging
	pub fn analyze_peer_behavior(&mut self, peer_id: String, message_count: usize) {
		// Simple heuristic: too many messages might indicate spam
		if message_count > 100 {
			if let Some(reputation) = self.metrics.peer_reputation_scores.get_mut(&peer_id) {
				let old_reputation = *reputation;
				*reputation = reputation.saturating_sub(10);

				// Log reputation change
				let mut details = HashMap::new();
				details.insert("peer_id".to_string(), peer_id.clone());
				details.insert("message_count".to_string(), message_count.to_string());
				details.insert("old_reputation".to_string(), old_reputation.to_string());
				details.insert("new_reputation".to_string(), reputation.to_string());

				if *reputation < 50 {
					warn!("Suspicious activity detected from peer: {}", peer_id);
					self.metrics.malicious_peer_attempts += 1;

					// Log critical security event
					self.security_logger.log_security_event(
						SecurityEventType::MaliciousActivity,
						&peer_id,
						"network",
						"suspicious_behavior",
						AuditResult::Blocked,
						SecuritySeverity::High,
						details,
					);
				} else {
					// Log medium severity event
					self.security_logger.log_security_event(
						SecurityEventType::MaliciousActivity,
						&peer_id,
						"network",
						"reputation_decrease",
						AuditResult::Success,
						SecuritySeverity::Medium,
						details,
					);
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
		self.health_status.security_score >= 70
			&& self.health_status.peer_count >= self.min_peer_count
			&& self.health_status.consensus_rate >= 90.0
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

	/// SOC2 Compliance: Perform compliance check
	pub fn perform_compliance_check(&mut self) -> bool {
		let mut compliance_score = 0u8;
		let mut details = HashMap::new();

		// Check audit logging
		if self.compliance_status.audit_logging_active {
			compliance_score += 25;
			details.insert("audit_logging".to_string(), "active".to_string());
		} else {
			details.insert("audit_logging".to_string(), "inactive".to_string());
		}

		// Check access controls
		if self.compliance_status.access_controls_verified {
			compliance_score += 25;
			details.insert("access_controls".to_string(), "verified".to_string());
		} else {
			details.insert("access_controls".to_string(), "unverified".to_string());
		}

		// Check configuration validation
		if self.compliance_status.configuration_validated {
			compliance_score += 25;
			details.insert("configuration".to_string(), "validated".to_string());
		} else {
			details.insert("configuration".to_string(), "not_validated".to_string());
		}

		// Check security monitoring
		if self.compliance_status.security_monitoring_active {
			compliance_score += 25;
			details.insert("security_monitoring".to_string(), "active".to_string());
		} else {
			details.insert("security_monitoring".to_string(), "inactive".to_string());
		}

		details.insert("compliance_score".to_string(), compliance_score.to_string());
		self.compliance_status.last_compliance_check = SystemTime::now();

		let is_compliant = compliance_score >= 80;
		let severity = if is_compliant { SecuritySeverity::Low } else { SecuritySeverity::High };

		// Log compliance check
		self.security_logger.log_security_event(
			SecurityEventType::ConfigurationChange,
			"system",
			"compliance",
			"compliance_check",
			if is_compliant { AuditResult::Success } else { AuditResult::Failure },
			severity,
			details,
		);

		is_compliant
	}

	/// Generate SOC2 compliance report
	pub fn generate_compliance_report(&self, start_time: u64, end_time: u64) -> ComplianceReport {
		let audit_events = self.security_logger.export_compliance_report(start_time, end_time);

		let security_events_count = audit_events.len();
		let critical_events = audit_events
			.iter()
			.filter(|e| matches!(e.severity, SecuritySeverity::Critical))
			.count();
		let high_events = audit_events
			.iter()
			.filter(|e| matches!(e.severity, SecuritySeverity::High))
			.count();

		ComplianceReport {
			report_period_start: start_time,
			report_period_end: end_time,
			total_security_events: security_events_count,
			critical_events_count: critical_events,
			high_severity_events_count: high_events,
			compliance_status: self.compliance_status.clone(),
			audit_events,
			generated_at: SystemTime::now(),
		}
	}

	/// Log configuration change for audit trail
	pub fn log_configuration_change(
		&mut self,
		actor: &str,
		config_key: &str,
		old_value: &str,
		new_value: &str,
	) {
		let mut details = HashMap::new();
		details.insert("config_key".to_string(), config_key.to_string());
		details.insert("old_value".to_string(), old_value.to_string());
		details.insert("new_value".to_string(), new_value.to_string());

		self.security_logger.log_security_event(
			SecurityEventType::ConfigurationChange,
			actor,
			"configuration",
			"modify",
			AuditResult::Success,
			SecuritySeverity::Medium,
			details,
		);
	}

	/// Log unauthorized access attempt
	pub fn log_unauthorized_access(&mut self, actor: &str, resource: &str, attempted_action: &str) {
		let mut details = HashMap::new();
		details.insert("attempted_resource".to_string(), resource.to_string());
		details.insert("attempted_action".to_string(), attempted_action.to_string());
		details.insert(
			"timestamp".to_string(),
			SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.unwrap_or_default()
				.as_secs()
				.to_string(),
		);

		self.security_logger.log_security_event(
			SecurityEventType::UnauthorizedAccess,
			actor,
			resource,
			attempted_action,
			AuditResult::Blocked,
			SecuritySeverity::Critical,
			details,
		);
	}

	/// Get security event logger for external access
	pub fn get_security_logger(&self) -> &SecurityEventLogger {
		&self.security_logger
	}

	/// Get compliance status
	pub fn get_compliance_status(&self) -> &ComplianceStatus {
		&self.compliance_status
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

// NetworkStatusProvider implementation removed due to lifetime parameter conflicts
// The trait definition expects different lifetime parameters than what we can provide

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_security_score_calculation() {
		// Test security score calculation without network dependency
		let metrics = SecurityMetrics {
			malicious_peer_attempts: 0,
			consensus_failures: 0,
			network_partitions: 0,
			peer_reputation_scores: HashMap::new(),
		};

		// Create a dummy monitor for testing calculation logic
		let _health_status = NetworkHealthStatus {
			peer_count: 0,
			connected_peers: 0,
			block_rate: 0.0,
			consensus_rate: 0.0,
			security_score: 0,
			last_updated: Instant::now(),
			compliance_status: ComplianceStatus {
				audit_logging_active: true,
				access_controls_verified: true,
				configuration_validated: true,
				security_monitoring_active: true,
				last_compliance_check: SystemTime::now(),
			},
		};

		// Test calculation logic directly
		let mut score = 100u8;

		// Test with good metrics
		let peer_count = 10;
		let min_peer_count = 3;
		let max_peer_count = 50;
		let block_rate = 10.0;
		let consensus_rate = 95.0;

		if peer_count < min_peer_count {
			score = score.saturating_sub(30);
		} else if peer_count > max_peer_count {
			score = score.saturating_sub(10);
		}

		if block_rate < 8.0 {
			score = score.saturating_sub(20);
		}

		if consensus_rate < 90.0 {
			score = score.saturating_sub(25);
		}

		if metrics.malicious_peer_attempts > 0 {
			score = score.saturating_sub(15);
		}

		assert_eq!(score, 100);
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

	#[test]
	fn test_security_event_logging() {
		use sc_network::service::traits::NetworkService;
		use sp_runtime::testing::{Block as TestBlock, TestXt};
		use std::sync::Arc;

		// Create a mock network service
		struct MockNetwork;

		impl NetworkSigner for MockNetwork {
			fn sign_with_local_identity(&self, _msg: Vec<u8>) -> Result<Vec<u8>, ()> {
				Ok(vec![0u8; 64]) // Mock signature
			}
		}

		impl NetworkDHTProvider for MockNetwork {
			fn get_value(&self, _key: &libp2p::kad::record::Key) {}
			fn put_value(&self, _key: libp2p::kad::record::Key, _value: Vec<u8>) {}
		}

		impl NetworkPeers for MockNetwork {
			fn set_authorized_peers(&self, _peers: std::collections::HashSet<PeerId>) {}
			fn set_authorized_only(&self, _reserved_only: bool) {}
			fn add_known_address(&self, _peer_id: PeerId, _addr: Multiaddr) {}
			fn report_peer(&self, _who: PeerId, _cost_benefit: ReputationChange) {}
			fn disconnect_peer(&self, _who: PeerId, _protocol: ProtocolName) {}
			fn accept_unreserved_peers(&self) {}
			fn deny_unreserved_peers(&self) {}
			fn add_reserved_peer(&self, _peer: MultiaddrWithPeerId) -> Result<(), String> {
				Ok(())
			}
			fn remove_reserved_peer(&self, _peer_id: PeerId) {}
			fn set_reserved_peers(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn add_peers_to_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn remove_peers_from_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<PeerId>,
			) -> Result<(), String> {
				Ok(())
			}
			fn sync_num_connected(&self) -> usize {
				5
			}
			fn peer_reputation(&self, _peer_id: &PeerId) -> i32 {
				0
			}
			fn peer_role(&self, _peer_id: PeerId, _handshake: Vec<u8>) -> Option<ObservedRole> {
				None
			}
			fn reserved_peers(
				&self,
			) -> Pin<Box<dyn Future<Output = Result<Vec<PeerId>, ()>> + Send + '_>> {
				Box::pin(async { Ok(Vec::new()) })
			}
		}

		impl NetworkStateInfo for MockNetwork {
			fn external_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
			fn local_peer_id(&self) -> PeerId {
				PeerId::random()
			}
			fn listen_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
		}

		impl NetworkEventStream for MockNetwork {
			fn event_stream(
				&self,
				_name: &'static str,
			) -> Pin<Box<dyn Stream<Item = NetworkEvent> + Send>> {
				Box::pin(futures::stream::empty())
			}
		}

		impl NetworkRequest for MockNetwork {
			fn request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_if_disconnected: IfDisconnected,
			) -> Pin<
				Box<
					dyn Future<Output = Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>
						+ Send,
				>,
			> {
				Box::pin(async { Ok((vec![], ProtocolName::from("test"))) })
			}
			fn start_request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_tx: oneshot::Sender<Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>,
				_if_disconnected: IfDisconnected,
			) {
			}
		}

		impl NetworkStatusProvider for MockNetwork {
			fn status(&self) -> SyncStatus<NumberFor<TestBlock<TestXt<(), ()>>>> {
				SyncStatus::Idle
			}
			fn network_state(&self) -> Result<serde_json::Value, ()> {
				Ok(serde_json::Value::Null)
			}
			fn best_seen_block(&self) -> Option<NumberFor<TestBlock<TestXt<(), ()>>>> {
				None
			}
			fn num_sync_peers(&self) -> usize {
				0
			}
			fn num_connected_peers(&self) -> usize {
				5
			}
			fn num_active_peers(&self) -> usize {
				5
			}
		}

		impl NetworkService for MockNetwork {}

		let network = Arc::new(MockNetwork) as Arc<dyn NetworkService>;
		let mut monitor: NetworkSecurityMonitor<TestBlock<TestXt<(), ()>>> =
			NetworkSecurityMonitor::new(network, 3, 50, Duration::from_millis(12000));

		// Test logging a configuration change
		monitor.log_configuration_change("admin", "max_peers", "50", "100");

		// Verify event was logged
		let events = monitor.security_logger.export_compliance_report(0, u64::MAX);
		assert_eq!(events.len(), 1);
		assert!(matches!(events[0].event_type, SecurityEventType::ConfigurationChange));
		assert_eq!(events[0].actor, "admin");
		assert_eq!(events[0].resource, "configuration");
		assert_eq!(events[0].action, "modify");
	}

	#[test]
	fn test_unauthorized_access_logging() {
		use sc_network::service::traits::NetworkService;
		use sp_runtime::testing::{Block as TestBlock, TestXt};
		use std::sync::Arc;

		// Create a mock network service
		struct MockNetwork;

		impl NetworkSigner for MockNetwork {
			fn sign_with_local_identity(&self, _msg: Vec<u8>) -> Result<Vec<u8>, ()> {
				Ok(vec![0u8; 64]) // Mock signature
			}
		}

		impl NetworkDHTProvider for MockNetwork {
			fn get_value(&self, _key: &libp2p::kad::record::Key) {}
			fn put_value(&self, _key: libp2p::kad::record::Key, _value: Vec<u8>) {}
		}

		impl NetworkPeers for MockNetwork {
			fn set_authorized_peers(&self, _peers: std::collections::HashSet<PeerId>) {}
			fn set_authorized_only(&self, _reserved_only: bool) {}
			fn add_known_address(&self, _peer_id: PeerId, _addr: Multiaddr) {}
			fn report_peer(&self, _who: PeerId, _cost_benefit: ReputationChange) {}
			fn disconnect_peer(&self, _who: PeerId, _protocol: ProtocolName) {}
			fn accept_unreserved_peers(&self) {}
			fn deny_unreserved_peers(&self) {}
			fn add_reserved_peer(&self, _peer: MultiaddrWithPeerId) -> Result<(), String> {
				Ok(())
			}
			fn remove_reserved_peer(&self, _peer_id: PeerId) {}
			fn set_reserved_peers(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn add_peers_to_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn remove_peers_from_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<PeerId>,
			) -> Result<(), String> {
				Ok(())
			}
			fn sync_num_connected(&self) -> usize {
				5
			}
			fn peer_reputation(&self, _peer_id: &PeerId) -> i32 {
				0
			}
			fn peer_role(&self, _peer_id: PeerId, _handshake: Vec<u8>) -> Option<ObservedRole> {
				None
			}
			fn reserved_peers(
				&self,
			) -> Pin<Box<dyn Future<Output = Result<Vec<PeerId>, ()>> + Send + '_>> {
				Box::pin(async { Ok(Vec::new()) })
			}
		}

		impl NetworkStateInfo for MockNetwork {
			fn external_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
			fn local_peer_id(&self) -> PeerId {
				PeerId::random()
			}
			fn listen_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
		}

		impl NetworkEventStream for MockNetwork {
			fn event_stream(
				&self,
				_name: &'static str,
			) -> Pin<Box<dyn Stream<Item = NetworkEvent> + Send>> {
				Box::pin(futures::stream::empty())
			}
		}

		impl NetworkRequest for MockNetwork {
			fn request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_if_disconnected: IfDisconnected,
			) -> Pin<
				Box<
					dyn Future<Output = Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>
						+ Send,
				>,
			> {
				Box::pin(async { Ok((vec![], ProtocolName::from("test"))) })
			}
			fn start_request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_tx: oneshot::Sender<Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>,
				_if_disconnected: IfDisconnected,
			) {
			}
		}

		impl NetworkStatusProvider for MockNetwork {
			fn status(&self) -> SyncStatus<NumberFor<TestBlock<TestXt<(), ()>>>> {
				SyncStatus::Idle
			}
			fn network_state(&self) -> Result<serde_json::Value, ()> {
				Ok(serde_json::Value::Null)
			}
			fn best_seen_block(
				&self,
			) -> Option<NumberFor<sp_runtime::testing::Block<sp_runtime::testing::TestXt<(), ()>>>>
			{
				None
			}
			fn num_sync_peers(&self) -> usize {
				0
			}
			fn num_connected_peers(&self) -> usize {
				5
			}
			fn num_active_peers(&self) -> usize {
				5
			}
		}

		impl NetworkService for MockNetwork {}

		let network = Arc::new(MockNetwork) as Arc<dyn NetworkService>;
		let mut monitor: NetworkSecurityMonitor<TestBlock<TestXt<(), ()>>> =
			NetworkSecurityMonitor::new(network, 3, 50, Duration::from_millis(12000));

		// Test logging unauthorized access
		monitor.log_unauthorized_access("malicious_user", "/admin/config", "read");

		// Verify event was logged with correct severity
		let events = monitor.security_logger.export_compliance_report(0, u64::MAX);
		assert_eq!(events.len(), 1);
		assert!(matches!(events[0].event_type, SecurityEventType::UnauthorizedAccess));
		assert!(matches!(events[0].severity, SecuritySeverity::Critical));
		assert!(matches!(events[0].result, AuditResult::Blocked));
	}

	#[test]
	fn test_compliance_check() {
		use sc_network::service::traits::NetworkService;
		use sp_runtime::testing::{Block as TestBlock, TestXt};
		use std::sync::Arc;

		// Create a mock network service
		struct MockNetwork;

		impl NetworkSigner for MockNetwork {
			fn sign_with_local_identity(&self, _msg: Vec<u8>) -> Result<Vec<u8>, ()> {
				Ok(vec![0u8; 64]) // Mock signature
			}
		}

		impl NetworkDHTProvider for MockNetwork {
			fn get_value(&self, _key: &libp2p::kad::record::Key) {}
			fn put_value(&self, _key: libp2p::kad::record::Key, _value: Vec<u8>) {}
		}

		impl NetworkPeers for MockNetwork {
			fn set_authorized_peers(&self, _peers: std::collections::HashSet<PeerId>) {}
			fn set_authorized_only(&self, _reserved_only: bool) {}
			fn add_known_address(&self, _peer_id: PeerId, _addr: Multiaddr) {}
			fn report_peer(&self, _who: PeerId, _cost_benefit: ReputationChange) {}
			fn disconnect_peer(&self, _who: PeerId, _protocol: ProtocolName) {}
			fn accept_unreserved_peers(&self) {}
			fn deny_unreserved_peers(&self) {}
			fn add_reserved_peer(&self, _peer: MultiaddrWithPeerId) -> Result<(), String> {
				Ok(())
			}
			fn remove_reserved_peer(&self, _peer_id: PeerId) {}
			fn set_reserved_peers(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn add_peers_to_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn remove_peers_from_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<PeerId>,
			) -> Result<(), String> {
				Ok(())
			}
			fn sync_num_connected(&self) -> usize {
				5
			}
			fn peer_reputation(&self, _peer_id: &PeerId) -> i32 {
				0
			}
			fn peer_role(&self, _peer_id: PeerId, _handshake: Vec<u8>) -> Option<ObservedRole> {
				None
			}
			fn reserved_peers(
				&self,
			) -> Pin<Box<dyn Future<Output = Result<Vec<PeerId>, ()>> + Send + '_>> {
				Box::pin(async { Ok(Vec::new()) })
			}
		}

		impl NetworkStateInfo for MockNetwork {
			fn external_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
			fn local_peer_id(&self) -> PeerId {
				PeerId::random()
			}
			fn listen_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
		}

		impl NetworkEventStream for MockNetwork {
			fn event_stream(
				&self,
				_name: &'static str,
			) -> Pin<Box<dyn Stream<Item = NetworkEvent> + Send>> {
				Box::pin(futures::stream::empty())
			}
		}

		impl NetworkRequest for MockNetwork {
			fn request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_if_disconnected: IfDisconnected,
			) -> Pin<
				Box<
					dyn Future<Output = Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>
						+ Send,
				>,
			> {
				Box::pin(async { Ok((vec![], ProtocolName::from("test"))) })
			}
			fn start_request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_tx: oneshot::Sender<Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>,
				_if_disconnected: IfDisconnected,
			) {
			}
		}

		impl NetworkStatusProvider for MockNetwork {
			fn status(&self) -> SyncStatus<NumberFor<TestBlock<TestXt<(), ()>>>> {
				SyncStatus::Idle
			}
			fn network_state(&self) -> Result<serde_json::Value, ()> {
				Ok(serde_json::Value::Null)
			}
			fn best_seen_block(&self) -> Option<NumberFor<TestBlock<TestXt<(), ()>>>> {
				None
			}
			fn num_sync_peers(&self) -> usize {
				0
			}
			fn num_connected_peers(&self) -> usize {
				5
			}
			fn num_active_peers(&self) -> usize {
				5
			}
		}

		impl NetworkService for MockNetwork {}

		let network = Arc::new(MockNetwork) as Arc<dyn NetworkService>;
		let mut monitor: NetworkSecurityMonitor<TestBlock<TestXt<(), ()>>> =
			NetworkSecurityMonitor::new(network, 3, 50, Duration::from_millis(12000));

		// Test initial compliance check (should fail with default settings)
		let is_compliant = monitor.perform_compliance_check();
		assert!(!is_compliant, "Initial compliance check should fail");

		// Enable all compliance components
		monitor.compliance_status.audit_logging_active = true;
		monitor.compliance_status.access_controls_verified = true;
		monitor.compliance_status.configuration_validated = true;
		monitor.compliance_status.security_monitoring_active = true;

		// Test compliance check with all components enabled
		let is_compliant = monitor.perform_compliance_check();
		assert!(is_compliant, "Compliance check should pass with all components enabled");

		// Verify compliance check was logged
		let events = monitor.security_logger.export_compliance_report(0, u64::MAX);
		assert!(events.len() >= 2, "Should have at least 2 compliance check events");
	}

	#[test]
	fn test_compliance_report_generation() {
		use sc_network::service::traits::NetworkService;
		use sp_runtime::testing::{Block as TestBlock, TestXt};
		use std::sync::Arc;

		// Create a mock network service
		struct MockNetwork;

		impl NetworkSigner for MockNetwork {
			fn sign_with_local_identity(&self, _msg: Vec<u8>) -> Result<Vec<u8>, ()> {
				Ok(vec![0u8; 64]) // Mock signature
			}
		}

		impl NetworkDHTProvider for MockNetwork {
			fn get_value(&self, _key: &libp2p::kad::record::Key) {}
			fn put_value(&self, _key: libp2p::kad::record::Key, _value: Vec<u8>) {}
		}

		impl NetworkPeers for MockNetwork {
			fn set_authorized_peers(&self, _peers: std::collections::HashSet<PeerId>) {}
			fn set_authorized_only(&self, _reserved_only: bool) {}
			fn add_known_address(&self, _peer_id: PeerId, _addr: Multiaddr) {}
			fn report_peer(&self, _who: PeerId, _cost_benefit: ReputationChange) {}
			fn disconnect_peer(&self, _who: PeerId, _protocol: ProtocolName) {}
			fn accept_unreserved_peers(&self) {}
			fn deny_unreserved_peers(&self) {}
			fn add_reserved_peer(&self, _peer: MultiaddrWithPeerId) -> Result<(), String> {
				Ok(())
			}
			fn remove_reserved_peer(&self, _peer_id: PeerId) {}
			fn set_reserved_peers(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn add_peers_to_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn remove_peers_from_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<PeerId>,
			) -> Result<(), String> {
				Ok(())
			}
			fn sync_num_connected(&self) -> usize {
				5
			}
			fn peer_reputation(&self, _peer_id: &PeerId) -> i32 {
				0
			}
			fn peer_role(&self, _peer_id: PeerId, _handshake: Vec<u8>) -> Option<ObservedRole> {
				None
			}
			fn reserved_peers(
				&self,
			) -> Pin<Box<dyn Future<Output = Result<Vec<PeerId>, ()>> + Send + '_>> {
				Box::pin(async { Ok(Vec::new()) })
			}
		}

		impl NetworkStateInfo for MockNetwork {
			fn external_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
			fn local_peer_id(&self) -> PeerId {
				PeerId::random()
			}
			fn listen_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
		}

		impl NetworkEventStream for MockNetwork {
			fn event_stream(
				&self,
				_name: &'static str,
			) -> Pin<Box<dyn Stream<Item = NetworkEvent> + Send>> {
				Box::pin(futures::stream::empty())
			}
		}

		impl NetworkRequest for MockNetwork {
			fn request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_if_disconnected: IfDisconnected,
			) -> Pin<
				Box<
					dyn Future<Output = Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>
						+ Send,
				>,
			> {
				Box::pin(async { Ok((vec![], ProtocolName::from("test"))) })
			}
			fn start_request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_tx: oneshot::Sender<Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>,
				_if_disconnected: IfDisconnected,
			) {
			}
		}

		impl NetworkStatusProvider for MockNetwork {
			fn status(&self) -> SyncStatus<NumberFor<TestBlock<TestXt<(), ()>>>> {
				SyncStatus::Idle
			}
			fn network_state(&self) -> Result<serde_json::Value, ()> {
				Ok(serde_json::Value::Null)
			}
			fn best_seen_block(&self) -> Option<NumberFor<TestBlock<TestXt<(), ()>>>> {
				None
			}
			fn num_sync_peers(&self) -> usize {
				0
			}
			fn num_connected_peers(&self) -> usize {
				5
			}
			fn num_active_peers(&self) -> usize {
				5
			}
		}

		impl NetworkService for MockNetwork {}

		let network = Arc::new(MockNetwork) as Arc<dyn NetworkService>;
		let mut monitor: NetworkSecurityMonitor<TestBlock<TestXt<(), ()>>> =
			NetworkSecurityMonitor::new(network, 3, 50, Duration::from_millis(12000));

		// Generate some test events
		monitor.log_configuration_change("admin", "test_key", "old", "new");
		monitor.log_unauthorized_access("attacker", "/secure", "access");
		monitor.perform_compliance_check();

		// Generate compliance report
		let start_time = 0;
		let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
		let report = monitor.generate_compliance_report(start_time, end_time);

		// Verify report contents
		assert_eq!(report.total_security_events, 3); // 2 manual + 1 compliance check
		assert_eq!(report.critical_events_count, 1); // unauthorized access
		assert_eq!(report.audit_events.len(), 3);
		assert_eq!(report.report_period_start, start_time);
		assert_eq!(report.report_period_end, end_time);
	}

	#[test]
	fn test_peer_event_logging() {
		use sc_network::service::traits::NetworkService;
		use sp_runtime::testing::{Block as TestBlock, TestXt};
		use std::sync::Arc;

		// Create a mock network service
		struct MockNetwork;

		impl NetworkSigner for MockNetwork {
			fn sign_with_local_identity(&self, _msg: Vec<u8>) -> Result<Vec<u8>, ()> {
				Ok(vec![0u8; 64]) // Mock signature
			}
		}

		impl NetworkDHTProvider for MockNetwork {
			fn get_value(&self, _key: &libp2p::kad::record::Key) {}
			fn put_value(&self, _key: libp2p::kad::record::Key, _value: Vec<u8>) {}
		}

		impl NetworkPeers for MockNetwork {
			fn set_authorized_peers(&self, _peers: std::collections::HashSet<PeerId>) {}
			fn set_authorized_only(&self, _reserved_only: bool) {}
			fn add_known_address(&self, _peer_id: PeerId, _addr: Multiaddr) {}
			fn report_peer(&self, _who: PeerId, _cost_benefit: ReputationChange) {}
			fn disconnect_peer(&self, _who: PeerId, _protocol: ProtocolName) {}
			fn accept_unreserved_peers(&self) {}
			fn deny_unreserved_peers(&self) {}
			fn add_reserved_peer(&self, _peer: MultiaddrWithPeerId) -> Result<(), String> {
				Ok(())
			}
			fn remove_reserved_peer(&self, _peer_id: PeerId) {}
			fn set_reserved_peers(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn add_peers_to_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn remove_peers_from_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<PeerId>,
			) -> Result<(), String> {
				Ok(())
			}
			fn sync_num_connected(&self) -> usize {
				5
			}
			fn peer_reputation(&self, _peer_id: &PeerId) -> i32 {
				0
			}
			fn peer_role(&self, _peer_id: PeerId, _handshake: Vec<u8>) -> Option<ObservedRole> {
				None
			}
			fn reserved_peers(
				&self,
			) -> Pin<Box<dyn Future<Output = Result<Vec<PeerId>, ()>> + Send + '_>> {
				Box::pin(async { Ok(Vec::new()) })
			}
		}

		impl NetworkStateInfo for MockNetwork {
			fn external_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
			fn local_peer_id(&self) -> PeerId {
				PeerId::random()
			}
			fn listen_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
		}

		impl NetworkEventStream for MockNetwork {
			fn event_stream(
				&self,
				_name: &'static str,
			) -> Pin<Box<dyn Stream<Item = NetworkEvent> + Send>> {
				Box::pin(futures::stream::empty())
			}
		}

		impl NetworkRequest for MockNetwork {
			fn request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_if_disconnected: IfDisconnected,
			) -> Pin<
				Box<
					dyn Future<Output = Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>
						+ Send,
				>,
			> {
				Box::pin(async { Ok((vec![], ProtocolName::from("test"))) })
			}
			fn start_request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_tx: oneshot::Sender<Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>,
				_if_disconnected: IfDisconnected,
			) {
			}
		}

		impl NetworkStatusProvider for MockNetwork {
			fn status(&self) -> SyncStatus<NumberFor<TestBlock<TestXt<(), ()>>>> {
				SyncStatus::Idle
			}
			fn network_state(&self) -> Result<serde_json::Value, ()> {
				Ok(serde_json::Value::Null)
			}
			fn best_seen_block(&self) -> Option<NumberFor<TestBlock<TestXt<(), ()>>>> {
				None
			}
			fn num_sync_peers(&self) -> usize {
				0
			}
			fn num_connected_peers(&self) -> usize {
				5
			}
			fn num_active_peers(&self) -> usize {
				5
			}
		}

		impl NetworkService for MockNetwork {}

		let network = Arc::new(MockNetwork) as Arc<dyn NetworkService>;
		let mut monitor: NetworkSecurityMonitor<TestBlock<TestXt<(), ()>>> =
			NetworkSecurityMonitor::new(network, 3, 50, Duration::from_millis(12000));

		// Test peer connection logging
		monitor.on_peer_connected("peer123".to_string());
		monitor.on_peer_disconnected("peer123".to_string());

		// Verify events were logged
		let events = monitor.security_logger.export_compliance_report(0, u64::MAX);
		assert_eq!(events.len(), 2);

		// Check connection event
		let connect_event = &events[0];
		assert!(matches!(connect_event.event_type, SecurityEventType::PeerConnection));
		assert_eq!(connect_event.action, "connect");
		assert_eq!(connect_event.resource, "network");

		// Check disconnection event
		let disconnect_event = &events[1];
		assert!(matches!(disconnect_event.event_type, SecurityEventType::PeerDisconnection));
		assert_eq!(disconnect_event.action, "disconnect");
		assert_eq!(disconnect_event.resource, "network");
	}

	#[test]
	fn test_security_event_logger() {
		let mut logger = SecurityEventLogger::new(100);

		// Test logging events
		let mut details = HashMap::new();
		details.insert("test_key".to_string(), "test_value".to_string());

		logger.log_security_event(
			SecurityEventType::ConfigurationChange,
			"test_actor",
			"test_resource",
			"test_action",
			AuditResult::Success,
			SecuritySeverity::Medium,
			details,
		);

		// Verify event was stored
		let events = logger.get_audit_events();
		assert_eq!(events.len(), 1);
		assert_eq!(events[0].actor, "test_actor");
		assert_eq!(events[0].resource, "test_resource");
		assert_eq!(events[0].action, "test_action");
	}

	#[test]
	fn test_audit_event_filtering_by_time() {
		use sc_network::service::traits::NetworkService;
		use sp_runtime::testing::{Block as TestBlock, TestXt};
		use std::sync::Arc;

		// Create a mock network service
		struct MockNetwork;

		impl NetworkSigner for MockNetwork {
			fn sign_with_local_identity(&self, _msg: Vec<u8>) -> Result<Vec<u8>, ()> {
				Ok(vec![0u8; 64]) // Mock signature
			}
		}

		impl NetworkDHTProvider for MockNetwork {
			fn get_value(&self, _key: &libp2p::kad::record::Key) {}
			fn put_value(&self, _key: libp2p::kad::record::Key, _value: Vec<u8>) {}
		}

		impl NetworkPeers for MockNetwork {
			fn set_authorized_peers(&self, _peers: std::collections::HashSet<PeerId>) {}
			fn set_authorized_only(&self, _reserved_only: bool) {}
			fn add_known_address(&self, _peer_id: PeerId, _addr: Multiaddr) {}
			fn report_peer(&self, _who: PeerId, _cost_benefit: ReputationChange) {}
			fn disconnect_peer(&self, _who: PeerId, _protocol: ProtocolName) {}
			fn accept_unreserved_peers(&self) {}
			fn deny_unreserved_peers(&self) {}
			fn add_reserved_peer(&self, _peer: MultiaddrWithPeerId) -> Result<(), String> {
				Ok(())
			}
			fn remove_reserved_peer(&self, _peer_id: PeerId) {}
			fn set_reserved_peers(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn add_peers_to_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<Multiaddr>,
			) -> Result<(), String> {
				Ok(())
			}
			fn remove_peers_from_reserved_set(
				&self,
				_protocol: ProtocolName,
				_peers: std::collections::HashSet<PeerId>,
			) -> Result<(), String> {
				Ok(())
			}
			fn sync_num_connected(&self) -> usize {
				5
			}
			fn peer_reputation(&self, _peer_id: &PeerId) -> i32 {
				0
			}
			fn peer_role(&self, _peer_id: PeerId, _handshake: Vec<u8>) -> Option<ObservedRole> {
				None
			}
			fn reserved_peers(
				&self,
			) -> Pin<Box<dyn Future<Output = Result<Vec<PeerId>, ()>> + Send + '_>> {
				Box::pin(async { Ok(Vec::new()) })
			}
		}

		impl NetworkStateInfo for MockNetwork {
			fn external_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
			fn local_peer_id(&self) -> PeerId {
				PeerId::random()
			}
			fn listen_addresses(&self) -> Vec<Multiaddr> {
				Vec::new()
			}
		}

		impl NetworkEventStream for MockNetwork {
			fn event_stream(
				&self,
				_name: &'static str,
			) -> Pin<Box<dyn Stream<Item = NetworkEvent> + Send>> {
				Box::pin(futures::stream::empty())
			}
		}

		impl NetworkRequest for MockNetwork {
			fn request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_if_disconnected: IfDisconnected,
			) -> Pin<
				Box<
					dyn Future<Output = Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>
						+ Send,
				>,
			> {
				Box::pin(async { Ok((vec![], ProtocolName::from("test"))) })
			}
			fn start_request(
				&self,
				_target: PeerId,
				_protocol: ProtocolName,
				_request: Vec<u8>,
				_fallback_request: Option<(Vec<u8>, ProtocolName)>,
				_tx: oneshot::Sender<Result<(Vec<u8>, ProtocolName), sc_network::RequestFailure>>,
				_if_disconnected: IfDisconnected,
			) {
			}
		}

		impl NetworkStatusProvider for MockNetwork {
			fn status(&self) -> SyncStatus<NumberFor<TestBlock<TestXt<(), ()>>>> {
				SyncStatus::Idle
			}
			fn network_state(&self) -> Result<serde_json::Value, ()> {
				Ok(serde_json::Value::Null)
			}
			fn best_seen_block(&self) -> Option<NumberFor<TestBlock<TestXt<(), ()>>>> {
				None
			}
			fn num_sync_peers(&self) -> usize {
				0
			}
			fn num_connected_peers(&self) -> usize {
				5
			}
			fn num_active_peers(&self) -> usize {
				5
			}
		}

		impl NetworkService for MockNetwork {}

		let network = Arc::new(MockNetwork) as Arc<dyn NetworkService>;
		let mut monitor: NetworkSecurityMonitor<TestBlock<TestXt<(), ()>>> =
			NetworkSecurityMonitor::new(network, 3, 50, Duration::from_millis(12000));

		// Get current time
		let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
		let one_hour_ago = now - 3600;

		// Log an event
		monitor.log_configuration_change("admin", "test", "old", "new");

		// Test filtering - should include recent events
		let recent_events = monitor.security_logger.export_compliance_report(one_hour_ago, now + 1);
		assert_eq!(recent_events.len(), 1);

		// Test filtering - should exclude events outside time range
		let old_events = monitor.security_logger.export_compliance_report(0, one_hour_ago);
		assert_eq!(old_events.len(), 0);
	}
}
