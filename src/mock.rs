use crate::{*, self as pallet_ddc_validator};
use frame_support::{
    parameter_types,
    weights::Weight,
    traits::{ConstU16, ConstU64, Currency, Everything, Nothing}
};
use frame_system::EnsureRoot;
use frame_system::offchain::SendTransactionTypes;
use sp_core::H256;
use sp_runtime::{ curve::PiecewiseLinear, generic, testing::{Header, TestXt}, traits::{BlakeTwo256, IdentityLookup, Verify, Extrinsic as ExtrinsicT, IdentifyAccount}, Perbill, MultiSignature, curve};
use pallet_contracts as contracts;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
// type AccountId = u64;

pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
// pub type Balance = u128;
pub type BlockNumber = u32;
pub type Moment = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
        Contracts: contracts,
        Session: pallet_session,
        Timestamp: pallet_timestamp,
        RandomnessCollectiveFlip: pallet_randomness_collective_flip,
        DdcStaking: pallet_ddc_staking,
        DdcValidator: pallet_ddc_validator,
	}
);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Call = Call;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    // u64; // sp_core::sr25519::Public;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    type Event = Event;
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

use contracts::{Config as contractsConfig};

type BalanceOf<T> = <<T as contractsConfig>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

impl contracts::Config for Test {
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Currency = Balances;
    type Event = Event;
    type CallStack = [pallet_contracts::Frame<Self>; 31];
    type WeightPrice = Self; //pallet_transaction_payment::Module<Self>;
    type WeightInfo = ();
    type ChainExtension = ();
    type DeletionQueueDepth = ();
    type DeletionWeightLimit = ();
    type Schedule = Schedule;
    type Call = Call;
    type CallFilter = Nothing;
    type DepositPerByte = DepositPerByte;
    type DepositPerItem = DepositPerItem;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
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

parameter_types! {
	pub const DdcValidatorsQuorumSize: u32 = 3;
}

impl pallet_session::Config for Test {
    type Event = ();
    type ValidatorId = AccountId;
    type ValidatorIdOf = ();
    type ShouldEndSession = ();
    type NextSessionRotation = ();
    type SessionManager = ();
    type SessionHandler = ();
    type Keys = ();
    type WeightInfo = ();
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
    type MaxNominations = ();
    type Currency = Balances;
    type UnixTime = Timestamp;
    type CurrencyToVote = ();
    type RewardRemainder = ();
    type Event = Event;
    type Slash = (); // send the slashed funds to the treasury.
    type Reward = (); // rewards are minted from the void
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    /// A super-majority of the council can cancel the slash.
    type SlashCancelOrigin = ();
    type SessionInterface = Self;
    type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
    type NextNewSession = Session;
    type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
    type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
    type ElectionProvider = ();
    type GenesisElectionProvider = ();
    type VoterList = ();
    type MaxUnlockingChunks = ConstU32<32>;
    type WeightInfo = pallet_staking::weights::SubstrateWeight<Test>;
    type BenchmarkingConfig = ();
}

impl pallet_ddc_staking::Config for Test {
    type BondingDuration = BondingDuration;
    type Currency = Balances;
    type Event = Event;
    type WeightInfo = pallet_ddc_staking::weights::SubstrateWeight<Test>;
}

impl pallet_ddc_validator::Config for Test {
    type DdcValidatorsQuorumSize = DdcValidatorsQuorumSize;
    type Event = Event;
    type Randomness = RandomnessCollectiveFlip;
    type Call = Call;
    type AuthorityId = pallet_ddc_validator::crypto::TestAuthId;
    type TimeProvider = pallet_timestamp::Pallet<Test>;
}

impl<LocalCall> SendTransactionTypes<LocalCall> for Test
    where
        Call: From<LocalCall>,
{
    type OverarchingCall = Call;
    type Extrinsic = Extrinsic;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxLocks: u32 = 10;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

pub type Extrinsic = TestXt<Call, ()>;

impl SigningTypes for Test {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

// impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
//     where
//         Call: From<C>,
// {
//     type OverarchingCall = Call;
//     type Extrinsic = TestXt<Call, ()>;
// }

// impl<LocalCall> SendTransactionTypes<LocalCall> for Test
//     where
//         Call: From<LocalCall>,
// {
//     type OverarchingCall = Call;
//     type Extrinsic = Extrinsic;
// }

impl<LocalCall> CreateSignedTransaction<LocalCall> for Test
    where
        Call: From<LocalCall>,
{
    fn create_transaction<C: AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        _public: <Signature as Verify>::Signer,
        _account: AccountId,
        nonce: u64,
    ) -> Option<(Call, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
        Some((call, (nonce, ())))
    }
}