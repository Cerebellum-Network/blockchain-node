//! Test utilities

#![allow(dead_code)]

use ddc_primitives::traits::staking::{StakingVisitor, StakingVisitorError};
use polkadot_sdk::frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU32, ConstU64},
};
use polkadot_sdk::frame_system::mocking::{MockBlock, MockUncheckedExtrinsic};
use polkadot_sdk::sp_io::TestExternalities;
use polkadot_sdk::sp_runtime::{traits::IdentityLookup, BuildStorage};

use crate::{self as pallet_ddc_nodes, *};

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Block = MockBlock<Test>;

construct_runtime!(
	pub enum Test
	{
		System: polkadot_sdk::frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Timestamp: polkadot_sdk::pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: polkadot_sdk::pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		DdcNodes: pallet_ddc_nodes::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
}

#[derive_impl(polkadot_sdk::frame_system::config_preludes::TestDefaultConfig)]
impl polkadot_sdk::frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = polkadot_sdk::pallet_balances::AccountData<Balance>;
}

impl polkadot_sdk::pallet_balances::Config for Test {
	type MaxLocks = ConstU32<1024>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type RuntimeFreezeReason = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type DoneSlashHandler = ();
}

impl polkadot_sdk::pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

impl crate::pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type StakingVisitor = TestStakingVisitor;
	type WeightInfo = ();
}

pub struct TestStakingVisitor;
impl<T: Config> StakingVisitor<T> for TestStakingVisitor {
	fn has_activated_stake(
		_node_pub_key: &NodePubKey,
		_cluster_id: &ClusterId,
	) -> Result<bool, StakingVisitorError> {
		Ok(false)
	}
	fn has_stake(_node_pub_key: &NodePubKey) -> bool {
		false
	}
	fn has_chilling_attempt(_node_pub_key: &NodePubKey) -> Result<bool, StakingVisitorError> {
		Ok(false)
	}
}

pub(crate) type TestRuntimeCall = <Test as polkadot_sdk::frame_system::Config>::RuntimeCall;

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> TestExternalities {
		polkadot_sdk::sp_tracing::try_init_simple();

		let mut t = polkadot_sdk::frame_system::GenesisConfig::<Test>::default()
			.build_storage()
			.unwrap();

		let _ = polkadot_sdk::pallet_balances::GenesisConfig::<Test> {
			balances: vec![(1, 100), (2, 100)],
			..Default::default()
		}
		.assimilate_storage(&mut t);

		TestExternalities::new(t)
	}
	pub fn build_and_execute(self, test: impl FnOnce()) {
		polkadot_sdk::sp_tracing::try_init_simple();
		let mut ext = self.build();
		ext.execute_with(test);
	}
}
