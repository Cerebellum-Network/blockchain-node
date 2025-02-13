// This file is part of Substrate.

// Copyright (C) 2018-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! The Substrate runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "512"]
use codec::{Decode, Encode, MaxEncodedLen};
use ddc_primitives::{
	traits::pallet::{GetDdcOrigin, PalletVisitor},
	AccountIndex, Balance, BlockNumber, Hash, Moment, Nonce, MAX_PAYOUT_BATCH_COUNT,
	MAX_PAYOUT_BATCH_SIZE,
};
pub use ddc_primitives::{AccountId, Signature};
use frame_election_provider_support::{
	bounds::ElectionBoundsBuilder, onchain, BalancingConfig, SequentialPhragmen, VoteWeight,
};
use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	pallet_prelude::Get,
	parameter_types,
	traits::{
		fungible::HoldConsideration,
		fungibles,
		fungibles::{Dust, Inspect, Unbalanced},
		tokens::{
			DepositConsequence, Fortitude, PayFromAccount, Preservation, Provenance,
			UnityAssetBalanceConversion, WithdrawConsequence,
		},
		ConstBool, ConstU128, ConstU16, ConstU32, Currency, EitherOf, EitherOfDiverse,
		EqualPrivilegeOnly, Imbalance, InstanceFilter, KeyOwnerProofSystem, LinearStoragePrice,
		Nothing, OnUnbalanced, VariantCountOf, WithdrawReasons,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		ConstantMultiplier, IdentityFee, Weight,
	},
	PalletId,
};
#[cfg(any(feature = "std", test))]
pub use frame_system::Call as SystemCall;
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureSigned,
};
#[cfg(any(feature = "std", test))]
pub use pallet_balances::Call as BalancesCall;
pub use pallet_chainbridge;
use pallet_contracts::Determinism;
use pallet_election_provider_multi_phase::SolutionAccuracyOf;
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical::{self as pallet_session_historical};
#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus;
#[cfg(any(feature = "std", test))]
pub use pallet_sudo::Call as SudoCall;
#[allow(deprecated)]
pub use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
use sp_api::impl_runtime_apis;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_core::{
	crypto::{AccountId32, KeyTypeId},
	OpaqueMetadata,
};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_io::hashing::blake2_128;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
	create_runtime_str,
	curve::PiecewiseLinear,
	generic, impl_opaque_keys,
	traits::{
		self, AccountIdConversion, BlakeTwo256, Block as BlockT, Bounded, Convert, ConvertInto,
		Identity as IdentityConvert, IdentityLookup, NumberFor, OpaqueKeys, SaturatedConversion,
		StaticLookup, Verify,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, DispatchError, DispatchResult, FixedPointNumber, FixedU128, Perbill,
	Percent, Permill, Perquintill, RuntimeDebug,
};
use sp_std::prelude::*;
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use static_assertions::const_assert;
/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;
/// Constant values used within the runtime.
use cere_runtime_common::{
	constants::{currency::*, time::*},
	CurrencyToVote,
};
use impls::Author;
use pallet_identity::legacy::IdentityInfo;
use sp_runtime::generic::Era;
use sp_std::marker::PhantomData;

// Governance configurations.
pub mod governance;
use governance::{
	ClusterProtocolActivator, ClusterProtocolUpdater, GeneralAdmin, StakingAdmin, Treasurer,
	TreasurySpender,
};
/// Generated voter bag information.
mod voter_bags;
use ismp::{
	consensus::{ConsensusClientId, StateMachineHeight, StateMachineId},
	host::StateMachine,
	router::{Request, Response},
};
use sp_core::H256;
mod hyperbridge_ismp;
mod weights;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. This means the client is built with \
		 `SKIP_WASM_BUILD` flag and it is only usable for production chains. Please rebuild with \
		 the flag disabled.",
	)
}

/// Runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("node"),
	impl_name: create_runtime_str!("substrate-node"),
	authoring_version: 10,
	// Per convention: if the runtime behavior changes, increment spec_version
	// and set impl_version to 0. If only runtime
	// implementation changes and behavior does not, then leave spec_version as
	// is and increment impl_version.
	spec_version: 73000,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 24,
	state_version: 0,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY,
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
	};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees, 50% to treasury, 50% to author
			let mut split = fees.ration(50, 50);
			if let Some(tips) = fees_then_tips.next() {
				// for tips, if any, 50% to treasury, 50% to author (though this can be anything)
				tips.ration_merge_into(50, 50, &mut split);
			}
			Treasury::on_unbalanced(split.0);
			Author::on_unbalanced(split.1);
		}
	}
}

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time, with maximum proof size.
const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

const_assert!(NORMAL_DISPATCH_RATIO.deconstruct() >= AVERAGE_ON_INITIALIZE_RATIO.deconstruct());

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type DbWeight = RocksDbWeight;
	type RuntimeTask = RuntimeTask;
	type Nonce = Nonce;
	type Hash = Hash;
	type AccountId = AccountId;
	type Lookup = Indices;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type Version = Version;
	type AccountData = pallet_balances::AccountData<Balance>;
	type SS58Prefix = ConstU16<54>;
	type MaxConsumers = ConstU32<16>;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u32 = 100;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const AnnouncementDepositBase: Balance = deposit(1, 8);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	Any,
	NonTransfer,
	Governance,
	Staking,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => !matches!(
				c,
				RuntimeCall::Balances(..) |
					RuntimeCall::Vesting(pallet_vesting::Call::vested_transfer { .. }) |
					RuntimeCall::Indices(pallet_indices::Call::transfer { .. }) |
					RuntimeCall::NominationPools(..) |
					RuntimeCall::ConvictionVoting(..) |
					RuntimeCall::Referenda(..) |
					RuntimeCall::Whitelist(..)
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Treasury(..) |
					RuntimeCall::ConvictionVoting(..) |
					RuntimeCall::Referenda(..) |
					RuntimeCall::Whitelist(..)
			),
			ProxyType::Staking => matches!(c, RuntimeCall::Staking(..)),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = ConstU32<32>;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
	type MaxPending = ConstU32<32>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const PreimageByteDeposit: Balance = deposit(0, 1);
	pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
	>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EitherOf<EnsureRoot<AccountId>, Treasurer>;
	type MaxScheduledPerBlock = ConstU32<512>;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
}

parameter_types! {
	// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type MaxNominators = MaxNominatorRewardedPerValidator;

	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;

	type EquivocationReportSystem =
		pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;

	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub const IndexDeposit: Balance = 10 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_indices::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = DOLLARS;
	// For weight estimation, we assume that the most locks on an individual account will be 50.
	// This number may need to be adjusted in the future if this assumption no longer holds true.
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Runtime>;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type RuntimeHoldReason = RuntimeHoldReason;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
	pub MaximumMultiplier: Multiplier = Bounded::max_value();
}

// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	#[allow(deprecated)]
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = TargetedFeeAdjustment<
		Self,
		TargetBlockFullness,
		AdjustmentVariable,
		MinimumMultiplier,
		MaximumMultiplier,
	>;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

impl_opaque_keys! {
	pub struct OldSessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub im_online: ImOnline,
		pub authority_discovery: AuthorityDiscovery,
	}
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub im_online: ImOnline,
		pub authority_discovery: AuthorityDiscovery,
		pub ddc_verification: DdcVerification,
	}
}

fn transform_session_keys(v: AccountId, old: OldSessionKeys) -> SessionKeys {
	SessionKeys {
		grandpa: old.grandpa,
		babe: old.babe,
		im_online: old.im_online,
		authority_discovery: old.authority_discovery,
		ddc_verification: {
			let mut id: ddc_primitives::sr25519::AuthorityId =
				sp_core::sr25519::Public::from_raw([0u8; 32]).into();
			let id_raw: &mut [u8] = id.as_mut();
			id_raw[0..32].copy_from_slice(v.as_ref());
			id_raw[0..4].copy_from_slice(b"cer!");
			id
		},
	}
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
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
	pub const MaxExposurePageSize: u32 = 512;
	pub const MaxNominatorRewardedPerValidator: u32 = 512;
	pub OffchainRepeat: BlockNumber = 5;
	pub HistoryDepth: u32 = 84;
	// 16
	pub const MaxNominations: u32 = <NposSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;
	pub const MaxControllersInDeprecationBatch: u32 = 5900;

}

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxNominators = ConstU32<1000>;
	type MaxValidators = ConstU32<1000>;
}

impl pallet_staking::Config for Runtime {
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = CurrencyToVote;
	type RewardRemainder = Treasury;
	type RuntimeEvent = RuntimeEvent;
	type Slash = Treasury; // send the slashed funds to the treasury.
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = EitherOf<EnsureRoot<Self::AccountId>, StakingAdmin>;
	type SessionInterface = Self;
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type MaxExposurePageSize = MaxExposurePageSize;
	type NextNewSession = Session;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type VoterList = VoterList;
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type MaxUnlockingChunks = ConstU32<32>;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type HistoryDepth = HistoryDepth;
	type EventListeners = NominationPools;
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<{ MaxNominations::get() }>;
	type DisablingStrategy = pallet_staking::UpToLimitDisablingStrategy;
}

impl pallet_fast_unstake::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ControlOrigin = EnsureRoot<AccountId>;
	type Deposit = ConstU128<{ DOLLARS }>;
	type Currency = Balances;
	type BatchSize = frame_support::traits::ConstU32<64>;
	type Staking = Staking;
	type MaxErasToCheckPerBlock = ConstU32<1>;
	type WeightInfo = ();
}

parameter_types! {
	// phase durations. 1/4 of the last session for each.
	pub const SignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;
	pub const UnsignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;

	// signed config
	pub const SignedRewardBase: Balance = DOLLARS;
	pub const SignedDepositByte: Balance = CENTS;


	// miner configs
	pub const MultiPhaseUnsignedPriority: TransactionPriority = StakingUnsignedPriority::get() - 1u64;
	pub MinerMaxWeight: Weight = RuntimeBlockWeights::get()
		.get(DispatchClass::Normal)
		.max_extrinsic.expect("Normal extrinsics have a weight limit configured; qed")
		.saturating_sub(BlockExecutionWeight::get());
	// Solution can occupy 90% of normal block size
	pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
		*RuntimeBlockLength::get()
		.max
		.get(DispatchClass::Normal);

	/// We take the top 10000 nominators as electing voters..
	pub const MaxElectingVoters: u32 = 10_000;
	/// ... and all of the validators as electable targets. Whilst this is the case, we cannot and
	/// shall not increase the size of the validator intentions.
	pub const MaxElectableTargets: u16 = u16::MAX;
	/// Setup election pallet to support maximum winners upto 1200. This will mean Staking Pallet
	/// cannot have active validators higher than this count.
	pub const MaxActiveValidators: u32 = 1200;
	/// We take the top 22500 nominators as electing voters and all of the validators as electable
	/// targets. Whilst this is the case, we cannot and shall not increase the size of the
	/// validator intentions.
	pub ElectionBounds: frame_election_provider_support::bounds::ElectionBounds =
		ElectionBoundsBuilder::default().voters_count(MaxElectingVoters::get().into()).build();
}

frame_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = MaxElectingVoters,
	>(16)
);

/// The numbers configured here could always be more than the the maximum limits of staking pallet
/// to ensure election snapshot will not run out of memory. For now, we set them to smaller values
/// since the staking is bounded and the weight pipeline takes hours for this single pallet.
pub struct ElectionProviderBenchmarkConfig;
impl pallet_election_provider_multi_phase::BenchmarkingConfig for ElectionProviderBenchmarkConfig {
	const VOTERS: [u32; 2] = [1000, 2000];
	const TARGETS: [u32; 2] = [500, 1000];
	const ACTIVE_VOTERS: [u32; 2] = [500, 800];
	const DESIRED_TARGETS: [u32; 2] = [200, 400];
	const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
	const MINER_MAXIMUM_VOTERS: u32 = 1000;
	const MAXIMUM_TARGETS: u32 = 300;
}

/// Maximum number of iterations for balancing that will be executed in the embedded OCW
/// miner of election provider multi phase.
pub const MINER_MAX_ITERATIONS: u32 = 10;

/// A source of random balance for NposSolver, which is meant to be run by the OCW election miner.
pub struct OffchainRandomBalancing;
impl Get<Option<BalancingConfig>> for OffchainRandomBalancing {
	fn get() -> Option<BalancingConfig> {
		use sp_runtime::traits::TrailingZeroInput;
		let iterations = match MINER_MAX_ITERATIONS {
			0 => 0,
			max => {
				let seed = sp_io::offchain::random_seed();
				let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
					.expect("input is padded with zeroes; qed") %
					max.saturating_add(1);
				random as usize
			},
		};

		let config = BalancingConfig { iterations, tolerance: 0 };
		Some(config)
	}
}
pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Runtime>,
	>;
	type DataProvider = <Runtime as pallet_election_provider_multi_phase::Config>::DataProvider;
	type MaxWinners = MaxActiveValidators;
	type Bounds = ElectionBounds;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = MinerMaxLength;
	type MaxWeight = MinerMaxWeight;
	type Solution = NposSolution16;
	type MaxVotesPerVoter = <
	<Self as pallet_election_provider_multi_phase::Config>::DataProvider
	as
	frame_election_provider_support::ElectionDataProvider
	>::MaxVotesPerVoter;
	type MaxWinners = MaxActiveValidators;

	// The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
	// weight estimate function is wired to this call's weight.
	fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
		<
		<Self as pallet_election_provider_multi_phase::Config>::WeightInfo
		as
		pallet_election_provider_multi_phase::WeightInfo
		>::submit_unsigned(v, t, a, d)
	}
}

/// Returning a fixed value to respect the initial logic.
/// This could depend on the length of the solution.
pub struct FixedSignedDepositBase;
impl Convert<usize, u128> for FixedSignedDepositBase {
	fn convert(_: usize) -> u128 {
		DOLLARS
	}
}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = SignedPhase;
	type UnsignedPhase = UnsignedPhase;
	type BetterSignedThreshold = ();
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = MultiPhaseUnsignedPriority;
	type SignedMaxWeight =
		<Self::MinerConfig as pallet_election_provider_multi_phase::MinerConfig>::MaxWeight;
	type MinerConfig = Self;
	type SignedMaxSubmissions = ConstU32<10>;
	type SignedRewardBase = SignedRewardBase;
	type SignedDepositBase = FixedSignedDepositBase;
	type SignedDepositByte = SignedDepositByte;
	type SignedMaxRefunds = ConstU32<3>;
	type SignedDepositWeight = ();
	type SlashHandler = (); // burn slashes
	type RewardHandler = (); // nothing to do upon rewards
	type DataProvider = Staking;
	type Fallback = frame_election_provider_support::NoElection<(
		AccountId,
		BlockNumber,
		Staking,
		MaxActiveValidators,
	)>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, OffchainRandomBalancing>;
	type ForceOrigin = EitherOf<EnsureRoot<Self::AccountId>, StakingAdmin>;
	type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
	type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Self>;
	type MaxWinners = MaxActiveValidators;
	type ElectionBounds = ElectionBounds;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &voter_bags::THRESHOLDS;
}

type VoterBagsListInstance = pallet_bags_list::Instance1;
impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ScoreProvider = Staking;
	type WeightInfo = pallet_bags_list::weights::SubstrateWeight<Runtime>;
	type BagThresholds = BagThresholds;
	type Score = VoteWeight;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 50_000 * DOLLARS;
	pub const SpendPeriod: BlockNumber = DAYS;
	pub const Burn: Permill = Permill::from_parts(25000);
	pub const TipCountdown: BlockNumber = DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 50_000 * DOLLARS;
	pub const DataDepositPerByte: Balance = DOLLARS;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const MaximumReasonLength: u32 = 16384;
	pub const MaxApprovals: u32 = 100;
}

parameter_types! {
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = Bounties;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type MaxApprovals = MaxApprovals;
	type SpendOrigin = TreasurySpender;
	type AssetKind = ();
	type Beneficiary = Self::AccountId;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type BalanceConverter = UnityAssetBalanceConversion;
	type PayoutPeriod = ConstU32<10>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: Balance = 10 * DOLLARS;
	pub const BountyDepositBase: Balance = 50_000 * DOLLARS;
	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub const CuratorDepositMin: Balance = DOLLARS;
	pub const CuratorDepositMax: Balance = 100 * DOLLARS;
	pub const BountyDepositPayoutDelay: BlockNumber = 8 * DAYS;
	pub const BountyUpdatePeriod: BlockNumber = 90 * DAYS;
}

impl pallet_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type BountyValueMinimum = BountyValueMinimum;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
	type ChildBountyManager = ChildBounties;
	type OnSlash = Treasury;
}

parameter_types! {
	pub const ChildBountyValueMinimum: Balance = DOLLARS;
}

impl pallet_child_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxActiveChildBountyCount = ConstU32<5>;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type WeightInfo = pallet_child_bounties::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	pub const DefaultDepositLimit: Balance = deposit(1024, 1024 * 1024);
	pub const MaxValueSize: u32 = 16 * 1024;
	// The lazy deletion runs inside on_initialize.
	pub DeletionWeightLimit: Weight = RuntimeBlockWeights::get()
		.per_class
		.get(DispatchClass::Normal)
		.max_total
		.unwrap_or(RuntimeBlockWeights::get().max_block);
	pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
	pub UnsafeUnstableInterface: bool = false;
	pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(0);
	pub const MaxDelegateDependencies: u32 = 32;
}

impl pallet_contracts::Config for Runtime {
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	/// The safest default is to allow no calls at all.
	///
	/// Runtimes should whitelist dispatchables that are allowed to be called from contracts
	/// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
	/// change because that would break already deployed contracts. The `Call` structure itself
	/// is not allowed to change the indices of existing pallets, too.
	type CallFilter = Nothing;
	type DepositPerItem = DepositPerItem;
	type DepositPerByte = DepositPerByte;
	type DefaultDepositLimit = DefaultDepositLimit;
	type CallStack = [pallet_contracts::Frame<Self>; 5];
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
	type ChainExtension = ();
	type Schedule = Schedule;
	type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
	type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
	type MaxStorageKeyLen = ConstU32<128>;
	type UnsafeUnstableInterface = ConstBool<false>;
	type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type MaxTransientStorageSize = ConstU32<{ 1024 * 1024 }>;
	type MaxDelegateDependencies = MaxDelegateDependencies;
	type RuntimeHoldReason = RuntimeHoldReason;
	type UploadOrigin = EnsureSigned<Self::AccountId>;
	type InstantiateOrigin = EnsureSigned<Self::AccountId>;
	type Debug = ();
	type Environment = ();
	type Migrations = ();
	type ApiVersion = ();
	type Xcm = ();
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::MAX;
	/// We prioritize im-online heartbeats over election solution submission.
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::MAX / 2;
	pub const MaxAuthorities: u32 = 100;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		public: <Signature as traits::Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(RuntimeCall, <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload)> {
		let tip = 0;
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let era = Era::mortal(period, current_block);
		let extra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(era),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			frame_metadata_hash_extension::CheckMetadataHash::new(false),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = Indices::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature, extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type RuntimeEvent = RuntimeEvent;
	type NextSessionRotation = Babe;
	type ValidatorSet = Historical;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

impl pallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

	type EquivocationReportSystem =
		pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;

	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type MaxNominators = MaxNominatorRewardedPerValidator;
}

parameter_types! {
	pub const BasicDeposit: Balance = 10 * DOLLARS;       // 258 bytes on-chain
	pub const ByteDeposit: Balance = deposit(0, 1);
	pub const SubAccountDeposit: Balance = 2 * DOLLARS;   // 53 bytes on-chain
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type ByteDeposit = ByteDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = EitherOf<EnsureRoot<Self::AccountId>, GeneralAdmin>;
	type RegistrarOrigin = EitherOf<EnsureRoot<Self::AccountId>, GeneralAdmin>;
	type OffchainSignature = Signature;
	type SigningPublicKey = <Signature as Verify>::Signer;
	type UsernameAuthorityOrigin = EitherOf<EnsureRoot<Self::AccountId>, GeneralAdmin>;
	type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
	type MaxSuffixLength = ConstU32<7>;
	type MaxUsernameLength = ConstU32<32>;
	type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ConfigDepositBase: Balance = 5 * DOLLARS;
	pub const FriendDepositFactor: Balance = 50 * CENTS;
	pub const MaxFriends: u16 = 9;
	pub const RecoveryDeposit: Balance = 5 * DOLLARS;
}

impl pallet_recovery::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_recovery::weights::SubstrateWeight<Runtime>;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ConfigDepositBase = ConfigDepositBase;
	type FriendDepositFactor = FriendDepositFactor;
	type MaxFriends = MaxFriends;
	type RecoveryDeposit = RecoveryDeposit;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = DOLLARS;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	// `VestingInfo` encode length is 36bytes. 28 schedules gets encoded as 1009 bytes, which is the
	// highest number of schedules that encodes less than 2^10.
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
	pub const ChainId: u8 = 1;
	pub const ProposalLifetime: BlockNumber = 1000;
	pub BridgeAccountId: AccountId = AccountIdConversion::<AccountId>::into_account_truncating(&pallet_chainbridge::MODULE_ID);
}

/// Configure the send data pallet
impl pallet_chainbridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type Proposal = RuntimeCall;
	type ChainIdentity = ChainId;
	type ProposalLifetime = ProposalLifetime;
	type BridgeAccountId = BridgeAccountId;
	type WeightInfo = pallet_chainbridge::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub HashId: pallet_chainbridge::ResourceId = pallet_chainbridge::derive_resource_id(1, &blake2_128(b"hash"));
	// Note: Chain ID is 0 indicating this is native to another chain
	pub NativeTokenId: pallet_chainbridge::ResourceId = pallet_chainbridge::derive_resource_id(0, &blake2_128(b"DAV"));

	pub NFTTokenId: pallet_chainbridge::ResourceId = pallet_chainbridge::derive_resource_id(1, &blake2_128(b"NFT"));
}

impl pallet_erc721::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Identifier = NFTTokenId;
	type WeightInfo = pallet_erc721::weights::SubstrateWeight<Runtime>;
}

impl pallet_erc20::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgeOrigin = pallet_chainbridge::EnsureBridge<Runtime>;
	type Currency = pallet_balances::Pallet<Runtime>;
	type HashId = HashId;
	type NativeTokenId = NativeTokenId;
	type Erc721Id = NFTTokenId;
	type WeightInfo = pallet_erc20::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
	// Allow pools that got slashed up to 90% to remain operational.
	pub const MaxPointsToBalance: u8 = 10;
}

impl pallet_nomination_pools::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RewardCounter = FixedU128;
	type BalanceToU256 = cere_runtime_common::BalanceToU256;
	type U256ToBalance = cere_runtime_common::U256ToBalance;
	type PostUnbondingPoolsWindow = frame_support::traits::ConstU32<4>;
	type MaxMetadataLen = frame_support::traits::ConstU32<256>;
	// we use the same number of allowed unlocking chunks as with staking.
	type MaxUnbonding = <Self as pallet_staking::Config>::MaxUnlockingChunks;
	type PalletId = PoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type WeightInfo = ();
	type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type StakeAdapter = pallet_nomination_pools::adapter::TransferStake<Self, Staking>;
}

parameter_types! {
	pub const DdcCustomersPalletId: PalletId = PalletId(*b"accounts"); // DDC maintainer's stake
	pub const UnlockingDelay: BlockNumber = 100800_u32; // 1 hour * 24 * 7 = 7 days; (1 hour is 600 blocks)
}

impl pallet_ddc_customers::Config for Runtime {
	type UnlockingDelay = UnlockingDelay;
	type Currency = Balances;
	type PalletId = DdcCustomersPalletId;
	type RuntimeEvent = RuntimeEvent;
	type ClusterProtocol = pallet_ddc_clusters::Pallet<Runtime>;
	type ClusterCreator = pallet_ddc_clusters::Pallet<Runtime>;
	type WeightInfo = pallet_ddc_customers::weights::SubstrateWeight<Runtime>;
}

impl pallet_ddc_clusters::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type NodeRepository = pallet_ddc_nodes::Pallet<Runtime>;
	type StakingVisitor = pallet_ddc_staking::Pallet<Runtime>;
	type StakerCreator = pallet_ddc_staking::Pallet<Runtime>;
	type Currency = Balances;
	type WeightInfo = pallet_ddc_clusters::weights::SubstrateWeight<Runtime>;
	type MinErasureCodingRequiredLimit = ConstU32<4>;
	type MinErasureCodingTotalLimit = ConstU32<6>;
	type MinReplicationTotalLimit = ConstU32<3>;
}

impl pallet_ddc_nodes::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StakingVisitor = pallet_ddc_staking::Pallet<Runtime>;
	type WeightInfo = pallet_ddc_nodes::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const PayoutsPalletId: PalletId = PalletId(*b"payouts_");
}

pub struct TreasuryWrapper;
impl<T: frame_system::Config> PalletVisitor<T> for TreasuryWrapper {
	fn get_account_id() -> T::AccountId {
		TreasuryPalletId::get().into_account_truncating()
	}
}

impl pallet_ddc_payouts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = PayoutsPalletId;
	type Currency = Balances;
	type CustomerCharger = DdcCustomers;
	type BucketManager = DdcCustomers;
	type ClusterProtocol = DdcClusters;
	type TreasuryVisitor = TreasuryWrapper;
	type NominatorsAndValidatorsList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type VoteScoreToU64 = IdentityConvert;
	type ValidatorVisitor = pallet_ddc_verification::Pallet<Runtime>;
	type NodeManager = pallet_ddc_nodes::Pallet<Runtime>;
	type AccountIdConverter = AccountId32;
	type Hasher = BlakeTwo256;
	type ClusterValidator = pallet_ddc_clusters::Pallet<Runtime>;
	type ValidatorsQuorum = MajorityOfValidators;
}

parameter_types! {
	pub const ClusterBondingAmount: Balance = 100 * GRAND;
	pub const ClusterUnboningDelay: BlockNumber = 28 * DAYS;
}

impl pallet_ddc_staking::Config for Runtime {
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_ddc_staking::weights::SubstrateWeight<Runtime>;
	type ClusterProtocol = pallet_ddc_clusters::Pallet<Runtime>;
	type ClusterCreator = pallet_ddc_clusters::Pallet<Runtime>;
	type ClusterManager = pallet_ddc_clusters::Pallet<Runtime>;
	type NodeManager = pallet_ddc_nodes::Pallet<Runtime>;
	type ClusterBondingAmount = ClusterBondingAmount;
	type ClusterUnboningDelay = ClusterUnboningDelay;
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 5 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

type TechCommCollective = pallet_collective::Instance3;
impl pallet_collective::Config<TechCommCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type SetMembersOrigin = EnsureRoot<AccountId>;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
}

parameter_types! {
	pub const ClustersGovPalletId: PalletId = PalletId(*b"clustgov");
	pub const ClusterProposalDuration: BlockNumber = 7 * DAYS;
	pub const MinValidatedNodesCount: u16 = 3;
	pub ClusterProtocolActivatorTrackOrigin: RuntimeOrigin = pallet_origins::Origin::ClusterProtocolActivator.into();
	pub ClusterProtocolUpdaterTrackOrigin: RuntimeOrigin = pallet_origins::Origin::ClusterProtocolUpdater.into();
	pub const ReferendumEnactmentDuration: BlockNumber = 1;
}

impl pallet_ddc_clusters_gov::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = ClustersGovPalletId;
	type Currency = Balances;
	type WeightInfo = pallet_ddc_clusters_gov::weights::SubstrateWeight<Runtime>;
	type OpenGovActivatorTrackOrigin = DdcOriginAsNative<ClusterProtocolActivatorTrackOrigin, Self>;
	type OpenGovActivatorOrigin = EitherOf<EnsureRoot<Self::AccountId>, ClusterProtocolActivator>;
	type OpenGovUpdaterTrackOrigin = DdcOriginAsNative<ClusterProtocolUpdaterTrackOrigin, Self>;
	type OpenGovUpdaterOrigin = EitherOf<EnsureRoot<Self::AccountId>, ClusterProtocolUpdater>;
	type ClusterProposalCall = RuntimeCall;
	type ClusterProposalDuration = ClusterProposalDuration;
	type ClusterManager = pallet_ddc_clusters::Pallet<Runtime>;
	type ClusterCreator = pallet_ddc_clusters::Pallet<Runtime>;
	type ClusterProtocol = pallet_ddc_clusters::Pallet<Runtime>;
	type NodeManager = pallet_ddc_nodes::Pallet<Runtime>;
	type SeatsConsensus = pallet_ddc_clusters_gov::Unanimous;
	type DefaultVote = pallet_ddc_clusters_gov::NayAsDefaultVote;
	type MinValidatedNodesCount = MinValidatedNodesCount;
	type ReferendumEnactmentDuration = ReferendumEnactmentDuration;
	#[cfg(feature = "runtime-benchmarks")]
	type StakerCreator = pallet_ddc_staking::Pallet<Runtime>;
}

pub struct ClustersGovWrapper;
impl<T: frame_system::Config> PalletVisitor<T> for ClustersGovWrapper {
	fn get_account_id() -> T::AccountId {
		ClustersGovPalletId::get().into_account_truncating()
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

parameter_types! {
	pub const VerificationPalletId: PalletId = PalletId(*b"verifypa");
	pub const MajorityOfAggregators: Percent = Percent::from_percent(67);
	pub const MajorityOfValidators: Percent = Percent::from_percent(67);
}

impl pallet_ddc_verification::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = VerificationPalletId;
	type WeightInfo = pallet_ddc_verification::weights::SubstrateWeight<Runtime>;
	type ClusterManager = pallet_ddc_clusters::Pallet<Runtime>;
	type ClusterValidator = pallet_ddc_clusters::Pallet<Runtime>;
	type NodeManager = pallet_ddc_nodes::Pallet<Runtime>;
	type PayoutProcessor = pallet_ddc_payouts::Pallet<Runtime>;
	type AuthorityId = ddc_primitives::sr25519::AuthorityId;
	type OffchainIdentifierId = ddc_primitives::crypto::OffchainIdentifierId;
	type Hasher = BlakeTwo256;
	const BLOCK_TO_START: u16 = 1; // every block
	const DAC_REDUNDANCY_FACTOR: u16 = 3;
	type AggregatorsQuorum = MajorityOfAggregators;
	type ValidatorsQuorum = MajorityOfValidators;
	const MAX_PAYOUT_BATCH_SIZE: u16 = MAX_PAYOUT_BATCH_SIZE;
	const MAX_PAYOUT_BATCH_COUNT: u16 = MAX_PAYOUT_BATCH_COUNT;
	type ValidatorStaking = pallet_staking::Pallet<Runtime>;
	type AccountIdConverter = AccountId32;
	type CustomerVisitor = pallet_ddc_customers::Pallet<Runtime>;
	const MAX_MERKLE_NODE_IDENTIFIER: u16 = 3;
	type Currency = Balances;
	const VERIFY_AGGREGATOR_RESPONSE_SIGNATURE: bool = true;
	const DISABLE_PAYOUTS_CUTOFF: bool = false;
	const DEBUG_MODE: bool = false;
	type BucketsStorageUsageProvider = DdcCustomers;
	type NodesStorageUsageProvider = DdcNodes;
	type ClusterProtocol = DdcClusters;
	#[cfg(feature = "runtime-benchmarks")]
	type CustomerDepositor = DdcCustomers;
	#[cfg(feature = "runtime-benchmarks")]
	type ClusterCreator = DdcClusters;
	type BucketManager = DdcCustomers;
}

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system::Pallet<Runtime>;

	#[runtime::pallet_index(1)]
	pub type Utility = pallet_utility::Pallet<Runtime>;

	#[runtime::pallet_index(2)]
	pub type Babe = pallet_babe::Pallet<Runtime>;

	#[runtime::pallet_index(3)]
	pub type Timestamp = pallet_timestamp::Pallet<Runtime>;

	// Authorship must be before session in order to note author in the correct session and era
	// for im-online and staking.
	#[runtime::pallet_index(4)]
	pub type Authorship = pallet_authorship::Pallet<Runtime>;

	#[runtime::pallet_index(5)]
	pub type Indices = pallet_indices::Pallet<Runtime>;

	#[runtime::pallet_index(6)]
	pub type Balances = pallet_balances::Pallet<Runtime>;

	#[runtime::pallet_index(7)]
	pub type TransactionPayment = pallet_transaction_payment::Pallet<Runtime>;

	#[runtime::pallet_index(8)]
	pub type ElectionProviderMultiPhase = pallet_election_provider_multi_phase::Pallet<Runtime>;

	#[runtime::pallet_index(9)]
	pub type Staking = pallet_staking::Pallet<Runtime>;

	#[runtime::pallet_index(10)]
	pub type Session = pallet_session::Pallet<Runtime>;

	#[runtime::pallet_index(11)]
	pub type Grandpa = pallet_grandpa::Pallet<Runtime>;

	#[runtime::pallet_index(12)]
	pub type Treasury = pallet_treasury::Pallet<Runtime>;

	#[runtime::pallet_index(13)]
	pub type Contracts = pallet_contracts::Pallet<Runtime>;

	#[runtime::pallet_index(14)]
	pub type Sudo = pallet_sudo::Pallet<Runtime>;

	#[runtime::pallet_index(15)]
	pub type ImOnline = pallet_im_online::Pallet<Runtime>;

	#[runtime::pallet_index(16)]
	pub type AuthorityDiscovery = pallet_authority_discovery::Pallet<Runtime>;

	#[runtime::pallet_index(17)]
	pub type Offences = pallet_offences::Pallet<Runtime>;

	#[runtime::pallet_index(18)]
	pub type Historical = pallet_session_historical::Pallet<Runtime>;

	#[runtime::pallet_index(19)]
	pub type RandomnessCollectiveFlip = pallet_insecure_randomness_collective_flip::Pallet<Runtime>;

	#[runtime::pallet_index(20)]
	pub type Identity = pallet_identity::Pallet<Runtime>;

	#[runtime::pallet_index(21)]
	pub type Recovery = pallet_recovery::Pallet<Runtime>;

	#[runtime::pallet_index(22)]
	pub type Vesting = pallet_vesting::Pallet<Runtime>;

	#[runtime::pallet_index(23)]
	pub type Preimage = pallet_preimage::Pallet<Runtime>;

	#[runtime::pallet_index(24)]
	pub type Scheduler = pallet_scheduler::Pallet<Runtime>;

	#[runtime::pallet_index(25)]
	pub type Proxy = pallet_proxy::Pallet<Runtime>;

	#[runtime::pallet_index(26)]
	pub type Multisig = pallet_multisig::Pallet<Runtime>;

	#[runtime::pallet_index(27)]
	pub type Bounties = pallet_bounties::Pallet<Runtime>;

	#[runtime::pallet_index(28)]
	pub type VoterList = pallet_bags_list::Pallet<Runtime, Instance1>;

	#[runtime::pallet_index(29)]
	pub type ChildBounties = pallet_child_bounties::Pallet<Runtime>;

	#[runtime::pallet_index(30)]
	pub type NominationPools = pallet_nomination_pools::Pallet<Runtime>;

	#[runtime::pallet_index(31)]
	pub type FastUnstake = pallet_fast_unstake::Pallet<Runtime>;

	#[runtime::pallet_index(32)]
	pub type ChainBridge = pallet_chainbridge::Pallet<Runtime>;

	#[runtime::pallet_index(33)]
	pub type Erc721 = pallet_erc721::Pallet<Runtime>;

	#[runtime::pallet_index(34)]
	pub type Erc20 = pallet_erc20::Pallet<Runtime>;

	#[runtime::pallet_index(35)]
	pub type DdcStaking = pallet_ddc_staking::Pallet<Runtime>;

	#[runtime::pallet_index(36)]
	pub type DdcCustomers = pallet_ddc_customers::Pallet<Runtime>;

	#[runtime::pallet_index(37)]
	pub type DdcNodes = pallet_ddc_nodes::Pallet<Runtime>;

	#[runtime::pallet_index(38)]
	pub type DdcClusters = pallet_ddc_clusters::Pallet<Runtime>;

	#[runtime::pallet_index(39)]
	pub type DdcPayouts = pallet_ddc_payouts::Pallet<Runtime>;

	#[runtime::pallet_index(40)]
	pub type DdcVerification = pallet_ddc_verification::Pallet<Runtime>;

	// Start OpenGov.
	#[runtime::pallet_index(41)]
	pub type ConvictionVoting = pallet_conviction_voting::Pallet<Runtime>;

	#[runtime::pallet_index(42)]
	pub type Referenda = pallet_referenda::Pallet<Runtime>;

	#[runtime::pallet_index(43)]
	pub type Origins = pallet_origins::Pallet<Runtime>;

	#[runtime::pallet_index(44)]
	pub type Whitelist = pallet_whitelist::Pallet<Runtime>;

	// End OpenGov.
	#[runtime::pallet_index(45)]
	pub type TechComm = pallet_collective::Pallet<Runtime, Instance3>;

	#[runtime::pallet_index(46)]
	pub type DdcClustersGov = pallet_ddc_clusters_gov::Pallet<Runtime>;

	#[runtime::pallet_index(47)]
	pub type Ismp = pallet_ismp::Pallet<Runtime>;

	#[runtime::pallet_index(48)]
	pub type IsmpGrandpa = ismp_grandpa::Pallet<Runtime>;

	#[runtime::pallet_index(49)]
	pub type Hyperbridge = pallet_hyperbridge::Pallet<Runtime>;

	#[runtime::pallet_index(50)]
	pub type TokenGateway = pallet_token_gateway::Pallet<Runtime>;
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
///
/// When you change this, you **MUST** modify [`sign`] in `bin/node/testing/src/keyring.rs`!
///
/// [`sign`]: <../../testing/src/keyring.rs.html>
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
// const IDENTITY_MIGRATION_KEY_LIMIT: u64 = u64::MAX; // for `pallet_identity` migration below

/// Runtime migrations
// type Migrations = (
// 	// Migrations related to substrate version upgrades
// 	// pallet_nomination_pools::migration::versioned::V5toV6<Runtime>,
// 	// pallet_nomination_pools::migration::versioned::V6ToV7<Runtime>,
// 	// pallet_nomination_pools::migration::versioned::V7ToV8<Runtime>,
// 	// pallet_staking::migrations::v14::MigrateToV14<Runtime>,
// 	// pallet_grandpa::migrations::MigrateV4ToV5<Runtime>,
// 	// pallet_identity::migration::versioned::V0ToV1<Runtime, IDENTITY_MIGRATION_KEY_LIMIT>,

// 	// The 'Unreleased' migration enables DAC Verification, that atm. is enabled at QANET only.
// 	// Uncomment this line when DAC is ready for TESTNET and MAINNET migrations::Unreleased,
// 	// migrations::Unreleased,
// );

// Migrations for DAC and Payouts on QANET
type Migrations = (
	pallet_ddc_payouts::migrations::v3::MigrateToV3<Runtime>,
	pallet_ddc_verification::migrations::v2::MigrateToV2<Runtime>,
);

pub mod migrations {
	use super::*;

	/// When this is removed, should also remove `OldSessionKeys`.
	pub struct UpgradeSessionKeys;
	impl frame_support::traits::OnRuntimeUpgrade for UpgradeSessionKeys {
		fn on_runtime_upgrade() -> Weight {
			Session::upgrade_keys::<OldSessionKeys, _>(transform_session_keys);
			Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block
		}
	}

	/// Migrations, unreleased to TESTNET or MAINNET
	pub type Unreleased = (
		pallet_ddc_customers::migration::v2::MigrateToV2<Runtime>,
		pallet_ddc_clusters::migrations::v3::MigrateToV3<Runtime>,
		pallet_ddc_nodes::migrations::v1::MigrateToV1<Runtime>,
		UpgradeSessionKeys,
		pallet_ddc_verification::migrations::v1::MigrateToV1<Runtime>,
		pallet_ddc_payouts::migrations::v1::MigrateToV1<Runtime>,
		pallet_ddc_payouts::migrations::v2::MigrateToV2<Runtime>,
		pallet_ddc_payouts::migrations::v3::MigrateToV3<Runtime>,
		pallet_ddc_verification::migrations::v2::MigrateToV2<Runtime>,
	);
}

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

type EventRecord = frame_system::EventRecord<
	<Runtime as frame_system::Config>::RuntimeEvent,
	<Runtime as frame_system::Config>::Hash,
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[pallet_babe, Babe]
		[pallet_bags_list, VoterList]
		[pallet_balances, Balances]
		[pallet_bounties, Bounties]
		[pallet_child_bounties, ChildBounties]
		[pallet_contracts, Contracts]
		[pallet_ddc_customers, DdcCustomers]
		[pallet_ddc_clusters, DdcClusters]
		[pallet_ddc_staking, DdcStaking]
		[pallet_ddc_nodes, DdcNodes]
		[pallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[pallet_election_provider_support_benchmarking, EPSBench::<Runtime>]
		[pallet_fast_unstake, FastUnstake]
		[pallet_grandpa, Grandpa]
		[pallet_identity, Identity]
		[pallet_im_online, ImOnline]
		[pallet_indices, Indices]
		[pallet_multisig, Multisig]
		[pallet_nomination_pools, NominationPoolsBench::<Runtime>]
		[pallet_offences, OffencesBench::<Runtime>]
		[pallet_proxy, Proxy]
		[pallet_preimage, Preimage]
		[pallet_scheduler, Scheduler]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_staking, Staking]
		[frame_system, SystemBench::<Runtime>]
		[pallet_timestamp, Timestamp]
		[pallet_treasury, Treasury]
		[pallet_utility, Utility]
		[pallet_vesting, Vesting]
		[pallet_conviction_voting, ConvictionVoting]
		[pallet_referenda, Referenda]
		[pallet_whitelist, Whitelist]
		[pallet_collective, TechComm]
		[pallet_ddc_clusters_gov, DdcClustersGov]
		[pallet_ddc_verification, DdcVerification]
		[pallet_token_gateway, TokenGateway]
	);
}

impl_runtime_apis! {
	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			vec![]
		}

		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}
	}
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header)-> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((fg_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(fg_primitives::OpaqueKeyOwnershipProof::new)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeConfiguration {
			let epoch_config = Babe::epoch_config().unwrap_or(BABE_GENESIS_EPOCH_CONFIG);
			sp_consensus_babe::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: epoch_config.c,
				authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: epoch_config.allowed_slots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: sp_consensus_babe::Slot,
			authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_contracts::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord> for Runtime
	{
		fn call(
			origin: AccountId,
			dest: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			input_data: Vec<u8>,
		) -> pallet_contracts::ContractExecResult<Balance, EventRecord> {
			let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
			Contracts::bare_call(
				origin,
				dest,
				value,
				gas_limit,
				storage_deposit_limit,
				input_data,
				pallet_contracts::DebugInfo::Skip,
				pallet_contracts::CollectEvents::Skip,
				pallet_contracts::Determinism::Enforced,
			)
		}

		fn instantiate(
			origin: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			code: pallet_contracts::Code<Hash>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> pallet_contracts::ContractInstantiateResult<AccountId, Balance, EventRecord>
		{
			let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
			Contracts::bare_instantiate(
				origin,
				value,
				gas_limit,
				storage_deposit_limit,
				code,
				data,
				salt,
				pallet_contracts::DebugInfo::Skip,
				pallet_contracts::CollectEvents::Skip
			)
		}

		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
			determinism: Determinism
		) -> pallet_contracts::CodeUploadResult<Hash, Balance>
		{
			Contracts::bare_upload_code(origin, code, storage_deposit_limit, determinism)
		}

		fn get_storage(
			address: AccountId,
			key: Vec<u8>,
		) -> pallet_contracts::GetStorageResult {
			Contracts::get_storage(address, key)
		}
	}

	impl pallet_ismp_runtime_api::IsmpRuntimeApi<Block, <Block as BlockT>::Hash> for Runtime {
		fn host_state_machine() -> StateMachine {
			<Runtime as pallet_ismp::Config>::HostStateMachine::get()
		}

		fn challenge_period(id: StateMachineId) -> Option<u64> {
			pallet_ismp::Pallet::<Runtime>::challenge_period(id)
		}

		fn block_events() -> Vec<ismp::events::Event> {
			pallet_ismp::Pallet::<Runtime>::block_events()
		}

		fn block_events_with_metadata() -> Vec<(ismp::events::Event, Option<u32>)> {
			pallet_ismp::Pallet::<Runtime>::block_events_with_metadata()
		}

		fn consensus_state(id: ConsensusClientId) -> Option<Vec<u8>> {
			pallet_ismp::Pallet::<Runtime>::consensus_states(id)
		}

		fn state_machine_update_time(height: StateMachineHeight) -> Option<u64> {
			pallet_ismp::Pallet::<Runtime>::state_machine_update_time(height)
		}

		fn latest_state_machine_height(id: StateMachineId) -> Option<u64> {
			pallet_ismp::Pallet::<Runtime>::latest_state_machine_height(id)
		}

		fn requests(commitments: Vec<H256>) -> Vec<Request> {
			pallet_ismp::Pallet::<Runtime>::requests(commitments)
		}

		fn responses(commitments: Vec<H256>) -> Vec<Response> {
			pallet_ismp::Pallet::<Runtime>::responses(commitments)
		}
	}
	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(call: RuntimeCall, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(call: RuntimeCall, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl pallet_nomination_pools_runtime_api::NominationPoolsApi<
		Block,
		AccountId,
		Balance,
	> for Runtime {
		fn pending_rewards(member: AccountId) -> Balance {
			NominationPools::api_pending_rewards(member).unwrap_or_default()
		}

		fn points_to_balance(pool_id: pallet_nomination_pools::PoolId, points: Balance) -> Balance {
			NominationPools::api_points_to_balance(pool_id, points)
		}

		fn balance_to_points(pool_id: pallet_nomination_pools::PoolId, new_funds: Balance) -> Balance {
			NominationPools::api_balance_to_points(pool_id, new_funds)
		}
		fn pool_pending_slash(pool_id: pallet_nomination_pools::PoolId) -> Balance {
			NominationPools::api_pool_pending_slash(pool_id)
		}

		fn member_pending_slash(member: AccountId) -> Balance {
			NominationPools::api_member_pending_slash(member)
		}

		fn pool_needs_delegate_migration(pool_id: pallet_nomination_pools::PoolId) -> bool {
			NominationPools::api_pool_needs_delegate_migration(pool_id)
		}

		fn member_needs_delegate_migration(member: AccountId) -> bool {
			NominationPools::api_member_needs_delegate_migration(member)
		}

		fn member_total_balance(member: AccountId) -> Balance {
			NominationPools::api_member_total_balance(member)
		}
		fn pool_balance(pool_id: pallet_nomination_pools::PoolId) -> Balance {
			NominationPools::api_pool_balance(pool_id)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade cere.");
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect,
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch};
			use sp_storage::TrackedStorageKey;

			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl pallet_session_benchmarking::Config for Runtime {}
			impl pallet_offences_benchmarking::Config for Runtime {}
			impl pallet_election_provider_support_benchmarking::Config for Runtime {}
			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}
			impl pallet_nomination_pools_benchmarking::Config for Runtime {}

			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
				// System BlockWeight
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef734abf5cb34d6244378cddbf18e849d96").to_vec().into(),
				// Treasury Account
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da95ecffd7b6c0f78751baa9d281e0bfa3a6d6f646c70792f74727372790000000000000000000000000000000000000000").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}
}

#[cfg(test)]
mod tests {
	use frame_election_provider_support::NposSolution;
	use frame_system::offchain::CreateSignedTransaction;
	use sp_runtime::UpperOf;

	use super::*;

	#[test]
	fn validate_transaction_submitter_bounds() {
		fn is_submit_signed_transaction<T>()
		where
			T: CreateSignedTransaction<RuntimeCall>,
		{
		}

		is_submit_signed_transaction::<Runtime>();
	}

	#[test]
	fn perbill_as_onchain_accuracy() {
		type OnChainAccuracy =
		<<Runtime as pallet_election_provider_multi_phase::MinerConfig>::Solution as NposSolution>::Accuracy;
		let maximum_chain_accuracy: Vec<UpperOf<OnChainAccuracy>> = (0..MaxNominations::get())
			.map(|_| <UpperOf<OnChainAccuracy>>::from(OnChainAccuracy::one().deconstruct()))
			.collect();
		let _: UpperOf<OnChainAccuracy> =
			maximum_chain_accuracy.iter().fold(0, |acc, x| acc.checked_add(*x).unwrap());
	}

	#[test]
	fn call_size() {
		let size = core::mem::size_of::<RuntimeCall>();
		assert!(
			size <= 256,
			"size of RuntimeCall {} is more than 256 bytes: some calls have too big arguments, use Box to reduce the
			size of RuntimeCall.
			If the limit is too strong, maybe consider increase the limit to 300.",
			size,
		);
	}
}
