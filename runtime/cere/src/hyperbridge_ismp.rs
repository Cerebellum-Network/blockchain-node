//! ISMP / hyperbridge wiring for the Cere runtime.
//!
//! Pallets used (declared in the workspace `[hyperbridge]` section):
//!   * `pallet-ismp`                  ISMP host
//!   * `ismp-grandpa`                 consensus client
//!   * `pallet-hyper-fungible-token`  cross-chain token transfers
//!   * `pallet-ismp-rpc` / `pallet-ismp-runtime-api`  exposed by the node side
//!
//! Mirrors the canonical layout in
//! `hyperbridge/parachain/runtimes/gargantua/src/ismp.rs`, stripped down to the
//! four crates we wire in.

use alloc::string::ToString;

use anyhow::anyhow;
use ismp::{
	error::Error,
	host::StateMachine,
	module::IsmpModule,
	router::{GetResponse, IsmpRouter, PostRequest, Request},
};
use pallet_ismp::ModuleId;
use polkadot_sdk::frame_support::{
	parameter_types,
	traits::{
		fungibles::{self, Dust, Unbalanced},
		tokens::{DepositConsequence, Fortitude, Preservation, Provenance, WithdrawConsequence},
		Get,
	},
	weights::WeightToFee,
};
use polkadot_sdk::frame_system::EnsureRoot;
use polkadot_sdk::sp_core::H256;
use polkadot_sdk::sp_runtime::{DispatchError, DispatchResult, Weight};

use super::*;

parameter_types! {
	// Unique chain identifier for this solochain. Keep in sync with the
	// `id` field of the ChainSpec.
	pub const HostStateMachine: StateMachine = StateMachine::Substrate(*b"cere");
}

// Coprocessor is the trusted state-machine that pallet-ismp uses both for
// addressing outgoing requests and for gating the withdrawal-source check on
// incoming coprocessor-originated messages. For cere mainnet this is the
// Hyperbridge parachain on Polkadot (paraId 3367); preserved from the runtime
// baseline that predates the polkadot-sdk umbrella swap.
pub struct Coprocessor;
impl Get<Option<StateMachine>> for Coprocessor {
	fn get() -> Option<StateMachine> {
		Some(StateMachine::Polkadot(3367))
	}
}

pub struct IsmpWeightToFee;
impl WeightToFee for IsmpWeightToFee {
	type Balance = Balance;
	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		<Runtime as polkadot_sdk::pallet_transaction_payment::Config>::WeightToFee::weight_to_fee(
			weight,
		)
	}
}

impl pallet_ismp::Config for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
	type HostStateMachine = HostStateMachine;
	type Coprocessor = Coprocessor;
	type TimestampProvider = Timestamp;
	type Balance = Balance;
	type Currency = Balances;
	type Router = Router;
	type ConsensusClients = (ismp_grandpa::consensus::GrandpaConsensusClient<Runtime>,);
	type OffchainDB = ();
	type FeeHandler = pallet_ismp::fee_handler::WeightFeeHandler<
		AccountId,
		Balances,
		IsmpWeightToFee,
		TreasuryPalletId,
		false,
	>;
	type MigrationWeightInfo = crate::weights::pallet_ismp::WeightInfo<Runtime>;
}

impl ismp_grandpa::Config for Runtime {
	type IsmpHost = pallet_ismp::Pallet<Runtime>;
	type WeightInfo = crate::weights::ismp_grandpa::WeightInfo<Runtime>;
	type RootOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	pub const HftDecimals: u8 = 10;
}

pub struct HftNativeAssetId;
impl Get<H256> for HftNativeAssetId {
	fn get() -> H256 {
		polkadot_sdk::sp_io::hashing::keccak_256(b"CERE").into()
	}
}

impl pallet_hyper_fungible_token::Config for Runtime {
	type Dispatcher = Ismp;
	// We don't ship a real multi-asset pallet yet. `MockAssets` satisfies the
	// trait bounds (`fungibles::Mutate` + `fungibles::metadata::Inspect`) but is
	// a no-op; swap in `pallet-assets` when HFT needs real assets.
	type Assets = MockAssets;
	type NativeCurrency = Balances;
	type NativeAssetId = HftNativeAssetId;
	type CreateOrigin = EnsureRoot<AccountId>;
	type Decimals = HftDecimals;
	type EvmToSubstrate = ();
	type WeightInfo = ();
}

/// Stub Assets registry that satisfies the trait bounds required by
/// `pallet-hyper-fungible-token`. All operations are no-ops; this must be
/// replaced with a real `pallet-assets` instance before HFT is exercised.
pub struct MockAssets;

impl fungibles::Inspect<AccountId> for MockAssets {
	type AssetId = H256;
	type Balance = Balance;
	fn total_issuance(_: Self::AssetId) -> Self::Balance {
		0
	}
	fn minimum_balance(_: Self::AssetId) -> Self::Balance {
		0
	}
	fn total_balance(_: Self::AssetId, _: &AccountId) -> Self::Balance {
		0
	}
	fn balance(_: Self::AssetId, _: &AccountId) -> Self::Balance {
		0
	}
	fn reducible_balance(
		_: Self::AssetId,
		_: &AccountId,
		_: Preservation,
		_: Fortitude,
	) -> Self::Balance {
		0
	}
	fn can_deposit(
		_: Self::AssetId,
		_: &AccountId,
		_: Self::Balance,
		_: Provenance,
	) -> DepositConsequence {
		DepositConsequence::UnknownAsset
	}
	fn can_withdraw(
		_: Self::AssetId,
		_: &AccountId,
		_: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		WithdrawConsequence::UnknownAsset
	}
	fn asset_exists(_: Self::AssetId) -> bool {
		false
	}
}

impl Unbalanced<AccountId> for MockAssets {
	fn handle_dust(_: Dust<AccountId, Self>) {}
	fn write_balance(
		_: Self::AssetId,
		_: &AccountId,
		_: Self::Balance,
	) -> Result<Option<Self::Balance>, DispatchError> {
		Ok(None)
	}
	fn set_total_issuance(_: Self::AssetId, _: Self::Balance) {}
}

impl fungibles::Mutate<AccountId> for MockAssets {}

impl fungibles::metadata::Inspect<AccountId> for MockAssets {
	fn name(_: Self::AssetId) -> Vec<u8> {
		Vec::new()
	}
	fn symbol(_: Self::AssetId) -> Vec<u8> {
		Vec::new()
	}
	fn decimals(_: Self::AssetId) -> u8 {
		0
	}
}

#[derive(Default)]
pub struct ProxyModule;

impl IsmpModule for ProxyModule {
	fn on_accept(&self, request: PostRequest) -> Result<Weight, anyhow::Error> {
		if request.dest != HostStateMachine::get() {
			return Ok(Weight::from_parts(0, 0));
		}
		let pallet_id =
			ModuleId::from_bytes(&request.to).map_err(|err| Error::Custom(err.to_string()))?;
		match pallet_id {
			pallet_hyper_fungible_token::PALLET_ID => {
				pallet_hyper_fungible_token::Pallet::<Runtime>::default().on_accept(request)
			},
			_ => Err(anyhow!("Destination module not found")),
		}
	}

	fn on_response(&self, response: GetResponse) -> Result<Weight, anyhow::Error> {
		if response.dest_chain() != HostStateMachine::get() {
			return Ok(Weight::from_parts(0, 0));
		}
		Ok(Weight::from_parts(0, 0))
	}

	fn on_timeout(&self, timeout: Request) -> Result<Weight, anyhow::Error> {
		let (from, source) = match &timeout {
			Request::Post(p) => (&p.from, p.source),
			Request::Get(g) => (&g.from, g.source),
		};
		if source != HostStateMachine::get() {
			return Ok(Weight::from_parts(0, 0));
		}
		let pallet_id = ModuleId::from_bytes(from).map_err(|err| Error::Custom(err.to_string()))?;
		match pallet_id {
			pallet_hyper_fungible_token::PALLET_ID => {
				pallet_hyper_fungible_token::Pallet::<Runtime>::default().on_timeout(timeout)
			},
			_ => Ok(Weight::from_parts(0, 0)),
		}
	}
}

#[derive(Default)]
pub struct Router;

impl IsmpRouter for Router {
	fn module_for_id(&self, _bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
		Ok(Box::new(ProxyModule))
	}
}

// Silence unused-import warning when DispatchResult isn't referenced above.
#[allow(dead_code)]
type _Unused = (DispatchResult,);
