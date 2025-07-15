use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    dispatch::DispatchResult,
    traits::Get,
    BoundedVec,
};
use sp_runtime::{RuntimeDebug, traits::{Hash, Saturating}, SaturatedConversion};
use sp_runtime::scale_info::TypeInfo;
use sp_std::vec::Vec;
use sp_io;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Audit event categories
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AuditEventCategory {
    /// System administration events
    SystemAdmin,
    /// User account management
    UserManagement,
    /// Data access and modification
    DataAccess,
    /// Configuration changes
    ConfigurationChange,
    /// Security events
    SecurityEvent,
    /// Compliance events
    ComplianceEvent,
    /// Financial transactions
    FinancialTransaction,
    /// Network events
    NetworkEvent,
    /// Consensus events
    ConsensusEvent,
    /// Smart contract events
    SmartContractEvent,
}

/// Audit event types
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AuditEventType {
    /// User login event
    UserLogin,
    /// User logout event
    UserLogout,
    /// Account creation
    AccountCreation,
    /// Account modification
    AccountModification,
    /// Account deletion
    AccountDeletion,
    /// Password change
    PasswordChange,
    /// Permission change
    PermissionChange,
    /// Data read access
    DataRead,
    /// Data write access
    DataWrite,
    /// Data deletion
    DataDeletion,
    /// Configuration read
    ConfigurationRead,
    /// Configuration write
    ConfigurationWrite,
    /// System startup
    SystemStartup,
    /// System shutdown
    SystemShutdown,
    /// Service start
    ServiceStart,
    /// Service stop
    ServiceStop,
    /// Transaction submitted
    TransactionSubmitted,
    /// Transaction executed
    TransactionExecuted,
    /// Transaction failed
    TransactionFailed,
    /// Block produced
    BlockProduced,
    /// Block finalized
    BlockFinalized,
    /// Consensus participation
    ConsensusParticipation,
    /// Smart contract deployed
    SmartContractDeployed,
    /// Smart contract executed
    SmartContractExecuted,
    /// Security violation
    SecurityViolation,
    /// Compliance check
    ComplianceCheck,
    /// Policy violation
    PolicyViolation,
    /// Audit log access
    AuditLogAccess,
    /// Backup created
    BackupCreated,
    /// Backup restored
    BackupRestored,
    /// Network connection
    NetworkConnection,
    /// Network disconnection
    NetworkDisconnection,
    /// Resource usage
    ResourceUsage,
    /// Error occurred
    ErrorOccurred,
    /// Warning issued
    WarningIssued,
    /// Information logged
    InformationLogged,
}

/// Audit event result
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AuditEventResult {
    /// Event completed successfully
    Success,
    /// Event failed
    Failure,
    /// Event was denied
    Denied,
    /// Event was partially completed
    Partial,
    /// Event is in progress
    InProgress,
    /// Event was cancelled
    Cancelled,
    /// Event requires approval
    PendingApproval,
}

/// Comprehensive audit event structure
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AuditEvent<AccountId> {
    /// Unique event identifier
    pub event_id: u64,
    /// Event category
    pub category: AuditEventCategory,
    /// Event type
    pub event_type: AuditEventType,
    /// Account that initiated the event
    pub initiator: Option<AccountId>,
    /// Target account or resource
    pub target: Option<AccountId>,
    /// Resource identifier
    pub resource_id: Option<BoundedVec<u8, frame_support::traits::ConstU32<256>>>,
    /// Event description
    pub description: BoundedVec<u8, frame_support::traits::ConstU32<1024>>,
    /// Additional event data
    pub event_data: BoundedVec<u8, frame_support::traits::ConstU32<2048>>,
    /// Event result
    pub result: AuditEventResult,
    /// Block number when event occurred
    pub block_number: u32,
    /// Block hash when event occurred
    pub block_hash: sp_core::H256,
    /// Timestamp when event occurred
    pub timestamp: u64,
    /// Session identifier
    pub session_id: Option<BoundedVec<u8, frame_support::traits::ConstU32<128>>>,
    /// IP address (if applicable)
    pub ip_address: Option<BoundedVec<u8, frame_support::traits::ConstU32<64>>>,
    /// User agent (if applicable)
    pub user_agent: Option<BoundedVec<u8, frame_support::traits::ConstU32<512>>>,
    /// Event hash for integrity verification
    pub event_hash: sp_core::H256,
    /// Previous event hash for chain integrity
    pub previous_event_hash: Option<sp_core::H256>,
}

/// Audit trail statistics
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AuditTrailStats {
    /// Total number of audit events
    pub total_events: u64,
    /// Number of events by category
    pub events_by_category: BoundedVec<(AuditEventCategory, u64), frame_support::traits::ConstU32<32>>,
    /// Number of successful events
    pub successful_events: u64,
    /// Number of failed events
    pub failed_events: u64,
    /// Number of denied events
    pub denied_events: u64,
    /// First event timestamp
    pub first_event_timestamp: u64,
    /// Last event timestamp
    pub last_event_timestamp: u64,
    /// Audit trail integrity status
    pub integrity_verified: bool,
}

/// Audit trail configuration
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AuditTrailConfig {
    /// Maximum number of events to retain
    pub max_events: u64,
    /// Event retention period in blocks
    pub retention_period: u64,
    /// Enable event integrity verification
    pub integrity_verification: bool,
    /// Enable event compression
    pub compression_enabled: bool,
    /// Event archival threshold
    pub archival_threshold: u64,
}

/// Audit trail manager
pub struct AuditTrailManager<T: Config> {
    _phantom: sp_std::marker::PhantomData<T>,
}

pub trait Config: frame_system::Config + pallet_timestamp::Config {
    /// Maximum size of event data
    type MaxEventDataSize: Get<u32>;
    /// Maximum size of event description
    type MaxEventDescriptionSize: Get<u32>;
}

impl<T: Config> AuditTrailManager<T> {
    /// Create a new audit event
    pub fn create_audit_event(
        category: AuditEventCategory,
        event_type: AuditEventType,
        initiator: Option<T::AccountId>,
        target: Option<T::AccountId>,
        resource_id: Option<Vec<u8>>,
        description: Vec<u8>,
        event_data: Vec<u8>,
        result: AuditEventResult,
        session_id: Option<Vec<u8>>,
        ip_address: Option<Vec<u8>>,
        user_agent: Option<Vec<u8>>,
    ) -> Result<AuditEvent<T::AccountId>, &'static str> {
        // Validate input sizes
        if description.len() > T::MaxEventDescriptionSize::get() as usize {
            return Err("Event description too large");
        }
        if event_data.len() > T::MaxEventDataSize::get() as usize {
            return Err("Event data too large");
        }

        let event_id = Self::get_next_event_id();
        let block_number: u32 = frame_system::Pallet::<T>::block_number().saturated_into();
        let block_hash: sp_core::H256 = frame_system::Pallet::<T>::block_hash(frame_system::Pallet::<T>::block_number()).into();
        let timestamp: u64 = pallet_timestamp::Pallet::<T>::get().saturated_into();
        
        // Create event structure for hashing
        let event_for_hash = (
            event_id,
            &category,
            &event_type,
            &initiator,
            &target,
            &resource_id,
            &description,
            &event_data,
            &result,
            &block_number,
            &block_hash,
            &timestamp,
            &session_id,
            &ip_address,
            &user_agent,
        );

        let event_hash = T::Hashing::hash_of(&event_for_hash);
        let previous_event_hash = Self::get_last_event_hash();

        Ok(AuditEvent {
            event_id,
            category,
            event_type,
            initiator,
            target,
            resource_id,
            description,
            event_data,
            result,
            block_number,
            block_hash,
            timestamp,
            session_id,
            ip_address,
            user_agent,
            event_hash,
            previous_event_hash,
        })
    }

    /// Log an audit event
    pub fn log_audit_event(
        category: AuditEventCategory,
        event_type: AuditEventType,
        initiator: Option<T::AccountId>,
        target: Option<T::AccountId>,
        resource_id: Option<Vec<u8>>,
        description: &str,
        event_data: Option<Vec<u8>>,
        result: AuditEventResult,
        session_id: Option<Vec<u8>>,
        ip_address: Option<Vec<u8>>,
        user_agent: Option<Vec<u8>>,
    ) -> DispatchResult {
        let audit_event = Self::create_audit_event(
            category,
            event_type,
            initiator,
            target,
            resource_id,
            description.as_bytes().to_vec(),
            event_data.unwrap_or_default(),
            result,
            session_id,
            ip_address,
            user_agent,
        ).map_err(|_| "Failed to create audit event")?;

        // Store the audit event (implementation depends on storage backend)
        Self::store_audit_event(&audit_event)?;

        // Update audit trail statistics
        Self::update_audit_stats(&audit_event)?;

        // Emit log for external monitoring
        #[cfg(feature = "std")]
        {
            use serde_json::json;
            let log_entry = json!({
                "event_id": audit_event.event_id,
                "category": audit_event.category,
                "event_type": audit_event.event_type,
                "initiator": audit_event.initiator,
                "target": audit_event.target,
                "resource_id": audit_event.resource_id,
                "description": String::from_utf8_lossy(&audit_event.description),
                "result": audit_event.result,
                "timestamp": audit_event.timestamp,
                "block_number": audit_event.block_number,
                "event_hash": format!("{:?}", audit_event.event_hash),
            });
            
            log::info!(
                target: "audit-trail",
                "AUDIT_EVENT: {}",
                log_entry
            );
        }

        Ok(())
    }

    /// Verify audit trail integrity
    pub fn verify_audit_trail_integrity() -> Result<bool, &'static str> {
        // Implementation would verify the chain of event hashes
        // This is a placeholder for the actual verification logic
        Ok(true)
    }

    /// Get audit events by category
    pub fn get_events_by_category(category: AuditEventCategory) -> Vec<u64> {
        // Implementation depends on storage backend
        Vec::new()
    }

    /// Get audit events by initiator
    pub fn get_events_by_initiator(initiator: &T::AccountId) -> Vec<u64> {
        // Implementation depends on storage backend
        Vec::new()
    }

    /// Get audit events by time range
    pub fn get_events_by_time_range(start_timestamp: u64, end_timestamp: u64) -> Vec<u64> {
        // Implementation depends on storage backend
        Vec::new()
    }

    /// Get audit trail statistics
    pub fn get_audit_trail_stats() -> AuditTrailStats {
        // Implementation depends on storage backend
        AuditTrailStats {
            total_events: 0,
            events_by_category: Vec::new(),
            successful_events: 0,
            failed_events: 0,
            denied_events: 0,
            first_event_timestamp: 0,
            last_event_timestamp: 0,
            integrity_verified: false,
        }
    }

    /// Archive old audit events
    pub fn archive_old_events(cutoff_timestamp: u64) -> Result<u64, &'static str> {
        // Implementation would archive events older than cutoff_timestamp
        Ok(0)
    }

    /// Search audit events
    pub fn search_audit_events(
        category: Option<AuditEventCategory>,
        event_type: Option<AuditEventType>,
        initiator: Option<T::AccountId>,
        target: Option<T::AccountId>,
        start_timestamp: Option<u64>,
        _end_timestamp: Option<u64>,
        _result: Option<AuditEventResult>,
    ) -> Vec<u64> {
        // Implementation depends on storage backend
        Vec::new()
    }

    /// Generate compliance report
    pub fn generate_compliance_report(
        _start_timestamp: u64,
        _end_timestamp: u64,
        _categories: Vec<AuditEventCategory>,
    ) -> Result<Vec<u8>, &'static str> {
        // Implementation would generate a compliance report
        Ok(Vec::new())
    }

    // Private helper methods
    fn get_next_event_id() -> u64 {
        // Implementation depends on storage backend
        1
    }

    fn get_last_event_hash() -> Option<sp_core::H256> {
        // Implementation depends on storage backend
        None
    }

    fn store_audit_event(event: &AuditEvent<T::AccountId>) -> DispatchResult {
        // Implementation depends on storage backend
        Ok(())
    }

    fn update_audit_stats(event: &AuditEvent<T::AccountId>) -> DispatchResult {
        // Implementation depends on storage backend
        Ok(())
    }
}

/// Audit trail helper functions
pub mod audit_helpers {
    use super::*;

    /// Log user login event
    pub fn log_user_login<T: Config>(
        user: T::AccountId,
        session_id: Option<Vec<u8>>,
        ip_address: Option<Vec<u8>>,
        user_agent: Option<Vec<u8>>,
        success: bool,
    ) -> DispatchResult {
        AuditTrailManager::<T>::log_audit_event(
            AuditEventCategory::UserManagement,
            AuditEventType::UserLogin,
            Some(user),
            None,
            None,
            "User login attempt",
            None,
            if success { AuditEventResult::Success } else { AuditEventResult::Failure },
            session_id,
            ip_address,
            user_agent,
        )
    }

    /// Log user logout event
    pub fn log_user_logout<T: Config>(
        user: T::AccountId,
        session_id: Option<Vec<u8>>,
    ) -> DispatchResult {
        AuditTrailManager::<T>::log_audit_event(
            AuditEventCategory::UserManagement,
            AuditEventType::UserLogout,
            Some(user),
            None,
            None,
            "User logout",
            None,
            AuditEventResult::Success,
            session_id,
            None,
            None,
        )
    }

    /// Log transaction submission
    pub fn log_transaction_submitted<T: Config>(
        submitter: T::AccountId,
        transaction_hash: sp_core::H256,
        transaction_type: &str,
    ) -> DispatchResult {
        AuditTrailManager::<T>::log_audit_event(
            AuditEventCategory::FinancialTransaction,
            AuditEventType::TransactionSubmitted,
            Some(submitter),
            None,
            Some(format!("{:?}", transaction_hash).into_bytes()),
            &format!("Transaction submitted: {}", transaction_type),
            None,
            AuditEventResult::Success,
            None,
            None,
            None,
        )
    }

    /// Log transaction execution
    pub fn log_transaction_executed<T: Config>(
        executor: Option<T::AccountId>,
        transaction_hash: sp_core::H256,
        success: bool,
        error_message: Option<&str>,
    ) -> DispatchResult {
        let description = if success {
            "Transaction executed successfully"
        } else {
            error_message.unwrap_or("Transaction execution failed")
        };

        AuditTrailManager::<T>::log_audit_event(
            AuditEventCategory::FinancialTransaction,
            AuditEventType::TransactionExecuted,
            executor,
            None,
            Some(format!("{:?}", transaction_hash).into_bytes()),
            description,
            None,
            if success { AuditEventResult::Success } else { AuditEventResult::Failure },
            None,
            None,
            None,
        )
    }

    /// Log configuration change
    pub fn log_configuration_change<T: Config>(
        changer: T::AccountId,
        configuration_key: &str,
        old_value: Option<&str>,
        new_value: &str,
    ) -> DispatchResult {
        let description = format!(
            "Configuration changed: {} from {:?} to {}",
            configuration_key,
            old_value,
            new_value
        );

        AuditTrailManager::<T>::log_audit_event(
            AuditEventCategory::ConfigurationChange,
            AuditEventType::ConfigurationWrite,
            Some(changer),
            None,
            Some(configuration_key.as_bytes().to_vec()),
            &description,
            None,
            AuditEventResult::Success,
            None,
            None,
            None,
        )
    }

    /// Log data access
    pub fn log_data_access<T: Config>(
        accessor: T::AccountId,
        resource_id: &str,
        access_type: &str,
        success: bool,
    ) -> DispatchResult {
        let description = format!("Data access: {} on {}", access_type, resource_id);

        AuditTrailManager::<T>::log_audit_event(
            AuditEventCategory::DataAccess,
            AuditEventType::DataRead,
            Some(accessor),
            None,
            Some(resource_id.as_bytes().to_vec()),
            &description,
            None,
            if success { AuditEventResult::Success } else { AuditEventResult::Failure },
            None,
            None,
            None,
        )
    }

    /// Log security violation
    pub fn log_security_violation<T: Config>(
        violator: Option<T::AccountId>,
        violation_type: &str,
        details: &str,
        ip_address: Option<Vec<u8>>,
    ) -> DispatchResult {
        let description = format!("Security violation: {} - {}", violation_type, details);

        AuditTrailManager::<T>::log_audit_event(
            AuditEventCategory::SecurityEvent,
            AuditEventType::SecurityViolation,
            violator,
            None,
            None,
            &description,
            None,
            AuditEventResult::Denied,
            None,
            ip_address,
            None,
        )
    }

    /// Log consensus event
    pub fn log_consensus_event<T: Config>(
        validator: Option<T::AccountId>,
        event_type: &str,
        block_number: u32,
        success: bool,
    ) -> DispatchResult {
        let description = format!("Consensus event: {} at block {:?}", event_type, block_number);

        AuditTrailManager::<T>::log_audit_event(
            AuditEventCategory::ConsensusEvent,
            AuditEventType::ConsensusParticipation,
            validator,
            None,
            Some(format!("{:?}", block_number).into_bytes()),
            &description,
            None,
            if success { AuditEventResult::Success } else { AuditEventResult::Failure },
            None,
            None,
            None,
        )
    }

    /// Log smart contract deployment
    pub fn log_smart_contract_deployment<T: Config>(
        deployer: T::AccountId,
        contract_address: T::AccountId,
        contract_code_hash: sp_core::H256,
        success: bool,
    ) -> DispatchResult {
        let description = format!(
            "Smart contract deployment: {:?} with code hash {:?}",
            contract_address,
            contract_code_hash
        );

        AuditTrailManager::<T>::log_audit_event(
            AuditEventCategory::SmartContractEvent,
            AuditEventType::SmartContractDeployed,
            Some(deployer),
            Some(contract_address),
            Some(format!("{:?}", contract_code_hash).into_bytes()),
            &description,
            None,
            if success { AuditEventResult::Success } else { AuditEventResult::Failure },
            None,
            None,
            None,
        )
    }

    /// Log system event
    pub fn log_system_event<T: Config>(
        event_type: AuditEventType,
        description: &str,
        details: Option<Vec<u8>>,
    ) -> DispatchResult {
        AuditTrailManager::<T>::log_audit_event(
            AuditEventCategory::SystemAdmin,
            event_type,
            None,
            None,
            None,
            description,
            details,
            AuditEventResult::Success,
            None,
            None,
            None,
        )
    }
} 
