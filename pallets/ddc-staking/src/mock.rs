//! Test utilities

#![allow(dead_code)]

use std::cell::RefCell;

use ddc_primitives::{
	traits::{
		cluster::{ClusterManager, ClusterManagerError, ClusterVisitor, ClusterVisitorError},
		node::{NodeVisitor, NodeVisitorError},
	},
	ClusterBondingParams, ClusterFeesParams, ClusterGovParams, ClusterParams, ClusterPricingParams,
	NodeParams, NodePubKey, StorageNodePubKey,
};
use frame_support::{
	construct_runtime,
	dispatch::DispatchResult,
	traits::{ConstU32, ConstU64, Everything, GenesisBuild},
	weights::constants::RocksDbWeight,
};
use frame_system::mocking::{MockBlock, MockUncheckedExtrinsic};
use lazy_static::lazy_static;
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perquintill,
};
use sp_std::collections::btree_map::BTreeMap;

use crate::{self as pallet_ddc_staking, *};

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
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type HoldIdentifier = ();
	type MaxHolds = ();
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

impl crate::pallet::Config for Test {
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type ClusterVisitor = TestClusterVisitor;
	type ClusterManager = TestClusterManager;
	type NodeVisitor = MockNodeVisitor;
	type NodeCreator = TestNodeCreator;
	type ClusterCreator = TestClusterCreator;
}

pub(crate) type DdcStakingCall = crate::Call<Test>;
pub(crate) type TestRuntimeCall = <Test as frame_system::Config>::RuntimeCall;
pub struct TestNodeCreator;
pub struct TestClusterCreator;
pub struct TestClusterVisitor;

impl<T: Config> NodeCreator<T> for TestNodeCreator {
	fn create_node(
		_node_pub_key: NodePubKey,
		_provider_id: T::AccountId,
		_node_params: NodeParams,
	) -> DispatchResult {
		Ok(())
	}
}

impl<T: Config> ClusterCreator<T, u128> for TestClusterCreator {
	fn create_new_cluster(
		_cluster_id: ClusterId,
		_cluster_manager_id: T::AccountId,
		_cluster_reserve_id: T::AccountId,
		_cluster_params: ClusterParams<T::AccountId>,
		_cluster_gov_params: ClusterGovParams<Balance, T::BlockNumber>,
	) -> DispatchResult {
		Ok(())
	}
}

impl<T: Config> ClusterVisitor<T> for TestClusterVisitor {
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
		Ok(ClusterPricingParams {
			unit_per_mb_stored: 2,
			unit_per_mb_streamed: 3,
			unit_per_put_request: 4,
			unit_per_get_request: 5,
		})
	}

	fn get_fees_params(_cluster_id: &ClusterId) -> Result<ClusterFeesParams, ClusterVisitorError> {
		Ok(ClusterFeesParams {
			treasury_share: Perquintill::from_percent(1),
			validators_share: Perquintill::from_percent(10),
			cluster_reserve_share: Perquintill::from_percent(2),
		})
	}

	fn get_reserve_account_id(
		_cluster_id: &ClusterId,
	) -> Result<T::AccountId, ClusterVisitorError> {
		Err(ClusterVisitorError::ClusterDoesNotExist)
	}

	fn get_bonding_params(
		cluster_id: &ClusterId,
	) -> Result<ClusterBondingParams<T::BlockNumber>, ClusterVisitorError> {
		Ok(ClusterBondingParams {
			storage_bond_size: <TestClusterVisitor as ClusterVisitor<T>>::get_bond_size(
				cluster_id,
				NodeType::Storage,
			)
			.unwrap_or_default(),
			storage_chill_delay: <TestClusterVisitor as ClusterVisitor<T>>::get_chill_delay(
				cluster_id,
				NodeType::Storage,
			)
			.unwrap_or_default(),
			storage_unbonding_delay:
				<TestClusterVisitor as ClusterVisitor<T>>::get_unbonding_delay(
					cluster_id,
					NodeType::Storage,
				)
				.unwrap_or_default(),
		})
	}
}

pub struct TestClusterManager;
impl<T: Config> ClusterManager<T> for TestClusterManager {
	fn contains_node(_cluster_id: &ClusterId, _node_pub_key: &NodePubKey) -> bool {
		true
	}

	fn add_node(
		_cluster_id: &ClusterId,
		_node_pub_key: &NodePubKey,
	) -> Result<(), ClusterManagerError> {
		Ok(())
	}

	fn remove_node(
		_cluster_id: &ClusterId,
		_node_pub_key: &NodePubKey,
	) -> Result<(), ClusterManagerError> {
		Ok(())
	}
}

lazy_static! {
	// We have to use the ReentrantMutex as every test's thread that needs to perform some configuration on the mock acquires the lock at least 2 times:
	// the first time when the mock configuration happens, and
	// the second time when the pallet calls the MockNodeVisitor during execution
	static ref MOCK_NODE: ReentrantMutex<RefCell<MockNode>> =
		ReentrantMutex::new(RefCell::new(MockNode::default()));
}

pub struct MockNode {
	pub cluster_id: Option<ClusterId>,
	pub exists: bool,
}

impl Default for MockNode {
	fn default() -> Self {
		Self { cluster_id: None, exists: true }
	}
}

pub struct MockNodeVisitor;

impl MockNodeVisitor {
	// Every test's thread must hold the lock till the end of its test
	pub fn set_and_hold_lock(mock: MockNode) -> ReentrantMutexGuard<'static, RefCell<MockNode>> {
		let lock = MOCK_NODE.lock();
		*lock.borrow_mut() = mock;
		lock
	}

	// Every test's thread must release the lock that it previously acquired in the end of its
	// test
	pub fn reset_and_release_lock(lock: ReentrantMutexGuard<'static, RefCell<MockNode>>) {
		*lock.borrow_mut() = MockNode::default();
	}
}

impl<T: Config> NodeVisitor<T> for MockNodeVisitor {
	fn get_cluster_id(_node_pub_key: &NodePubKey) -> Result<Option<ClusterId>, NodeVisitorError> {
		let lock = MOCK_NODE.lock();
		let mock_ref = lock.borrow();
		Ok(mock_ref.cluster_id)
	}
	fn exists(_node_pub_key: &NodePubKey) -> bool {
		let lock = MOCK_NODE.lock();
		let mock_ref = lock.borrow();
		mock_ref.exists
	}
}

pub struct ExtBuilder {
	has_storages: bool,
	stakes: BTreeMap<AccountId, Balance>,
	storages: Vec<(AccountId, AccountId, Balance, ClusterId)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { has_storages: true, stakes: Default::default(), storages: Default::default() }
	}
}

impl ExtBuilder {
	pub fn has_storages(mut self, has: bool) -> Self {
		self.has_storages = has;
		self
	}
	pub fn set_stake(mut self, who: AccountId, stake: Balance) -> Self {
		self.stakes.insert(who, stake);
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
	pub fn build(self) -> TestExternalities {
		sp_tracing::try_init_simple();
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let _ = pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 100),
				(2, 100),
				(3, 100),
				(4, 100),
				// storage controllers
				(10, 100),
				(20, 100),
				(30, 100),
				(40, 100),
				// storage stashes
				(11, 100),
				(21, 100),
				(31, 100),
				(41, 100),
			],
		}
		.assimilate_storage(&mut storage);
		let mut storages = vec![];
		if self.has_storages {
			storages = vec![
				// (stash, controller, node, stake, cluster)
				(
					11,
					10,
					NodePubKey::StoragePubKey(StorageNodePubKey::new([12; 32])),
					100,
					ClusterId::from([1; 20]),
				),
				(
					21,
					20,
					NodePubKey::StoragePubKey(StorageNodePubKey::new([22; 32])),
					100,
					ClusterId::from([1; 20]),
				),
				(
					31,
					30,
					NodePubKey::StoragePubKey(StorageNodePubKey::new([32; 32])),
					100,
					ClusterId::from([1; 20]),
				),
				(
					41,
					40,
					NodePubKey::StoragePubKey(StorageNodePubKey::new([42; 32])),
					100,
					ClusterId::from([1; 20]),
				),
			];
		}

		let _ =
			pallet_ddc_staking::GenesisConfig::<Test> { storages }.assimilate_storage(&mut storage);

		TestExternalities::new(storage)
	}
	pub fn build_and_execute(self, test: impl FnOnce()) {
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
