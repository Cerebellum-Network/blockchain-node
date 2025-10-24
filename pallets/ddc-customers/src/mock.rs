//! Test utilities

use ddc_primitives::{
	traits::cluster::{ClusterCreator, ClusterManager, ClusterProtocol, ClusterQuery},
	ClusterBondingParams, ClusterFeesParams, ClusterId, ClusterNodeKind, ClusterNodeState,
	ClusterNodeStatus, ClusterNodesStats, ClusterParams, ClusterPricingParams,
	ClusterProtocolParams, ClusterStatus, InspectionDryRunParams, NodePubKey, NodeType,
};
use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstBool, ConstU32, ConstU64, Nothing},
};

use frame_system::{mocking::MockBlock, EnsureSigned};
use sp_io::TestExternalities;
use sp_runtime::{
	traits::{Convert, IdentityLookup},
	BuildStorage, DispatchError, DispatchResult, Perbill, Perquintill, Weight,
};

use crate::{self as pallet_ddc_customers, *};
use pallet_contracts as contracts;

/// The AccountId alias in this test module.
pub(crate) type AccountId = u128;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type Block = MockBlock<Test>;

construct_runtime!(
	pub enum Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances,
		DdcCustomers: pallet_ddc_customers::{Pallet, Call, Storage, Config<T>, Event<T>},
		Contracts: contracts::{Pallet, Call, Storage, Event<T>, HoldReason},
		Randomness: pallet_insecure_randomness_collective_flip::{Pallet, Storage},
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
	type BlockHashCount = ConstU64<250>;
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
	type DoneSlashHandler = ();
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

parameter_types! {
	pub const DdcCustomersPalletId: PalletId = PalletId(*b"accounts"); // DDC maintainer's stake
	pub const UnlockingDelay: BlockNumber = 10u64; // 10 blocks for test
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

impl Convert<Weight, BalanceOf<Self>> for Test {
	fn convert(w: Weight) -> BalanceOf<Self> {
		w.ref_time().into()
	}
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
	type MaxTransientStorageSize = ();
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

impl crate::pallet::Config for Test {
	type UnlockingDelay = UnlockingDelay;
	type Currency = Balances;
	type PalletId = DdcCustomersPalletId;
	type RuntimeEvent = RuntimeEvent;
	type ClusterProtocol = TestClusterProtocol;
	type ClusterCreator = TestClusterCreator;
	type WeightInfo = ();
	type ContractMigrator = TestContractMigrator;
}

pub struct TestClusterProtocol;
impl ClusterQuery<AccountId> for TestClusterProtocol {
	fn cluster_exists(_cluster_id: &ClusterId) -> bool {
		true
	}

	fn get_cluster_status(_cluster_id: &ClusterId) -> Result<ClusterStatus, DispatchError> {
		unimplemented!()
	}

	fn get_manager_and_reserve_id(
		_cluster_id: &ClusterId,
	) -> Result<(AccountId, AccountId), DispatchError> {
		unimplemented!()
	}
}

impl ClusterProtocol<AccountId, BlockNumber, Balance> for TestClusterProtocol {
	fn get_bond_size(_cluster_id: &ClusterId, _node_type: NodeType) -> Result<u128, DispatchError> {
		Ok(10)
	}

	fn get_chill_delay(
		_cluster_id: &ClusterId,
		_node_type: NodeType,
	) -> Result<BlockNumber, DispatchError> {
		Ok(10)
	}

	fn get_unbonding_delay(
		_cluster_id: &ClusterId,
		_node_type: NodeType,
	) -> Result<BlockNumber, DispatchError> {
		Ok(10)
	}

	fn get_pricing_params(_cluster_id: &ClusterId) -> Result<ClusterPricingParams, DispatchError> {
		Ok(ClusterPricingParams {
			unit_per_mb_stored: 1,
			unit_per_mb_streamed: 2,
			unit_per_put_request: 3,
			unit_per_get_request: 4,
		})
	}

	fn get_fees_params(_cluster_id: &ClusterId) -> Result<ClusterFeesParams, DispatchError> {
		Ok(ClusterFeesParams {
			treasury_share: Perquintill::from_percent(1),
			validators_share: Perquintill::from_percent(10),
			cluster_reserve_share: Perquintill::from_percent(2),
		})
	}

	fn get_bonding_params(
		cluster_id: &ClusterId,
	) -> Result<ClusterBondingParams<BlockNumber>, DispatchError> {
		Ok(ClusterBondingParams {
			storage_bond_size: <TestClusterProtocol as ClusterProtocol<
				AccountId,
				BlockNumber,
				Balance,
			>>::get_bond_size(cluster_id, NodeType::Storage)
			.unwrap_or_default(),
			storage_chill_delay: <TestClusterProtocol as ClusterProtocol<
				AccountId,
				BlockNumber,
				Balance,
			>>::get_chill_delay(cluster_id, NodeType::Storage)
			.unwrap_or_default(),
			storage_unbonding_delay: <TestClusterProtocol as ClusterProtocol<
				AccountId,
				BlockNumber,
				Balance,
			>>::get_unbonding_delay(cluster_id, NodeType::Storage)
			.unwrap_or_default(),
		})
	}

	fn get_reserve_account_id(_cluster_id: &ClusterId) -> Result<AccountId, DispatchError> {
		unimplemented!()
	}

	fn get_customer_deposit_contract(_cluster_id: &ClusterId) -> Result<AccountId, DispatchError> {
		unimplemented!()
	}

	fn activate_cluster_protocol(_cluster_id: &ClusterId) -> DispatchResult {
		unimplemented!()
	}

	fn update_cluster_protocol(
		_cluster_id: &ClusterId,
		_cluster_protocol_params: ClusterProtocolParams<Balance, BlockNumber, AccountId>,
	) -> DispatchResult {
		unimplemented!()
	}

	fn bond_cluster(_cluster_id: &ClusterId) -> DispatchResult {
		unimplemented!()
	}

	fn start_unbond_cluster(_cluster_id: &ClusterId) -> DispatchResult {
		unimplemented!()
	}

	fn end_unbond_cluster(_cluster_id: &ClusterId) -> DispatchResult {
		unimplemented!()
	}
}

#[allow(dead_code)]
pub struct TestClusterManager;
impl ClusterQuery<AccountId> for TestClusterManager {
	fn cluster_exists(_cluster_id: &ClusterId) -> bool {
		true
	}

	fn get_cluster_status(_cluster_id: &ClusterId) -> Result<ClusterStatus, DispatchError> {
		unimplemented!()
	}

	fn get_manager_and_reserve_id(
		_cluster_id: &ClusterId,
	) -> Result<(AccountId, AccountId), DispatchError> {
		unimplemented!()
	}
}

impl ClusterManager<AccountId, BlockNumber> for TestClusterManager {
	fn get_manager_account_id(_cluster_id: &ClusterId) -> Result<AccountId, DispatchError> {
		unimplemented!()
	}

	fn get_nodes(_cluster_id: &ClusterId) -> Result<Vec<NodePubKey>, DispatchError> {
		unimplemented!()
	}

	fn contains_node(
		_cluster_id: &ClusterId,
		_node_pub_key: &NodePubKey,
		_validation_status: Option<ClusterNodeStatus>,
	) -> bool {
		true
	}

	fn add_node(
		_cluster_id: &ClusterId,
		_node_pub_key: &NodePubKey,
		_node_kind: &ClusterNodeKind,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	fn remove_node(
		_cluster_id: &ClusterId,
		_node_pub_key: &NodePubKey,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	fn get_node_state(
		_cluster_id: &ClusterId,
		_node_pub_key: &NodePubKey,
	) -> Result<ClusterNodeState<BlockNumber>, DispatchError> {
		unimplemented!()
	}

	fn get_nodes_stats(_cluster_id: &ClusterId) -> Result<ClusterNodesStats, DispatchError> {
		unimplemented!()
	}

	fn validate_node(
		_cluster_id: &ClusterId,
		_node_pub_key: &NodePubKey,
		_succeeded: bool,
	) -> Result<(), DispatchError> {
		unimplemented!()
	}

	fn get_clusters(_status: ClusterStatus) -> Result<Vec<ClusterId>, DispatchError> {
		unimplemented!()
	}

	fn get_inspection_dry_run_params(
		_cluster_id: &ClusterId,
	) -> Result<Option<InspectionDryRunParams>, DispatchError> {
		unimplemented!()
	}
}

pub struct TestClusterCreator;
impl ClusterCreator<AccountId, BlockNumber, Balance> for TestClusterCreator {
	fn create_cluster(
		_cluster_id: ClusterId,
		_cluster_manager_id: AccountId,
		_cluster_reserve_id: AccountId,
		_cluster_params: ClusterParams<AccountId>,
		_cluster_protocol_params: ClusterProtocolParams<Balance, BlockNumber, AccountId>,
	) -> DispatchResult {
		Ok(())
	}
}

pub struct TestContractMigrator;
impl ContractMigrator<AccountId, Balance> for TestContractMigrator {
	fn deploy_contract(
		_deployer: AccountId,
		_value: Balance,
		_gas_limit: Weight,
		_storage_deposit_limit: Option<Balance>,
		_code: Vec<u8>,
		_data: Vec<u8>,
		_salt: Vec<u8>,
	) -> DispatchResult {
		Ok(())
	}

	fn call_contract(
		_caller: AccountId,
		_dest: AccountId,
		_value: Balance,
		_gas_limit: Weight,
		_storage_deposit_limit: Option<Balance>,
		_data: Vec<u8>,
	) -> DispatchResult {
		Ok(())
	}
}

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> TestExternalities {
		sp_tracing::try_init_simple();

		let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		let _balance_genesis = pallet_balances::GenesisConfig::<Test> {
			balances: vec![(1, 100), (2, 100), (3, 1000), (99, 1_000_000)],
			..Default::default()
		}
		.assimilate_storage(&mut storage);

		let _customer_genesis = pallet_ddc_customers::GenesisConfig::<Test> {
			feeder_account: None,
			buckets: Default::default(),
		}
		.assimilate_storage(&mut storage);

		TestExternalities::new(storage)
	}
	pub fn build_and_execute(self, test: impl FnOnce()) {
		sp_tracing::try_init_simple();
		let mut ext = self.build();
		ext.execute_with(test);
	}
}
