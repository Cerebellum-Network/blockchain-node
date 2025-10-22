//! Benchmarking for the DAC Registry pallet.

use super::*;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;
use sp_core::Get;
use sp_core::H256;
use sp_runtime::traits::Hash;
use sp_std::vec;
use sp_std::vec::Vec;
use sp_std::iter;

/// Helper function to create test WASM code of a given size
fn create_test_wasm_code(size: u32) -> Vec<u8> {
	// Create a simple WASM module with the specified size
	let mut code = vec![
		0x00, 0x61, 0x73, 0x6d, // WASM magic number
		0x01, 0x00, 0x00, 0x00, // Version 1
	];

	// Add padding to reach the desired size
	let padding_size = size.saturating_sub(code.len() as u32);
	code.extend(iter::repeat_n(0x00, padding_size as usize));

	code
}

benchmarks! {
	register_code {
		let code_size = T::MaxCodeSize::get();
		let wasm_code = create_test_wasm_code(code_size);
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 0u32.into();
	}: _(RawOrigin::Root, wasm_code, api_version, semver, allowed_from)

	update_meta {
		// First register some code
		let code_size = 1024; // Use a smaller size for setup
		let wasm_code = create_test_wasm_code(code_size);
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 0u32.into();
		let code_hash = H256::from_slice(<T as frame_system::Config>::Hashing::hash(&wasm_code).as_ref());

		// Register the code first
		Pallet::<T>::register_code(RawOrigin::Root.into(), wasm_code, api_version, semver, allowed_from)?;

		// Create new metadata for update
		let new_api_version = (2, 0);
		let new_semver = (1, 1, 0);
		let new_allowed_from = 100u32.into();
	}: _(RawOrigin::Root, code_hash, new_api_version, new_semver, new_allowed_from)

	deregister_code {
		// First register some code
		let code_size = 1024; // Use a smaller size for setup
		let wasm_code = create_test_wasm_code(code_size);
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 0u32.into();
		let code_hash = H256::from_slice(<T as frame_system::Config>::Hashing::hash(&wasm_code).as_ref());

		// Register the code first
		Pallet::<T>::register_code(RawOrigin::Root.into(), wasm_code, api_version, semver, allowed_from)?;
	}: _(RawOrigin::Root, code_hash)

	// Benchmark for query operations
	is_code_active {
		// First register some code
		let code_size = 1024;
		let wasm_code = create_test_wasm_code(code_size);
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 0u32.into();
		let code_hash = H256::from_slice(<T as frame_system::Config>::Hashing::hash(&wasm_code).as_ref());

		// Register the code first
		Pallet::<T>::register_code(RawOrigin::Root.into(), wasm_code, api_version, semver, allowed_from)?;
	}: {
		let _ = Pallet::<T>::is_code_active(code_hash);
	}

	is_code_ready {
		// First register some code
		let code_size = 1024;
		let wasm_code = create_test_wasm_code(code_size);
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 0u32.into();
		let code_hash = H256::from_slice(<T as frame_system::Config>::Hashing::hash(&wasm_code).as_ref());

		// Register the code first
		Pallet::<T>::register_code(RawOrigin::Root.into(), wasm_code, api_version, semver, allowed_from)?;
	}: {
		let _ = Pallet::<T>::is_code_ready(code_hash);
	}

	get_code {
		// First register some code
		let code_size = 1024;
		let wasm_code = create_test_wasm_code(code_size);
		let api_version = (1, 0);
		let semver = (1, 0, 0);
		let allowed_from = 0u32.into();
		let code_hash = H256::from_slice(<T as frame_system::Config>::Hashing::hash(&wasm_code).as_ref());

		// Register the code first
		Pallet::<T>::register_code(RawOrigin::Root.into(), wasm_code, api_version, semver, allowed_from)?;
	}: {
		let _ = Pallet::<T>::get_code(code_hash);
	}
}
