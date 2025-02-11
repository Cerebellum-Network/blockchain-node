//! Test utilities

#![allow(dead_code)]

#[cfg(feature = "runtime-benchmarks")]
use ddc_primitives::BucketParams;
use ddc_primitives::{
	traits::{
		bucket::BucketManager, cluster::ClusterProtocol, customer::CustomerCharger,
		node::NodeManager, pallet::PalletVisitor, ClusterQuery, ValidatorVisitor,
	},
	BucketUsage, ClusterBondingParams, ClusterFeesParams, ClusterPricingParams,
	ClusterProtocolParams, ClusterStatus, NodeParams, NodePubKey, NodeType, DOLLARS,
};
use frame_election_provider_support::SortedListProvider;
use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU32, ConstU64, ExistenceRequirement, Randomness},
	PalletId,
};
use frame_system::mocking::{MockBlock, MockUncheckedExtrinsic};
use sp_core::H256;
use sp_io::TestExternalities;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
use sp_runtime::{
	traits::{BlakeTwo256, IdentifyAccount, Identity, IdentityLookup, Verify},
	AccountId32, BuildStorage, DispatchError, MultiSignature, Perquintill,
};
use sp_std::prelude::*;

use crate::{self as pallet_ddc_payouts, *};

pub type Signature = MultiSignature;
/// The AccountId alias in this test module.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Block = MockBlock<Test>;
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

construct_runtime!(
	pub enum Test
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
	type RuntimeHoldReason = ();
}

parameter_types! {
	pub const PayoutsPalletId: PalletId = PalletId(*b"payouts_");
	pub const MajorityOfValidators: Percent = Percent::from_percent(67);
}

impl crate::pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = PayoutsPalletId;
	type Currency = Balances;
	type CustomerCharger = TestCustomerCharger;
	type BucketManager = TestBucketManager;
	type ClusterProtocol = TestClusterProtocol;
	type TreasuryVisitor = TestTreasuryVisitor;
	type NominatorsAndValidatorsList = TestValidatorVisitor<Self>;
	type VoteScoreToU64 = Identity;
	type ValidatorVisitor = MockValidatorVisitor;
	type NodeManager = MockNodeManager;
	type AccountIdConverter = AccountId;
	type Hasher = BlakeTwo256;
	type ClusterValidator = MockClusterValidator;
	type ValidatorsQuorum = MajorityOfValidators;
}

pub struct MockClusterValidator;
impl<T: Config> ClusterValidator<T> for MockClusterValidator {
	fn set_last_paid_era(_cluster_id: &ClusterId, _era_id: DdcEra) -> Result<(), DispatchError> {
		unimplemented!()
	}

	fn get_last_paid_era(_cluster_id: &ClusterId) -> Result<DdcEra, DispatchError> {
		Ok(Default::default())
	}
}

pub struct MockNodeManager;
impl<T: Config> NodeManager<T> for MockNodeManager
where
	<T as frame_system::Config>::AccountId: From<AccountId>,
{
	fn get_cluster_id(_node_pub_key: &NodePubKey) -> Result<Option<ClusterId>, DispatchError> {
		unimplemented!()
	}

	fn exists(_node_pub_key: &NodePubKey) -> bool {
		unimplemented!()
	}

	fn update_total_node_usage(
		_node_key: &NodePubKey,
		_payable_usage: &NodeUsage,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	fn get_node_provider_id(pub_key: &NodePubKey) -> Result<T::AccountId, DispatchError> {
		match pub_key {
			NodePubKey::StoragePubKey(key) if key == &NODE1_PUB_KEY_32 =>
				Ok(NODE_PROVIDER1_KEY_32.clone().into()),
			NodePubKey::StoragePubKey(key) if key == &NODE2_PUB_KEY_32 =>
				Ok(NODE_PROVIDER2_KEY_32.clone().into()),
			NodePubKey::StoragePubKey(key) if key == &NODE3_PUB_KEY_32 =>
				Ok(NODE_PROVIDER3_KEY_32.clone().into()),
			NodePubKey::StoragePubKey(key) if key == &NODE4_PUB_KEY_32 =>
				Ok(NODE_PROVIDER4_KEY_32.clone().into()),
			NodePubKey::StoragePubKey(key) if key == &NODE5_PUB_KEY_32 =>
				Ok(NODE_PROVIDER5_KEY_32.clone().into()),
			NodePubKey::StoragePubKey(key) if key == &NODE6_PUB_KEY_32 =>
				Ok(NODE_PROVIDER6_KEY_32.clone().into()),
			NodePubKey::StoragePubKey(key) if key == &NODE7_PUB_KEY_32 =>
				Ok(NODE_PROVIDER7_KEY_32.clone().into()),
			NodePubKey::StoragePubKey(key) if key == &NODE8_PUB_KEY_32 =>
				Ok(NODE_PROVIDER8_KEY_32.clone().into()),
			NodePubKey::StoragePubKey(key) if key == &NODE9_PUB_KEY_32 =>
				Ok(NODE_PROVIDER9_KEY_32.clone().into()),
			NodePubKey::StoragePubKey(key) if key == &NODE10_PUB_KEY_32 =>
				Ok(NODE_PROVIDER10_KEY_32.clone().into()),

			_ => Err(DispatchError::Other("Unexpected node pub_key")),
		}
	}

	fn get_node_params(_node_pub_key: &NodePubKey) -> Result<NodeParams, DispatchError> {
		unimplemented!()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn create_node(
		_node_pub_key: NodePubKey,
		_provider_id: T::AccountId,
		_node_params: NodeParams,
	) -> DispatchResult {
		unimplemented!()
	}
}

pub struct MockValidatorVisitor;
impl<T: Config> ValidatorVisitor<T> for MockValidatorVisitor
where
	<T as frame_system::Config>::AccountId: From<AccountId>,
{
	fn is_ocw_validator(caller: T::AccountId) -> bool {
		caller == VALIDATOR_OCW_KEY_32.into()
	}
	fn is_quorum_reached(_quorum: Percent, _members_count: usize) -> bool {
		true
	}
}

pub struct TestBucketManager;
impl<T: Config> BucketManager<T> for TestBucketManager
where
	<T as frame_system::Config>::AccountId: From<AccountId>,
{
	fn get_bucket_owner_id(bucket_id: BucketId) -> Result<T::AccountId, DispatchError> {
		match bucket_id {
			BUCKET_ID1 => Ok(CUSTOMER1_KEY_32.clone().into()),
			BUCKET_ID2 => Ok(CUSTOMER2_KEY_32.clone().into()),
			BUCKET_ID3 => Ok(CUSTOMER3_KEY_32.clone().into()),
			BUCKET_ID4 => Ok(CUSTOMER4_KEY_32.clone().into()),
			BUCKET_ID5 => Ok(CUSTOMER5_KEY_32.clone().into()),
			BUCKET_ID6 => Ok(CUSTOMER6_KEY_32.clone().into()),
			BUCKET_ID7 => Ok(CUSTOMER7_KEY_32.clone().into()),

			BUCKET_ID100 => Ok(CUSTOMER100_KEY_32.clone().into()),
			BUCKET_ID101 => Ok(CUSTOMER101_KEY_32.clone().into()),
			BUCKET_ID102 => Ok(CUSTOMER102_KEY_32.clone().into()),
			BUCKET_ID103 => Ok(CUSTOMER103_KEY_32.clone().into()),
			BUCKET_ID104 => Ok(CUSTOMER104_KEY_32.clone().into()),
			BUCKET_ID105 => Ok(CUSTOMER105_KEY_32.clone().into()),
			BUCKET_ID106 => Ok(CUSTOMER106_KEY_32.clone().into()),
			BUCKET_ID107 => Ok(CUSTOMER107_KEY_32.clone().into()),
			BUCKET_ID108 => Ok(CUSTOMER108_KEY_32.clone().into()),
			BUCKET_ID109 => Ok(CUSTOMER109_KEY_32.clone().into()),

			_ => Err(DispatchError::Other("Unexpected bucket_id")),
		}
	}

	fn get_total_bucket_usage(
		_cluster_id: &ClusterId,
		_bucket_id: BucketId,
		_bucket_owner: &T::AccountId,
	) -> Result<Option<BucketUsage>, DispatchError> {
		Ok(None)
	}

	fn update_total_bucket_usage(
		_cluster_id: &ClusterId,
		_bucket_id: BucketId,
		_bucket_owner: T::AccountId,
		_payable_usage: &BucketUsage,
	) -> DispatchResult {
		Ok(())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn create_bucket(
		_cluster_id: &ClusterId,
		_bucket_id: BucketId,
		_owner_id: T::AccountId,
		_bucket_params: BucketParams,
	) -> Result<(), DispatchError> {
		unimplemented!()
	}
}

pub struct TestCustomerCharger;
impl<T: Config> CustomerCharger<T> for TestCustomerCharger
where
	<T as frame_system::Config>::AccountId: From<AccountId>,
{
	fn charge_customer(
		content_owner: T::AccountId,
		payout_vault: T::AccountId,
		amount: u128,
	) -> Result<u128, DispatchError> {
		let mut amount_to_charge = amount;
		let account_1: T::AccountId = CUSTOMER1_KEY_32.into();
		let account_2: T::AccountId = CUSTOMER2_KEY_32.into();
		let account_3: T::AccountId = CUSTOMER3_KEY_32.into();
		let account_4: T::AccountId = CUSTOMER4_KEY_32.into();
		let account_5: T::AccountId = CUSTOMER5_KEY_32.into();

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
			assert!(CUSTOMER2_BALANCE < amount);
			amount_to_charge = CUSTOMER2_BALANCE; // for user 2
		}

		let charge = amount_to_charge.saturated_into::<BalanceOf<T>>();
		<T as pallet::Config>::Currency::transfer(
			&content_owner,
			&payout_vault,
			charge,
			ExistenceRequirement::AllowDeath,
		)?;
		Ok(amount_to_charge)
	}
}

pub const RESERVE_ACCOUNT_ID: [u8; 32] = [9; 32];
pub const TREASURY_ACCOUNT_ID: [u8; 32] = [8; 32];
pub const VALIDATOR1_ACCOUNT_ID: [u8; 32] = [111; 32];
pub const VALIDATOR2_ACCOUNT_ID: [u8; 32] = [222; 32];
pub const VALIDATOR3_ACCOUNT_ID: [u8; 32] = [250; 32];
pub const VALIDATOR_OCW_KEY_32: AccountId32 = AccountId32::new([123; 32]);

pub const VALIDATOR1_SCORE: u64 = 30;
pub const VALIDATOR2_SCORE: u64 = 45;
pub const VALIDATOR3_SCORE: u64 = 25;

pub const PARTIAL_CHARGE: u128 = 10;
// < PARTIAL_CHARGE
pub const CUSTOMER2_BALANCE: u128 = 5;
// > PARTIAL_CHARGE
pub const CUSTOMER3_BALANCE: u128 = 1000;

pub const NODE1_PUB_KEY_32: AccountId32 = AccountId32::new([
	48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66, 235,
	212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
]);
pub const NODE2_PUB_KEY_32: AccountId32 = AccountId32::new([
	49, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66, 235,
	212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
]);
pub const NODE3_PUB_KEY_32: AccountId32 = AccountId32::new([
	50, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66, 235,
	212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
]);
pub const NODE4_PUB_KEY_32: AccountId32 = AccountId32::new([
	51, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66, 235,
	212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
]);
pub const NODE5_PUB_KEY_32: AccountId32 = AccountId32::new([
	52, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66, 235,
	212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
]);
pub const NODE6_PUB_KEY_32: AccountId32 = AccountId32::new([
	53, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66, 235,
	212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
]);
pub const NODE7_PUB_KEY_32: AccountId32 = AccountId32::new([
	54, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66, 235,
	212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
]);
pub const NODE8_PUB_KEY_32: AccountId32 = AccountId32::new([
	55, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66, 235,
	212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
]);
pub const NODE9_PUB_KEY_32: AccountId32 = AccountId32::new([
	56, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66, 235,
	212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
]);
pub const NODE10_PUB_KEY_32: AccountId32 = AccountId32::new([
	57, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66, 235,
	212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
]);

pub const NODE_PROVIDER1_KEY_32: AccountId32 = AccountId32::new([10; 32]);
pub const NODE_PROVIDER2_KEY_32: AccountId32 = AccountId32::new([11; 32]);
pub const NODE_PROVIDER3_KEY_32: AccountId32 = AccountId32::new([12; 32]);
pub const NODE_PROVIDER4_KEY_32: AccountId32 = AccountId32::new([13; 32]);
pub const NODE_PROVIDER5_KEY_32: AccountId32 = AccountId32::new([14; 32]);
pub const NODE_PROVIDER6_KEY_32: AccountId32 = AccountId32::new([15; 32]);
pub const NODE_PROVIDER7_KEY_32: AccountId32 = AccountId32::new([16; 32]);
pub const NODE_PROVIDER8_KEY_32: AccountId32 = AccountId32::new([17; 32]);
pub const NODE_PROVIDER9_KEY_32: AccountId32 = AccountId32::new([18; 32]);
pub const NODE_PROVIDER10_KEY_32: AccountId32 = AccountId32::new([19; 32]);

pub const BUCKET_ID1: BucketId = 1;
pub const BUCKET_ID2: BucketId = 2;
pub const BUCKET_ID3: BucketId = 3;
pub const BUCKET_ID4: BucketId = 4;
pub const BUCKET_ID5: BucketId = 5;
pub const BUCKET_ID6: BucketId = 6;
pub const BUCKET_ID7: BucketId = 7;
pub const BUCKET_ID8: BucketId = 8;
pub const BUCKET_ID9: BucketId = 9;
pub const BUCKET_ID10: BucketId = 10;

pub const BUCKET_ID100: BucketId = 100;
pub const BUCKET_ID101: BucketId = 101;
pub const BUCKET_ID102: BucketId = 102;
pub const BUCKET_ID103: BucketId = 103;
pub const BUCKET_ID104: BucketId = 104;
pub const BUCKET_ID105: BucketId = 105;
pub const BUCKET_ID106: BucketId = 106;
pub const BUCKET_ID107: BucketId = 107;
pub const BUCKET_ID108: BucketId = 108;
pub const BUCKET_ID109: BucketId = 109;

pub const CUSTOMER1_KEY_32: AccountId32 = AccountId32::new([1; 32]);
pub const CUSTOMER2_KEY_32: AccountId32 = AccountId32::new([2; 32]);
pub const CUSTOMER3_KEY_32: AccountId32 = AccountId32::new([3; 32]);
pub const CUSTOMER4_KEY_32: AccountId32 = AccountId32::new([4; 32]);
pub const CUSTOMER5_KEY_32: AccountId32 = AccountId32::new([5; 32]);
pub const CUSTOMER6_KEY_32: AccountId32 = AccountId32::new([6; 32]);
pub const CUSTOMER7_KEY_32: AccountId32 = AccountId32::new([7; 32]);

pub const CUSTOMER100_KEY_32: AccountId32 = AccountId32::new([100; 32]);
pub const CUSTOMER101_KEY_32: AccountId32 = AccountId32::new([101; 32]);
pub const CUSTOMER102_KEY_32: AccountId32 = AccountId32::new([102; 32]);
pub const CUSTOMER103_KEY_32: AccountId32 = AccountId32::new([103; 32]);
pub const CUSTOMER104_KEY_32: AccountId32 = AccountId32::new([104; 32]);
pub const CUSTOMER105_KEY_32: AccountId32 = AccountId32::new([105; 32]);
pub const CUSTOMER106_KEY_32: AccountId32 = AccountId32::new([106; 32]);
pub const CUSTOMER107_KEY_32: AccountId32 = AccountId32::new([107; 32]);
pub const CUSTOMER108_KEY_32: AccountId32 = AccountId32::new([108; 32]);
pub const CUSTOMER109_KEY_32: AccountId32 = AccountId32::new([109; 32]);

pub const BANK_KEY_32: AccountId32 = AccountId32::new([200; 32]);

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

pub const DEFAULT_PAYERS_ROOT: H256 = H256([112; 32]);
pub const DEFAULT_PAYEES_ROOT: H256 = H256([113; 32]);

pub struct TestTreasuryVisitor;
impl<T: frame_system::Config> PalletVisitor<T> for TestTreasuryVisitor {
	fn get_account_id() -> T::AccountId {
		let reserve_account: [u8; 32] = TREASURY_ACCOUNT_ID;
		T::AccountId::decode(&mut &reserve_account[..]).unwrap()
	}
}

fn create_account_id_from_u128<T: frame_system::Config>(id: [u8; 32]) -> T::AccountId {
	T::AccountId::decode(&mut &id[..]).unwrap()
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

	#[cfg(feature = "try-runtime")]
	fn try_state() -> Result<(), TryRuntimeError> {
		Ok(())
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

	fn get_pricing_params(cluster_id: &ClusterId) -> Result<ClusterPricingParams, DispatchError> {
		Ok(get_pricing(cluster_id))
	}

	fn get_fees_params(cluster_id: &ClusterId) -> Result<ClusterFeesParams, DispatchError> {
		Ok(get_fees(cluster_id))
	}

	fn get_bonding_params(
		_cluster_id: &ClusterId,
	) -> Result<ClusterBondingParams<BlockNumberFor<T>>, DispatchError> {
		unimplemented!()
	}

	fn get_reserve_account_id(_cluster_id: &ClusterId) -> Result<T::AccountId, DispatchError> {
		let reserve_account: [u8; 32] = RESERVE_ACCOUNT_ID;
		Ok(T::AccountId::decode(&mut &reserve_account[..]).unwrap())
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

pub(crate) type TestRuntimeCall = <Test as frame_system::Config>::RuntimeCall;

pub struct ExtBuilder;

impl ExtBuilder {
	fn build(self) -> TestExternalities {
		sp_tracing::try_init_simple();

		let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		let _balance_genesis = pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(CUSTOMER1_KEY_32, 10000000000000000000000000000),
				(CUSTOMER2_KEY_32, CUSTOMER2_BALANCE),
				(CUSTOMER3_KEY_32, CUSTOMER3_BALANCE),
				(CUSTOMER4_KEY_32, 1000000000000000000000000),
				(CUSTOMER5_KEY_32, 1000000000000000000000000),
				(CUSTOMER6_KEY_32, 1000000000000000000000000),
				(CUSTOMER7_KEY_32, 1000000000000000000000000),
				(BANK_KEY_32, 10000000000000000000000000000),
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
