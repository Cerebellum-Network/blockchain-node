//! Test utilities

use ddc_primitives::{
	traits::cluster::{
		ClusterCreator, ClusterManager, ClusterManagerError, ClusterVisitor, ClusterVisitorError,
	},
	ClusterBondingParams, ClusterFeesParams, ClusterGovParams, ClusterId, ClusterParams,
	ClusterPricingParams, NodePubKey, NodeType,
};
use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU32, ConstU64, Everything},
	weights::constants::RocksDbWeight,
};
use frame_system::mocking::MockBlock;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, DispatchResult, Perquintill,
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
	type ClusterVisitor = TestClusterVisitor;
	type ClusterCreator = TestClusterCreator;
	type WeightInfo = ();
}

pub struct TestClusterVisitor;
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
	) -> Result<BlockNumberFor<T>, ClusterVisitorError> {
		Ok(BlockNumberFor::<T>::from(10u32))
	}
	fn get_unbonding_delay(
		_cluster_id: &ClusterId,
		_node_type: NodeType,
	) -> Result<BlockNumberFor<T>, ClusterVisitorError> {
		Ok(BlockNumberFor::<T>::from(10u32))
	}

	fn get_pricing_params(
		_cluster_id: &ClusterId,
	) -> Result<ClusterPricingParams, ClusterVisitorError> {
		Ok(ClusterPricingParams {
			unit_per_mb_stored: 1,
			unit_per_mb_streamed: 2,
			unit_per_put_request: 3,
			unit_per_get_request: 4,
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
	) -> Result<ClusterBondingParams<BlockNumberFor<T>>, ClusterVisitorError> {
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

pub struct TestClusterCreator;
impl<T: Config> ClusterCreator<T, Balance> for TestClusterCreator {
	fn create_new_cluster(
		_cluster_id: ClusterId,
		_cluster_manager_id: T::AccountId,
		_cluster_reserve_id: T::AccountId,
		_cluster_params: ClusterParams<T::AccountId>,
		_cluster_gov_params: ClusterGovParams<Balance, BlockNumberFor<T>>,
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
