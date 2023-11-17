//! Test utilities

#![allow(dead_code)]

use crate::{self as pallet_ddc_payouts, *};
use ddc_primitives::{ClusterPricingParams, NodePubKey, NodeType};
use ddc_traits::{
	cluster::{ClusterVisitor, ClusterVisitorError},
	customer::CustomerCharger,
};

use frame_support::{
	construct_runtime,
	error::BadOrigin,
	parameter_types,
	traits::{ConstU32, ConstU64, Everything},
	weights::constants::RocksDbWeight,
	PalletId,
};
use frame_system::mocking::{MockBlock, MockUncheckedExtrinsic};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

/// The AccountId alias in this test module.
pub type AccountId = u64;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Block = MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		DdcPayouts: pallet_ddc_payouts::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = RocksDbWeight;
	type RuntimeOrigin = RuntimeOrigin;
	type Index = AccountIndex;
	type BlockNumber = BlockNumber;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
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
}

parameter_types! {
	pub const PayoutsPalletId: PalletId = PalletId(*b"payouts_");
}

impl crate::pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = PayoutsPalletId;
	type Currency = Balances;
	type CustomerCharger = TestCustomerCharger;
	type ClusterVisitor = TestClusterVisitor;
}

pub struct TestCustomerCharger;
impl<T: Config> CustomerCharger<T> for TestCustomerCharger {
	fn charge_content_owner(
		_content_owner: T::AccountId,
		_billing_vault: T::AccountId,
		amount: u128,
	) -> sp_runtime::DispatchResult {
		ensure!(amount > 1_000_000_000, BadOrigin); //  any error will do
		Ok(())
	}
}

pub const PRICING_PARAMS: ClusterPricingParams = ClusterPricingParams {
	unit_per_mb_streamed: 2_000_000,
	unit_per_mb_stored: 3_000_000,
	unit_per_put_request: 4_000_000,
	unit_per_get_request: 5_000_000,
};

pub struct TestClusterVisitor;
impl<T: Config> ClusterVisitor<T> for TestClusterVisitor {
	fn cluster_has_node(_cluster_id: &ClusterId, _node_pub_key: &NodePubKey) -> bool {
		true
	}
	fn ensure_cluster(_cluster_id: &ClusterId) -> Result<(), ClusterVisitorError> {
		Ok(())
	}
	fn get_bond_size(
		_cluster_id: &ClusterId,
		_node_type: NodeType,
	) -> Result<u128, ClusterVisitorError> {
		Ok(10)
	}
	fn get_chill_delay(
		_cluster_id: &ClusterId,
		_node_type: NodeType,
	) -> Result<T::BlockNumber, ClusterVisitorError> {
		Ok(T::BlockNumber::from(10u32))
	}
	fn get_unbonding_delay(
		_cluster_id: &ClusterId,
		_node_type: NodeType,
	) -> Result<T::BlockNumber, ClusterVisitorError> {
		Ok(T::BlockNumber::from(10u32))
	}

	fn get_pricing_params(
		_cluster_id: &ClusterId,
	) -> Result<ClusterPricingParams, ClusterVisitorError> {
		Ok(PRICING_PARAMS)
	}
}

pub(crate) type TestRuntimeCall = <Test as frame_system::Config>::RuntimeCall;

pub struct ExtBuilder;

impl ExtBuilder {
	fn build(self) -> TestExternalities {
		sp_tracing::try_init_simple();
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let _ = pallet_balances::GenesisConfig::<Test> { balances: vec![(1, 100), (2, 100)] }
			.assimilate_storage(&mut storage);

		TestExternalities::new(storage)
	}
	pub fn build_and_execute(self, test: impl FnOnce()) {
		sp_tracing::try_init_simple();
		let mut ext = self.build();
		ext.execute_with(test);
	}
}
