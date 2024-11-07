#![cfg(test)]

use frame_support::{derive_impl, ord_parameter_types, parameter_types, weights::Weight};
use frame_system::{self as system};
pub use pallet_balances as balances;
use pallet_chainbridge as bridge;
use sp_core::hashing::blake2_128;
use sp_runtime::{testing::Header, BuildStorage, Perbill};

use crate::{self as erc721, Config};

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = Weight::from_parts(1024, 0);
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = u64;
	type AccountData = balances::AccountData<u64>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

ord_parameter_types! {
	pub const One: u64 = 1;
}

impl pallet_balances::Config for Test {
	type Balance = u64;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = ();
	type FreezeIdentifier = ();
	type RuntimeFreezeReason = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
}

parameter_types! {
	pub Erc721Id: bridge::ResourceId = bridge::derive_resource_id(1, &blake2_128(b"NFT"));
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Identifier = Erc721Id;
	type WeightInfo = ();
}

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, u64, RuntimeCall, ()>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: system::{Pallet, Call, Event<T>},
		Balances: balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Erc721: erc721::{Pallet, Call, Storage, Event<T>},
	}
);

pub const USER_A: u64 = 0x1;
pub const USER_B: u64 = 0x2;
pub const USER_C: u64 = 0x3;
pub const ENDOWED_BALANCE: u64 = 100_000_000;

pub fn new_test_ext() -> sp_io::TestExternalities {
	RuntimeGenesisConfig {
		balances: balances::GenesisConfig { balances: vec![(USER_A, ENDOWED_BALANCE)] },
	}
	.build_storage()
	.unwrap()
	.into()
}
