//! Test utilities

#![allow(dead_code)]

use ddc_primitives::traits::staking::{StakingVisitor, StakingVisitorError};
use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU32, ConstU64},
};
use frame_system::mocking::{MockBlock, MockUncheckedExtrinsic};
use sp_io::TestExternalities;
use sp_runtime::{
	traits::{IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature,
};

use crate::{self as pallet_ddc_nodes, *};

/// The AccountId alias in this test module.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub type Signature = MultiSignature;

type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Block = MockBlock<Test>;

construct_runtime!(
	pub enum Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		DdcNodes: pallet_ddc_nodes::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<Balance>;
}

impl pallet_balances::Config for Test {
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
}

impl pallet_timestamp::Config for Test {
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

	fn stash_by_ctrl(_controller: &T::AccountId) -> Result<T::AccountId, StakingVisitorError> {
		todo!()
	}
}

pub(crate) type TestRuntimeCall = <Test as frame_system::Config>::RuntimeCall;

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> TestExternalities {
		sp_tracing::try_init_simple();

		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		let account_id1 = AccountId::from([1; 32]);
		let account_id2 = AccountId::from([2; 32]);
		let _ = pallet_balances::GenesisConfig::<Test> {
			balances: vec![(account_id1, 100), (account_id2, 100)],
		}
		.assimilate_storage(&mut t);

		TestExternalities::new(t)
	}
	pub fn build_and_execute(self, test: impl FnOnce()) {
		sp_tracing::try_init_simple();
		let mut ext = self.build();
		ext.execute_with(test);
	}
}
