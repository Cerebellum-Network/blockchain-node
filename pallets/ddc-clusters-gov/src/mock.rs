//! Test utilities

#![allow(dead_code)]

use std::cell::RefCell;

use ddc_primitives::{
	traits::{
		pallet::{GetDdcOrigin, PalletsOriginOf},
		SeatsConsensus,
	},
	ClusterId, ClusterNodeKind, ClusterParams, ClusterProtocolParams, NodeParams, NodePubKey,
	StorageNodeParams, DOLLARS,
};
use frame_support::{
	parameter_types,
	traits::{
		ConstBool, ConstU32, ConstU64, EnsureOriginWithArg, EqualPrivilegeOnly, Everything, Nothing,
	},
	weights::constants::RocksDbWeight,
	PalletId,
};
use frame_system::{
	mocking::{MockBlock, MockUncheckedExtrinsic},
	EnsureRoot,
};
use lazy_static::lazy_static;
use pallet_ddc_clusters::cluster::Cluster;
use pallet_ddc_nodes::StorageNode;
use pallet_referenda::Curve;
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	traits::{BlakeTwo256, Convert, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature, Perbill,
};

use crate::{self as pallet_ddc_clusters_gov, *};

/// The AccountId alias in this test module.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type AccountIndex = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SECS_PER_BLOCK: u64 = MILLISECS_PER_BLOCK / 1000;
pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
pub const CERE: u128 = 10000000000;

pub type Signature = MultiSignature;
type UncheckedExtrinsic = MockUncheckedExtrinsic<Test>;
type Block = MockBlock<Test>;

frame_support::construct_runtime!(
	pub struct Test
	{
		System: frame_system::{Pallet, Call, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>},
		Referenda: pallet_referenda::{Pallet, Call, Storage, Event<T>},
		ConvictionVoting: pallet_conviction_voting::{Pallet, Call, Storage, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>, HoldReason},
		Randomness: pallet_insecure_randomness_collective_flip::{Pallet, Storage},
		DdcNodes: pallet_ddc_nodes::{Pallet, Call, Storage, Event<T>},
		DdcClusters: pallet_ddc_clusters::{Pallet, Call, Storage, Config<T>, Event<T>},
		DdcStaking: pallet_ddc_staking::{Pallet, Call, Storage, Event<T>},
		Origins: pallet_mock_origins::{Origin},
		DdcClustersGov: pallet_ddc_clusters_gov::{Pallet, Call, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
	}

);

pub type BalanceOf<T> = <<T as pallet_referenda::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

impl Convert<Weight, BalanceOf<Self>> for Test {
	fn convert(w: Weight) -> BalanceOf<Self> {
		w.ref_time().into()
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
	type RuntimeTask = RuntimeTask;
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

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
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
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = 0;
	pub const PreimageByteDeposit: Balance = 0;
}

impl pallet_preimage::Config for Test {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = ();
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = DOLLARS;
	pub const UndecidingTimeout: BlockNumber = 5 * MINUTES;
}

impl pallet_referenda::Config for Test {
	type WeightInfo = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	type SubmitOrigin = EnsureOfPermittedReferendaOrigin<Self>;
	type CancelOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;
	type Slash = ();
	type Votes = pallet_conviction_voting::VotesOf<Test>;
	type Tally = pallet_conviction_voting::TallyOf<Test>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
	type Preimages = Preimage;
}

parameter_types! {
	pub const VoteLockingPeriod: BlockNumber = 3 * MINUTES;
}

impl pallet_conviction_voting::Config for Test {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type VoteLockingPeriod = VoteLockingPeriod;
	type MaxVotes = ConstU32<512>;
	type MaxTurnout = frame_support::traits::TotalIssuanceOf<Balances, Self::AccountId>;
	type Polls = Referenda;
}

impl pallet_scheduler::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = ();
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = ConstU32<512>;
	type WeightInfo = ();
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
}

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

impl pallet_contracts::Config for Test {
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
	type MaxDelegateDependencies = MaxDelegateDependencies;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Debug = ();
	type Environment = ();
	type Migrations = ();
	type Xcm = ();
}

impl pallet_insecure_randomness_collective_flip::Config for Test {}

impl pallet_ddc_nodes::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type StakingVisitor = pallet_ddc_staking::Pallet<Test>;
	type WeightInfo = ();
}

impl pallet_ddc_clusters::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type NodeRepository = pallet_ddc_nodes::Pallet<Test>;
	type StakingVisitor = pallet_ddc_staking::Pallet<Test>;
	type StakerCreator = pallet_ddc_staking::Pallet<Test>;
	type Currency = Balances;
	type WeightInfo = ();
	type MinErasureCodingRequiredLimit = ConstU32<0>;
	type MinErasureCodingTotalLimit = ConstU32<0>;
	type MinReplicationTotalLimit = ConstU32<0>;
}

parameter_types! {
	pub const ClusterBondingAmount: Balance = DOLLARS;
	pub const ClusterUnboningDelay: BlockNumber = MINUTES;
}

impl pallet_ddc_staking::Config for Test {
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type ClusterProtocol = pallet_ddc_clusters::Pallet<Test>;
	type ClusterCreator = pallet_ddc_clusters::Pallet<Test>;
	type ClusterManager = pallet_ddc_clusters::Pallet<Test>;
	type NodeVisitor = pallet_ddc_nodes::Pallet<Test>;
	type NodeCreator = pallet_ddc_nodes::Pallet<Test>;
	type ClusterBondingAmount = ClusterBondingAmount;
	type ClusterUnboningDelay = ClusterUnboningDelay;
}

impl pallet_mock_origins::Config for Test {}

parameter_types! {
	pub const ClustersGovPalletId: PalletId = PalletId(*b"clustgov");
	pub const ClusterProposalDuration: BlockNumber = MINUTES;
	pub const MinValidatedNodesCount: u16 = 3;
	pub ClusterProtocolActivatorTrackOrigin: RuntimeOrigin = pallet_mock_origins::Origin::ClusterProtocolActivator.into();
	pub ClusterProtocolUpdaterTrackOrigin: RuntimeOrigin = pallet_mock_origins::Origin::ClusterProtocolUpdater.into();
	pub const ReferendumEnactmentDuration: BlockNumber = 1;
}

impl crate::pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = ClustersGovPalletId;
	type Currency = Balances;
	type WeightInfo = ();
	type OpenGovActivatorTrackOrigin = DdcOriginAsNative<ClusterProtocolActivatorTrackOrigin, Self>;
	type OpenGovActivatorOrigin = pallet_mock_origins::ClusterProtocolActivator;
	type OpenGovUpdaterTrackOrigin = DdcOriginAsNative<ClusterProtocolUpdaterTrackOrigin, Self>;
	type OpenGovUpdaterOrigin = pallet_mock_origins::ClusterProtocolUpdater;
	type ClusterProposalCall = RuntimeCall;
	type ClusterProposalDuration = ClusterProposalDuration;
	type ClusterManager = pallet_ddc_clusters::Pallet<Test>;
	type ClusterCreator = pallet_ddc_clusters::Pallet<Test>;
	type ClusterProtocol = pallet_ddc_clusters::Pallet<Test>;
	type NodeVisitor = pallet_ddc_nodes::Pallet<Test>;
	type SeatsConsensus = MockedSeatsConsensus;
	type DefaultVote = MockedDefaultVote; // pallet_ddc_clusters_gov::PrimeDefaultVote;
	type MinValidatedNodesCount = MinValidatedNodesCount;
	type ReferendumEnactmentDuration = ReferendumEnactmentDuration;
	#[cfg(feature = "runtime-benchmarks")]
	type NodeCreator = pallet_ddc_nodes::Pallet<Test>;
	#[cfg(feature = "runtime-benchmarks")]
	type StakerCreator = pallet_ddc_staking::Pallet<Test>;
}

pub enum DefaultVoteVariant {
	NayAsDefaultVote,
	PrimeDefaultVote,
}

lazy_static! {
	// We have to use the ReentrantMutex as every test's thread that needs to perform some configuration on the mock acquires the lock at least 2 times:
	// the first time when the mock configuration happens, and
	// the second time when the pallet calls the MockedDefaultVote during execution
	static ref MOCK_DEFAULT_VOTE: ReentrantMutex<RefCell<MockDefaultVote>> =
		ReentrantMutex::new(RefCell::new(MockDefaultVote::default()));
}
pub struct MockDefaultVote {
	pub strategy: DefaultVoteVariant,
}

impl Default for MockDefaultVote {
	fn default() -> Self {
		Self { strategy: DefaultVoteVariant::PrimeDefaultVote }
	}
}

pub struct MockedDefaultVote;
impl MockedDefaultVote {
	// Every test's thread must hold the lock till the end of its test
	pub fn set_and_hold_lock(
		mock: MockDefaultVote,
	) -> ReentrantMutexGuard<'static, RefCell<MockDefaultVote>> {
		let lock = MOCK_DEFAULT_VOTE.lock();
		*lock.borrow_mut() = mock;
		lock
	}

	// Every test's thread must release the lock that it previously acquired in the end of its
	// test
	pub fn reset_and_release_lock(lock: ReentrantMutexGuard<'static, RefCell<MockDefaultVote>>) {
		*lock.borrow_mut() = MockDefaultVote::default();
	}
}

impl DefaultVote for MockedDefaultVote {
	fn default_vote(
		prime_vote: Option<bool>,
		yes_votes: MemberCount,
		no_votes: MemberCount,
		len: MemberCount,
	) -> bool {
		let lock = MOCK_DEFAULT_VOTE.lock();
		let mock_ref = lock.borrow();
		match mock_ref.strategy {
			DefaultVoteVariant::NayAsDefaultVote =>
				NayAsDefaultVote::default_vote(prime_vote, yes_votes, no_votes, len),
			DefaultVoteVariant::PrimeDefaultVote =>
				PrimeDefaultVote::default_vote(prime_vote, yes_votes, no_votes, len),
		}
	}
}

pub enum ConsensusVariant {
	Supermajority,
	Unanimous,
}

lazy_static! {
	// We have to use the ReentrantMutex as every test's thread that needs to perform some configuration on the mock acquires the lock at least 2 times:
	// the first time when the mock configuration happens, and
	// the second time when the pallet calls the MockedSeatsConsensus during execution
	static ref MOCK_SEATS_CONSENSUS: ReentrantMutex<RefCell<MockSeatsConsensus>> =
		ReentrantMutex::new(RefCell::new(MockSeatsConsensus::default()));
}
pub struct MockSeatsConsensus {
	pub consensus: ConsensusVariant,
}

impl Default for MockSeatsConsensus {
	fn default() -> Self {
		Self { consensus: ConsensusVariant::Supermajority }
	}
}

pub struct MockedSeatsConsensus;
impl MockedSeatsConsensus {
	// Every test's thread must hold the lock till the end of its test
	pub fn set_and_hold_lock(
		mock: MockSeatsConsensus,
	) -> ReentrantMutexGuard<'static, RefCell<MockSeatsConsensus>> {
		let lock = MOCK_SEATS_CONSENSUS.lock();
		*lock.borrow_mut() = mock;
		lock
	}

	// Every test's thread must release the lock that it previously acquired in the end of its
	// test
	pub fn reset_and_release_lock(lock: ReentrantMutexGuard<'static, RefCell<MockSeatsConsensus>>) {
		*lock.borrow_mut() = MockSeatsConsensus::default();
	}
}

impl SeatsConsensus for MockedSeatsConsensus {
	fn get_threshold(seats: MemberCount) -> MemberCount {
		let lock = MOCK_SEATS_CONSENSUS.lock();
		let mock_ref = lock.borrow();
		match mock_ref.consensus {
			ConsensusVariant::Supermajority => Supermajority::get_threshold(seats),
			ConsensusVariant::Unanimous => Unanimous::get_threshold(seats),
		}
	}
}

pub struct DdcOriginAsNative<DdcOrigin, RuntimeOrigin>(PhantomData<(DdcOrigin, RuntimeOrigin)>);
impl<DdcOrigin: Get<T::RuntimeOrigin>, T: frame_system::Config> GetDdcOrigin<T>
	for DdcOriginAsNative<DdcOrigin, T>
{
	fn get() -> T::RuntimeOrigin {
		DdcOrigin::get()
	}
}

pub struct EnsureOfPermittedReferendaOrigin<T>(PhantomData<T>);
impl<T: frame_system::Config> EnsureOriginWithArg<T::RuntimeOrigin, PalletsOriginOf<T>>
	for EnsureOfPermittedReferendaOrigin<T>
where
	<T as frame_system::Config>::RuntimeOrigin: OriginTrait<PalletsOrigin = OriginCaller>,
{
	type Success = T::AccountId;

	fn try_origin(
		o: T::RuntimeOrigin,
		proposal_origin: &PalletsOriginOf<T>,
	) -> Result<Self::Success, T::RuntimeOrigin> {
		let origin = <frame_system::EnsureSigned<_> as EnsureOrigin<_>>::try_origin(o.clone())?;

		let track_id =
			match <TracksInfo as pallet_referenda::TracksInfo<Balance, BlockNumber>>::track_for(
				proposal_origin,
			) {
				Ok(track_id) => track_id,
				Err(_) => return Err(o),
			};

		if track_id == CLUSTER_PROTOCOL_ACTIVATOR_TRACK_ID ||
			track_id == CLUSTER_PROTOCOL_UPDATER_TRACK_ID
		{
			let clusters_governance = ClustersGovPalletId::get().into_account_truncating();
			if origin == clusters_governance {
				Ok(origin)
			} else {
				Err(o)
			}
		} else {
			Ok(origin)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(
		_proposal_origin: &PalletsOriginOf<T>,
	) -> Result<T::RuntimeOrigin, ()> {
		let origin = frame_benchmarking::account::<T::AccountId>("successful_origin", 0, 0);
		Ok(frame_system::RawOrigin::Signed(origin).into())
	}
}

const fn percent(x: i32) -> sp_arithmetic::FixedI64 {
	sp_arithmetic::FixedI64::from_rational(x as u128, 100)
}

const APP_CLUSTER_PROTOCOL_ACTIVATOR: Curve = Curve::make_linear(10, 28, percent(0), percent(10));
const SUP_CLUSTER_PROTOCOL_ACTIVATOR: Curve =
	Curve::make_reciprocal(1, 28, percent(4), percent(0), percent(10));
const APP_CLUSTER_PROTOCOL_UPDATER: Curve = Curve::make_linear(10, 28, percent(0), percent(10));
const SUP_CLUSTER_PROTOCOL_UPDATER: Curve =
	Curve::make_reciprocal(1, 28, percent(4), percent(0), percent(10));

pub const CLUSTER_PROTOCOL_ACTIVATOR_DECISION_DEPOSIT: Balance = 30 * DOLLARS;
pub const CLUSTER_PROTOCOL_UPDATER_DECISION_DEPOSIT: Balance = 20 * DOLLARS;

pub const CLUSTER_PROTOCOL_ACTIVATOR_TRACK_ID: u16 = 100;
pub const CLUSTER_PROTOCOL_UPDATER_TRACK_ID: u16 = 101;

const TRACKS_DATA: [(u16, pallet_referenda::TrackInfo<Balance, BlockNumber>); 2] = [
	(
		CLUSTER_PROTOCOL_ACTIVATOR_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "cluster_protocol_activator",
			max_deciding: 50,
			decision_deposit: CLUSTER_PROTOCOL_ACTIVATOR_DECISION_DEPOSIT,
			prepare_period: 0,
			decision_period: MINUTES / 2,
			confirm_period: MINUTES / 4,
			min_enactment_period: 0,
			min_approval: APP_CLUSTER_PROTOCOL_ACTIVATOR,
			min_support: SUP_CLUSTER_PROTOCOL_ACTIVATOR,
		},
	),
	(
		CLUSTER_PROTOCOL_UPDATER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "cluster_protocol_updater",
			max_deciding: 50,
			decision_deposit: CLUSTER_PROTOCOL_UPDATER_DECISION_DEPOSIT,
			prepare_period: 0,
			decision_period: MINUTES / 2,
			confirm_period: MINUTES / 4,
			min_enactment_period: 0,
			min_approval: APP_CLUSTER_PROTOCOL_UPDATER,
			min_support: SUP_CLUSTER_PROTOCOL_UPDATER,
		},
	),
];

pub struct TracksInfo;
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = u16;
	type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;
	fn tracks() -> &'static [(Self::Id, pallet_referenda::TrackInfo<Balance, BlockNumber>)] {
		&TRACKS_DATA[..]
	}
	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		if let Ok(custom_origin) = pallet_mock_origins::Origin::try_from(id.clone()) {
			match custom_origin {
				pallet_mock_origins::Origin::ClusterProtocolActivator =>
					Ok(CLUSTER_PROTOCOL_ACTIVATOR_TRACK_ID),
				pallet_mock_origins::Origin::ClusterProtocolUpdater =>
					Ok(CLUSTER_PROTOCOL_UPDATER_TRACK_ID),
			}
		} else {
			Err(())
		}
	}
}
pallet_referenda::impl_tracksinfo_get!(TracksInfo, Balance, BlockNumber);

#[allow(unused_imports)]
#[frame_support::pallet]
mod pallet_mock_origins {
	use frame_support::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[derive(PartialEq, Eq, Clone, MaxEncodedLen, Encode, Decode, TypeInfo, RuntimeDebug)]
	#[pallet::origin]
	pub enum Origin {
		ClusterProtocolActivator,
		ClusterProtocolUpdater,
	}

	macro_rules! decl_unit_ensures {
		( $name:ident: $success_type:ty = $success:expr ) => {
			pub struct $name;
			impl<O: Into<Result<Origin, O>> + From<Origin>>
				EnsureOrigin<O> for $name
			{
				type Success = $success_type;
				fn try_origin(o: O) -> Result<Self::Success, O> {
					o.into().and_then(|o| match o {
						Origin::$name => Ok($success),
						r => Err(O::from(r)),
					})
				}
				#[cfg(feature = "runtime-benchmarks")]
				fn try_successful_origin() -> Result<O, ()> {
					Ok(O::from(Origin::$name))
				}
			}
		};
		( $name:ident ) => { decl_unit_ensures! { $name : () = () } };
		( $name:ident: $success_type:ty = $success:expr, $( $rest:tt )* ) => {
			decl_unit_ensures! { $name: $success_type = $success }
			decl_unit_ensures! { $( $rest )* }
		};
		( $name:ident, $( $rest:tt )* ) => {
			decl_unit_ensures! { $name }
			decl_unit_ensures! { $( $rest )* }
		};
		() => {}
	}
	decl_unit_ensures!(ClusterProtocolActivator, ClusterProtocolUpdater,);

	#[allow(unused_macros)]
	macro_rules! decl_ensure {
		(
			$vis:vis type $name:ident: EnsureOrigin<Success = $success_type:ty> {
				$( $item:ident = $success:expr, )*
			}
		) => {
			$vis struct $name;
			impl<O: Into<Result<Origin, O>> + From<Origin>>
				EnsureOrigin<O> for $name
			{
				type Success = $success_type;
				fn try_origin(o: O) -> Result<Self::Success, O> {
					o.into().and_then(|o| match o {
						$(
							Origin::$item => Ok($success),
						)*
						r => Err(O::from(r)),
					})
				}
				#[cfg(feature = "runtime-benchmarks")]
				fn try_successful_origin() -> Result<O, ()> {
					// By convention the more privileged origins go later, so for greatest chance
					// of success, we want the last one.
					let _result: Result<O, ()> = Err(());
					$(
						let _result: Result<O, ()> = Ok(O::from(Origin::$item));
					)*
					_result
				}
			}
		}
	}
}

pub const CLUSTER_ID: [u8; 20] = [1; 20];
pub const CLUSTER_MANAGER_ID: [u8; 32] = [1; 32];
pub const CLUSTER_RESERVE_ID: [u8; 32] = [2; 32];

pub const NODE_PROVIDER_ID_1: [u8; 32] = [11; 32];
pub const NODE_PROVIDER_ID_2: [u8; 32] = [12; 32];
pub const NODE_PROVIDER_ID_3: [u8; 32] = [13; 32];

pub const NODE_PUB_KEY_1: [u8; 32] = [111; 32];
pub const NODE_PUB_KEY_2: [u8; 32] = [112; 32];
pub const NODE_PUB_KEY_3: [u8; 32] = [113; 32];

pub const ENDOWMENT: u128 = 1000 * CERE;

#[allow(clippy::type_complexity)]
pub type BuiltCluster = (Cluster<AccountId>, ClusterProtocolParams<Balance, BlockNumber>);
#[allow(clippy::type_complexity)]
pub type BuiltNode = (NodePubKey, StorageNode<Test>, ClusterNodeStatus, ClusterNodeKind);

pub fn build_cluster(
	cluster_id: [u8; 20],
	manager_id: [u8; 32],
	reserve_id: [u8; 32],
	params: ClusterParams<AccountId>,
	protocol_params: ClusterProtocolParams<Balance, BlockNumber>,
	status: ClusterStatus,
) -> BuiltCluster {
	let mut cluster = Cluster::new(
		ClusterId::from(cluster_id),
		AccountId::from(manager_id),
		AccountId::from(reserve_id),
		params,
	);
	cluster.status = status;
	(cluster, protocol_params)
}

pub fn build_cluster_node(
	pub_key: [u8; 32],
	provider_id: [u8; 32],
	params: StorageNodeParams,
	cluster_id: [u8; 20],
	status: ClusterNodeStatus,
	kind: ClusterNodeKind,
) -> BuiltNode {
	let key = NodePubKey::StoragePubKey(AccountId::from(pub_key));
	let mut node = StorageNode::new(
		key.clone(),
		AccountId::from(provider_id),
		NodeParams::StorageParams(params),
	)
	.unwrap();
	node.cluster_id = Some(ClusterId::from(cluster_id));
	(key, node, status, kind)
}

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self, cluster: BuiltCluster, cluster_nodes: Vec<BuiltNode>) -> TestExternalities {
		sp_tracing::try_init_simple();
		let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		let mut balances: Vec<(AccountId, Balance)> = Vec::new();
		let mut storage_nodes: Vec<StorageNode<Test>> = Vec::new();
		let mut cluster_storage_nodes: Vec<(NodePubKey, ClusterNodeKind, ClusterNodeStatus)> =
			Vec::new();

		for (pub_key, node, status, kind) in cluster_nodes.iter() {
			balances.push((node.provider_id.clone(), ENDOWMENT));
			cluster_storage_nodes.push((pub_key.clone(), kind.clone(), status.clone()));
			storage_nodes.push(node.clone());
		}

		let (clust, cluster_protocol_params) = cluster;
		balances.push((clust.manager_id.clone(), ENDOWMENT));
		balances.push((clust.reserve_id.clone(), ENDOWMENT));

		// endow system account to allow dispatching transaction
		balances.push((DdcClustersGov::account_id(), ENDOWMENT));

		let _ =
			pallet_balances::GenesisConfig::<Test> { balances }.assimilate_storage(&mut storage);

		let _ = pallet_ddc_nodes::GenesisConfig::<Test> { storage_nodes }
			.assimilate_storage(&mut storage);

		let _ = pallet_ddc_clusters::GenesisConfig::<Test> {
			clusters: vec![clust.clone()],
			clusters_protocol_params: vec![(clust.cluster_id, cluster_protocol_params)],
			clusters_nodes: vec![(clust.cluster_id, cluster_storage_nodes)],
		}
		.assimilate_storage(&mut storage);

		TestExternalities::new(storage)
	}

	pub fn build_and_execute(
		self,
		cluster: BuiltCluster,
		cluster_nodes: Vec<BuiltNode>,
		test: impl FnOnce(),
	) {
		sp_tracing::try_init_simple();
		let mut ext = self.build(cluster, cluster_nodes);
		ext.execute_with(test);
	}
}
