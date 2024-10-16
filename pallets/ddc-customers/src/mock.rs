//! Test utilities

use ddc_primitives::{
	traits::cluster::{ClusterCreator, ClusterManager, ClusterProtocol, ClusterQuery},
	ClusterBondingParams, ClusterFeesParams, ClusterId, ClusterNodeKind, ClusterNodeState,
	ClusterNodeStatus, ClusterNodesStats, ClusterParams, ClusterPricingParams,
	ClusterProtocolParams, ClusterStatus, NodePubKey, NodeType,
};
use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU32, ConstU64, Everything},
	weights::constants::RocksDbWeight,
};
use frame_system::mocking::MockBlock;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, DispatchError, DispatchResult, Perquintill,
};

use crate::{self as pallet_ddc_customers, *};

/// The AccountId alias in this test module.
pub(crate) type AccountId = u128;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type Block = MockBlock<Test>;

construct_runtime!(
	pub struct Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		DdcCustomers: pallet_ddc_customers::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
}

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
	type RuntimeHoldReason = ();
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

impl crate::pallet::Config for Test {
	type UnlockingDelay = UnlockingDelay;
	type Currency = Balances;
	type PalletId = DdcCustomersPalletId;
	type RuntimeEvent = RuntimeEvent;
	type ClusterProtocol = TestClusterProtocol;
	type ClusterCreator = TestClusterCreator;
	type WeightInfo = ();
}

pub struct TestClusterProtocol;
impl<T: Config> ClusterQuery<T> for TestClusterProtocol {
	fn cluster_exists(_cluster_id: &ClusterId) -> bool {
		true
	}

	fn get_cluster_status(_cluster_id: &ClusterId) -> Result<ClusterStatus, DispatchError> {
		unimplemented!()
	}

	fn get_manager_and_reserve_id(
		_cluster_id: &ClusterId,
	) -> Result<(T::AccountId, T::AccountId), DispatchError> {
		unimplemented!()
	}
}

impl<T: Config> ClusterProtocol<T, BalanceOf<T>> for TestClusterProtocol {
	fn get_bond_size(_cluster_id: &ClusterId, _node_type: NodeType) -> Result<u128, DispatchError> {
		Ok(10)
	}

	fn get_chill_delay(
		_cluster_id: &ClusterId,
		_node_type: NodeType,
	) -> Result<BlockNumberFor<T>, DispatchError> {
		Ok(BlockNumberFor::<T>::from(10u32))
	}

	fn get_unbonding_delay(
		_cluster_id: &ClusterId,
		_node_type: NodeType,
	) -> Result<BlockNumberFor<T>, DispatchError> {
		Ok(BlockNumberFor::<T>::from(10u32))
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
	) -> Result<ClusterBondingParams<BlockNumberFor<T>>, DispatchError> {
		Ok(ClusterBondingParams {
			storage_bond_size:
				<TestClusterProtocol as ClusterProtocol<T, BalanceOf<T>>>::get_bond_size(
					cluster_id,
					NodeType::Storage,
				)
				.unwrap_or_default(),
			storage_chill_delay:
				<TestClusterProtocol as ClusterProtocol<T, BalanceOf<T>>>::get_chill_delay(
					cluster_id,
					NodeType::Storage,
				)
				.unwrap_or_default(),
			storage_unbonding_delay:
				<TestClusterProtocol as ClusterProtocol<T, BalanceOf<T>>>::get_unbonding_delay(
					cluster_id,
					NodeType::Storage,
				)
				.unwrap_or_default(),
		})
	}

	fn get_reserve_account_id(_cluster_id: &ClusterId) -> Result<T::AccountId, DispatchError> {
		unimplemented!()
	}

	fn activate_cluster_protocol(_cluster_id: &ClusterId) -> DispatchResult {
		unimplemented!()
	}

	fn update_cluster_protocol(
		_cluster_id: &ClusterId,
		_cluster_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>,
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

pub struct TestClusterManager;
impl<T: Config> ClusterQuery<T> for TestClusterManager {
	fn cluster_exists(_cluster_id: &ClusterId) -> bool {
		true
	}

	fn get_cluster_status(_cluster_id: &ClusterId) -> Result<ClusterStatus, DispatchError> {
		unimplemented!()
	}

	fn get_manager_and_reserve_id(
		_cluster_id: &ClusterId,
	) -> Result<(T::AccountId, T::AccountId), DispatchError> {
		unimplemented!()
	}
}

impl<T: Config> ClusterManager<T> for TestClusterManager {
	fn get_manager_account_id(_cluster_id: &ClusterId) -> Result<T::AccountId, DispatchError> {
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
	) -> Result<ClusterNodeState<BlockNumberFor<T>>, DispatchError> {
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
}

pub struct TestClusterCreator;
impl<T: Config> ClusterCreator<T, Balance> for TestClusterCreator {
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

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> TestExternalities {
		sp_tracing::try_init_simple();

		let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		let _balance_genesis = pallet_balances::GenesisConfig::<Test> {
			balances: vec![(1, 100), (2, 100), (3, 1000)],
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
