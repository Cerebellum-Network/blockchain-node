use frame_support::{
	parameter_types,
	traits::{ConstU32, Everything},
	PalletId,
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

use crate as pallet_fee_handler;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		FeeHandler: pallet_fee_handler,
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
	type RuntimeTask = ();
	type Nonce = u64;
	type Block = Block;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type ExtensionsWeightInfo = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type Balance = u128;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type FreezeIdentifier = [u8; 8];
	type MaxFreezes = ConstU32<1>;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const FeeHandlerPalletId: PalletId = PalletId(*b"py/feehd");
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
}

impl pallet_fee_handler::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type GovernanceOrigin = frame_system::EnsureRoot<u64>;
	type PalletId = FeeHandlerPalletId;
	type TreasuryPalletId = TreasuryPalletId;
	type WeightInfo = pallet_fee_handler::weights::SubstrateWeight<Test>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

#[cfg(test)]
mod tests {
	use frame_support::{
		assert_ok, parameter_types,
		traits::{
			fungible::{Inspect, Mutate},
			ConstU16, ConstU32, ConstU64,
		},
	};
	use sp_core::H256;
	use sp_runtime::{
		traits::{BlakeTwo256, IdentityLookup},
	};
	use sp_std::vec::Vec;

	use super::*;

	type Block = frame_system::mocking::MockBlock<Test>;

	frame_support::construct_runtime!(
		pub enum Test {
			System: frame_system,
			Balances: pallet_balances,
			FeeHandler: pallet_fee_handler,
		}
	);

	impl frame_system::Config for Test {
		type BaseCallFilter = frame_support::traits::Everything;
		type BlockWeights = ();
		type BlockLength = ();
		type DbWeight = ();
		type RuntimeOrigin = RuntimeOrigin;
		type RuntimeCall = RuntimeCall;
		type RuntimeEvent = RuntimeEvent;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Block = Block;
		type RuntimeTask = ();
		type BlockHashCount = ConstU64<250>;
		type Version = ();
		type PalletInfo = PalletInfo;
		type AccountData = pallet_balances::AccountData<u64>;
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type SystemWeightInfo = ();
		type SS58Prefix = ConstU16<42>;
		type OnSetCode = ();
		type MaxConsumers = ConstU32<16>;
		type Nonce = u64;
		type ExtensionsWeightInfo = ();
		type SingleBlockMigrations = ();
		type MultiBlockMigrator = ();
		type PreInherents = ();
		type PostInherents = ();
		type PostTransactions = ();
	}

	parameter_types! {
		pub const ExistentialDeposit: u64 = 1;
	}

	impl pallet_balances::Config for Test {
		type Balance = u64;
		type DustRemoval = ();
		type RuntimeEvent = RuntimeEvent;
		type ExistentialDeposit = ExistentialDeposit;
		type AccountStore = System;
		type WeightInfo = ();
		type MaxLocks = ();
		type MaxReserves = ConstU32<50>;
		type ReserveIdentifier = [u8; 8];
		type RuntimeHoldReason = RuntimeHoldReason;
		type RuntimeFreezeReason = RuntimeFreezeReason;
		type FreezeIdentifier = ();
		type MaxFreezes = ConstU32<0>;
		type DoneSlashHandler = ();
	}

	parameter_types! {
		pub const FeeHandlerPalletId: PalletId = PalletId(*b"fee/hand");
		pub const TreasuryPalletId: PalletId = PalletId(*b"treasury");
	}

	impl pallet_fee_handler::Config for Test {
		type RuntimeEvent = RuntimeEvent;
		type Currency = Balances;
		type GovernanceOrigin = frame_system::EnsureRoot<u64>;
		type PalletId = FeeHandlerPalletId;
		type TreasuryPalletId = TreasuryPalletId;
		type WeightInfo = crate::weights::SubstrateWeight<Test>;
	}

	pub fn new_test_ext() -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| {
			System::set_block_number(1);
		});
		ext
	}

	#[test]
	fn test_manual_topup() {
		new_test_ext().execute_with(|| {
			let user = 1u64;
			let amount = 100u64;
			Balances::set_balance(&user, amount * 2);
			assert_ok!(FeeHandler::manual_topup(RuntimeOrigin::signed(user), amount as u128));
			assert_eq!(Balances::balance(&user), amount);
		});
	}

	#[test]
	fn test_burn_native_tokens() {
		new_test_ext().execute_with(|| {
			let user = 1u64;
			let amount = 100u64;
			Balances::set_balance(&user, amount * 2);
			assert_ok!(FeeHandler::burn_native_tokens(RuntimeOrigin::root(), user, amount as u128));
			assert_eq!(Balances::balance(&user), amount);
		});
	}
}
