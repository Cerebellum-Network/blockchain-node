//! Test utilities

#![allow(dead_code)]

use crate::{self as pallet_ddc_staking, *};
use frame_support::{
	construct_runtime,
	traits::{ConstU32, ConstU64, Everything, GenesisBuild},
	weights::constants::RocksDbWeight,
};
use frame_system::mocking::{MockBlock, MockUncheckedExtrinsic};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::collections::btree_map::BTreeMap;

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
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
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		DdcStaking: pallet_ddc_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
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

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

parameter_types! {
	pub const BondingDuration: EraIndex = 10;
	pub const DefaultEdgeBondSize: Balance = 100;
	pub const DefaultEdgeChillDelay: EraIndex = 1;
	pub const DefaultStorageBondSize: Balance = 100;
	pub const DefaultStorageChillDelay: EraIndex = 1;
}

impl crate::pallet::Config for Test {
	type BondingDuration = BondingDuration;
	type Currency = Balances;
	type DefaultEdgeBondSize = DefaultEdgeBondSize;
	type DefaultEdgeChillDelay = DefaultEdgeChillDelay;
	type DefaultStorageBondSize = DefaultStorageBondSize;
	type DefaultStorageChillDelay = DefaultStorageChillDelay;
	type RuntimeEvent = RuntimeEvent;
	type UnixTime = Timestamp;
	type WeightInfo = ();
}

pub(crate) type DdcStakingCall = crate::Call<Test>;
pub(crate) type TestRuntimeCall = <Test as frame_system::Config>::RuntimeCall;

pub struct ExtBuilder {
	has_edges: bool,
	has_storages: bool,
	stakes: BTreeMap<AccountId, Balance>,
	edges: Vec<(AccountId, AccountId, Balance, ClusterId)>,
	storages: Vec<(AccountId, AccountId, Balance, ClusterId)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			has_edges: true,
			has_storages: true,
			stakes: Default::default(),
			edges: Default::default(),
			storages: Default::default(),
		}
	}
}

impl ExtBuilder {
	pub fn has_edges(mut self, has: bool) -> Self {
		self.has_edges = has;
		self
	}
	pub fn has_storages(mut self, has: bool) -> Self {
		self.has_storages = has;
		self
	}
	pub fn set_stake(mut self, who: AccountId, stake: Balance) -> Self {
		self.stakes.insert(who, stake);
		self
	}
	pub fn add_edge(
		mut self,
		stash: AccountId,
		controller: AccountId,
		stake: Balance,
		cluster: ClusterId,
	) -> Self {
		self.edges.push((stash, controller, stake, cluster));
		self
	}
	pub fn add_storage(
		mut self,
		stash: AccountId,
		controller: AccountId,
		stake: Balance,
		cluster: ClusterId,
	) -> Self {
		self.storages.push((stash, controller, stake, cluster));
		self
	}
	fn build(self) -> TestExternalities {
		sp_tracing::try_init_simple();
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let _ = pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 100),
				(2, 100),
				(3, 100),
				(4, 100),
				// edge controllers
				(10, 100),
				(20, 100),
				// storage controllers
				(30, 100),
				(40, 100),
				// edge stashes
				(11, 100),
				(21, 100),
				// storage stashes
				(31, 100),
				(41, 100),
			],
		}
		.assimilate_storage(&mut storage);
		let mut edges = vec![];
		if self.has_edges {
			edges = vec![
				// (stash, controller, stake, cluster)
				(11, 10, 100, 1),
				(21, 20, 100, 1),
			];
		}
		let mut storages = vec![];
		if self.has_storages {
			storages = vec![
				// (stash, controller, stake, cluster)
				(31, 30, 100, 1),
				(41, 40, 100, 1),
			];
		}

		let _ = pallet_ddc_staking::GenesisConfig::<Test> { edges, storages, ..Default::default() }
			.assimilate_storage(&mut storage);

		TestExternalities::new(storage)
	}
	pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
		sp_tracing::try_init_simple();
		let mut ext = self.build();
		ext.execute_with(test);
		ext.execute_with(post_condition);
	}
}

fn post_condition() {
	check_ledgers();
}

fn check_ledgers() {
	// check the ledger of all stakers.
	Bonded::<Test>::iter().for_each(|(_, controller)| assert_ledger_consistent(controller))
}

fn assert_ledger_consistent(controller: AccountId) {
	// ensures ledger.total == ledger.active + sum(ledger.unlocking).
	let ledger = DdcStaking::ledger(controller).expect("Not a controller.");
	let real_total: Balance = ledger.unlocking.iter().fold(ledger.active, |a, c| a + c.value);
	assert_eq!(real_total, ledger.total);
	assert!(
		ledger.active >= Balances::minimum_balance() || ledger.active == 0,
		"{}: active ledger amount ({}) must be greater than ED {}",
		controller,
		ledger.active,
		Balances::minimum_balance()
	);
}
