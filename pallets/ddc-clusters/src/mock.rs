//! Test utilities

#![allow(dead_code)]

use ddc_primitives::{
	traits::staking::{StakerCreator, StakingVisitor, StakingVisitorError},
	ClusterId, NodePubKey,
};
use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstBool, ConstU32, ConstU64, Nothing},
};
use frame_system::{
	mocking::{MockBlock, MockUncheckedExtrinsic},
	EnsureSigned,
};
use pallet_contracts as contracts;
use sp_io::TestExternalities;
use sp_runtime::{
	testing::TestXt,
	traits::{Convert, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, DispatchResult, MultiSignature, Perbill, Perquintill,
};

use crate::{self as pallet_ddc_clusters, *};

/// The AccountId alias in this test module.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

pub type Signature = MultiSignature;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Block = MockBlock<Test>;

construct_runtime!(
	pub enum Test
	{
		Contracts: contracts::{Pallet, Call, Storage, Event<T>, HoldReason},
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		DdcNodes: pallet_ddc_nodes::{Pallet, Call, Storage, Event<T>},
		DdcClusters: pallet_ddc_clusters::{Pallet, Call, Storage, Event<T>},
		Randomness: pallet_insecure_randomness_collective_flip::{Pallet, Storage},
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
	pub static DefaultDepositLimit: Balance = 10_000_000;
	pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(0);
	pub const MaxDelegateDependencies: u32 = 32;
}

impl Convert<Weight, BalanceOf<Self>> for Test {
	fn convert(w: Weight) -> BalanceOf<Self> {
		w.ref_time().into()
	}
}

type BalanceOf<T> = <<T as crate::pallet::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

impl contracts::Config for Test {
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

use frame_system::offchain::SigningTypes;

pub type Extrinsic = TestXt<RuntimeCall, ()>;

impl SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::CreateTransactionBase<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	type RuntimeCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}
impl pallet_insecure_randomness_collective_flip::Config for Test {}

impl<LocalCall> frame_system::offchain::CreateTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	type Extension = ();

	fn create_transaction(call: RuntimeCall, _extension: Self::Extension) -> Extrinsic {
		Extrinsic::new_transaction(call, ())
	}
}
impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_signed_transaction<
		C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>,
	>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<Extrinsic> {
		Some(Extrinsic::new_signed(call, nonce, (), ()))
	}
}

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Nonce = u64;
	type Block = Block;
	type AccountId = AccountId;
	type BlockHashCount = ConstU64<250>;
	type AccountData = pallet_balances::AccountData<Balance>;
	type MaxConsumers = ConstU32<16>;
	type Lookup = IdentityLookup<Self::AccountId>;
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
	type StakerCreator = TestStaker;
	type WeightInfo = ();
	type MinErasureCodingRequiredLimit = ConstU32<4>;
	type MinErasureCodingTotalLimit = ConstU32<6>;
	type MinReplicationTotalLimit = ConstU32<3>;
}

pub(crate) type DdcStakingCall = crate::Call<Test>;
pub(crate) type TestRuntimeCall = <Test as frame_system::Config>::RuntimeCall;
pub struct TestStakingVisitor;
pub struct TestStaker;

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
	fn stash_by_ctrl(_controller: &T::AccountId) -> Result<T::AccountId, StakingVisitorError> {
		todo!()
	}
}

impl<T: Config> StakerCreator<T, BalanceOf<T>> for TestStaker {
	fn bond_stake_and_participate(
		_stash: T::AccountId,
		_controller: T::AccountId,
		_node: NodePubKey,
		_value: BalanceOf<T>,
		_cluster_id: ClusterId,
	) -> DispatchResult {
		Ok(())
	}

	fn bond_cluster(
		_cluster_stash: T::AccountId,
		_cluster_controller: T::AccountId,
		_cluster_id: ClusterId,
	) -> DispatchResult {
		Ok(())
	}
}

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> TestExternalities {
		sp_tracing::try_init_simple();

		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		let _ = pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(AccountId::from([1; 32]), 100),
				(AccountId::from([2; 32]), 100),
				(AccountId::from([3; 32]), 100),
				(AccountId::from([4; 32]), 100),
			],
			..Default::default()
		}
		.assimilate_storage(&mut t);

		let cluster_protocol_params = ClusterProtocolParams {
			treasury_share: Perquintill::from_float(0.05),
			validators_share: Perquintill::from_float(0.01),
			cluster_reserve_share: Perquintill::from_float(0.02),
			storage_bond_size: 100,
			storage_chill_delay: 50,
			storage_unbonding_delay: 50,
			unit_per_mb_stored: 10,
			unit_per_mb_streamed: 10,
			unit_per_put_request: 10,
			unit_per_get_request: 10,
		};

		let node_pub_key = NodePubKey::StoragePubKey(AccountId::from([0; 32]));

		let cluster = Cluster::new(
			ClusterId::from([0; 20]),
			AccountId::from([0; 32]),
			AccountId::from([0; 32]),
			ClusterParams {
				node_provider_auth_contract: Some(AccountId::from([0; 32])),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3,
			},
		);

		let _ = pallet_ddc_clusters::GenesisConfig::<Test> {
			clusters: vec![cluster],
			clusters_protocol_params: vec![(ClusterId::from([0; 20]), cluster_protocol_params)],
			clusters_nodes: vec![(
				ClusterId::from([0; 20]),
				vec![(
					node_pub_key,
					ClusterNodeKind::Genesis,
					ClusterNodeStatus::ValidationSucceeded,
				)],
			)],
		}
		.assimilate_storage(&mut t);

		TestExternalities::new(t)
	}
	pub fn build_and_execute(self, test: impl FnOnce()) {
		sp_tracing::try_init_simple();
		let mut ext = self.build();
		ext.execute_with(test);
	}
}
