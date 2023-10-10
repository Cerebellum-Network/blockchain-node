use crate::{self as pallet_ddc_validator, *};
use frame_election_provider_support::{onchain, SequentialPhragmen};
use frame_support::{
	parameter_types,
	traits::{Everything, Nothing, U128CurrencyToVote},
	weights::Weight,
	PalletId,
};
use frame_system::offchain::SendTransactionTypes;
use pallet_contracts as contracts;
use pallet_session::ShouldEndSession;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	curve::PiecewiseLinear,
	generic, impl_opaque_keys,
	testing::{TestXt, UintAuthorityId},
	traits::{
		BlakeTwo256, Convert, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify,
	},
	MultiSignature, Perbill,
};
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type BlockNumber = u32;
pub type Moment = u64;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Contracts: contracts,
		Timestamp: pallet_timestamp,
		Session: pallet_session,
		Staking: pallet_staking,
		DdcAccounts: pallet_ddc_accounts,
		DdcStaking: pallet_ddc_staking,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip,
		DdcValidator: pallet_ddc_validator,
	}
);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const MaximumBlockWeight: Weight = Weight::from_ref_time(1024);
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type RuntimeCall = RuntimeCall;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	// u64; // sp_core::sr25519::Public;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const SignedClaimHandicap: BlockNumber = 2;
	pub const TombstoneDeposit: Balance = 16;
	pub const StorageSizeOffset: u32 = 8;
	pub const RentByteFee: Balance = 4;
	pub const RentDepositOffset: Balance = 10_000;
	pub const SurchargeReward: Balance = 150;
	pub const MaxDepth: u32 = 100;
	pub const MaxValueSize: u32 = 16_384;
	pub Schedule: pallet_contracts::Schedule<Test> = Default::default();
}

pub struct TestWeightToFee;
impl Convert<u64, u128> for TestWeightToFee {
	fn convert(weight: u64) -> u128 {
		weight as u128
	}
}

impl contracts::Config for Test {
	type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
	type RuntimeCall = RuntimeCall;
	type CallFilter = Nothing;
	type CallStack = [pallet_contracts::Frame<Self>; 31];
	type ChainExtension = ();
	type ContractAccessWeight = ();
	type Currency = Balances;
	type DeletionQueueDepth = ();
	type DeletionWeightLimit = ();
	type DepositPerByte = DepositPerByte;
	type DepositPerItem = DepositPerItem;
	type RuntimeEvent = RuntimeEvent;
	type MaxCodeLen = ConstU32<{ 128 * 1024 }>;
	type MaxStorageKeyLen = ConstU32<128>;
	type Randomness = RandomnessCollectiveFlip;
	type Schedule = Schedule;
	type Time = Timestamp;
	type WeightInfo = ();
	type WeightPrice = ();
}

parameter_types! {
	pub const TransactionByteFee: u64 = 0;
	pub const DepositPerItem: Balance = 0;
	pub const DepositPerByte: Balance = 0;
}

parameter_types! {
	pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Test {
	type Moment = Moment;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

impl pallet_randomness_collective_flip::Config for Test {}

pub struct TestShouldEndSession;
impl ShouldEndSession<u32> for TestShouldEndSession {
	fn should_end_session(now: u32) -> bool {
		now % 10 == 0 // every 10 blocks
	}
}

impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub dummy: UintAuthorityId,
	}
}

impl From<UintAuthorityId> for MockSessionKeys {
	fn from(dummy: UintAuthorityId) -> Self {
		Self { dummy }
	}
}

impl pallet_session::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = ();
	type ShouldEndSession = TestShouldEndSession;
	type NextSessionRotation = ();
	type SessionManager = ();
	type SessionHandler = pallet_session::TestSessionHandler;
	type Keys = MockSessionKeys;
	type WeightInfo = ();
}

impl pallet_session::historical::Config for Test {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Test>;
}

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_000_100,
		max_inflation: 0_050_000,
		ideal_stake: 0_200_000,
		falloff: 0_050_000,
		max_piece_count: 100,
		test_precision: 0_050_000,
	);
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Test;
	type Solver = SequentialPhragmen<AccountId, Perbill>;
	type DataProvider = Staking;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Test>;
}

parameter_types! {
	pub const SessionsPerEra: sp_staking::SessionIndex = 6;
	pub const BondingDuration: sp_staking::EraIndex = 3;
	pub const SlashDeferDuration: sp_staking::EraIndex = 2;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxNominatorRewardedPerValidator: u32 = 256;
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
	pub OffchainRepeat: BlockNumber = 5;
}

impl pallet_staking::Config for Test {
	type MaxNominations = ConstU32<16>;
	type Currency = Balances;
	type UnixTime = Timestamp;
	type CurrencyToVote = U128CurrencyToVote;
	type RewardRemainder = ();
	type RuntimeEvent = RuntimeEvent;
	type Slash = (); // send the slashed funds to the treasury.
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type SessionInterface = Self;
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type ElectionProvider = onchain::UnboundedExecution<OnChainSeqPhragmen>;
	type GenesisElectionProvider = Self::ElectionProvider;
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type MaxUnlockingChunks = ConstU32<32>;
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Test>;
	type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
	type CurrencyBalance = Balance;
	type OnStakerSlash = ();
	type HistoryDepth = ConstU32<84>;
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
}

parameter_types! {
	pub const DdcAccountsPalletId: PalletId = PalletId(*b"accounts");
}

impl pallet_ddc_accounts::Config for Test {
	type BondingDuration = BondingDuration;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type PalletId = DdcAccountsPalletId;
}

parameter_types! {
	pub const DefaultEdgeBondSize: Balance = 100;
	pub const DefaultEdgeChillDelay: EraIndex = 2;
	pub const DefaultStorageBondSize: Balance = 100;
	pub const DefaultStorageChillDelay: EraIndex = 2;
}

impl pallet_ddc_staking::Config for Test {
	type BondingDuration = BondingDuration;
	type Currency = Balances;
	type DefaultEdgeBondSize = DefaultEdgeBondSize;
	type DefaultEdgeChillDelay = DefaultEdgeChillDelay;
	type DefaultStorageBondSize = DefaultStorageBondSize;
	type DefaultStorageChillDelay = DefaultStorageChillDelay;
	type RuntimeEvent = RuntimeEvent;
	type StakersPayoutSource = DdcAccountsPalletId;
	type UnixTime = Timestamp;
	type WeightInfo = pallet_ddc_staking::weights::SubstrateWeight<Test>;
}

parameter_types! {
	pub const DdcValidatorsQuorumSize: u32 = 3;
	pub const ValidationThreshold: u32 = 5;
}

impl pallet_ddc_validator::Config for Test {
	type DdcValidatorsQuorumSize = DdcValidatorsQuorumSize;
	type RuntimeEvent = RuntimeEvent;
	type Randomness = RandomnessCollectiveFlip;
	type RuntimeCall = RuntimeCall;
	type AuthorityId = pallet_ddc_validator::crypto::TestAuthId;
	type ValidationThreshold = ValidationThreshold;
	type ValidatorsMax = ();
}

impl<LocalCall> SendTransactionTypes<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxLocks: u32 = 10;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let balances = vec![
		// edge stash
		(AccountId::from([0x1; 32]), 1000),
		// edge controller
		(AccountId::from([0x11; 32]), 1000),
		// validator1 stash; has to be equal to the OCW key in the current implementation
		(
			AccountId::from([
				0xd2, 0xbf, 0x4b, 0x84, 0x4d, 0xfe, 0xfd, 0x67, 0x72, 0xa8, 0x84, 0x3e, 0x66, 0x9f,
				0x94, 0x34, 0x08, 0x96, 0x6a, 0x97, 0x7e, 0x3a, 0xe2, 0xaf, 0x1d, 0xd7, 0x8e, 0x0f,
				0x55, 0xf4, 0xdf, 0x67,
			]),
			10000,
		),
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
	];
	let _ = pallet_balances::GenesisConfig::<Test> { balances }.assimilate_storage(&mut storage);

	let stakers = vec![
		(
			AccountId::from([
				0xd2, 0xbf, 0x4b, 0x84, 0x4d, 0xfe, 0xfd, 0x67, 0x72, 0xa8, 0x84, 0x3e, 0x66, 0x9f,
				0x94, 0x34, 0x08, 0x96, 0x6a, 0x97, 0x7e, 0x3a, 0xe2, 0xaf, 0x1d, 0xd7, 0x8e, 0x0f,
				0x55, 0xf4, 0xdf, 0x67,
			]),
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
	];
	let _ = pallet_staking::GenesisConfig::<Test> { stakers, ..Default::default() }
		.assimilate_storage(&mut storage);

	let edges = vec![(
		AccountId::from([0x1; 32]),
		AccountId::from([0x11; 32]),
		AccountId::from([0x21; 32]),
		100,
		1,
	)];
	let storages = vec![];
	let _ = pallet_ddc_staking::GenesisConfig::<Test> { edges, storages, ..Default::default() }
		.assimilate_storage(&mut storage);

	TestExternalities::new(storage)
}

pub type Extrinsic = TestXt<RuntimeCall, ()>;

impl SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> CreateSignedTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(RuntimeCall, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}
