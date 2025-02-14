use frame_support::parameter_types;
use frame_system::EnsureRoot;
use ismp::{error::Error, host::StateMachine, module::IsmpModule, router::IsmpRouter};
use ismp_grandpa::consensus::GrandpaConsensusClient;
use pallet_token_gateway::types::EvmToSubstrate;
use sp_core::H160;

use super::*;

parameter_types! {
	// The hyperbridge parachain on Polkadot
	pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Kusama(4009));
	// The host state machine of this pallet, this must be unique to all every solochain
	pub const HostStateMachine: StateMachine = StateMachine::Substrate(*b"cere"); // your unique chain id here
}

impl pallet_ismp::Config for Runtime {
	// Configure the runtime event
	type RuntimeEvent = RuntimeEvent;
	// Permissioned origin who can create or update consensus clients
	type AdminOrigin = EnsureRoot<AccountId>;
	// The state machine identifier for this state machine
	type HostStateMachine = HostStateMachine;
	// The pallet_timestamp pallet
	type TimestampProvider = Timestamp;
	// The currency implementation that is offered to relayers
	// this could also be `frame_support::traits::tokens::fungible::ItemOf`
	type Currency = Balances;
	// The balance type for the currency implementation
	type Balance = Balance;
	// Router implementation for routing requests/responses to their respective modules
	type Router = ModuleRouter;
	// Optional coprocessor for incoming requests/responses
	type Coprocessor = Coprocessor;
	// Supported consensus clients
	type ConsensusClients = (
		// Add the grandpa consensus client here
		GrandpaConsensusClient<Runtime>,
	);
	// Offchain database implementation. Outgoing requests and responses are
	// inserted in this database, while their commitments are stored onchain.
	//
	// The default implementation for `()` should suffice
	type OffchainDB = ();
	// Weight provider for local modules
	type WeightProvider = ();
}

impl pallet_hyperbridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
}

#[derive(Default)]
pub struct ModuleRouter;

impl IsmpRouter for ModuleRouter {
	fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
		return match id.as_slice() {
			id if TokenGateway::is_token_gateway(id) => Ok(Box::new(TokenGateway::default())),
			pallet_hyperbridge::PALLET_HYPERBRIDGE_ID =>
				Ok(Box::new(pallet_hyperbridge::Pallet::<Runtime>::default())),
			_ => Err(Error::ModuleNotFound(id).into()),
		};
	}
}

impl ismp_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
	type WeightInfo = weights::ismp_grandpa::WeightInfo<Runtime>;
}

parameter_types! {
	// A constant that should represent the native asset id, this id must be unique to the native currency
	pub const NativeAssetId: u32 = 0;
	// Set the correct decimals for the native currency
	pub const Decimals: u8 = 10;
}

/// Should provide an account that is funded and can be used to pay for asset creation
pub struct AssetAdmin;
impl Get<AccountId> for AssetAdmin {
	fn get() -> AccountId {
		Treasury::account_id()
	}
}

pub struct MockAssets;

impl fungibles::metadata::Inspect<AccountId> for MockAssets {
	fn name(_asset: Self::AssetId) -> Vec<u8> {
		vec![]
	}

	fn symbol(_asset: Self::AssetId) -> Vec<u8> {
		vec![]
	}

	fn decimals(_asset: Self::AssetId) -> u8 {
		0
	}
}
impl Unbalanced<AccountId> for MockAssets {
	fn handle_dust(_dust: Dust<AccountId, Self>) {}

	fn write_balance(
		_asset: Self::AssetId,
		_who: &AccountId,
		_amount: Self::Balance,
	) -> Result<Option<Self::Balance>, DispatchError> {
		Ok(None)
	}

	fn set_total_issuance(_asset: Self::AssetId, _amount: Self::Balance) {}
}

impl fungibles::Mutate<AccountId> for MockAssets {}

impl Inspect<AccountId> for MockAssets {
	type AssetId = Balance;
	type Balance = Balance;

	fn total_issuance(_asset: Self::AssetId) -> Self::Balance {
		0
	}

	fn minimum_balance(_asset: Self::AssetId) -> Self::Balance {
		0
	}

	fn total_balance(_asset: Self::AssetId, _who: &AccountId) -> Self::Balance {
		0
	}

	fn balance(_asset: Self::AssetId, _who: &AccountId) -> Self::Balance {
		0
	}

	fn reducible_balance(
		_asset: Self::AssetId,
		_who: &AccountId,
		_preservation: Preservation,
		_force: Fortitude,
	) -> Self::Balance {
		0
	}

	fn can_deposit(
		_asset: Self::AssetId,
		_who: &AccountId,
		_amount: Self::Balance,
		_provenance: Provenance,
	) -> DepositConsequence {
		DepositConsequence::UnknownAsset
	}

	fn can_withdraw(
		_asset: Self::AssetId,
		_who: &AccountId,
		_amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		WithdrawConsequence::UnknownAsset
	}

	fn asset_exists(_asset: Self::AssetId) -> bool {
		false
	}
}

impl fungibles::Create<AccountId> for MockAssets {
	fn create(
		_id: Self::AssetId,
		_admin: AccountId,
		_is_sufficient: bool,
		_min_balance: Self::Balance,
	) -> DispatchResult {
		Ok(())
	}
}

impl fungibles::metadata::Mutate<AccountId> for MockAssets {
	fn set(
		_asset: Self::AssetId,
		_from: &AccountId,
		_name: Vec<u8>,
		_symbol: Vec<u8>,
		_decimals: u8,
	) -> frame_support::dispatch::DispatchResult {
		Ok(())
	}
}

impl fungibles::roles::Inspect<AccountId> for MockAssets {
	fn owner(_asset: Self::AssetId) -> Option<AccountId> {
		None
	}

	fn issuer(_asset: Self::AssetId) -> Option<AccountId> {
		None
	}

	fn admin(_asset: Self::AssetId) -> Option<AccountId> {
		None
	}

	fn freezer(_asset: Self::AssetId) -> Option<AccountId> {
		None
	}
}

pub struct EvmToSubstrateFactory;

impl EvmToSubstrate<Runtime> for EvmToSubstrateFactory {
	fn convert(addr: H160) -> AccountId {
		let mut account = [0u8; 32];
		account[12..].copy_from_slice(&addr.0);
		account.into()
	}
}

impl pallet_token_gateway::Config for Runtime {
	// configure the runtime event
	type RuntimeEvent = RuntimeEvent;
	// Configured as Pallet Ismp
	type Dispatcher = pallet_hyperbridge::Pallet<Runtime>;
	// Configured as Pallet Assets
	type Assets = MockAssets;
	// Configured as Pallet balances
	type NativeCurrency = Balances;
	// AssetAdmin account
	type AssetAdmin = AssetAdmin;
	// The Native asset Id
	type NativeAssetId = NativeAssetId;
	// The precision of the native asset
	type Decimals = Decimals;
	type EvmToSubstrate = EvmToSubstrateFactory;
	type WeightInfo = weights::pallet_token_gateway::SubstrateWeight<Runtime>;
	type CreateOrigin = EnsureSigned<AccountId>;
}
