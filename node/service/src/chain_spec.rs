#[cfg(feature = "cere-native")]
use cere_runtime as cere;

#[cfg(feature = "cere-dev-native")]
use cere_dev_runtime as cere_dev;

pub use node_primitives::{AccountId, Balance, Block, Signature};
use sc_chain_spec::ChainSpecExtension;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
	/// The light sync state extension used by the sync-state rpc.
	pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

// Dummy chain spec, in case when we don't have the native runtime.
pub type DummyChainSpec = sc_service::GenericChainSpec<(), Extensions>;

/// The `ChainSpec` parameterized for the cere runtime.
#[cfg(feature = "cere-native")]
pub type CereChainSpec = sc_service::GenericChainSpec<cere::GenesisConfig, Extensions>;

/// The `ChainSpec` parameterized for the cere runtime.
// Dummy chain spec, but that is fine when we don't have the native runtime.
#[cfg(not(feature = "cere-native"))]
pub type CereChainSpec = DummyChainSpec;

/// The `ChainSpec` parameterized for the cere-dev runtime.
#[cfg(feature = "cere-dev-native")]
pub type CereDevChainSpec = sc_service::GenericChainSpec<cere_dev::GenesisConfig, Extensions>;

/// The `ChainSpec` parameterized for the cere-dev runtime.
// Dummy chain spec, but that is fine when we don't have the native runtime.
#[cfg(not(feature = "cere-dev-native"))]
pub type CereDevChainSpec = DummyChainSpec;

pub fn cere_mainnet_config() -> Result<CereChainSpec, String> {
	CereChainSpec::from_json_bytes(&include_bytes!("../chain-specs/mainnet.json")[..])
}

pub fn cere_testnet_config() -> Result<CereChainSpec, String> {
	CereChainSpec::from_json_bytes(&include_bytes!("../chain-specs/testnet.json")[..])
}

pub fn cere_qanet_config() -> Result<CereChainSpec, String> {
	CereChainSpec::from_json_bytes(&include_bytes!("../chain-specs/qanet.json")[..])
}

pub fn cere_devnet_config() -> Result<CereDevChainSpec, String> {
	CereDevChainSpec::from_json_bytes(&include_bytes!("../chain-specs/devnet.json")[..])
}
