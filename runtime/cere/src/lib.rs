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
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use ddc_primitives::{
	traits::pallet::{GetDdcOrigin, PalletVisitor},
	AccountIndex, Balance, BlockNumber, Hash, Moment, Nonce, MAX_PAYOUT_BATCH_COUNT,
	MAX_PAYOUT_BATCH_SIZE,
};
pub use ddc_primitives::{AccountId, Signature};
use frame_election_provider_support::{
	bounds::ElectionBoundsBuilder, onchain, BalancingConfig, SequentialPhragmen, VoteWeight,
};
use pallet_balances::WeightInfo;
extern crate alloc;
use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	pallet_prelude::Get,
	parameter_types,
	traits::{
		fungible::{Credit, Debt, HoldConsideration},
		fungibles,
		fungibles::{Dust, Inspect, Unbalanced},
		tokens::{
			imbalance::ResolveTo, DepositConsequence, Fortitude, PayFromAccount, Preservation,
			Provenance, UnityAssetBalanceConversion, WithdrawConsequence,
		},
		ConstBool, ConstU128, ConstU16, ConstU32, ConstU64, Currency, EitherOf, EitherOfDiverse,
		EqualPrivilegeOnly, ExistenceRequirement, Imbalance, InstanceFilter, KeyOwnerProofSystem,
		LinearStoragePrice, Nothing, OnUnbalanced, VariantCountOf, WithdrawReasons,
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
//use pallet_treasury::{NegativeImbalanceOf, PositiveImbalanceOf};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical::{self as pallet_session_historical};
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
	spec_name: alloc::borrow::Cow::Borrowed("node"),
	impl_name: alloc::borrow::Cow::Borrowed("substrate-node"),
	authoring_version: 10,
	// Per convention: if the runtime behavior changes, increment spec_version
	// and set impl_version to 0. If only runtime
	// implementation changes and behavior does not, then leave spec_version as
	// is and increment impl_version.
	spec_version: 73160,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 25,
	system_version: 0,
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

#[frame_support::runtime]
mod runtime {
	use super::*;
	
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

	// ... existing code ...

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
		type MultiBlockMigrator = MultiBlockMigrations;
	}

	impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

	impl pallet_utility::Config for Runtime {
		type RuntimeEvent = RuntimeEvent;
		type RuntimeCall = RuntimeCall;
		type PalletsOrigin = OriginCaller;
		type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
	}

	// ... [Continue with all other configuration implementations] ...

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
	// pub type DdcPayouts = pallet_ddc_payouts::Pallet<Runtime>;
	#[runtime::pallet_index(40)]
	// pub type DdcVerification = pallet_ddc_verification::Pallet<Runtime>;

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
	// Migrations pallet
	#[runtime::pallet_index(51)]
	pub type MultiBlockMigrations = pallet_migrations;

	#[runtime::pallet_index(52)]
	pub type FeeHandler = pallet_fee_handler::Pallet<Runtime>;

	#[runtime::pallet_index(53)]
	pub type NetworkMonitor = pallet_network_monitor::Pallet<Runtime>;
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
pub type TxExtension = (
	frame_system::CheckNonZeroSender<runtime::Runtime>,
	frame_system::CheckSpecVersion<runtime::Runtime>,
	frame_system::CheckTxVersion<runtime::Runtime>,
	frame_system::CheckGenesis<runtime::Runtime>,
	frame_system::CheckEra<runtime::Runtime>,
	frame_system::CheckNonce<runtime::Runtime>,
	frame_system::CheckWeight<runtime::Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<runtime::Runtime>,
	frame_metadata_hash_extension::CheckMetadataHash<runtime::Runtime>,
	frame_system::WeightReclaim<runtime::Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, runtime::RuntimeCall, Signature, TxExtension>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<runtime::RuntimeCall, TxExtension>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, runtime::RuntimeCall, TxExtension>;

// ... existing code ...
