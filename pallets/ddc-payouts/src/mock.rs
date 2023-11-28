//! Test utilities

#![allow(dead_code)]

use ddc_primitives::{
	ClusterBondingParams, ClusterFeesParams, ClusterGovParams, ClusterParams, ClusterPricingParams,
	NodeType,
};
use ddc_traits::{
	cluster::{ClusterCreator, ClusterVisitor, ClusterVisitorError},
	customer::{CustomerCharger, CustomerDepositor},
	pallet::PalletVisitor,
};
use frame_election_provider_support::SortedListProvider;
use frame_support::{
	construct_runtime,
	dispatch::DispatchError,
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
use sp_std::prelude::*;

use crate::{self as pallet_ddc_payouts, *};

/// The AccountId alias in this test module.
pub type AccountId = u64;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Block = MockBlock<Test>;
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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
	type CustomerDepositor = TestCustomerDepositor;
	type ClusterVisitor = TestClusterVisitor;
	type TreasuryVisitor = TestTreasuryVisitor;
	type ValidatorList = TestValidatorVisitor<Self>;
	type ClusterCreator = TestClusterCreator;
	type WeightInfo = ();
}

pub struct TestCustomerCharger;
impl<T: Config> CustomerCharger<T> for TestCustomerCharger {
	fn charge_content_owner(
		content_owner: T::AccountId,
		billing_vault: T::AccountId,
		amount: u128,
	) -> Result<u128, DispatchError> {
		ensure!(amount > 1_000_000, DispatchError::BadOrigin); //  any error will do

		let mut amount_to_charge = amount;
		if amount_to_charge < 50_000_000 {
			amount_to_charge = PARTIAL_CHARGE; // for user 3
		}

		let charge = amount_to_charge.saturated_into::<BalanceOf<T>>();

		<T as pallet::Config>::Currency::transfer(
			&content_owner,
			&billing_vault,
			charge,
			ExistenceRequirement::KeepAlive,
		)?;
		Ok(amount_to_charge)
	}
}

pub struct TestClusterCreator;
impl<T: Config> ClusterCreator<T, Balance> for TestClusterCreator {
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

pub struct TestCustomerDepositor;
impl<T: Config> CustomerDepositor<T> for TestCustomerDepositor {
	fn deposit(_customer: T::AccountId, _amount: u128) -> Result<(), DispatchError> {
		Ok(())
	}
	fn deposit_extra(_customer: T::AccountId, _amount: u128) -> Result<(), DispatchError> {
		Ok(())
	}
}

pub const RESERVE_ACCOUNT_ID: AccountId = 999;
pub const TREASURY_ACCOUNT_ID: AccountId = 888;
pub const VALIDATOR1_ACCOUNT_ID: AccountId = 111;
pub const VALIDATOR2_ACCOUNT_ID: AccountId = 222;
pub const VALIDATOR3_ACCOUNT_ID: AccountId = 333;
pub const PARTIAL_CHARGE: u128 = 100;
pub const USER3_BALANCE: u128 = 1000;

pub const FREE_CLUSTER_ID: ClusterId = ClusterId::zero();

pub const PRICING_PARAMS: ClusterPricingParams = ClusterPricingParams {
	unit_per_mb_streamed: 2_000_000,
	unit_per_mb_stored: 3_000_000,
	unit_per_put_request: 4_000_000,
	unit_per_get_request: 5_000_000,
};

pub const PRICING_FEES: ClusterFeesParams = ClusterFeesParams {
	treasury_share: Perbill::from_percent(1),
	validators_share: Perbill::from_percent(10),
	cluster_reserve_share: Perbill::from_percent(2),
};

pub const PRICING_FEES_ZERO: ClusterFeesParams = ClusterFeesParams {
	treasury_share: Perbill::from_percent(0),
	validators_share: Perbill::from_percent(0),
	cluster_reserve_share: Perbill::from_percent(0),
};

pub struct TestTreasuryVisitor;
impl<T: frame_system::Config> PalletVisitor<T> for TestTreasuryVisitor {
	fn get_account_id() -> T::AccountId {
		let reserve_account = TREASURY_ACCOUNT_ID.to_ne_bytes();
		T::AccountId::decode(&mut &reserve_account[..]).unwrap()
	}
}

fn create_account_id_from_u64<T: frame_system::Config>(id: u64) -> T::AccountId {
	let bytes = id.to_ne_bytes();
	T::AccountId::decode(&mut &bytes[..]).unwrap()
}

pub struct TestValidatorVisitor<T>(sp_std::marker::PhantomData<T>);
impl<T: frame_system::Config> SortedListProvider<T::AccountId> for TestValidatorVisitor<T> {
	type Score = u64;
	type Error = ();

	/// Returns iterator over voter list, which can have `take` called on it.
	fn iter() -> Box<dyn Iterator<Item = T::AccountId>> {
		Box::new(
			vec![
				create_account_id_from_u64::<T>(VALIDATOR1_ACCOUNT_ID),
				create_account_id_from_u64::<T>(VALIDATOR2_ACCOUNT_ID),
				create_account_id_from_u64::<T>(VALIDATOR3_ACCOUNT_ID),
			]
			.into_iter(),
		)
	}
	fn iter_from(
		_start: &T::AccountId,
	) -> Result<Box<dyn Iterator<Item = T::AccountId>>, Self::Error> {
		unimplemented!()
	}
	fn count() -> u32 {
		3
	}
	fn contains(_id: &T::AccountId) -> bool {
		unimplemented!()
	}
	fn on_insert(_: T::AccountId, _weight: Self::Score) -> Result<(), Self::Error> {
		// nothing to do on insert.
		Ok(())
	}
	fn get_score(_id: &T::AccountId) -> Result<Self::Score, Self::Error> {
		unimplemented!()
	}
	fn on_update(_: &T::AccountId, _weight: Self::Score) -> Result<(), Self::Error> {
		// nothing to do on update.
		Ok(())
	}
	fn on_remove(_: &T::AccountId) -> Result<(), Self::Error> {
		// nothing to do on remove.
		Ok(())
	}
	fn unsafe_regenerate(
		_: impl IntoIterator<Item = T::AccountId>,
		_: Box<dyn Fn(&T::AccountId) -> Self::Score>,
	) -> u32 {
		// nothing to do upon regenerate.
		0
	}

	fn try_state() -> Result<(), &'static str> {
		unimplemented!()
	}

	fn unsafe_clear() {
		unimplemented!()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn score_update_worst_case(_who: &T::AccountId, _is_increase: bool) -> Self::Score {
		unimplemented!()
	}
}

pub fn get_fees(cluster_id: &ClusterId) -> Result<ClusterFeesParams, ClusterVisitorError> {
	if *cluster_id == FREE_CLUSTER_ID {
		Ok(PRICING_FEES_ZERO)
	} else {
		Ok(PRICING_FEES)
	}
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

	fn get_fees_params(cluster_id: &ClusterId) -> Result<ClusterFeesParams, ClusterVisitorError> {
		get_fees(cluster_id)
	}

	fn get_reserve_account_id(
		_cluster_id: &ClusterId,
	) -> Result<T::AccountId, ClusterVisitorError> {
		let reserve_account = RESERVE_ACCOUNT_ID.to_ne_bytes();
		Ok(T::AccountId::decode(&mut &reserve_account[..]).unwrap())
	}

	fn get_bonding_params(
		_cluster_id: &ClusterId,
	) -> Result<ClusterBondingParams<T::BlockNumber>, ClusterVisitorError> {
		unimplemented!()
	}
}

pub(crate) type TestRuntimeCall = <Test as frame_system::Config>::RuntimeCall;

pub struct ExtBuilder;

impl ExtBuilder {
	fn build(self) -> TestExternalities {
		sp_tracing::try_init_simple();
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let _ = pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 1000000000000000000000000),
				(2, 10),            // < PARTIAL_CHARGE
				(3, USER3_BALANCE), // > PARTIAL_CHARGE
				(4, 1000000000000000000000000),
			],
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
