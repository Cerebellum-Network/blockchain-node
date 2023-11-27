//! Test utilities

#![allow(dead_code)]

use crate::{self as pallet_ddc_clusters, *};
use ddc_primitives::{ClusterId, NodePubKey};
use ddc_traits::staking::{StakingVisitor, StakingVisitorError};
use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU32, ConstU64, Everything, Nothing},
	weights::constants::RocksDbWeight,
};
use frame_system::mocking::{MockBlock, MockUncheckedExtrinsic};
use pallet_contracts as contracts;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{
		BlakeTwo256, Convert, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify,
	},
	MultiSignature,
};

/// The AccountId alias in this test module.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

pub type Signature = MultiSignature;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Block = MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		Contracts: contracts::{Pallet, Call, Storage, Event<T>},
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		DdcNodes: pallet_ddc_nodes::{Pallet, Call, Storage, Event<T>},
		DdcClusters: pallet_ddc_clusters::{Pallet, Call, Storage, Event<T>},
		Randomness: pallet_randomness_collective_flip::{Pallet, Storage},
	}
);

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
}

impl Convert<Weight, BalanceOf<Self>> for Test {
	fn convert(w: Weight) -> BalanceOf<Self> {
		w.ref_time().into()
	}
}

use contracts::Config as contractsConfig;

type BalanceOf<T> =
	<<T as contractsConfig>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

impl contracts::Config for Test {
	type Time = Timestamp;
	type Randomness = Randomness;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type CallStack = [pallet_contracts::Frame<Self>; 31];
	type WeightPrice = Self; //pallet_transaction_payment::Module<Self>;
	type WeightInfo = ();
	type ChainExtension = ();
	type DeletionQueueDepth = ();
	type DeletionWeightLimit = ();
	type Schedule = Schedule;
	type RuntimeCall = RuntimeCall;
	type CallFilter = Nothing;
	type DepositPerByte = DepositPerByte;
	type DepositPerItem = DepositPerItem;
	type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
	type MaxCodeLen = ConstU32<{ 128 * 1024 }>;
	type MaxStorageKeyLen = ConstU32<128>;
}

use frame_system::offchain::{
	AppCrypto, CreateSignedTransaction, SendTransactionTypes, SigningTypes,
};

pub type Extrinsic = TestXt<RuntimeCall, ()>;

impl SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> SendTransactionTypes<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}
impl pallet_randomness_collective_flip::Config for Test {}

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

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

impl pallet_ddc_nodes::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type StakingVisitor = TestStakingVisitor;
	type WeightInfo = ();
}

impl crate::pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type NodeRepository = DdcNodes;
	type StakingVisitor = TestStakingVisitor;
}

pub(crate) type DdcStakingCall = crate::Call<Test>;
pub(crate) type TestRuntimeCall = <Test as frame_system::Config>::RuntimeCall;
pub struct TestStakingVisitor;
impl<T: Config> StakingVisitor<T> for TestStakingVisitor {
	fn has_activated_stake(
		_node_pub_key: &NodePubKey,
		_cluster_id: &ClusterId,
	) -> Result<bool, StakingVisitorError> {
		Ok(true)
	}
	fn has_stake(_node_pub_key: &NodePubKey) -> bool {
		true
	}
	fn has_chilling_attempt(_node_pub_key: &NodePubKey) -> Result<bool, StakingVisitorError> {
		Ok(false)
	}
}

pub struct ExtBuilder;

impl ExtBuilder {
	fn build(self) -> TestExternalities {
		sp_tracing::try_init_simple();
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let _ = pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(AccountId::from([1; 32]), 100),
				(AccountId::from([2; 32]), 100),
				(AccountId::from([3; 32]), 100),
				(AccountId::from([4; 32]), 100),
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
