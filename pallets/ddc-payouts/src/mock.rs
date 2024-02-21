//! Test utilities

#![allow(dead_code)]

use ddc_primitives::{
	traits::{
		cluster::{ClusterCreator, ClusterVisitor, ClusterVisitorError},
		customer::{CustomerCharger, CustomerDepositor},
		pallet::PalletVisitor,
	},
	ClusterBondingParams, ClusterFeesParams, ClusterGovParams, ClusterParams, ClusterPricingParams,
	NodeType, DOLLARS,
};
use frame_election_provider_support::SortedListProvider;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU32, ConstU64, Everything, Randomness},
	weights::constants::RocksDbWeight,
	PalletId,
};
use frame_system::mocking::{MockBlock, MockUncheckedExtrinsic};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	traits::{BlakeTwo256, Identity, IdentityLookup},
	BuildStorage, DispatchError, Perquintill,
};
use sp_std::prelude::*;

use crate::{self as pallet_ddc_payouts, *};

/// The AccountId alias in this test module.
pub type AccountId = u128;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Block = MockBlock<Test>;
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

construct_runtime!(
	pub struct Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		DdcPayouts: pallet_ddc_payouts::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

pub static MAX_DUST: u16 = 100;

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
}

#[derive(Default, Clone)]
pub struct MockRandomness(H256);

impl Randomness<H256, BlockNumber> for MockRandomness {
	fn random(subject: &[u8]) -> (H256, BlockNumber) {
		let (mut r, b) = Self::random_seed();
		r.as_mut()[0..subject.len()].copy_from_slice(subject);
		(r, b)
	}

	fn random_seed() -> (H256, BlockNumber) {
		(H256::default(), BlockNumber::default())
	}
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
	type NominatorsAndValidatorsList = TestValidatorVisitor<Self>;
	type ClusterCreator = TestClusterCreator;

	type VoteScoreToU64 = Identity;
	type WeightInfo = ();
}

pub struct TestCustomerCharger;
impl<T: Config> CustomerCharger<T> for TestCustomerCharger {
	fn charge_content_owner(
		content_owner: T::AccountId,
		billing_vault: T::AccountId,
		amount: u128,
	) -> Result<u128, DispatchError> {
		let mut amount_to_charge = amount;
		let mut temp = ACCOUNT_ID_1.to_ne_bytes();
		let account_1 = T::AccountId::decode(&mut &temp[..]).unwrap();
		temp = ACCOUNT_ID_2.to_ne_bytes();
		let account_2 = T::AccountId::decode(&mut &temp[..]).unwrap();
		temp = ACCOUNT_ID_3.to_ne_bytes();
		let account_3 = T::AccountId::decode(&mut &temp[..]).unwrap();
		temp = ACCOUNT_ID_4.to_ne_bytes();
		let account_4 = T::AccountId::decode(&mut &temp[..]).unwrap();
		temp = ACCOUNT_ID_5.to_ne_bytes();
		let account_5 = T::AccountId::decode(&mut &temp[..]).unwrap();

		if content_owner == account_1 ||
			content_owner == account_3 ||
			content_owner == account_4 ||
			content_owner == account_5
		{
			ensure!(amount > 1_000_000, DispatchError::BadOrigin); //  any error will do
		}

		if amount_to_charge < 50_000_000 && content_owner == account_3 {
			assert!(PARTIAL_CHARGE < amount);
			amount_to_charge = PARTIAL_CHARGE; // for user 3
		}

		if content_owner == account_2 {
			assert!(USER2_BALANCE < amount);
			amount_to_charge = USER2_BALANCE; // for user 2
		}

		let charge = amount_to_charge.saturated_into::<BalanceOf<T>>();
		<T as pallet::Config>::Currency::transfer(
			&content_owner,
			&billing_vault,
			charge,
			ExistenceRequirement::AllowDeath,
		)?;
		Ok(amount_to_charge)
	}
}

pub const ACCOUNT_ID_1: AccountId = 1;
pub const ACCOUNT_ID_2: AccountId = 2;
pub const ACCOUNT_ID_3: AccountId = 3;
pub const ACCOUNT_ID_4: AccountId = 4;
pub const ACCOUNT_ID_5: AccountId = 5;
pub const ACCOUNT_ID_6: AccountId = 6;
pub const ACCOUNT_ID_7: AccountId = 7;
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

pub const VALIDATOR1_SCORE: u64 = 30;
pub const VALIDATOR2_SCORE: u64 = 45;
pub const VALIDATOR3_SCORE: u64 = 25;

pub const PARTIAL_CHARGE: u128 = 10;
pub const USER2_BALANCE: u128 = 5;
pub const USER3_BALANCE: u128 = 1000;

pub const NO_FEE_CLUSTER_ID: ClusterId = ClusterId::zero();
pub const ONE_CLUSTER_ID: ClusterId = ClusterId::repeat_byte(4u8);
pub const CERE_CLUSTER_ID: ClusterId = ClusterId::repeat_byte(10u8);
pub const HIGH_FEES_CLUSTER_ID: ClusterId = ClusterId::repeat_byte(5u8);
pub const GET_PUT_ZERO_CLUSTER_ID: ClusterId = ClusterId::repeat_byte(3u8);
pub const STORAGE_ZERO_CLUSTER_ID: ClusterId = ClusterId::repeat_byte(6u8);
pub const STREAM_ZERO_CLUSTER_ID: ClusterId = ClusterId::repeat_byte(7u8);
pub const PUT_ZERO_CLUSTER_ID: ClusterId = ClusterId::repeat_byte(8u8);
pub const GET_ZERO_CLUSTER_ID: ClusterId = ClusterId::repeat_byte(9u8);
pub const STORAGE_STREAM_ZERO_CLUSTER_ID: ClusterId = ClusterId::repeat_byte(11u8);
pub const PRICING_PARAMS: ClusterPricingParams = ClusterPricingParams {
	unit_per_mb_streamed: 2_000_000,
	unit_per_mb_stored: 3_000_000,
	unit_per_put_request: 4_000_000,
	unit_per_get_request: 5_000_000,
};

pub const PRICING_PARAMS_STREAM_ZERO: ClusterPricingParams = ClusterPricingParams {
	unit_per_mb_streamed: 0,
	unit_per_mb_stored: 3_000_000,
	unit_per_put_request: 4_000_000,
	unit_per_get_request: 5_000_000,
};

pub const PRICING_PARAMS_STORAGE_ZERO: ClusterPricingParams = ClusterPricingParams {
	unit_per_mb_streamed: 2_000_000,
	unit_per_mb_stored: 0,
	unit_per_put_request: 4_000_000,
	unit_per_get_request: 5_000_000,
};

pub const PRICING_PARAMS_GET_ZERO: ClusterPricingParams = ClusterPricingParams {
	unit_per_mb_streamed: 2_000_000,
	unit_per_mb_stored: 3_000_000,
	unit_per_put_request: 4_000_000,
	unit_per_get_request: 0,
};

pub const PRICING_PARAMS_PUT_ZERO: ClusterPricingParams = ClusterPricingParams {
	unit_per_mb_streamed: 2_000_000,
	unit_per_mb_stored: 3_000_000,
	unit_per_put_request: 0,
	unit_per_get_request: 5_000_000,
};

pub const PRICING_PARAMS_ONE: ClusterPricingParams = ClusterPricingParams {
	unit_per_mb_streamed: 10_000_000_000,
	unit_per_mb_stored: 10_000_000_000,
	unit_per_put_request: 10_000_000_000,
	unit_per_get_request: 10_000_000_000,
};

pub const PRICING_PARAMS_CERE: ClusterPricingParams = ClusterPricingParams {
	unit_per_mb_streamed: DOLLARS,
	unit_per_mb_stored: DOLLARS,
	unit_per_put_request: DOLLARS,
	unit_per_get_request: DOLLARS,
};

pub const PRICING_FEES: ClusterFeesParams = ClusterFeesParams {
	treasury_share: Perquintill::from_percent(1),
	validators_share: Perquintill::from_percent(10),
	cluster_reserve_share: Perquintill::from_percent(2),
};

pub const PRICING_FEES_HIGH: ClusterFeesParams = ClusterFeesParams {
	treasury_share: Perquintill::from_percent(10),
	validators_share: Perquintill::from_percent(20),
	cluster_reserve_share: Perquintill::from_percent(20),
};

pub const PRICING_FEES_ZERO: ClusterFeesParams = ClusterFeesParams {
	treasury_share: Perquintill::from_percent(0),
	validators_share: Perquintill::from_percent(0),
	cluster_reserve_share: Perquintill::from_percent(0),
};

pub struct TestTreasuryVisitor;
impl<T: frame_system::Config> PalletVisitor<T> for TestTreasuryVisitor {
	fn get_account_id() -> T::AccountId {
		let reserve_account = TREASURY_ACCOUNT_ID.to_ne_bytes();
		T::AccountId::decode(&mut &reserve_account[..]).unwrap()
	}
}

fn create_account_id_from_u128<T: frame_system::Config>(id: u128) -> T::AccountId {
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
				create_account_id_from_u128::<T>(VALIDATOR1_ACCOUNT_ID),
				create_account_id_from_u128::<T>(VALIDATOR2_ACCOUNT_ID),
				create_account_id_from_u128::<T>(VALIDATOR3_ACCOUNT_ID),
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
	fn get_score(validator_id: &T::AccountId) -> Result<Self::Score, Self::Error> {
		if *validator_id == create_account_id_from_u128::<T>(VALIDATOR1_ACCOUNT_ID) {
			Ok(VALIDATOR1_SCORE)
		} else if *validator_id == create_account_id_from_u128::<T>(VALIDATOR2_ACCOUNT_ID) {
			Ok(VALIDATOR2_SCORE)
		} else {
			Ok(VALIDATOR3_SCORE)
		}
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

	fn unsafe_clear() {
		unimplemented!()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn score_update_worst_case(_who: &T::AccountId, _is_increase: bool) -> Self::Score {
		unimplemented!()
	}
}

pub fn get_fees(cluster_id: &ClusterId) -> ClusterFeesParams {
	if *cluster_id == NO_FEE_CLUSTER_ID ||
		*cluster_id == ONE_CLUSTER_ID ||
		*cluster_id == CERE_CLUSTER_ID
	{
		PRICING_FEES_ZERO
	} else if *cluster_id == HIGH_FEES_CLUSTER_ID {
		PRICING_FEES_HIGH
	} else {
		PRICING_FEES
	}
}

pub fn get_pricing(cluster_id: &ClusterId) -> ClusterPricingParams {
	if *cluster_id == ONE_CLUSTER_ID || *cluster_id == NO_FEE_CLUSTER_ID {
		PRICING_PARAMS_ONE
	} else if *cluster_id == CERE_CLUSTER_ID {
		PRICING_PARAMS_CERE
	} else if *cluster_id == STORAGE_ZERO_CLUSTER_ID {
		PRICING_PARAMS_STORAGE_ZERO
	} else if *cluster_id == STREAM_ZERO_CLUSTER_ID {
		PRICING_PARAMS_STREAM_ZERO
	} else if *cluster_id == PUT_ZERO_CLUSTER_ID {
		PRICING_PARAMS_PUT_ZERO
	} else if *cluster_id == GET_ZERO_CLUSTER_ID {
		PRICING_PARAMS_GET_ZERO
	} else {
		PRICING_PARAMS
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
		cluster_id: &ClusterId,
	) -> Result<ClusterPricingParams, ClusterVisitorError> {
		Ok(get_pricing(cluster_id))
	}

	fn get_fees_params(cluster_id: &ClusterId) -> Result<ClusterFeesParams, ClusterVisitorError> {
		Ok(get_fees(cluster_id))
	}

	fn get_reserve_account_id(
		_cluster_id: &ClusterId,
	) -> Result<T::AccountId, ClusterVisitorError> {
		let reserve_account = RESERVE_ACCOUNT_ID.to_ne_bytes();
		Ok(T::AccountId::decode(&mut &reserve_account[..]).unwrap())
	}

	fn get_bonding_params(
		_cluster_id: &ClusterId,
	) -> Result<ClusterBondingParams<BlockNumberFor<T>>, ClusterVisitorError> {
		unimplemented!()
	}
}

pub(crate) type TestRuntimeCall = <Test as frame_system::Config>::RuntimeCall;

pub struct ExtBuilder;

impl ExtBuilder {
	fn build(self) -> TestExternalities {
		sp_tracing::try_init_simple();

		let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		let _balance_genesis = pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 10000000000000000000000000000),
				(2, USER2_BALANCE), // < PARTIAL_CHARGE
				(3, USER3_BALANCE), // > PARTIAL_CHARGE
				(4, 1000000000000000000000000),
				(5, 1000000000000000000000000),
				(6, 1000000000000000000000000),
				(7, 1000000000000000000000000),
			],
		}
		.assimilate_storage(&mut storage);

		let _payout_genesis = pallet_ddc_payouts::GenesisConfig::<Test> {
			feeder_account: None,
			debtor_customers: Default::default(),
			authorised_caller: None,
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
