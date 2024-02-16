//! Test utilities

#![allow(dead_code)]

use ddc_primitives::{
	traits::{
		pallet::{GetDdcOrigin, PalletsOriginOf},
		SeatsConsensus,
	},
	ClusterGovParams, ClusterId, ClusterNodeKind, ClusterParams, NodeParams, NodePubKey,
	StorageNodeParams, DOLLARS,
};
use frame_support::{
	construct_runtime, parameter_types,
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
use pallet_ddc_clusters::cluster::Cluster;
use pallet_ddc_nodes::StorageNode;
use pallet_referenda::Curve;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, Convert, IdentifyAccount, IdentityLookup, Verify},
	MultiSignature,
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

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>},
		Referenda: pallet_referenda::{Pallet, Call, Storage, Event<T>},
		ConvictionVoting: pallet_conviction_voting::{Pallet, Call, Storage, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>},
		Randomness: pallet_insecure_randomness_collective_flip::{Pallet, Storage},
		DdcNodes: pallet_ddc_nodes::{Pallet, Call, Storage, Event<T>},
		DdcClusters: pallet_ddc_clusters::{Pallet, Call, Storage, Event<T>},
		DdcStaking: pallet_ddc_staking::{Pallet, Call, Storage, Event<T>},
		Origins: pallet_custom_origins::{Origin},
		DdcClustersGov: pallet_ddc_clusters_gov::{Pallet, Call, Storage, Event<T>},
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
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = 1 * DOLLARS;
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
	pub Schedule: pallet_contracts::Schedule<Test> = Default::default();
}

impl pallet_contracts::Config for Test {
	type Time = Timestamp;
	type Randomness = Randomness;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type CallStack = [pallet_contracts::Frame<Self>; 5];
	type WeightPrice = Self;
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
	type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
	type MaxStorageKeyLen = ConstU32<128>;
	type UnsafeUnstableInterface = ConstBool<false>;
	type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
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
}

parameter_types! {
	pub const ClusterBondingAmount: Balance = 1 * DOLLARS;
	pub const ClusterUnboningDelay: BlockNumber = 1 * MINUTES;
}

impl pallet_ddc_staking::Config for Test {
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type ClusterEconomics = pallet_ddc_clusters::Pallet<Test>;
	type ClusterCreator = pallet_ddc_clusters::Pallet<Test>;
	type ClusterManager = pallet_ddc_clusters::Pallet<Test>;
	type NodeVisitor = pallet_ddc_nodes::Pallet<Test>;
	type NodeCreator = pallet_ddc_nodes::Pallet<Test>;
	type ClusterBondingAmount = ClusterBondingAmount;
	type ClusterUnboningDelay = ClusterUnboningDelay;
}

impl pallet_custom_origins::Config for Test {}

parameter_types! {
	pub const ClustersGovPalletId: PalletId = PalletId(*b"clustgov");
	pub const ClusterProposalDuration: BlockNumber = 1 * MINUTES;
	pub const MinValidatedNodesCount: u16 = 3;
	pub ClusterActivatorTrackOrigin: RuntimeOrigin = pallet_custom_origins::Origin::ClusterActivator.into();
	pub ClusterEconomicsUpdaterTrackOrigin: RuntimeOrigin = pallet_custom_origins::Origin::ClusterEconomicsUpdater.into();
	pub const ReferendumEnactmentDuration: BlockNumber = 1;
}

impl crate::pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = ClustersGovPalletId;
	type Currency = Balances;
	type WeightInfo = ();
	type OpenGovActivatorTrackOrigin = DdcOriginAsNative<ClusterActivatorTrackOrigin, Self>;
	type OpenGovActivatorOrigin = pallet_custom_origins::ClusterActivator;
	type OpenGovUpdaterTrackOrigin = DdcOriginAsNative<ClusterEconomicsUpdaterTrackOrigin, Self>;
	type OpenGovUpdaterOrigin = pallet_custom_origins::ClusterEconomicsUpdater;
	type ClusterProposalCall = RuntimeCall;
	type ClusterProposalDuration = ClusterProposalDuration;
	type ClusterManager = pallet_ddc_clusters::Pallet<Test>;
	type ClusterCreator = pallet_ddc_clusters::Pallet<Test>;
	type ClusterEconomics = pallet_ddc_clusters::Pallet<Test>;
	type NodeVisitor = pallet_ddc_nodes::Pallet<Test>;
	type SeatsConsensus = pallet_ddc_clusters_gov::Supermajority;
	type DefaultVote = pallet_ddc_clusters_gov::PrimeDefaultVote;
	type MinValidatedNodesCount = MinValidatedNodesCount;
	type ReferendumEnactmentDuration = ReferendumEnactmentDuration;
	#[cfg(feature = "runtime-benchmarks")]
	type NodeCreator = pallet_ddc_nodes::Pallet<Test>;
	#[cfg(feature = "runtime-benchmarks")]
	type StakerCreator = pallet_ddc_staking::Pallet<Test>;
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

		if track_id == CLUSTER_ACTIVATOR_TRACK_ID || track_id == CLUSTER_ECONOMICS_UPDATER_TRACK_ID
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
	fn try_successful_origin(proposal_origin: &PalletsOriginOf<T>) -> Result<T::RuntimeOrigin, ()> {
		let origin = frame_benchmarking::account::<T::AccountId>("successful_origin", 0, 0);
		Ok(frame_system::RawOrigin::Signed(origin).into())
	}
}

const fn percent(x: i32) -> sp_arithmetic::FixedI64 {
	sp_arithmetic::FixedI64::from_rational(x as u128, 100)
}

const APP_CLUSTER_ACTIVATOR: Curve = Curve::make_linear(10, 28, percent(0), percent(10));
const SUP_CLUSTER_ACTIVATOR: Curve =
	Curve::make_reciprocal(1, 28, percent(4), percent(0), percent(10));
const APP_CLUSTER_ECONOMICS_UPDATER: Curve = Curve::make_linear(10, 28, percent(0), percent(10));
const SUP_CLUSTER_ECONOMICS_UPDATER: Curve =
	Curve::make_reciprocal(1, 28, percent(4), percent(0), percent(10));

pub const CLUSTER_ACTIVATOR_DECISION_DEPOSIT: Balance = 30 * DOLLARS;
pub const CLUSTER_ECONOMICS_UPDATE_DECISION_DEPOSIT: Balance = 20 * DOLLARS;

const TRACKS_DATA: [(u16, pallet_referenda::TrackInfo<Balance, BlockNumber>); 2] = [
	(
		100,
		pallet_referenda::TrackInfo {
			name: "cluster_activator",
			max_deciding: 50,
			decision_deposit: CLUSTER_ACTIVATOR_DECISION_DEPOSIT,
			prepare_period: 0,
			decision_period: MINUTES / 2,
			confirm_period: MINUTES / 4,
			min_enactment_period: 0,
			min_approval: APP_CLUSTER_ACTIVATOR,
			min_support: SUP_CLUSTER_ACTIVATOR,
		},
	),
	(
		101,
		pallet_referenda::TrackInfo {
			name: "cluster_economics_updater",
			max_deciding: 50,
			decision_deposit: CLUSTER_ECONOMICS_UPDATE_DECISION_DEPOSIT,
			prepare_period: 0,
			decision_period: MINUTES / 2,
			confirm_period: MINUTES / 4,
			min_enactment_period: 0,
			min_approval: APP_CLUSTER_ECONOMICS_UPDATER,
			min_support: SUP_CLUSTER_ECONOMICS_UPDATER,
		},
	),
];

pub const CLUSTER_ACTIVATOR_TRACK_ID: u16 = 100;
pub const CLUSTER_ECONOMICS_UPDATER_TRACK_ID: u16 = 101;
pub struct TracksInfo;
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = u16;
	type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;
	fn tracks() -> &'static [(Self::Id, pallet_referenda::TrackInfo<Balance, BlockNumber>)] {
		&TRACKS_DATA[..]
	}
	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		if let Ok(custom_origin) = pallet_custom_origins::Origin::try_from(id.clone()) {
			match custom_origin {
				pallet_custom_origins::Origin::ClusterActivator => Ok(CLUSTER_ACTIVATOR_TRACK_ID),
				pallet_custom_origins::Origin::ClusterEconomicsUpdater =>
					Ok(CLUSTER_ECONOMICS_UPDATER_TRACK_ID),
			}
		} else {
			Err(())
		}
	}
}
pallet_referenda::impl_tracksinfo_get!(TracksInfo, Balance, BlockNumber);

#[frame_support::pallet]
mod pallet_custom_origins {
	use frame_support::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[derive(PartialEq, Eq, Clone, MaxEncodedLen, Encode, Decode, TypeInfo, RuntimeDebug)]
	#[pallet::origin]
	pub enum Origin {
		ClusterActivator,
		ClusterEconomicsUpdater,
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
	decl_unit_ensures!(ClusterActivator, ClusterEconomicsUpdater,);

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
pub type BuiltCluster = (Cluster<AccountId>, ClusterGovParams<Balance, BlockNumber>);
#[allow(clippy::type_complexity)]
pub type BuiltNode = (NodePubKey, StorageNode<Test>, ClusterNodeStatus, ClusterNodeKind);

pub fn build_cluster(
	cluster_id: [u8; 20],
	manager_id: [u8; 32],
	reserve_id: [u8; 32],
	params: ClusterParams<AccountId>,
	economic_params: ClusterGovParams<Balance, BlockNumber>,
	status: ClusterStatus,
) -> BuiltCluster {
	let mut cluster = Cluster::new(
		ClusterId::from(cluster_id),
		AccountId::from(manager_id),
		AccountId::from(reserve_id),
		params,
	);
	cluster.status = status;
	(cluster, economic_params)
}

pub fn build_cluster_node(
	pub_key: [u8; 32],
	provider_id: [u8; 32],
	params: StorageNodeParams,
	cluster_id: [u8; 20],
	status: ClusterNodeStatus,
	kind: ClusterNodeKind,
) -> BuiltNode {
	let key = NodePubKey::StoragePubKey(AccountId::from(pub_key.clone()));
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
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		let mut balances: Vec<(AccountId, Balance)> = Vec::new();
		let mut storage_nodes: Vec<StorageNode<Test>> = Vec::new();
		let mut cluster_storage_nodes: Vec<(NodePubKey, ClusterNodeKind, ClusterNodeStatus)> =
			Vec::new();

		for (pub_key, node, status, kind) in cluster_nodes.iter() {
			balances.push((node.provider_id.clone(), ENDOWMENT));
			cluster_storage_nodes.push((pub_key.clone(), kind.clone(), status.clone()));
			storage_nodes.push(node.clone());
		}

		let (clust, cluster_gov_params) = cluster;
		balances.push((clust.manager_id.clone(), ENDOWMENT));
		balances.push((clust.reserve_id.clone(), ENDOWMENT));

		let _ =
			pallet_balances::GenesisConfig::<Test> { balances }.assimilate_storage(&mut storage);

		let _ = pallet_ddc_nodes::GenesisConfig::<Test> { storage_nodes }
			.assimilate_storage(&mut storage);

		let _ = pallet_ddc_clusters::GenesisConfig::<Test> {
			clusters: vec![clust.clone()],
			clusters_gov_params: vec![(clust.cluster_id.clone(), cluster_gov_params)],
			clusters_nodes: vec![(clust.cluster_id.clone(), cluster_storage_nodes)],
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
