use super::*;
use crate::{mock::*, security_logger};
use frame_support::{assert_noop, assert_ok, traits::Get};
use frame_system::RawOrigin;

#[test]
fn test_record_security_event() {
	new_test_ext().execute_with(|| {
		// Test recording a security event
		assert_ok!(SecurityAudit::record_security_event(
			RuntimeOrigin::signed(1),
			SecurityEventType::SuspiciousActivity,
			SecuritySeverity::High,
			Some(1),
			Some(2),
			b"test_action".to_vec(),
			b"test_details".to_vec(),
			SecurityEventResult::Investigating,
			Some(b"192.168.1.1".to_vec()),
			Some(b"test_user_agent".to_vec()),
			Some(b"session_123".to_vec()),
		));

		// Check that the event was recorded
		assert_eq!(SecurityAudit::security_event_count(), 1);

		// Check that the event exists
		let event = SecurityAudit::security_events(0).unwrap();
		assert_eq!(event.event_type, SecurityEventType::SuspiciousActivity);
		assert_eq!(event.severity, SecuritySeverity::High);
		assert_eq!(event.actor, Some(1));
		assert_eq!(event.target, Some(2));
		assert_eq!(event.action, b"test_action".to_vec());
		assert_eq!(event.details, b"test_details".to_vec());
		assert_eq!(event.result, SecurityEventResult::Investigating);
		assert_eq!(event.source_ip, Some(b"192.168.1.1".to_vec()));
		assert_eq!(event.user_agent, Some(b"test_user_agent".to_vec()));
		assert_eq!(event.session_id, Some(b"session_123".to_vec()));

		// Check that indexes were updated
		assert_eq!(SecurityAudit::get_security_events_by_account(&1), vec![0]);
		assert_eq!(
			SecurityAudit::get_security_events_by_type(&SecurityEventType::SuspiciousActivity),
			vec![0]
		);
		assert_eq!(
			SecurityAudit::get_security_events_by_severity(&SecuritySeverity::High),
			vec![0]
		);
	});
}

#[test]
fn test_record_security_event_with_large_details() {
	new_test_ext().execute_with(|| {
		// Test that large details are rejected
		let large_details = vec![0u8; (MaxEventDetailsSize::get() + 1) as usize];

		assert_noop!(
			SecurityAudit::record_security_event(
				RuntimeOrigin::signed(1),
				SecurityEventType::SuspiciousActivity,
				SecuritySeverity::High,
				Some(1),
				None,
				b"test_action".to_vec(),
				large_details,
				SecurityEventResult::Investigating,
				None,
				None,
				None,
			),
			Error::<Test>::EventDetailsTooLarge
		);
	});
}

#[test]
fn test_record_security_event_with_large_action() {
	new_test_ext().execute_with(|| {
		// Test that large action is rejected
		let large_action = vec![0u8; (MaxActionSize::get() + 1) as usize];

		assert_noop!(
			SecurityAudit::record_security_event(
				RuntimeOrigin::signed(1),
				SecurityEventType::SuspiciousActivity,
				SecuritySeverity::High,
				Some(1),
				None,
				large_action,
				b"test_details".to_vec(),
				SecurityEventResult::Investigating,
				None,
				None,
				None,
			),
			Error::<Test>::ActionTooLarge
		);
	});
}

#[test]
fn test_clear_security_events() {
	new_test_ext().execute_with(|| {
		// Record some security events
		assert_ok!(SecurityAudit::record_security_event(
			RuntimeOrigin::signed(1),
			SecurityEventType::SuspiciousActivity,
			SecuritySeverity::High,
			Some(1),
			None,
			b"test_action".to_vec(),
			b"test_details".to_vec(),
			SecurityEventResult::Investigating,
			None,
			None,
			None,
		));

		assert_ok!(SecurityAudit::record_security_event(
			RuntimeOrigin::signed(1),
			SecurityEventType::UnauthorizedAccess,
			SecuritySeverity::Critical,
			Some(2),
			None,
			b"test_action_2".to_vec(),
			b"test_details_2".to_vec(),
			SecurityEventResult::Blocked,
			None,
			None,
			None,
		));

		assert_eq!(SecurityAudit::security_event_count(), 2);

		// Clear the first event (only root can do this)
		assert_ok!(SecurityAudit::clear_security_events(RuntimeOrigin::root(), vec![0]));

		// Check that the event was cleared
		assert!(SecurityAudit::security_events(0).is_none());
		assert!(SecurityAudit::security_events(1).is_some());
	});
}

#[test]
fn test_clear_security_events_non_root() {
	new_test_ext().execute_with(|| {
		// Record a security event
		assert_ok!(SecurityAudit::record_security_event(
			RuntimeOrigin::signed(1),
			SecurityEventType::SuspiciousActivity,
			SecuritySeverity::High,
			Some(1),
			None,
			b"test_action".to_vec(),
			b"test_details".to_vec(),
			SecurityEventResult::Investigating,
			None,
			None,
			None,
		));

		// Try to clear as non-root (should fail)
		assert_noop!(
			SecurityAudit::clear_security_events(RuntimeOrigin::signed(1), vec![0]),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn test_security_metrics() {
	new_test_ext().execute_with(|| {
		// Initially, metrics should be zero
		let metrics = SecurityAudit::get_security_metrics();
		assert_eq!(metrics.total_events, 0);
		assert_eq!(metrics.critical_events, 0);
		assert_eq!(metrics.high_severity_events, 0);

		// Record a high severity event
		assert_ok!(SecurityAudit::record_security_event(
			RuntimeOrigin::signed(1),
			SecurityEventType::SuspiciousActivity,
			SecuritySeverity::High,
			Some(1),
			None,
			b"test_action".to_vec(),
			b"test_details".to_vec(),
			SecurityEventResult::Investigating,
			None,
			None,
			None,
		));

		// Check metrics
		let metrics = SecurityAudit::get_security_metrics();
		assert_eq!(metrics.total_events, 1);
		assert_eq!(metrics.critical_events, 0);
		assert_eq!(metrics.high_severity_events, 1);

		// Record a critical severity event
		assert_ok!(SecurityAudit::record_security_event(
			RuntimeOrigin::signed(1),
			SecurityEventType::PrivilegeEscalation,
			SecuritySeverity::Critical,
			Some(2),
			None,
			b"test_action_2".to_vec(),
			b"test_details_2".to_vec(),
			SecurityEventResult::Blocked,
			None,
			None,
			None,
		));

		// Check metrics again
		let metrics = SecurityAudit::get_security_metrics();
		assert_eq!(metrics.total_events, 2);
		assert_eq!(metrics.critical_events, 1);
		assert_eq!(metrics.high_severity_events, 1);
	});
}

#[test]
fn test_security_logger_functions() {
	new_test_ext().execute_with(|| {
		// Test suspicious activity logger
		assert_ok!(security_logger::log_suspicious_activity::<Test>(
			Some(1),
			b"suspicious behavior detected"
		));

		// Test unauthorized access logger
		assert_ok!(security_logger::log_unauthorized_access::<Test>(
			Some(1),
			Some(2),
			b"attempted unauthorized access"
		));

		// Test authentication failure logger
		assert_ok!(security_logger::log_authentication_failure::<Test>(
			Some(1),
			b"authentication failed"
		));

		// Test privilege escalation logger
		assert_ok!(security_logger::log_privilege_escalation::<Test>(
			Some(1),
			Some(2),
			b"privilege escalation attempt"
		));

		// Test consensus security event logger
		assert_ok!(security_logger::log_consensus_security_event::<Test>(
			b"consensus anomaly detected",
			SecuritySeverity::High
		));

		// Test RPC security event logger
		assert_ok!(security_logger::log_rpc_security_event::<Test>(
			Some(1),
			b"system_health",
			b"suspicious RPC call pattern"
		));

		// Check that all events were recorded
		assert_eq!(SecurityAudit::security_event_count(), 6);
	});
}

#[test]
fn test_is_security_threat() {
	new_test_ext().execute_with(|| {
		// Test various combinations
		assert!(SecurityAudit::is_security_threat(
			&SecurityEventType::SuspiciousActivity,
			&SecuritySeverity::Medium
		));

		assert!(SecurityAudit::is_security_threat(
			&SecurityEventType::UnauthorizedAccess,
			&SecuritySeverity::Low
		));

		assert!(SecurityAudit::is_security_threat(
			&SecurityEventType::NetworkSecurityEvent,
			&SecuritySeverity::Critical
		));

		assert!(!SecurityAudit::is_security_threat(
			&SecurityEventType::NetworkSecurityEvent,
			&SecuritySeverity::Low
		));
	});
}

#[test]
fn test_get_security_events_by_filters() {
	new_test_ext().execute_with(|| {
		// Record multiple different events
		assert_ok!(SecurityAudit::record_security_event(
			RuntimeOrigin::signed(1),
			SecurityEventType::SuspiciousActivity,
			SecuritySeverity::High,
			Some(1),
			None,
			b"action1".to_vec(),
			b"details1".to_vec(),
			SecurityEventResult::Investigating,
			None,
			None,
			None,
		));

		assert_ok!(SecurityAudit::record_security_event(
			RuntimeOrigin::signed(2),
			SecurityEventType::UnauthorizedAccess,
			SecuritySeverity::Critical,
			Some(2),
			None,
			b"action2".to_vec(),
			b"details2".to_vec(),
			SecurityEventResult::Blocked,
			None,
			None,
			None,
		));

		assert_ok!(SecurityAudit::record_security_event(
			RuntimeOrigin::signed(1),
			SecurityEventType::SuspiciousActivity,
			SecuritySeverity::Medium,
			Some(1),
			None,
			b"action3".to_vec(),
			b"details3".to_vec(),
			SecurityEventResult::Success,
			None,
			None,
			None,
		));

		// Test filtering by account
		let events_by_account1 = SecurityAudit::get_security_events_by_account(&1);
		assert_eq!(events_by_account1, vec![0, 2]);

		let events_by_account2 = SecurityAudit::get_security_events_by_account(&2);
		assert_eq!(events_by_account2, vec![1]);

		// Test filtering by event type
		let suspicious_events =
			SecurityAudit::get_security_events_by_type(&SecurityEventType::SuspiciousActivity);
		assert_eq!(suspicious_events, vec![0, 2]);

		let unauthorized_events =
			SecurityAudit::get_security_events_by_type(&SecurityEventType::UnauthorizedAccess);
		assert_eq!(unauthorized_events, vec![1]);

		// Test filtering by severity
		let high_severity_events =
			SecurityAudit::get_security_events_by_severity(&SecuritySeverity::High);
		assert_eq!(high_severity_events, vec![0]);

		let critical_events =
			SecurityAudit::get_security_events_by_severity(&SecuritySeverity::Critical);
		assert_eq!(critical_events, vec![1]);

		let medium_events =
			SecurityAudit::get_security_events_by_severity(&SecuritySeverity::Medium);
		assert_eq!(medium_events, vec![2]);
	});
}

#[test]
fn test_total_security_events() {
	new_test_ext().execute_with(|| {
		assert_eq!(SecurityAudit::get_total_security_events(), 0);

		// Record some events
		assert_ok!(SecurityAudit::record_security_event(
			RuntimeOrigin::signed(1),
			SecurityEventType::SuspiciousActivity,
			SecuritySeverity::High,
			Some(1),
			None,
			b"action1".to_vec(),
			b"details1".to_vec(),
			SecurityEventResult::Investigating,
			None,
			None,
			None,
		));

		assert_eq!(SecurityAudit::get_total_security_events(), 1);

		assert_ok!(SecurityAudit::record_security_event(
			RuntimeOrigin::signed(2),
			SecurityEventType::UnauthorizedAccess,
			SecuritySeverity::Critical,
			Some(2),
			None,
			b"action2".to_vec(),
			b"details2".to_vec(),
			SecurityEventResult::Blocked,
			None,
			None,
			None,
		));

		assert_eq!(SecurityAudit::get_total_security_events(), 2);
	});
}
