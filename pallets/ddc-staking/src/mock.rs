//! Test utilities

#![allow(dead_code)]

use ddc_primitives::{
	ClusterNodeKind, ClusterNodeStatus, ClusterParams, ClusterProtocolParams, ClusterStatus,
	NodeParams, NodePubKey, StorageNodeParams, StorageNodePubKey,
};
use frame_support::{
	construct_runtime, derive_impl,
	traits::{ConstBool, ConstU32, ConstU64, Everything, Nothing},
	weights::constants::RocksDbWeight,
};
use frame_system::{
	mocking::{MockBlock, MockUncheckedExtrinsic},
	EnsureSigned,
};
use pallet_ddc_clusters::cluster::Cluster;
use pallet_ddc_nodes::StorageNode;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	traits::{BlakeTwo256, Convert, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature, Perbill, Perquintill,
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
		DdcNodes: pallet_ddc_nodes::{Pallet, Call, Storage, Event<T>},
		DdcClusters: pallet_ddc_clusters::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
	pub static ClusterBondingAmount: Balance = 50;
	pub static ClusterUnboningDelay: BlockNumber = 2;
}

impl Convert<Weight, BalanceOf<Self>> for Test {
	fn convert(w: Weight) -> BalanceOf<Self> {
		w.ref_time().into()
	}
}

type BalanceOf<T> = <<T as crate::pallet::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type DbWeight = RocksDbWeight;
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type Block = Block;
	type RuntimeCall = RuntimeCall;
	type RuntimeTask = RuntimeTask;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
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
	type RuntimeFreezeReason = ();
	type MaxFreezes = ();
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
	type UploadOrigin = EnsureSigned<AccountId>;
	type InstantiateOrigin = EnsureSigned<AccountId>;
	type Debug = ();
	type Environment = ();
	type Migrations = ();
	type ApiVersion = ();
	type Xcm = ();
}

impl pallet_insecure_randomness_collective_flip::Config for Test {}

impl pallet_ddc_nodes::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type StakingVisitor = pallet_ddc_staking::Pallet<Test>;
	type WeightInfo = ();
}

impl pallet_ddc_clusters::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type NodeRepository = pallet_ddc_nodes::Pallet<Test>;
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
	type ClusterCreator = pallet_ddc_clusters::Pallet<Test>;
	type NodeVisitor = pallet_ddc_nodes::Pallet<Test>;
	type NodeCreator = pallet_ddc_nodes::Pallet<Test>;
	type ClusterBondingAmount = ClusterBondingAmount;
	type ClusterUnboningDelay = ClusterUnboningDelay;
}

pub(crate) type DdcStakingCall = crate::Call<Test>;
pub(crate) type TestRuntimeCall = <Test as frame_system::Config>::RuntimeCall;

// (stash, controller, cluster)
#[allow(clippy::type_complexity)]
pub(crate) type BuiltClusterBond = (AccountId, AccountId, ClusterId);
// (stash, controller, node, stake, cluster)
#[allow(clippy::type_complexity)]
pub(crate) type BuiltNodeBond = (AccountId, AccountId, NodePubKey, Balance, ClusterId);

#[allow(clippy::type_complexity)]
pub(crate) type BuiltCluster = (Cluster<AccountId>, ClusterProtocolParams<Balance, BlockNumber>);

#[allow(clippy::type_complexity)]
pub(crate) type BuiltNode = (NodePubKey, StorageNode<Test>, Option<ClusterAssignment>);

pub(crate) struct ClusterAssignment {
	pub(crate) cluster_id: [u8; 20],
	pub(crate) status: ClusterNodeStatus,
	pub(crate) kind: ClusterNodeKind,
}

pub const NODE_KEY_1: [u8; 32] = [12; 32];
pub const NODE_STASH_1: [u8; 32] = [11; 32];
pub const NODE_CONTROLLER_1: [u8; 32] = [10; 32];

pub const NODE_KEY_2: [u8; 32] = [22; 32];
pub const NODE_STASH_2: [u8; 32] = [21; 32];
pub const NODE_CONTROLLER_2: [u8; 32] = [20; 32];

pub const NODE_KEY_3: [u8; 32] = [32; 32];
pub const NODE_STASH_3: [u8; 32] = [31; 32];
pub const NODE_CONTROLLER_3: [u8; 32] = [30; 32];

pub const NODE_KEY_4: [u8; 32] = [42; 32];
pub const NODE_STASH_4: [u8; 32] = [41; 32];
pub const NODE_CONTROLLER_4: [u8; 32] = [40; 32];

pub const CLUSTER_ID: [u8; 20] = [1; 20];
pub const CLUSTER_STASH: [u8; 32] = [102; 32];
pub const CLUSTER_CONTROLLER: [u8; 32] = [101; 32];

pub const USER_KEY_1: [u8; 32] = [1; 32];
pub const USER_KEY_2: [u8; 32] = [2; 32];
pub const USER_KEY_3: [u8; 32] = [3; 32];
pub const USER_KEY_4: [u8; 32] = [4; 32];

pub const NODE_KEY_5: [u8; 32] = [52; 32];
pub const NODE_KEY_6: [u8; 32] = [62; 32];

pub const ENDOWMENT: u128 = 100;

pub(crate) fn build_default_setup(
) -> (Vec<BuiltCluster>, Vec<BuiltNode>, Vec<BuiltClusterBond>, Vec<BuiltNodeBond>) {
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
			ClusterStatus::Activated,
		)],
		vec![
			build_node(
				NODE_KEY_1,
				NODE_CONTROLLER_1,
				StorageNodeParams::default(),
				Some(ClusterAssignment {
					cluster_id: CLUSTER_ID,
					status: ClusterNodeStatus::ValidationSucceeded,
					kind: ClusterNodeKind::Genesis,
				}),
			),
			build_node(
				NODE_KEY_2,
				NODE_CONTROLLER_2,
				StorageNodeParams::default(),
				Some(ClusterAssignment {
					cluster_id: CLUSTER_ID,
					status: ClusterNodeStatus::ValidationSucceeded,
					kind: ClusterNodeKind::Genesis,
				}),
			),
			build_node(
				NODE_KEY_3,
				NODE_CONTROLLER_3,
				StorageNodeParams::default(),
				Some(ClusterAssignment {
					cluster_id: CLUSTER_ID,
					status: ClusterNodeStatus::ValidationSucceeded,
					kind: ClusterNodeKind::Genesis,
				}),
			),
			build_node(
				NODE_KEY_4,
				NODE_CONTROLLER_4,
				StorageNodeParams::default(),
				Some(ClusterAssignment {
					cluster_id: CLUSTER_ID,
					status: ClusterNodeStatus::ValidationSucceeded,
					kind: ClusterNodeKind::Genesis,
				}),
			),
		],
		vec![build_cluster_bond(CLUSTER_STASH, CLUSTER_CONTROLLER, CLUSTER_ID)],
		vec![
			build_node_bond(NODE_STASH_1, NODE_CONTROLLER_1, NODE_KEY_1, ENDOWMENT, CLUSTER_ID),
			build_node_bond(NODE_STASH_2, NODE_CONTROLLER_2, NODE_KEY_2, ENDOWMENT, CLUSTER_ID),
			build_node_bond(NODE_STASH_3, NODE_CONTROLLER_3, NODE_KEY_3, ENDOWMENT, CLUSTER_ID),
			build_node_bond(NODE_STASH_4, NODE_CONTROLLER_4, NODE_KEY_4, ENDOWMENT, CLUSTER_ID),
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

pub fn build_node(
	pub_key: [u8; 32],
	provider_id: [u8; 32],
	params: StorageNodeParams,
	assignment: Option<ClusterAssignment>,
) -> BuiltNode {
	let key = NodePubKey::StoragePubKey(AccountId::from(pub_key));
	let mut node = StorageNode::new(
		key.clone(),
		AccountId::from(provider_id),
		NodeParams::StorageParams(params),
	)
	.unwrap();

	let cluster_id_opt = if let Some(ClusterAssignment { cluster_id, .. }) = assignment {
		Some(ClusterId::from(cluster_id))
	} else {
		None
	};
	node.cluster_id = cluster_id_opt;

	(key, node, assignment)
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

fn insert_unique_balance(
	account_vec: &mut Vec<(AccountId, Balance)>,
	account_id: AccountId,
	balance: Balance,
) {
	if !account_vec.iter().any(|(id, _)| *id == account_id) {
		account_vec.push((account_id, balance));
	}
}

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(
		self,
		clusts: Vec<BuiltCluster>,
		nodes: Vec<BuiltNode>,
		clusters_bonds: Vec<BuiltClusterBond>,
		nodes_bondes: Vec<BuiltNodeBond>,
	) -> TestExternalities {
		sp_tracing::try_init_simple();
		let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		let mut balances: Vec<(AccountId, Balance)> = vec![
			(AccountId::from(USER_KEY_1), ENDOWMENT),
			(AccountId::from(USER_KEY_2), ENDOWMENT),
			(AccountId::from(USER_KEY_3), ENDOWMENT),
			(AccountId::from(USER_KEY_4), ENDOWMENT),
		];

		let mut storage_nodes = Vec::new();
		let mut clusters_storage_nodes = Vec::new();
		for (pub_key, node, cluster_assignment) in nodes.iter() {
			storage_nodes.push(node.clone());
			if let Some(ClusterAssignment { cluster_id, kind, status }) = cluster_assignment {
				clusters_storage_nodes.push((
					ClusterId::from(cluster_id),
					vec![(pub_key.clone(), kind.clone(), status.clone())],
				));
			}
		}

		let mut clusters = Vec::new();
		let mut clusters_protocol_params = Vec::new();
		for (cluster, cluster_protocol_params) in clusts.iter() {
			clusters.push(cluster.clone());
			clusters_protocol_params.push((cluster.cluster_id, cluster_protocol_params.clone()));
		}

		for cluster in clusters.iter() {
			insert_unique_balance(&mut balances, cluster.manager_id.clone(), ENDOWMENT);
			insert_unique_balance(&mut balances, cluster.reserve_id.clone(), ENDOWMENT);
		}

		for (stash, controller, _) in clusters_bonds.iter() {
			insert_unique_balance(&mut balances, stash.clone(), ENDOWMENT);
			insert_unique_balance(&mut balances, controller.clone(), ENDOWMENT);
		}

		for (_, node, _) in nodes.iter() {
			insert_unique_balance(&mut balances, node.provider_id.clone(), ENDOWMENT);
		}

		for (stash, controller, _, _, _) in nodes_bondes.iter() {
			insert_unique_balance(&mut balances, stash.clone(), ENDOWMENT);
			insert_unique_balance(&mut balances, controller.clone(), ENDOWMENT);
		}

		let _ =
			pallet_balances::GenesisConfig::<Test> { balances }.assimilate_storage(&mut storage);

		let _ = pallet_ddc_nodes::GenesisConfig::<Test> { storage_nodes }
			.assimilate_storage(&mut storage);

		let _ = pallet_ddc_clusters::GenesisConfig::<Test> {
			clusters,
			clusters_protocol_params,
			clusters_nodes: clusters_storage_nodes,
		}
		.assimilate_storage(&mut storage);

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
		clusters_nodes: Vec<BuiltNode>,
		clusters_bonds: Vec<BuiltClusterBond>,
		nodes_bondes: Vec<BuiltNodeBond>,
		test: impl FnOnce(),
	) {
		sp_tracing::try_init_simple();
		let mut ext = self.build(clusters, clusters_nodes, clusters_bonds, nodes_bondes);
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
