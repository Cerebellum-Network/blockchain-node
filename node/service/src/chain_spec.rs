#[cfg(feature = "cere-native")]
use cere_runtime as cere;

#[cfg(feature = "cere-dev-native")]
use cere_dev_runtime as cere_dev;
#[cfg(feature = "cere-dev-native")]
use cere_dev_runtime_constants::currency::DOLLARS as TEST_UNITS;

pub use node_primitives::{AccountId, Balance, Block, Signature};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

const DEFAULT_PROTOCOL_ID: &str = "cere";

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

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(
	seed: &str,
) -> (AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

#[cfg(feature = "cere-dev-native")]
fn cere_dev_session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> cere_dev::SessionKeys {
	cere_dev::SessionKeys { grandpa, babe, im_online, authority_discovery }
}

/// Helper function to create Cere Dev `GenesisConfig` for testing
#[cfg(feature = "cere-dev-native")]
pub fn cere_dev_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	initial_nominators: Vec<AccountId>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> cere_dev::GenesisConfig {
	let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		]
	});

	// endow all authorities and nominators.
	initial_authorities
		.iter()
		.map(|x| &x.0)
		.chain(initial_nominators.iter())
		.for_each(|x| {
			if !endowed_accounts.contains(x) {
				endowed_accounts.push(x.clone())
			}
		});

	// stakers: all validators and nominators.
	let mut rng = rand::thread_rng();
	let stakers = initial_authorities
		.iter()
		.map(|x| (x.0.clone(), x.1.clone(), STASH, cere_dev::StakerStatus::Validator))
		.chain(initial_nominators.iter().map(|x| {
			use rand::{seq::SliceRandom, Rng};
			let limit = (cere_dev::MaxNominations::get() as usize).min(initial_authorities.len());
			let count = rng.gen::<usize>() % limit;
			let nominations = initial_authorities
				.as_slice()
				.choose_multiple(&mut rng, count)
				.into_iter()
				.map(|choice| choice.0.clone())
				.collect::<Vec<_>>();
			(x.clone(), x.clone(), STASH, cere_dev::StakerStatus::Nominator(nominations))
		}))
		.collect::<Vec<_>>();

	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: Balance = 10_000_000 * TEST_UNITS;
	const STASH: Balance = ENDOWMENT / 1000;

	cere_dev::GenesisConfig {
		system: cere_dev::SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
		},
		balances: cere_dev::BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect(),
		},
		indices: cere_dev::IndicesConfig { indices: vec![] },
		session: cere_dev::SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						cere_dev_session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
					)
				})
				.collect::<Vec<_>>(),
		},
		staking: cere_dev::StakingConfig {
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: initial_authorities.len() as u32,
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			..Default::default()
		},
		ddc_staking: cere_dev::DdcStakingConfig::default(),
		democracy: cere_dev::DemocracyConfig::default(),
		elections: cere_dev::ElectionsConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		},
		council: cere_dev::CouncilConfig::default(),
		technical_committee: cere_dev::TechnicalCommitteeConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		},
		sudo: cere_dev::SudoConfig { key: Some(root_key) },
		babe: cere_dev::BabeConfig {
			authorities: vec![],
			epoch_config: Some(cere_dev::BABE_GENESIS_EPOCH_CONFIG),
		},
		im_online: cere_dev::ImOnlineConfig { keys: vec![] },
		authority_discovery: cere_dev::AuthorityDiscoveryConfig { keys: vec![] },
		grandpa: cere_dev::GrandpaConfig { authorities: vec![] },
		technical_membership: Default::default(),
		treasury: Default::default(),
		society: cere_dev::SocietyConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			pot: 0,
			max_members: 999,
		},
		vesting: Default::default(),
		transaction_payment: Default::default(),
		ddc_accounts: Default::default(),
		nomination_pools: Default::default(),
	}
}

/// Helper function to create Cere `GenesisConfig` for testing
#[cfg(feature = "cere-dev-native")]
fn cere_dev_config_genesis(wasm_binary: &[u8]) -> cere_dev::GenesisConfig {
	cere_dev_genesis(
		wasm_binary,
		// Initial authorities
		vec![authority_keys_from_seed("Alice")],
		// Initial nominators
		vec![],
		// Sudo account
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		// Pre-funded accounts
		Some(vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		]),
	)
}

#[cfg(feature = "cere-dev-native")]
pub fn cere_dev_development_config() -> Result<CereDevChainSpec, String> {
	let wasm_binary = cere_dev::WASM_BINARY.ok_or("Cere Dev development wasm not available")?;

	Ok(CereDevChainSpec::from_genesis(
		"Development",
		"cere_dev",
		ChainType::Development,
		move || cere_dev_config_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		None,
		Default::default(),
	))
}

#[cfg(feature = "cere-dev-native")]
fn cere_dev_local_testnet_genesis(wasm_binary: &[u8]) -> cere_dev::GenesisConfig {
	cere_dev_genesis(
		wasm_binary,
		// Initial authorities
		vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
		// Initial nominators
		vec![],
		// Sudo account
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		// Pre-funded accounts
		Some(vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		]),
	)
}

#[cfg(feature = "cere-dev-native")]
pub fn cere_dev_local_testnet_config() -> Result<CereDevChainSpec, String> {
	let wasm_binary = cere_dev::WASM_BINARY.ok_or("Cere Dev development wasm not available")?;

	Ok(CereDevChainSpec::from_genesis(
		"Local Testnet",
		"cere_dev_local_testnet",
		ChainType::Local,
		move || cere_dev_local_testnet_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		None,
		Default::default(),
	))
}

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
