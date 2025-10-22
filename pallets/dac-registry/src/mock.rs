//! Mock runtime for testing the DAC Registry pallet

use crate as pallet_dac_registry;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU32, Everything},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		DacRegistry: pallet_dac_registry,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type RuntimeTask = ();
	type ExtensionsWeightInfo = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

parameter_types! {
	pub const MaxCodeSize: u32 = 1024 * 1024; // 1MB
}

impl pallet_dac_registry::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type GovernanceOrigin = frame_system::EnsureRoot<u64>;
	type MaxCodeSize = MaxCodeSize;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = system::GenesisConfig::<Test>::default().build_storage().unwrap();
	t.into()
}

// Mock governance origin for testing
#[allow(dead_code)]
pub struct MockGovernanceOrigin;

impl frame_support::traits::EnsureOrigin<RuntimeOrigin> for MockGovernanceOrigin {
	type Success = ();

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		// In tests, we'll use root origin as governance
		frame_system::EnsureRoot::<u64>::try_origin(o)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

// Helper functions for testing
pub mod test_utils {
	use super::*;
	use sp_core::H256;

	pub fn create_test_code() -> Vec<u8> {
		b"test_wasm_code".to_vec()
	}

	pub fn create_test_code_hash() -> H256 {
		let code = create_test_code();
		let hash = sp_io::hashing::blake2_256(&code);
		H256::from_slice(&hash)
	}

	#[allow(dead_code)]
	pub fn create_test_metadata() -> pallet_dac_registry::CodeMeta {
		pallet_dac_registry::CodeMeta {
			api_version: (1, 0),
			semver: (1, 0, 0),
			allowed_from: 10,
			length: 14,
		}
	}

	pub fn create_large_test_code(size: usize) -> Vec<u8> {
		vec![0u8; size]
	}
}
