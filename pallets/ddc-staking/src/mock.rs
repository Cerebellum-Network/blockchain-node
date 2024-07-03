//! Test utilities

#![allow(dead_code)]

use std::cell::RefCell;

use ddc_primitives::{
	traits::{
		cluster::{ClusterManager, ClusterProtocol},
		node::NodeVisitor,
		ClusterQuery,
	},
	ClusterBondingParams, ClusterFeesParams, ClusterNodeKind, ClusterNodeState, ClusterNodeStatus,
	ClusterNodesStats, ClusterParams, ClusterPricingParams, ClusterProtocolParams, ClusterStatus,
	NodeParams, NodePubKey, StorageNodePubKey,
};
use frame_support::{
	construct_runtime,
	traits::{ConstBool, ConstU32, ConstU64, Everything, Nothing},
	weights::constants::RocksDbWeight,
};
use frame_system::mocking::{MockBlock, MockUncheckedExtrinsic};
use lazy_static::lazy_static;
use pallet_ddc_clusters::cluster::Cluster;
use pallet_ddc_nodes::{Node, NodeRepository, NodeRepositoryError};
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	traits::{BlakeTwo256, Convert, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, DispatchError, DispatchResult, MultiSignature, Perbill, Perquintill,
};

use crate::{self as pallet_ddc_staking, *};

pub type Signature = MultiSignature;

/// The AccountId alias in this test module.
pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Block = MockBlock<Test>;

construct_runtime!(
	pub struct Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>, HoldReason},
		Randomness: pallet_insecure_randomness_collective_flip::{Pallet, Storage},
		DdcStaking: pallet_ddc_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
		DdcClusters: pallet_ddc_clusters::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
	pub static ClusterBondingAmount: Balance = 1;
	pub static ClusterUnboningDelay: BlockNumber = 1;
}

impl Convert<Weight, BalanceOf<Self>> for Test {
	fn convert(w: Weight) -> BalanceOf<Self> {
		w.ref_time().into()
	}
}

type BalanceOf<T> = <<T as crate::pallet::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = RocksDbWeight;
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type Block = Block;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
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
	type MaxHolds = ();
	type RuntimeHoldReason = RuntimeHoldReason;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

parameter_types! {
	pub const DepositPerItem: Balance = 0;
	pub const DepositPerByte: Balance = 0;
	pub const SignedClaimHandicap: BlockNumber = 2;
	pub const TombstoneDeposit: Balance = 16;
	pub const StorageSizeOffset: u32 = 8;
	pub const RentByteFee: Balance = 4;
	pub const RentDepositOffset: Balance = 10_000;
	pub const SurchargeReward: Balance = 150;
	pub const MaxDepth: u32 = 100;
	pub const MaxValueSize: u32 = 16_384;
	pub Schedule: pallet_contracts::Schedule<Test> = Default::default();
	pub static DefaultDepositLimit: Balance = 10_000_000;
	pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(0);
	pub const MaxDelegateDependencies: u32 = 32;
}

impl pallet_contracts::Config for Test {
	type Time = Timestamp;
	type Randomness = Randomness;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type CallStack = [pallet_contracts::Frame<Self>; 5];
	type WeightPrice = Self; //pallet_transaction_payment::Module<Self>;
	type WeightInfo = ();
	type ChainExtension = ();
	type Schedule = Schedule;
	type RuntimeCall = RuntimeCall;
	type CallFilter = Nothing;
	type DepositPerByte = DepositPerByte;
	type DepositPerItem = DepositPerItem;
	type DefaultDepositLimit = DefaultDepositLimit;
	type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
	type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
	type MaxStorageKeyLen = ConstU32<128>;
	type UnsafeUnstableInterface = ConstBool<false>;
	type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type MaxDelegateDependencies = MaxDelegateDependencies;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Debug = ();
	type Environment = ();
	type Migrations = ();
}

impl pallet_insecure_randomness_collective_flip::Config for Test {}

impl pallet_ddc_clusters::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type NodeRepository = MockNodeRepository;
	type StakingVisitor = pallet_ddc_staking::Pallet<Test>;
	type StakerCreator = pallet_ddc_staking::Pallet<Test>;
	type Currency = Balances;
	type WeightInfo = ();
	type MinErasureCodingRequiredLimit = ConstU32<0>;
	type MinErasureCodingTotalLimit = ConstU32<0>;
	type MinReplicationTotalLimit = ConstU32<0>;
}

impl crate::pallet::Config for Test {
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type ClusterProtocol = pallet_ddc_clusters::Pallet<Test>;
	type ClusterManager = pallet_ddc_clusters::Pallet<Test>;
	type NodeVisitor = MockNodeVisitor;
	type NodeCreator = TestNodeCreator;
	type ClusterCreator = TestClusterCreator;
	type ClusterBondingAmount = ClusterBondingAmount;
	type ClusterUnboningDelay = ClusterUnboningDelay;
}

pub(crate) type DdcStakingCall = crate::Call<Test>;
pub(crate) type TestRuntimeCall = <Test as frame_system::Config>::RuntimeCall;
pub struct TestNodeCreator;
pub struct TestClusterCreator;
pub struct TestClusterProtocol;

pub struct MockNodeRepository;
impl<T: Config> NodeRepository<T> for MockNodeRepository {
	fn create(node: Node<T>) -> Result<(), NodeRepositoryError> {
		unimplemented!()
	}
	fn get(node_pub_key: NodePubKey) -> Result<Node<T>, NodeRepositoryError> {
		unimplemented!()
	}
	fn update(node: Node<T>) -> Result<(), NodeRepositoryError> {
		unimplemented!()
	}
	fn delete(node_pub_key: NodePubKey) -> Result<(), NodeRepositoryError> {
		unimplemented!()
	}
}

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
	fn create_cluster(
		_cluster_id: ClusterId,
		_cluster_manager_id: T::AccountId,
		_cluster_reserve_id: T::AccountId,
		_cluster_params: ClusterParams<T::AccountId>,
		_cluster_protocol_params: ClusterProtocolParams<Balance, BlockNumberFor<T>>,
	) -> DispatchResult {
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
	pub node_provider_id: AccountId,
}

impl Default for MockNode {
	fn default() -> Self {
		Self { cluster_id: None, exists: true, node_provider_id: AccountId::from([128; 32]) }
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

impl<T: Config> NodeVisitor<T> for MockNodeVisitor
where
	<T as frame_system::Config>::AccountId: From<AccountId>,
{
	fn get_cluster_id(_node_pub_key: &NodePubKey) -> Result<Option<ClusterId>, DispatchError> {
		let lock = MOCK_NODE.lock();
		let mock_ref = lock.borrow();
		Ok(mock_ref.cluster_id)
	}
	fn exists(_node_pub_key: &NodePubKey) -> bool {
		let lock = MOCK_NODE.lock();
		let mock_ref = lock.borrow();
		mock_ref.exists
	}

	fn get_node_provider_id(_node_pub_key: &NodePubKey) -> Result<T::AccountId, DispatchError> {
		let lock = MOCK_NODE.lock();
		let mock_ref = lock.borrow();
		Ok(mock_ref.node_provider_id.clone().into())
	}
}

// (stash, controller, cluster)
#[allow(clippy::type_complexity)]
pub(crate) type BuiltClusterBond = (AccountId, AccountId, ClusterId);
// (stash, controller, node, stake, cluster)
#[allow(clippy::type_complexity)]
pub(crate) type BuiltNodeBond = (AccountId, AccountId, NodePubKey, Balance, ClusterId);

#[allow(clippy::type_complexity)]
pub(crate) type BuiltCluster = (Cluster<AccountId>, ClusterProtocolParams<Balance, BlockNumber>);

pub const KEY_1: [u8; 32] = [1; 32];
pub const KEY_2: [u8; 32] = [2; 32];
pub const KEY_3: [u8; 32] = [3; 32];
pub const KEY_4: [u8; 32] = [4; 32];

pub const NODE_PUB_KEY_1: [u8; 32] = [12; 32];
pub const NODE_STASH_1: [u8; 32] = [11; 32];
pub const NODE_CONTROLLER_1: [u8; 32] = [10; 32];

pub const NODE_PUB_KEY_2: [u8; 32] = [22; 32];
pub const NODE_STASH_2: [u8; 32] = [21; 32];
pub const NODE_CONTROLLER_2: [u8; 32] = [20; 32];

pub const NODE_PUB_KEY_3: [u8; 32] = [32; 32];
pub const NODE_STASH_3: [u8; 32] = [31; 32];
pub const NODE_CONTROLLER_3: [u8; 32] = [30; 32];

pub const NODE_PUB_KEY_4: [u8; 32] = [42; 32];
pub const NODE_STASH_4: [u8; 32] = [41; 32];
pub const NODE_CONTROLLER_4: [u8; 32] = [40; 32];

pub const CLUSTER_ID: [u8; 20] = [1; 20];
pub const CLUSTER_STASH: [u8; 32] = [102; 32];
pub const CLUSTER_CONTROLLER: [u8; 32] = [101; 32];

pub const ENDOWMENT: u128 = 100;

pub(crate) fn build_default_setup() -> (Vec<BuiltCluster>, Vec<BuiltClusterBond>, Vec<BuiltNodeBond>)
{
	(
		vec![build_cluster(
			CLUSTER_ID,
			CLUSTER_STASH,
			CLUSTER_CONTROLLER,
			ClusterParams::default(),
			ClusterProtocolParams {
				treasury_share: Perquintill::from_percent(1),
				validators_share: Perquintill::from_percent(10),
				cluster_reserve_share: Perquintill::from_percent(2),
				storage_bond_size: 10,
				storage_chill_delay: 10u32.into(),
				storage_unbonding_delay: 10u32.into(),
				unit_per_mb_stored: 2,
				unit_per_mb_streamed: 3,
				unit_per_put_request: 4,
				unit_per_get_request: 5,
			},
			ClusterStatus::Unbonded,
		)],
		vec![build_cluster_bond(CLUSTER_STASH, CLUSTER_CONTROLLER, CLUSTER_ID)],
		vec![
			build_node_bond(NODE_STASH_1, NODE_CONTROLLER_1, NODE_PUB_KEY_1, ENDOWMENT, CLUSTER_ID),
			build_node_bond(NODE_STASH_2, NODE_CONTROLLER_2, NODE_PUB_KEY_2, ENDOWMENT, CLUSTER_ID),
			build_node_bond(NODE_STASH_3, NODE_CONTROLLER_3, NODE_PUB_KEY_3, ENDOWMENT, CLUSTER_ID),
			build_node_bond(NODE_STASH_4, NODE_CONTROLLER_4, NODE_PUB_KEY_4, ENDOWMENT, CLUSTER_ID),
		],
	)
}

pub(crate) fn build_cluster(
	cluster_id: [u8; 20],
	manager_id: [u8; 32],
	reserve_id: [u8; 32],
	params: ClusterParams<AccountId>,
	protocol_params: ClusterProtocolParams<Balance, BlockNumber>,
	status: ClusterStatus,
) -> BuiltCluster {
	let mut cluster = Cluster::new(
		ClusterId::from(cluster_id),
		AccountId::from(manager_id),
		AccountId::from(reserve_id),
		params,
	);
	cluster.status = status;
	(cluster, protocol_params)
}

pub(crate) fn build_cluster_bond(
	stash: [u8; 32],
	controller: [u8; 32],
	cluster_id: [u8; 20],
) -> BuiltClusterBond {
	(AccountId::from(stash), AccountId::from(controller), ClusterId::from(cluster_id))
}

pub(crate) fn build_node_bond(
	stash: [u8; 32],
	controller: [u8; 32],
	node_key: [u8; 32],
	bond: Balance,
	cluster_id: [u8; 20],
) -> BuiltNodeBond {
	(
		AccountId::from(stash),
		AccountId::from(controller),
		NodePubKey::StoragePubKey(StorageNodePubKey::from(node_key)),
		bond,
		ClusterId::from(cluster_id),
	)
}

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(
		self,
		clusts: Vec<BuiltCluster>,
		clusters_bonds: Vec<BuiltClusterBond>,
		nodes_bondes: Vec<BuiltNodeBond>,
	) -> TestExternalities {
		sp_tracing::try_init_simple();
		let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		let mut balances: Vec<(AccountId, Balance)> = vec![
			(AccountId::from(KEY_1), ENDOWMENT),
			(AccountId::from(KEY_2), ENDOWMENT),
			(AccountId::from(KEY_3), ENDOWMENT),
			(AccountId::from(KEY_4), ENDOWMENT),
		];

		for (stash, controller, _) in clusters_bonds.iter() {
			balances.push((stash.clone(), ENDOWMENT));
			balances.push((controller.clone(), ENDOWMENT));
		}

		for (stash, controller, _, _, _) in nodes_bondes.iter() {
			balances.push((stash.clone(), ENDOWMENT));
			balances.push((controller.clone(), ENDOWMENT));
		}

		let mut clusters = Vec::new();
		let mut clusters_protocol_params = Vec::new();
		for (cluster, cluster_protocol_params) in clusts.iter() {
			clusters.push(cluster.clone());
			clusters_protocol_params.push((cluster.cluster_id, cluster_protocol_params.clone()));
		}

		let _ = pallet_ddc_clusters::GenesisConfig::<Test> {
			clusters,
			clusters_protocol_params,
			clusters_nodes: vec![],
		}
		.assimilate_storage(&mut storage);

		let _ =
			pallet_balances::GenesisConfig::<Test> { balances }.assimilate_storage(&mut storage);

		let _ = pallet_ddc_staking::GenesisConfig::<Test> {
			storages: nodes_bondes,
			clusters: clusters_bonds,
		}
		.assimilate_storage(&mut storage);

		TestExternalities::new(storage)
	}

	pub fn build_and_execute(
		self,
		clusters: Vec<BuiltCluster>,
		clusters_bonds: Vec<BuiltClusterBond>,
		nodes_bondes: Vec<BuiltNodeBond>,
		test: impl FnOnce(),
	) {
		sp_tracing::try_init_simple();
		let mut ext = self.build(clusters, clusters_bonds, nodes_bondes);
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
	let ledger = DdcStaking::ledger(controller.clone()).expect("Not a controller.");
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
