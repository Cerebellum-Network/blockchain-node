#![cfg_attr(not(feature = "std"), no_std)]

//! # Security Audit Pallet
//!
//! This pallet provides comprehensive security event logging and audit trail functionality
//! for blockchain nodes. It tracks security-relevant events, provides metrics for monitoring,
//! and maintains an immutable audit trail.

pub mod mock;
#[cfg(test)]
pub mod tests;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::*,
	traits::{Get, StorageVersion},
	BoundedVec, PalletId,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_runtime::traits::Convert;
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub use pallet::*;

/// The current storage version.
const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

/// Security event types
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum SecurityEventType {
	/// Suspicious activity detected
	SuspiciousActivity,
	/// Unauthorized access attempt
	UnauthorizedAccess,
	/// Authentication failure
	AuthenticationFailure,
	/// Privilege escalation attempt
	PrivilegeEscalation,
	/// Data access violation
	DataAccessViolation,
	/// System configuration change
	SystemConfigurationChange,
	/// Network security event
	NetworkSecurityEvent,
	/// Consensus security event
	ConsensusSecurityEvent,
	/// Transaction pool security event
	TransactionPoolSecurityEvent,
	/// RPC security event
	RPCSecurityEvent,
	/// Database security event
	DatabaseSecurityEvent,
	/// Smart contract security event
	SmartContractSecurityEvent,
}

/// Security event severity levels
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum SecuritySeverity {
	/// Low severity - informational
	Low,
	/// Medium severity - warning
	Medium,
	/// High severity - requires attention
	High,
	/// Critical severity - immediate action required
	Critical,
}

/// Security event result status
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum SecurityEventResult {
	/// Event was successfully handled
	Success,
	/// Event resulted in failure
	Failure,
	/// Event was blocked/prevented
	Blocked,
	/// Event is under investigation
	Investigating,
}

/// Comprehensive security event structure
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SecurityEvent<AccountId, BlockNumber> {
	/// Event identifier
	pub event_id: u64,
	/// Event type
	pub event_type: SecurityEventType,
	/// Event severity
	pub severity: SecuritySeverity,
	/// Account that triggered the event (if applicable)
	pub actor: Option<AccountId>,
	/// Target resource or account
	pub target: Option<AccountId>,
	/// Action that was attempted
	pub action: BoundedVec<u8, ConstU32<256>>,
	/// Additional event details
	pub details: BoundedVec<u8, ConstU32<1024>>,
	/// Event result
	pub result: SecurityEventResult,
	/// Block number when event occurred
	pub block_number: BlockNumber,
	/// Timestamp when event occurred
	pub timestamp: u64,
	/// Source IP address (if available)
	pub source_ip: Option<BoundedVec<u8, ConstU32<64>>>,
	/// User agent (if available)
	pub user_agent: Option<BoundedVec<u8, ConstU32<256>>>,
	/// Session ID (if applicable)
	pub session_id: Option<BoundedVec<u8, ConstU32<128>>>,
}

/// Security metrics structure
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SecurityMetricsData {
	/// Total number of security events
	pub total_events: u64,
	/// Number of critical events
	pub critical_events: u64,
	/// Number of high severity events
	pub high_severity_events: u64,
	/// Number of blocked events
	pub blocked_events: u64,
	/// Number of failed events
	pub failed_events: u64,
	/// Last event timestamp
	pub last_event_timestamp: u64,
}

impl Default for SecurityMetricsData {
	fn default() -> Self {
		SecurityMetricsData {
			total_events: 0,
			critical_events: 0,
			high_severity_events: 0,
			blocked_events: 0,
			failed_events: 0,
			last_event_timestamp: 0,
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The pallet ID for security audit
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Maximum number of security events to store
		#[pallet::constant]
		type MaxSecurityEvents: Get<u32>;

		/// Maximum size of event details
		#[pallet::constant]
		type MaxEventDetailsSize: Get<u32>;

		/// Maximum size of action description
		#[pallet::constant]
		type MaxActionSize: Get<u32>;

		/// Origin that can manage security configurations
		type SecurityOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Converter for timestamp to u64
		type TimestampToU64: Convert<Self::Moment, u64>;
	}

	#[pallet::storage]
	#[pallet::getter(fn security_events)]
	pub type SecurityEvents<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u64,
		SecurityEvent<T::AccountId, BlockNumberFor<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn security_event_count)]
	pub type SecurityEventCount<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn security_metrics)]
	pub type SecurityMetrics<T: Config> = StorageValue<_, SecurityMetricsData, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn security_event_index)]
	pub type SecurityEventsByAccount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<u64, T::MaxSecurityEvents>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn security_events_by_type)]
	pub type SecurityEventsByType<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		SecurityEventType,
		BoundedVec<u64, T::MaxSecurityEvents>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn security_events_by_severity)]
	pub type SecurityEventsBySeverity<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		SecuritySeverity,
		BoundedVec<u64, T::MaxSecurityEvents>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Security event recorded
		SecurityEventRecorded {
			event_id: u64,
			event_type: SecurityEventType,
			severity: SecuritySeverity,
			actor: Option<T::AccountId>,
			target: Option<T::AccountId>,
		},
		/// Security alert triggered
		SecurityAlertTriggered {
			event_id: u64,
			severity: SecuritySeverity,
			event_type: SecurityEventType,
		},
		/// Security metrics updated
		SecurityMetricsUpdated {
			total_events: u64,
			critical_events: u64,
			high_severity_events: u64,
		},
		/// Security event cleared
		SecurityEventCleared { event_id: u64 },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Event details too large
		EventDetailsTooLarge,
		/// Action description too large
		ActionTooLarge,
		/// Security event not found
		SecurityEventNotFound,
		/// Maximum security events exceeded
		MaxSecurityEventsExceeded,
		/// Invalid event type
		InvalidEventType,
		/// Invalid severity level
		InvalidSeverity,
		/// Insufficient permissions
		InsufficientPermissions,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Record a security event
		#[pallet::call_index(0)]
		#[pallet::weight(10_000)]
		pub fn record_security_event(
			origin: OriginFor<T>,
			event_type: SecurityEventType,
			severity: SecuritySeverity,
			actor: Option<T::AccountId>,
			target: Option<T::AccountId>,
			action: BoundedVec<u8, ConstU32<256>>,
			details: BoundedVec<u8, ConstU32<1024>>,
			result: SecurityEventResult,
			source_ip: Option<BoundedVec<u8, ConstU32<64>>>,
			user_agent: Option<BoundedVec<u8, ConstU32<256>>>,
			session_id: Option<BoundedVec<u8, ConstU32<128>>>,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			let event_id = Self::security_event_count();
			let block_number = frame_system::Pallet::<T>::block_number();
			let timestamp = pallet_timestamp::Pallet::<T>::get();
			let timestamp_u64 = T::TimestampToU64::convert(timestamp);

			let security_event = SecurityEvent {
				event_id,
				event_type: event_type.clone(),
				severity: severity.clone(),
				actor: actor.clone(),
				target: target.clone(),
				action,
				details,
				result,
				block_number,
				timestamp: timestamp_u64,
				source_ip,
				user_agent,
				session_id,
			};

			// Store the security event
			SecurityEvents::<T>::insert(event_id, &security_event);

			// Update indexes
			if let Some(actor_account) = &actor {
				SecurityEventsByAccount::<T>::try_mutate(actor_account, |events| {
					events.try_push(event_id).map_err(|_| Error::<T>::MaxSecurityEventsExceeded)
				})?;
			}

			SecurityEventsByType::<T>::try_mutate(&event_type, |events| {
				events.try_push(event_id).map_err(|_| Error::<T>::MaxSecurityEventsExceeded)
			})?;

			SecurityEventsBySeverity::<T>::try_mutate(&severity, |events| {
				events.try_push(event_id).map_err(|_| Error::<T>::MaxSecurityEventsExceeded)
			})?;

			// Update metrics
			Self::update_security_metrics(&severity, timestamp_u64)?;

			// Increment event count
			SecurityEventCount::<T>::put(event_id.saturating_add(1));

			// Emit event
			Self::deposit_event(Event::SecurityEventRecorded {
				event_id,
				event_type: event_type.clone(),
				severity: severity.clone(),
				actor,
				target,
			});

			// Trigger alert for high severity events
			if matches!(severity, SecuritySeverity::High | SecuritySeverity::Critical) {
				Self::deposit_event(Event::SecurityAlertTriggered {
					event_id,
					severity,
					event_type,
				});
			}

			Ok(())
		}

		/// Clear old security events (admin only)
		#[pallet::call_index(1)]
		#[pallet::weight(10_000)]
		pub fn clear_security_events(origin: OriginFor<T>, event_ids: Vec<u64>) -> DispatchResult {
			T::SecurityOrigin::ensure_origin(origin)?;

			for event_id in event_ids {
				if SecurityEvents::<T>::contains_key(event_id) {
					SecurityEvents::<T>::remove(event_id);
					Self::deposit_event(Event::SecurityEventCleared { event_id });
				}
			}

			Ok(())
		}

		/// Update security configuration (admin only)
		#[pallet::call_index(2)]
		#[pallet::weight(10_000)]
		pub fn update_security_config(origin: OriginFor<T>) -> DispatchResult {
			T::SecurityOrigin::ensure_origin(origin)?;
			// Implementation for updating security configuration
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Update security metrics
		fn update_security_metrics(severity: &SecuritySeverity, timestamp: u64) -> DispatchResult {
			SecurityMetrics::<T>::try_mutate(|metrics| {
				metrics.total_events = metrics.total_events.saturating_add(1);
				metrics.last_event_timestamp = timestamp;

				match severity {
					SecuritySeverity::Critical => {
						metrics.critical_events = metrics.critical_events.saturating_add(1);
					},
					SecuritySeverity::High => {
						metrics.high_severity_events =
							metrics.high_severity_events.saturating_add(1);
					},
					_ => {},
				}

				Self::deposit_event(Event::SecurityMetricsUpdated {
					total_events: metrics.total_events,
					critical_events: metrics.critical_events,
					high_severity_events: metrics.high_severity_events,
				});

				Ok(())
			})
		}

		/// Log a security event (convenience method)
		pub fn log_security_event(
			event_type: SecurityEventType,
			severity: SecuritySeverity,
			actor: Option<T::AccountId>,
			target: Option<T::AccountId>,
			action: &[u8],
			details: &[u8],
			result: SecurityEventResult,
		) -> DispatchResult {
			let event_id = Self::security_event_count();
			let block_number = frame_system::Pallet::<T>::block_number();
			let timestamp = pallet_timestamp::Pallet::<T>::get();
			let timestamp_u64 = T::TimestampToU64::convert(timestamp);

			let action_bounded =
				action.to_vec().try_into().map_err(|_| Error::<T>::ActionTooLarge)?;
			let details_bounded =
				details.to_vec().try_into().map_err(|_| Error::<T>::EventDetailsTooLarge)?;

			let security_event = SecurityEvent {
				event_id,
				event_type: event_type.clone(),
				severity: severity.clone(),
				actor: actor.clone(),
				target: target.clone(),
				action: action_bounded,
				details: details_bounded,
				result: result.clone(),
				block_number,
				timestamp: timestamp_u64,
				source_ip: None,
				user_agent: None,
				session_id: None,
			};

			// Store the security event
			SecurityEvents::<T>::insert(event_id, &security_event);

			// Update indexes
			if let Some(actor_account) = &actor {
				SecurityEventsByAccount::<T>::try_mutate(actor_account, |events| {
					events.try_push(event_id).map_err(|_| Error::<T>::MaxSecurityEventsExceeded)
				})?;
			}

			SecurityEventsByType::<T>::try_mutate(&event_type, |events| {
				events.try_push(event_id).map_err(|_| Error::<T>::MaxSecurityEventsExceeded)
			})?;

			SecurityEventsBySeverity::<T>::try_mutate(&severity, |events| {
				events.try_push(event_id).map_err(|_| Error::<T>::MaxSecurityEventsExceeded)
			})?;

			// Update metrics
			Self::update_security_metrics(&severity, timestamp_u64)?;

			// Increment event count
			SecurityEventCount::<T>::put(event_id.saturating_add(1));

			// Emit event
			Self::deposit_event(Event::SecurityEventRecorded {
				event_id,
				event_type: event_type.clone(),
				severity: severity.clone(),
				actor: actor.clone(),
				target: target.clone(),
			});

			#[cfg(feature = "std")]
			{
				use serde_json::json;
				let event_json = json!({
					"event_id": event_id,
					"event_type": event_type,
					"severity": severity,
					"actor": actor,
					"target": target,
					"action": String::from_utf8_lossy(action),
					"details": String::from_utf8_lossy(details),
					"result": result,
					"block_number": block_number,
					"timestamp": timestamp_u64,
				});

				log::warn!(
					target: "security-audit",
					"SECURITY_EVENT: {}",
					event_json
				);
			}

			Ok(())
		}

		/// Get security events by account
		pub fn get_security_events_by_account(account: &T::AccountId) -> Vec<u64> {
			SecurityEventsByAccount::<T>::get(account).into_inner()
		}

		/// Get security events by type
		pub fn get_security_events_by_type(event_type: &SecurityEventType) -> Vec<u64> {
			SecurityEventsByType::<T>::get(event_type).into_inner()
		}

		/// Get security events by severity
		pub fn get_security_events_by_severity(severity: &SecuritySeverity) -> Vec<u64> {
			SecurityEventsBySeverity::<T>::get(severity).into_inner()
		}

		/// Get total security event count
		pub fn get_total_security_events() -> u64 {
			Self::security_event_count()
		}

		/// Get security metrics
		pub fn get_security_metrics() -> SecurityMetricsData {
			Self::security_metrics()
		}

		/// Check if an event is a security threat
		pub fn is_security_threat(
			event_type: &SecurityEventType,
			severity: &SecuritySeverity,
		) -> bool {
			matches!(severity, SecuritySeverity::High | SecuritySeverity::Critical)
				|| matches!(
					event_type,
					SecurityEventType::SuspiciousActivity
						| SecurityEventType::UnauthorizedAccess
						| SecurityEventType::PrivilegeEscalation
						| SecurityEventType::DataAccessViolation
				)
		}
	}
}

/// Security event logging helper functions
pub mod security_logger {
	use super::*;

	/// Log suspicious activity
	pub fn log_suspicious_activity<T: Config>(
		actor: Option<T::AccountId>,
		details: &[u8],
	) -> DispatchResult {
		Pallet::<T>::log_security_event(
			SecurityEventType::SuspiciousActivity,
			SecuritySeverity::High,
			actor,
			None,
			b"suspicious_activity_detected",
			details,
			SecurityEventResult::Investigating,
		)
	}

	/// Log unauthorized access attempt
	pub fn log_unauthorized_access<T: Config>(
		actor: Option<T::AccountId>,
		target: Option<T::AccountId>,
		details: &[u8],
	) -> DispatchResult {
		Pallet::<T>::log_security_event(
			SecurityEventType::UnauthorizedAccess,
			SecuritySeverity::High,
			actor,
			target,
			b"unauthorized_access_attempt",
			details,
			SecurityEventResult::Blocked,
		)
	}

	/// Log authentication failure
	pub fn log_authentication_failure<T: Config>(
		actor: Option<T::AccountId>,
		details: &[u8],
	) -> DispatchResult {
		Pallet::<T>::log_security_event(
			SecurityEventType::AuthenticationFailure,
			SecuritySeverity::Medium,
			actor,
			None,
			b"authentication_failed",
			details,
			SecurityEventResult::Failure,
		)
	}

	/// Log privilege escalation attempt
	pub fn log_privilege_escalation<T: Config>(
		actor: Option<T::AccountId>,
		target: Option<T::AccountId>,
		details: &[u8],
	) -> DispatchResult {
		Pallet::<T>::log_security_event(
			SecurityEventType::PrivilegeEscalation,
			SecuritySeverity::Critical,
			actor,
			target,
			b"privilege_escalation_attempt",
			details,
			SecurityEventResult::Blocked,
		)
	}

	/// Log consensus security event
	pub fn log_consensus_security_event<T: Config>(
		event_details: &[u8],
		severity: SecuritySeverity,
	) -> DispatchResult {
		Pallet::<T>::log_security_event(
			SecurityEventType::ConsensusSecurityEvent,
			severity,
			None,
			None,
			b"consensus_security_event",
			event_details,
			SecurityEventResult::Investigating,
		)
	}

	/// Log RPC security event
	pub fn log_rpc_security_event<T: Config>(
		actor: Option<T::AccountId>,
		method: &[u8],
		details: &[u8],
	) -> DispatchResult {
		Pallet::<T>::log_security_event(
			SecurityEventType::RPCSecurityEvent,
			SecuritySeverity::Medium,
			actor,
			None,
			method,
			details,
			SecurityEventResult::Investigating,
		)
	}
}
