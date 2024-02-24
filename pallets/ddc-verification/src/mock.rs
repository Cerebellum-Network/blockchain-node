<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
#[cfg(feature = "runtime-benchmarks")]
use ddc_primitives::traits::{BucketManager, ClusterCreator, CustomerDepositor};
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
#[cfg(feature = "runtime-benchmarks")]
use ddc_primitives::traits::{BucketManager, ClusterCreator, CustomerDepositor};
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
use ddc_primitives::{
	crypto, sr25519,
	traits::{ClusterManager, ClusterQuery},
	BucketId, ClusterNodeKind, ClusterNodeState, ClusterNodeStatus, ClusterNodesStats,
<<<<<<< HEAD
<<<<<<< HEAD
	ClusterStatus, PayoutError, PayoutState, StorageNodeMode, StorageNodePubKey,
	MAX_PAYOUT_BATCH_COUNT, MAX_PAYOUT_BATCH_SIZE,
=======
use ddc_primitives::{
	crypto, sr25519,
	traits::{ClusterManager, ClusterQuery, StorageUsageProvider},
	BillingFingerprintParams, BucketId, BucketStorageUsage, ClusterNodeKind, ClusterNodeState,
	ClusterNodeStatus, ClusterNodesStats, ClusterStatus, Fingerprint, NodeStorageUsage,
	PayoutError, PayoutState, StorageNodeMode, StorageNodePubKey, MAX_PAYOUT_BATCH_COUNT,
	MAX_PAYOUT_BATCH_SIZE,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
};
#[cfg(feature = "runtime-benchmarks")]
use ddc_primitives::{
	traits::{BucketManager, ClusterCreator, CustomerDepositor},
	BillingReportParams, BucketParams, ClusterId, ClusterParams, ClusterProtocolParams,
<<<<<<< HEAD
=======
	ClusterStatus, PayoutError, PayoutState, StorageNodePubKey, MAX_PAYOUT_BATCH_COUNT,
	MAX_PAYOUT_BATCH_SIZE,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	ClusterStatus, PayoutError, PayoutState, StorageNodeMode, StorageNodePubKey,
	MAX_PAYOUT_BATCH_COUNT, MAX_PAYOUT_BATCH_SIZE,
>>>>>>> 7d165a87 (Allow all nodes to participate (#399))
=======
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
};
use frame_election_provider_support::{
	bounds::{ElectionBounds, ElectionBoundsBuilder},
	onchain, SequentialPhragmen,
};
use frame_support::{
<<<<<<< HEAD
	derive_impl,
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
	pallet_prelude::ConstU32,
	parameter_types,
	traits::{ConstU16, ConstU64},
	PalletId,
};
use frame_system::mocking::MockBlock;
use pallet_staking::BalanceOf;
<<<<<<< HEAD
<<<<<<< HEAD
use scale_info::prelude::string::String;
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
use scale_info::prelude::string::String;
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
use sp_core::{ByteArray, H256};
use sp_runtime::{
	curve::PiecewiseLinear,
	testing::{TestXt, UintAuthorityId},
	traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify, Zero},
<<<<<<< HEAD
<<<<<<< HEAD
	BuildStorage, MultiSignature, Perbill, Percent,
=======
	BuildStorage, MultiSignature, Perbill,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	BuildStorage, MultiSignature, Perbill, Percent,
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
};
use sp_staking::{EraIndex, SessionIndex};

use crate::{self as pallet_ddc_verification, *};

type Block = MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub struct Test {
		System: frame_system,
		DdcVerification: pallet_ddc_verification,
		Timestamp: pallet_timestamp,
		Balances: pallet_balances,
		Staking: pallet_staking,
		Session: pallet_session,
	}
);

pub type Extrinsic = TestXt<RuntimeCall, ()>;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
type Balance = u64;
type BlockNumber = u64;

<<<<<<< HEAD
#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
<<<<<<< HEAD
	type RuntimeTask = RuntimeTask;
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU64<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type MaxHolds = ();
<<<<<<< HEAD
	type RuntimeFreezeReason = ();
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
}

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000u64,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	pub static ElectionsBounds: ElectionBounds = ElectionBoundsBuilder::default().build();
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Test;
	type Solver = SequentialPhragmen<AccountId, Perbill>;
	type DataProvider = Staking;
	type WeightInfo = ();
	type MaxWinners = ConstU32<100>;
	type Bounds = ElectionsBounds;
}
parameter_types! {
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub static Offset: BlockNumber = 0;
	pub const Period: BlockNumber = 1;
	pub static SessionsPerEra: SessionIndex = 6;
	pub static SlashDeferDuration: EraIndex = 2;
	pub const BondingDuration: EraIndex = 3;
	pub static LedgerSlashPerEra: (BalanceOf<Test>, BTreeMap<EraIndex, BalanceOf<Test>>) = (Zero::zero(), BTreeMap::new());
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
<<<<<<< HEAD
	pub static MaxControllersInDeprecationBatch: u32 = 5900;
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
}

impl pallet_staking::Config for Test {
	type Currency = Balances;
	type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = ();
	type RewardRemainder = ();
	type RuntimeEvent = RuntimeEvent;
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type BondingDuration = BondingDuration;
	type SessionInterface = Self;
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
<<<<<<< HEAD
	type MaxExposurePageSize = ConstU32<64>;
=======
	type MaxNominatorRewardedPerValidator = ConstU32<64>;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GenesisElectionProvider = Self::ElectionProvider;
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
	type MaxUnlockingChunks = ConstU32<32>;
	type HistoryDepth = ConstU32<84>;
<<<<<<< HEAD
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type EventListeners = ();
	type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
	type WeightInfo = ();
}

pub struct OtherSessionHandler;
impl OneSessionHandler<AccountId> for OtherSessionHandler {
	type Key = UintAuthorityId;

<<<<<<< HEAD
	#[allow(clippy::multiple_bound_locations)]
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
	fn on_genesis_session<'a, I: 'a>(_: I)
	where
		I: Iterator<Item = (&'a AccountId, Self::Key)>,
		AccountId: 'a,
	{
	}

<<<<<<< HEAD
	#[allow(clippy::multiple_bound_locations)]
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
	fn on_new_session<'a, I: 'a>(_: bool, _: I, _: I)
	where
		I: Iterator<Item = (&'a AccountId, Self::Key)>,
		AccountId: 'a,
	{
	}

	fn on_disabled(_validator_index: u32) {}
}

impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
	type Public = UintAuthorityId;
}

impl pallet_session::historical::Config for Test {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Test>;
}

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub other: OtherSessionHandler,
	}
}

impl pallet_session::Config for Test {
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type Keys = SessionKeys;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionHandler = (OtherSessionHandler,);
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Test>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type WeightInfo = ();
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}
parameter_types! {
	pub const VerificationPalletId: PalletId = PalletId(*b"verifypa");
<<<<<<< HEAD
<<<<<<< HEAD
	pub const MajorityOfAggregators: Percent = Percent::from_percent(67);
<<<<<<< HEAD
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	pub const MajorityOfAggregators: Percent = Percent::from_percent(67);
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
=======
	pub const VerifyAggregatorResponseSignature: bool = false;
<<<<<<< HEAD
>>>>>>> a1202386 (Disable aggregates resp sig verification for tests)
=======
	pub const MajorityOfValidators: Percent = Percent::from_percent(67);
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = VerificationPalletId;
	type WeightInfo = ();
	type ClusterManager = TestClusterManager;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
	type ClusterValidator = TestClusterValidator;
=======
	type ClusterValidator = MockClusterValidator;
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
	type NodeManager = MockNodeManager;
<<<<<<< HEAD
<<<<<<< HEAD
	type PayoutProcessor = MockPayoutProcessor;
=======
=======
	type ClusterValidator = TestClusterValidator;
>>>>>>> 99095ecd (verified copy of PR#393 (#402))
	type NodeVisitor = MockNodeVisitor;
=======
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
	type PayoutVisitor = MockPayoutVisitor;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	type PayoutProcessor = MockPayoutProcessor;
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
	type AuthorityId = sr25519::AuthorityId;
	type OffchainIdentifierId = crypto::OffchainIdentifierId;
	type Hasher = sp_runtime::traits::BlakeTwo256;
	const BLOCK_TO_START: u16 = 100;
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 5bbd19d4 (chore: renaming according to domain language)
	const DAC_REDUNDANCY_FACTOR: u16 = 3;
	type AggregatorsQuorum = MajorityOfAggregators;
	type ValidatorsQuorum = MajorityOfValidators;
	const MAX_PAYOUT_BATCH_SIZE: u16 = MAX_PAYOUT_BATCH_SIZE;
	const MAX_PAYOUT_BATCH_COUNT: u16 = MAX_PAYOUT_BATCH_COUNT;
	type ValidatorStaking = Staking;
	type AccountIdConverter = AccountId;
	type CustomerVisitor = MockCustomerVisitor;
	const MAX_MERKLE_NODE_IDENTIFIER: u16 = 4;
<<<<<<< HEAD
<<<<<<< HEAD
	type Currency = Balances;
	const VERIFY_AGGREGATOR_RESPONSE_SIGNATURE: bool = false;
	type BucketsStorageUsageProvider = MockBucketValidator;
	type NodesStorageUsageProvider = MockNodeValidator;
	#[cfg(feature = "runtime-benchmarks")]
	type CustomerDepositor = MockCustomerDepositor;
	#[cfg(feature = "runtime-benchmarks")]
	type ClusterCreator = MockClusterCreator;
	#[cfg(feature = "runtime-benchmarks")]
	type BucketManager = MockBucketManager;
<<<<<<< HEAD
=======
>>>>>>> 18369b0b (Challenge sub trees and make them ready for payout (#434))
=======
	type Currency = Balances;
>>>>>>> 9e865129 (feat: benchmarking for ddc-verification pallet calls part 2)
=======
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
}

pub struct MockBucketValidator;
impl StorageUsageProvider<BucketId, BucketStorageUsage<AccountId>> for MockBucketValidator {
	type Error = ();

	fn iter_storage_usage<'a>(
		_cluster_id: &'a ClusterId,
	) -> Box<dyn Iterator<Item = BucketStorageUsage<AccountId>> + 'a> {
		unimplemented!()
	}

	fn iter_storage_usage_from<'a>(
		_cluster_id: &'a ClusterId,
		_from: &'a BucketId,
	) -> Result<Box<dyn Iterator<Item = BucketStorageUsage<AccountId>> + 'a>, ()> {
		unimplemented!()
	}
}

pub struct MockNodeValidator;
impl StorageUsageProvider<StorageNodePubKey, NodeStorageUsage<AccountId>> for MockNodeValidator {
	type Error = ();

	fn iter_storage_usage<'a>(
		_cluster_id: &'a ClusterId,
	) -> Box<dyn Iterator<Item = NodeStorageUsage<AccountId>> + 'a> {
		unimplemented!()
	}

	fn iter_storage_usage_from<'a>(
		_cluster_id: &'a ClusterId,
		_from: &'a StorageNodePubKey,
	) -> Result<Box<dyn Iterator<Item = NodeStorageUsage<AccountId>> + 'a>, ()> {
		unimplemented!()
	}
}

pub struct MockCustomerVisitor;
impl<T: Config> CustomerVisitor<T> for MockCustomerVisitor {
	fn get_bucket_owner(_bucket_id: &BucketId) -> Result<T::AccountId, DispatchError> {
		let temp: AccountId = AccountId::from([0xa; 32]);
		let account_1 = T::AccountId::decode(&mut &temp.as_slice()[..]).unwrap();

		Ok(account_1)
	}
}

<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
#[cfg(feature = "runtime-benchmarks")]
pub struct MockCustomerDepositor;
#[cfg(feature = "runtime-benchmarks")]
impl<T: Config> CustomerDepositor<T> for MockCustomerDepositor {
	fn deposit(_customer: T::AccountId, _amount: u128) -> Result<(), DispatchError> {
		unimplemented!()
	}
	fn deposit_extra(_customer: T::AccountId, _amount: u128) -> Result<(), DispatchError> {
		unimplemented!()
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct MockClusterCreator;
#[cfg(feature = "runtime-benchmarks")]
impl<T: Config> ClusterCreator<T, Balance> for MockClusterCreator {
	fn create_cluster(
		_cluster_id: ClusterId,
		_cluster_manager_id: T::AccountId,
		_cluster_reserve_id: T::AccountId,
		_cluster_params: ClusterParams<T::AccountId>,
		_initial_protocol_params: ClusterProtocolParams<Balance, BlockNumberFor<T>>,
	) -> DispatchResult {
		unimplemented!()
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct MockBucketManager;
#[cfg(feature = "runtime-benchmarks")]
impl<T: Config> BucketManager<T> for MockBucketManager {
	fn get_bucket_owner_id(_bucket_id: BucketId) -> Result<T::AccountId, DispatchError> {
		unimplemented!()
	}

	fn get_total_bucket_usage(
		_cluster_id: &ClusterId,
		_bucket_id: BucketId,
<<<<<<< HEAD
		_content_owner: &T::AccountId,
<<<<<<< HEAD
<<<<<<< HEAD
=======
		_bucket_owner: &T::AccountId,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
	) -> Result<Option<BucketUsage>, DispatchError> {
=======
	) -> Result<Option<CustomerUsage>, DispatchError> {
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
	) -> Result<Option<BucketUsage>, DispatchError> {
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
		unimplemented!()
	}

	fn update_total_bucket_usage(
		_cluster_id: &ClusterId,
		_bucket_id: BucketId,
<<<<<<< HEAD
		_content_owner: T::AccountId,
<<<<<<< HEAD
<<<<<<< HEAD
		_customer_usage: &BucketUsage,
=======
		_customer_usage: &CustomerUsage,
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
		_customer_usage: &BucketUsage,
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
=======
		_bucket_owner: T::AccountId,
		_payable_usage: &BucketUsage,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
	) -> DispatchResult {
		unimplemented!()
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

<<<<<<< HEAD
=======
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
=======
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
pub(crate) const VALIDATOR_VERIFICATION_PUB_KEY_HEX: &str =
	"4e7b7f176f8778a2dbef829f50466170634e747ab5c5e64cb131c9c5a01d975f";
pub(crate) const VALIDATOR_VERIFICATION_PRIV_KEY_HEX: &str =
	"b6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa318";

<<<<<<< HEAD
=======
	const MIN_DAC_NODES_FOR_CONSENSUS: u16 = 3;
	const MAX_PAYOUT_BATCH_SIZE: u16 = MAX_PAYOUT_BATCH_SIZE;
	const MAX_PAYOUT_BATCH_COUNT: u16 = MAX_PAYOUT_BATCH_COUNT;
	type ActivityHash = H256;
	type StakingVisitor = Staking;
	type AccountIdConverter = AccountId;
	type CustomerVisitor = MockCustomerVisitor;
}

<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
pub struct MockCustomerVisitor;
impl<T: Config> CustomerVisitor<T> for MockCustomerVisitor {
	fn get_bucket_owner(_bucket_id: &BucketId) -> Result<T::AccountId, DispatchError> {
		let temp: AccountId = AccountId::from([0xa; 32]);
		let account_1 = T::AccountId::decode(&mut &temp.as_slice()[..]).unwrap();

		Ok(account_1)
	}
}
>>>>>>> 91b446cb (Changes to fetch minimum sub-trees which are in consensus (#424))
=======
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	let balances = vec![
		// validator1 stash; has to be equal to the OCW key in the current implementation
		(AccountId::from([0xa; 32]), 10000),
		// validator1 controller
		(AccountId::from([0xaa; 32]), 10000),
		// validator2 stash
		(AccountId::from([0xb; 32]), 10000),
		// validator2 controller
		(AccountId::from([0xbb; 32]), 10000),
		// validator3 stash
		(AccountId::from([0xc; 32]), 10000),
		// validator3 controller
		(AccountId::from([0xcc; 32]), 10000),
		// validator4 stash
		(AccountId::from([0xd; 32]), 10000),
		// validator4 controller
		(AccountId::from([0xdd; 32]), 10000),
		// validator5 stash
		(AccountId::from([0xe; 32]), 10000),
		// validator5 controller
		(AccountId::from([0xee; 32]), 10000),
		// validator6 stash
		(AccountId::from([0xf; 32]), 10000),
		// validator6 controller
		(AccountId::from([0xff; 32]), 10000),
	];
	let _ = pallet_balances::GenesisConfig::<Test> { balances }.assimilate_storage(&mut storage);

	let stakers = vec![
		(
			AccountId::from([0xa; 32]),
			AccountId::from([0xaa; 32]),
			1000,
			pallet_staking::StakerStatus::Validator,
		),
		(
			AccountId::from([0xb; 32]),
			AccountId::from([0xbb; 32]),
			1000,
			pallet_staking::StakerStatus::Validator,
		),
		(
			AccountId::from([0xc; 32]),
			AccountId::from([0xcc; 32]),
			1000,
			pallet_staking::StakerStatus::Validator,
		),
		(
			AccountId::from([0xd; 32]),
			AccountId::from([0xdd; 32]),
			1000,
			pallet_staking::StakerStatus::Validator,
		),
		(
			AccountId::from([0xe; 32]),
			AccountId::from([0xee; 32]),
			1000,
			pallet_staking::StakerStatus::Validator,
		),
		(
			AccountId::from([0xf; 32]),
			AccountId::from([0xff; 32]),
			1000,
			pallet_staking::StakerStatus::Validator,
		),
	];
	let _ =
		pallet_staking::GenesisConfig::<Test> { stakers: stakers.clone(), ..Default::default() }
			.assimilate_storage(&mut storage);

<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
	let arr = hex::decode(VALIDATOR_VERIFICATION_PUB_KEY_HEX)
		.expect("Test verification pub key to be extracted");

	let verification_key = AccountId::decode(&mut &arr[..]).unwrap();

	let _ = pallet_ddc_verification::GenesisConfig::<Test> { validators: vec![verification_key] }
		.assimilate_storage(&mut storage);

<<<<<<< HEAD
	sp_io::TestExternalities::new(storage)
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

pub struct MockPayoutProcessor;
impl<T: Config> PayoutProcessor<T> for MockPayoutProcessor {
	fn commit_billing_fingerprint(
		_validator: T::AccountId,
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_start_era: i64,
		_end_era: i64,
		_payers_merkle_root: PayableUsageHash,
		_payees_merkle_root: PayableUsageHash,
		_cluster_usage: NodeUsage,
	) -> DispatchResult {
		unimplemented!()
	}

	fn begin_billing_report(
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_fingerprint: Fingerprint,
	) -> DispatchResult {
		unimplemented!()
	}

	fn begin_charging_customers(
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_max_batch_index: BatchIndex,
	) -> DispatchResult {
		unimplemented!()
	}

	fn send_charging_customers_batch(
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_batch_index: BatchIndex,
<<<<<<< HEAD
		_payers: &[(NodePubKey, BucketId, BucketUsage)],
<<<<<<< HEAD
=======
		_payers: &[(BucketId, BucketUsage)],
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
		_batch_proof: MMRProof,
	) -> DispatchResult {
		unimplemented!()
	}

	fn end_charging_customers(_cluster_id: ClusterId, _era_id: DdcEra) -> DispatchResult {
		unimplemented!()
	}

	fn begin_rewarding_providers(
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_max_batch_index: BatchIndex,
	) -> DispatchResult {
		unimplemented!()
	}

	fn send_rewarding_providers_batch(
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_batch_index: BatchIndex,
		_payees: &[(NodePubKey, NodeUsage)],
		_batch_proof: MMRProof,
	) -> DispatchResult {
		unimplemented!()
	}

	fn end_rewarding_providers(_cluster_id: ClusterId, _era_id: DdcEra) -> DispatchResult {
		unimplemented!()
	}

	fn end_billing_report(_cluster_id: ClusterId, _era_id: DdcEra) -> DispatchResult {
		unimplemented!()
	}

=======
=======
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
	sp_io::TestExternalities::new(storage)
}

pub struct TestClusterValidator;
impl<T: Config> ClusterValidator<T> for TestClusterValidator {
	fn set_last_paid_era(_cluster_id: &ClusterId, _era_id: DdcEra) -> Result<(), DispatchError> {
		unimplemented!()
	}

	fn get_last_paid_era(_cluster_id: &ClusterId) -> Result<DdcEra, DispatchError> {
		Ok(Default::default())
	}
}

<<<<<<< HEAD
pub struct MockPayoutVisitor;
impl<T: Config> PayoutVisitor<T> for MockPayoutVisitor {
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
pub struct MockPayoutProcessor;
impl<T: Config> PayoutProcessor<T> for MockPayoutProcessor {
	fn begin_billing_report(
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_start_era: i64,
		_end_era: i64,
	) -> DispatchResult {
		unimplemented!()
	}

	fn begin_charging_customers(
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_max_batch_index: BatchIndex,
	) -> DispatchResult {
		unimplemented!()
	}

	fn send_charging_customers_batch(
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_batch_index: BatchIndex,
		_payers: &[(NodePubKey, BucketId, CustomerUsage)],
=======
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
		_batch_proof: MMRProof,
	) -> DispatchResult {
		unimplemented!()
	}

	fn end_charging_customers(_cluster_id: ClusterId, _era_id: DdcEra) -> DispatchResult {
		unimplemented!()
	}

	fn begin_rewarding_providers(
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_max_batch_index: BatchIndex,
		_total_node_usage: NodeUsage,
	) -> DispatchResult {
		unimplemented!()
	}

	fn send_rewarding_providers_batch(
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_batch_index: BatchIndex,
		_payees: &[(NodePubKey, NodeUsage)],
		_batch_proof: MMRProof,
	) -> DispatchResult {
		unimplemented!()
	}

	fn end_rewarding_providers(_cluster_id: ClusterId, _era_id: DdcEra) -> DispatchResult {
		unimplemented!()
	}

	fn end_billing_report(_cluster_id: ClusterId, _era_id: DdcEra) -> DispatchResult {
		unimplemented!()
	}

>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
	fn get_next_customer_batch_for_payment(
		_cluster_id: &ClusterId,
		_era_id: DdcEra,
	) -> Result<Option<BatchIndex>, PayoutError> {
		Ok(None)
	}

	fn get_next_provider_batch_for_payment(
		_cluster_id: &ClusterId,
		_era_id: DdcEra,
	) -> Result<Option<BatchIndex>, PayoutError> {
		Ok(None)
	}

	fn all_customer_batches_processed(_cluster_id: &ClusterId, _era_id: DdcEra) -> bool {
		true
	}

	fn all_provider_batches_processed(_cluster_id: &ClusterId, _era_id: DdcEra) -> bool {
		true
	}

	fn get_billing_report_status(_cluster_id: &ClusterId, _era_id: DdcEra) -> PayoutState {
		PayoutState::NotInitialized
	}
<<<<<<< HEAD
<<<<<<< HEAD

<<<<<<< HEAD
=======

<<<<<<< HEAD
	#[cfg(feature = "runtime-benchmarks")]
>>>>>>> 2bdeb4c7 (fix: cleaning up and fixing fmt and clippy issues)
=======
>>>>>>> f870601e (Alow `create_billing_report` at runtime)
	fn create_billing_report(_vault: T::AccountId, _params: BillingReportParams) {
		unimplemented!()
	}

	fn create_billing_fingerprint(_params: BillingFingerprintParams<T::AccountId>) -> Fingerprint {
		unimplemented!()
	}
}

pub struct MockNodeManager;
impl<T: Config> NodeManager<T> for MockNodeManager {
<<<<<<< HEAD
	fn get_total_usage(_node_pub_key: &NodePubKey) -> Result<Option<NodeUsage>, DispatchError> {
		Ok(None) // todo! add more complex mock
	}

=======
	fn begin_billing_report(
		_origin: T::AccountId,
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_start_era: i64,
		_end_era: i64,
	) -> DispatchResult {
		Ok(())
	}

	fn begin_charging_customers(
		_origin: T::AccountId,
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_max_batch_index: BatchIndex,
	) -> DispatchResult {
		Ok(())
	}

	fn send_charging_customers_batch(
		_origin: T::AccountId,
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_batch_index: BatchIndex,
		_payers: &[(NodePubKey, BucketId, CustomerUsage)],
		_batch_proof: MMRProof,
	) -> DispatchResult {
		Ok(())
	}

	fn end_charging_customers(
		_origin: T::AccountId,
		_cluster_id: ClusterId,
		_era_id: DdcEra,
	) -> DispatchResult {
		Ok(())
	}

	fn begin_rewarding_providers(
		_origin: T::AccountId,
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_max_batch_index: BatchIndex,
		_total_node_usage: NodeUsage,
	) -> DispatchResult {
		Ok(())
	}

	fn send_rewarding_providers_batch(
		_origin: T::AccountId,
		_cluster_id: ClusterId,
		_era_id: DdcEra,
		_batch_index: BatchIndex,
		_payees: &[(NodePubKey, NodeUsage)],
		_batch_proof: MMRProof,
	) -> DispatchResult {
		Ok(())
	}

	fn end_rewarding_providers(
		_origin: T::AccountId,
		_cluster_id: ClusterId,
		_era_id: DdcEra,
	) -> DispatchResult {
		Ok(())
	}

	fn end_billing_report(
		_origin: T::AccountId,
		_cluster_id: ClusterId,
		_era_id: DdcEra,
	) -> DispatchResult {
		Ok(())
	}
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
}

<<<<<<< HEAD
pub struct MockNodeVisitor;
impl<T: Config> NodeVisitor<T> for MockNodeVisitor {
<<<<<<< HEAD
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
=======
pub struct MockNodeManager;
impl<T: Config> NodeManager<T> for MockNodeManager {
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
	fn get_total_usage(_node_pub_key: &NodePubKey) -> Result<Option<NodeUsage>, DispatchError> {
		Ok(None) // todo! add more complex mock
	}

>>>>>>> e0ce0e5b (node integer delta usage (#412))
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
	fn get_node_params(node_pub_key: &NodePubKey) -> Result<NodeParams, DispatchError> {
		let key1 =
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a",
			)));
		let key2 =
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e",
			)));
		let key3 =
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"dcb83f51e6554fb3fca04807f98336d160419bf0c54f479d760b76df1e04bda2",
			)));

		let key4 =
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"48dbb875df3f77816cd01b5a8ce6f32944ae4ac3b4453b9345c3320689445e88",
			)));
		let key5 =
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395",
			)));
		let key6 =
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"f2f521014e436b426e4277b23267655ae04d1858c84756d9ed970d17271d19e4",
			)));

		let key7 =
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"1f50f1455f60f5774564233d321a116ca45ae3188b2200999445706d04839d72",
			)));
		let key8 =
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"69b1897f5f7a8a775ee3a4e00f32e20bb9d30e1cdd42149ce1bd50a9aa206040",
			)));
		let _key9 =
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"bf5ca1c9406094b4dea7981ba076f1520c218f18ace853300a3300c5cfe9c2af",
			)));

		let storage_node_params = if node_pub_key == &key1 {
			StorageNodeParams {
				mode: StorageNodeMode::DAC,
				host: "178.251.228.236".as_bytes().to_vec(),
				domain: vec![2u8; 255],
				ssl: false,
				http_port: 8080u16,
				grpc_port: 25000u16,
				p2p_port: 15000u16,
			}
		} else if node_pub_key == &key2 {
			StorageNodeParams {
				mode: StorageNodeMode::DAC,
				host: "95.217.8.119".as_bytes().to_vec(),
				domain: vec![2u8; 255],
				ssl: false,
				http_port: 8080u16,
				grpc_port: 25000u16,
				p2p_port: 15000u16,
			}
		} else if node_pub_key == &key3 {
			StorageNodeParams {
				mode: StorageNodeMode::DAC,
				host: "178.251.228.42".as_bytes().to_vec(),
				domain: vec![2u8; 255],
				ssl: false,
				http_port: 8080u16,
				grpc_port: 25000u16,
				p2p_port: 15000u16,
			}
		} else if node_pub_key == &key4 {
			StorageNodeParams {
				mode: StorageNodeMode::DAC,
				host: "37.27.30.47".as_bytes().to_vec(),
				domain: vec![2u8; 255],
				ssl: false,
				http_port: 8080u16,
				grpc_port: 25000u16,
				p2p_port: 15000u16,
			}
		} else if node_pub_key == &key5 {
			StorageNodeParams {
				mode: StorageNodeMode::DAC,
				host: "178.251.228.49".as_bytes().to_vec(),
				domain: vec![2u8; 255],
				ssl: false,
				http_port: 8080u16,
				grpc_port: 25000u16,
				p2p_port: 15000u16,
			}
		} else if node_pub_key == &key6 {
			StorageNodeParams {
				mode: StorageNodeMode::DAC,
				host: "159.69.207.65".as_bytes().to_vec(),
				domain: vec![2u8; 255],
				ssl: false,
				http_port: 8080u16,
				grpc_port: 25000u16,
				p2p_port: 15000u16,
			}
		} else if node_pub_key == &key7 {
			StorageNodeParams {
				mode: StorageNodeMode::DAC,
				host: "178.251.228.165".as_bytes().to_vec(),
				domain: vec![2u8; 255],
				ssl: false,
				http_port: 8080u16,
				grpc_port: 25000u16,
				p2p_port: 15000u16,
			}
		} else if node_pub_key == &key8 {
			StorageNodeParams {
				mode: StorageNodeMode::DAC,
				host: "49.13.211.157".as_bytes().to_vec(),
				domain: vec![2u8; 255],
				ssl: false,
				http_port: 8080u16,
				grpc_port: 25000u16,
				p2p_port: 15000u16,
			}
		} else {
			StorageNodeParams {
				mode: StorageNodeMode::DAC,
				host: "178.251.228.44".as_bytes().to_vec(),
				domain: vec![2u8; 255],
				ssl: false,
				http_port: 8080u16,
				grpc_port: 25000u16,
				p2p_port: 15000u16,
			}
		};

		Ok(NodeParams::StorageParams(storage_node_params))
	}

	fn get_cluster_id(_node_pub_key: &NodePubKey) -> Result<Option<ClusterId>, DispatchError> {
		unimplemented!()
	}
<<<<<<< HEAD
<<<<<<< HEAD

=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======

>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
	fn exists(_node_pub_key: &NodePubKey) -> bool {
		unimplemented!()
	}

	fn get_node_provider_id(_node_pub_key: &NodePubKey) -> Result<T::AccountId, DispatchError> {
		let temp: AccountId = AccountId::from([0xa; 32]);
		let account_1 = T::AccountId::decode(&mut &temp.as_slice()[..]).unwrap();

		Ok(account_1)
	}
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)

	fn update_total_node_usage(
		_node_key: &NodePubKey,
		_payable_usage: &NodeUsage,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn create_node(
		_node_pub_key: NodePubKey,
		_provider_id: T::AccountId,
		_node_params: NodeParams,
	) -> DispatchResult {
		unimplemented!()
	}
<<<<<<< HEAD
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
}

pub struct TestClusterManager;
impl<T: Config> ClusterQuery<T> for TestClusterManager {
	fn cluster_exists(_cluster_id: &ClusterId) -> bool {
		unimplemented!()
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
	fn contains_node(
		_cluster_id: &ClusterId,
		_node_pub_key: &NodePubKey,
		_validation_status: Option<ClusterNodeStatus>,
	) -> bool {
		unimplemented!()
	}

	fn get_nodes(_cluster_id: &ClusterId) -> Result<Vec<NodePubKey>, DispatchError> {
		Ok(vec![
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"9ef98ad9c3626ba725e78d76cfcfc4b4d07e84f0388465bc7eb992e3e117234a",
			))),
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"48594f1fd4f05135914c42b03e63b61f6a3e4c537ccee3dbac555ef6df371b7e",
			))),
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"dcb83f51e6554fb3fca04807f98336d160419bf0c54f479d760b76df1e04bda2",
			))),
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"48dbb875df3f77816cd01b5a8ce6f32944ae4ac3b4453b9345c3320689445e88",
			))),
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"302f937df3a0ec4c658e8122439e748d227442ebd493cef521a1e14943844395",
			))),
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"f2f521014e436b426e4277b23267655ae04d1858c84756d9ed970d17271d19e4",
			))),
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"1f50f1455f60f5774564233d321a116ca45ae3188b2200999445706d04839d72",
			))),
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"69b1897f5f7a8a775ee3a4e00f32e20bb9d30e1cdd42149ce1bd50a9aa206040",
			))),
			NodePubKey::StoragePubKey(StorageNodePubKey::new(array_bytes::hex_n_into_unchecked(
				"bf5ca1c9406094b4dea7981ba076f1520c218f18ace853300a3300c5cfe9c2af",
			))),
		])
	}

	fn add_node(
		_cluster_id: &ClusterId,
		_node_pub_key: &NodePubKey,
		_node_kind: &ClusterNodeKind,
	) -> Result<(), DispatchError> {
		unimplemented!()
	}

	fn remove_node(
		_cluster_id: &ClusterId,
		_node_pub_key: &NodePubKey,
	) -> Result<(), DispatchError> {
		unimplemented!()
	}

	fn get_manager_account_id(_cluster_id: &ClusterId) -> Result<T::AccountId, DispatchError> {
		unimplemented!()
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
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)

	fn get_clusters(_status: ClusterStatus) -> Result<Vec<ClusterId>, DispatchError> {
		Ok(vec![ClusterId::from([12; 20])])
	}
<<<<<<< HEAD
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> 36ff1650 (feat: supporting multicluster environment in DAC validation)
}

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(RuntimeCall, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}
