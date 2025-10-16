//! Tests for the DAC Registry pallet

#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{
	assert_ok, assert_noop,
	traits::OnInitialize,
};
use sp_core::H256;
use sp_runtime::DispatchError;

#[test]
fn test_register_code_success() {
	new_test_ext().execute_with(|| {
		let code = test_utils::create_test_code();
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 10;

		// Register code
		assert_ok!(DacRegistry::register_code(
			RuntimeOrigin::root(),
			code.clone(),
			api_version,
			semver,
			allowed_from
		));

		// Check storage
		let code_hash = test_utils::create_test_code_hash();
		assert!(DacRegistry::code_by_hash(code_hash).is_some());
		assert!(DacRegistry::wasm_code(code_hash).is_some());
		assert!(!DacRegistry::deregistered(code_hash));

		// Check metadata
		let meta = DacRegistry::code_by_hash(code_hash).unwrap();
		assert_eq!(meta.api_version, api_version);
		assert_eq!(meta.semver, semver);
		assert_eq!(meta.allowed_from, allowed_from);
		assert_eq!(meta.length, code.len() as u32);

		// Check public interface
		assert!(DacRegistry::is_code_active(code_hash));
		assert!(!DacRegistry::is_code_ready(code_hash)); // Not ready yet (block 0 < 10)
	});
}

#[test]
fn test_register_code_validation() {
	new_test_ext().execute_with(|| {
		let code = test_utils::create_test_code();
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 10;

		// Test empty code
		assert_noop!(
			DacRegistry::register_code(
				RuntimeOrigin::root(),
				vec![],
				api_version,
				semver,
				allowed_from
			),
			Error::<Test>::EmptyCode
		);

		// Test code too large
		let large_code = test_utils::create_large_test_code(2 * 1024 * 1024); // 2MB
		assert_noop!(
			DacRegistry::register_code(
				RuntimeOrigin::root(),
				large_code,
				api_version,
				semver,
				allowed_from
			),
			Error::<Test>::CodeTooLarge
		);

		// Test invalid API version
		assert_noop!(
			DacRegistry::register_code(
				RuntimeOrigin::root(),
				code.clone(),
				(0, 0),
				semver,
				allowed_from
			),
			Error::<Test>::InvalidApiVersion
		);

		// Test invalid semantic version
		assert_noop!(
			DacRegistry::register_code(
				RuntimeOrigin::root(),
				code.clone(),
				api_version,
				(0, 0, 0),
				allowed_from
			),
			Error::<Test>::InvalidSemVer
		);

		// Test invalid activation block (in the past)
		assert_noop!(
			DacRegistry::register_code(
				RuntimeOrigin::root(),
				code.clone(),
				api_version,
				semver,
				0
			),
			Error::<Test>::InvalidActivationBlock
		);
	});
}

#[test]
fn test_register_code_duplicate() {
	new_test_ext().execute_with(|| {
		let code = test_utils::create_test_code();
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 10;

		// Register code first time
		assert_ok!(DacRegistry::register_code(
			RuntimeOrigin::root(),
			code.clone(),
			api_version,
			semver,
			allowed_from
		));

		// Try to register same code again
		assert_noop!(
			DacRegistry::register_code(
				RuntimeOrigin::root(),
				code,
				api_version,
				semver,
				allowed_from
			),
			Error::<Test>::CodeAlreadyExists
		);
	});
}

#[test]
fn test_update_meta_success() {
	new_test_ext().execute_with(|| {
		let code = test_utils::create_test_code();
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 10;

		// Register code
		assert_ok!(DacRegistry::register_code(
			RuntimeOrigin::root(),
			code,
			api_version,
			semver,
			allowed_from
		));

		let code_hash = test_utils::create_test_code_hash();

		// Update metadata
		let new_api_version = (2, 0);
		let new_semver = (2, 0, 0);
		let new_allowed_from = 20;

		assert_ok!(DacRegistry::update_meta(
			RuntimeOrigin::root(),
			code_hash,
			new_api_version,
			new_semver,
			new_allowed_from
		));

		// Check updated metadata
		let meta = DacRegistry::code_by_hash(code_hash).unwrap();
		assert_eq!(meta.api_version, new_api_version);
		assert_eq!(meta.semver, new_semver);
		assert_eq!(meta.allowed_from, new_allowed_from);
		assert_eq!(meta.length, 14); // Original length preserved
	});
}

#[test]
fn test_update_meta_validation() {
	new_test_ext().execute_with(|| {
		let code = test_utils::create_test_code();
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 10;

		// Register code
		assert_ok!(DacRegistry::register_code(
			RuntimeOrigin::root(),
			code,
			api_version,
			semver,
			allowed_from
		));

		let code_hash = test_utils::create_test_code_hash();

		// Test updating non-existent code
		let fake_hash = H256::from_slice(&[1u8; 32]);
		assert_noop!(
			DacRegistry::update_meta(
				RuntimeOrigin::root(),
				fake_hash,
				(2, 0),
				(2, 0, 0),
				20
			),
			Error::<Test>::CodeNotFound
		);

		// Test invalid API version
		assert_noop!(
			DacRegistry::update_meta(
				RuntimeOrigin::root(),
				code_hash,
				(0, 0),
				(2, 0, 0),
				20
			),
			Error::<Test>::InvalidApiVersion
		);

		// Test invalid semantic version
		assert_noop!(
			DacRegistry::update_meta(
				RuntimeOrigin::root(),
				code_hash,
				(2, 0),
				(0, 0, 0),
				20
			),
			Error::<Test>::InvalidSemVer
		);

		// Test invalid activation block
		assert_noop!(
			DacRegistry::update_meta(
				RuntimeOrigin::root(),
				code_hash,
				(2, 0),
				(2, 0, 0),
				0
			),
			Error::<Test>::InvalidActivationBlock
		);
	});
}

#[test]
fn test_deregister_code_success() {
	new_test_ext().execute_with(|| {
		let code = test_utils::create_test_code();
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 10;

		// Register code
		assert_ok!(DacRegistry::register_code(
			RuntimeOrigin::root(),
			code,
			api_version,
			semver,
			allowed_from
		));

		let code_hash = test_utils::create_test_code_hash();

		// Deregister code
		assert_ok!(DacRegistry::deregister_code(
			RuntimeOrigin::root(),
			code_hash
		));

		// Check deregistered flag
		assert!(DacRegistry::deregistered(code_hash));

		// Check public interface
		assert!(!DacRegistry::is_code_active(code_hash));
		assert!(!DacRegistry::is_code_ready(code_hash));
		assert!(DacRegistry::get_code(code_hash).is_none());
		assert!(DacRegistry::get_metadata(code_hash).is_none());
	});
}

#[test]
fn test_deregister_code_validation() {
	new_test_ext().execute_with(|| {
		// Test deregistering non-existent code
		let fake_hash = H256::from_slice(&[1u8; 32]);
		assert_noop!(
			DacRegistry::deregister_code(
				RuntimeOrigin::root(),
				fake_hash
			),
			Error::<Test>::CodeNotFound
		);
	});
}

#[test]
fn test_deregister_code_duplicate() {
	new_test_ext().execute_with(|| {
		let code = test_utils::create_test_code();
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 10;

		// Register code
		assert_ok!(DacRegistry::register_code(
			RuntimeOrigin::root(),
			code,
			api_version,
			semver,
			allowed_from
		));

		let code_hash = test_utils::create_test_code_hash();

		// Deregister code first time
		assert_ok!(DacRegistry::deregister_code(
			RuntimeOrigin::root(),
			code_hash
		));

		// Try to deregister again
		assert_noop!(
			DacRegistry::deregister_code(
				RuntimeOrigin::root(),
				code_hash
			),
			Error::<Test>::CodeDeregistered
		);
	});
}

#[test]
fn test_update_meta_after_deregister() {
	new_test_ext().execute_with(|| {
		let code = test_utils::create_test_code();
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 10;

		// Register code
		assert_ok!(DacRegistry::register_code(
			RuntimeOrigin::root(),
			code,
			api_version,
			semver,
			allowed_from
		));

		let code_hash = test_utils::create_test_code_hash();

		// Deregister code
		assert_ok!(DacRegistry::deregister_code(
			RuntimeOrigin::root(),
			code_hash
		));

		// Try to update metadata after deregister
		assert_noop!(
			DacRegistry::update_meta(
				RuntimeOrigin::root(),
				code_hash,
				(2, 0),
				(2, 0, 0),
				20
			),
			Error::<Test>::CodeDeregistered
		);
	});
}

#[test]
fn test_code_ready_check() {
	new_test_ext().execute_with(|| {
		let code = test_utils::create_test_code();
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 5;

		// Register code with activation block 5
		assert_ok!(DacRegistry::register_code(
			RuntimeOrigin::root(),
			code,
			api_version,
			semver,
			allowed_from
		));

		let code_hash = test_utils::create_test_code_hash();

		// At block 0, code should be active but not ready
		assert!(DacRegistry::is_code_active(code_hash));
		assert!(!DacRegistry::is_code_ready(code_hash));

		// Advance to block 5
		System::set_block_number(5);
		System::on_initialize(5);

		// Now code should be ready
		assert!(DacRegistry::is_code_active(code_hash));
		assert!(DacRegistry::is_code_ready(code_hash));
	});
}

#[test]
fn test_governance_origin() {
	new_test_ext().execute_with(|| {
		let code = test_utils::create_test_code();
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 10;

		// Test with non-root origin
		assert_noop!(
			DacRegistry::register_code(
				RuntimeOrigin::signed(1),
				code.clone(),
				api_version,
				semver,
				allowed_from
			),
			DispatchError::BadOrigin
		);

		// Test with root origin (should work)
		assert_ok!(DacRegistry::register_code(
			RuntimeOrigin::root(),
			code,
			api_version,
			semver,
			allowed_from
		));
	});
}

#[test]
fn test_events() {
	new_test_ext().execute_with(|| {
		// Set block number to 1 so events are registered
		System::set_block_number(1);
		
		let code = test_utils::create_test_code();
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 10;

		// Register code and check event
		assert_ok!(DacRegistry::register_code(
			RuntimeOrigin::root(),
			code.clone(),
			api_version,
			semver,
			allowed_from
		));

		let code_hash = test_utils::create_test_code_hash();

		// Check CodeRegistered event
		System::assert_has_event(RuntimeEvent::DacRegistry(
			Event::CodeRegistered {
				code_hash,
				api_version,
				semver,
				length: code.len() as u32,
			}
		));

		// Update metadata and check event
		assert_ok!(DacRegistry::update_meta(
			RuntimeOrigin::root(),
			code_hash,
			(2, 0),
			(2, 0, 0),
			20
		));

		// Check CodeMetaUpdated event
		System::assert_has_event(RuntimeEvent::DacRegistry(
			Event::CodeMetaUpdated {
				code_hash,
				api_version: (2, 0),
				semver: (2, 0, 0),
				allowed_from: 20,
			}
		));

		// Deregister code and check event
		assert_ok!(DacRegistry::deregister_code(
			RuntimeOrigin::root(),
			code_hash
		));

		// Check CodeDeregistered event
		System::assert_has_event(RuntimeEvent::DacRegistry(
			Event::CodeDeregistered { code_hash }
		));
	});
}
