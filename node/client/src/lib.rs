use std::sync::Arc;

use ddc_primitives::Nonce;
pub use ddc_primitives::{AccountId, Balance, Block, BlockNumber, Hash, Header, Signature};
use polkadot_sdk::sc_client_api::{
	AuxStore, Backend as BackendT, BlockchainEvents, KeysIter, MerkleValue, PairsIter,
	UsageProvider,
};
use polkadot_sdk::sc_executor::WasmExecutor;
use polkadot_sdk::sp_api::{CallApiAt, ProvideRuntimeApi};
use polkadot_sdk::sp_blockchain::{HeaderBackend, HeaderMetadata};
use polkadot_sdk::sp_consensus::BlockStatus;
use polkadot_sdk::sp_core::H256;
use polkadot_sdk::sp_runtime::{
	generic::SignedBlock,
	traits::{BlakeTwo256, Block as BlockT, NumberFor},
	Justifications,
};
use polkadot_sdk::sp_storage::{ChildInfo, StorageData, StorageKey};

pub type FullBackend = polkadot_sdk::sc_service::TFullBackend<Block>;

#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = (polkadot_sdk::sp_io::SubstrateHostFunctions, ddc_dac_host::ddc_dac::HostFunctions);

#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = (
	polkadot_sdk::sp_io::SubstrateHostFunctions,
	polkadot_sdk::frame_benchmarking::benchmarking::HostFunctions,
	ddc_dac_host::ddc_dac::HostFunctions,
);

pub type ChainExecutor = WasmExecutor<HostFunctions>;
pub type FullClient<RuntimeApi> = polkadot_sdk::sc_service::TFullClient<Block, RuntimeApi, ChainExecutor>;

#[cfg(not(any(feature = "cere", feature = "cere-dev",)))]
compile_error!("at least one runtime feature must be enabled");

pub trait AbstractClient<Block, Backend>:
	BlockchainEvents<Block>
	+ Sized
	+ Send
	+ Sync
	+ ProvideRuntimeApi<Block>
	+ HeaderBackend<Block>
	+ CallApiAt<Block, StateBackend = Backend::State>
	+ AuxStore
	+ UsageProvider<Block>
	+ HeaderMetadata<Block, Error = polkadot_sdk::sp_blockchain::Error>
where
	Block: BlockT,
	Backend: BackendT<Block>,
	Backend::State: polkadot_sdk::sc_client_api::backend::StateBackend<BlakeTwo256>,
	Self::Api: RuntimeApiCollection,
{
}

impl<Block, Backend, Client> AbstractClient<Block, Backend> for Client
where
	Block: BlockT,
	Backend: BackendT<Block>,
	Backend::State: polkadot_sdk::sc_client_api::backend::StateBackend<BlakeTwo256>,
	Client: BlockchainEvents<Block>
		+ ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ AuxStore
		+ UsageProvider<Block>
		+ Sized
		+ Send
		+ Sync
		+ CallApiAt<Block, StateBackend = Backend::State>
		+ HeaderMetadata<Block, Error = polkadot_sdk::sp_blockchain::Error>,
	Client::Api: RuntimeApiCollection,
{
}

/// See [`ExecuteWithClient`] for more information.
#[derive(Clone)]
pub enum Client {
	#[cfg(feature = "cere")]
	Cere(Arc<FullClient<cere_runtime::RuntimeApi>>),
	#[cfg(feature = "cere-dev")]
	CereDev(Arc<FullClient<cere_dev_runtime::RuntimeApi>>),
}

macro_rules! with_client {
  {
    $self:ident,
    $client:ident,
    {
      $( $code:tt )*
    }
  } => {
    match $self {
      #[cfg(feature = "cere")]
      Self::Cere($client) => { $( $code )* },
      #[cfg(feature = "cere-dev")]
      Self::CereDev($client) => { $( $code )* }
    }
  }
}

pub trait ExecuteWithClient {
	/// The return type when calling this instance.
	type Output;

	/// Execute whatever should be executed with the given client instance.
	fn execute_with_client<Client, Api, Backend>(self, client: Arc<Client>) -> Self::Output
	where
		Backend: polkadot_sdk::sc_client_api::Backend<Block>,
		Backend::State: polkadot_sdk::sc_client_api::backend::StateBackend<BlakeTwo256>,
		Api: crate::RuntimeApiCollection,
		Client: AbstractClient<Block, Backend, Api = Api> + 'static;
}

pub trait ClientHandle {
	/// Execute the given something with the client.
	fn execute_with<T: ExecuteWithClient>(&self, t: T) -> T::Output;
}

impl ClientHandle for Client {
	fn execute_with<T: ExecuteWithClient>(&self, t: T) -> T::Output {
		with_client! {
			self,
			client,
			{
				T::execute_with_client::<_, _, FullBackend>(t, client.clone())
			}
		}
	}
}

impl UsageProvider<Block> for Client {
	fn usage_info(&self) -> polkadot_sdk::sc_client_api::ClientInfo<Block> {
		with_client! {
			self,
			client,
			{
				client.usage_info()
			}
		}
	}
}

impl polkadot_sdk::sc_client_api::BlockBackend<Block> for Client {
	fn block_body(
		&self,
		hash: <Block as BlockT>::Hash,
	) -> polkadot_sdk::sp_blockchain::Result<Option<Vec<<Block as BlockT>::Extrinsic>>> {
		with_client! {
			self,
			client,
			{
				client.block_body(hash)
			}
		}
	}

	fn block(
		&self,
		hash: <Block as BlockT>::Hash,
	) -> polkadot_sdk::sp_blockchain::Result<Option<SignedBlock<Block>>> {
		with_client! {
			self,
			client,
			{
				client.block(hash)
			}
		}
	}

	fn block_status(&self, hash: <Block as BlockT>::Hash) -> polkadot_sdk::sp_blockchain::Result<BlockStatus> {
		with_client! {
			self,
			client,
			{
				client.block_status(hash)
			}
		}
	}

	fn justifications(
		&self,
		hash: <Block as BlockT>::Hash,
	) -> polkadot_sdk::sp_blockchain::Result<Option<Justifications>> {
		with_client! {
			self,
			client,
			{
				client.justifications(hash)
			}
		}
	}

	fn block_hash(
		&self,
		number: NumberFor<Block>,
	) -> polkadot_sdk::sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		with_client! {
			self,
			client,
			{
				client.block_hash(number)
			}
		}
	}

	fn indexed_transaction(
		&self,
		hash: <Block as BlockT>::Hash,
	) -> polkadot_sdk::sp_blockchain::Result<Option<Vec<u8>>> {
		with_client! {
			self,
			client,
			{
				client.indexed_transaction(hash)
			}
		}
	}

	fn block_indexed_body(
		&self,
		hash: <Block as BlockT>::Hash,
	) -> polkadot_sdk::sp_blockchain::Result<Option<Vec<Vec<u8>>>> {
		with_client! {
			self,
			client,
			{
				client.block_indexed_body(hash)
			}
		}
	}

	fn requires_full_sync(&self) -> bool {
		with_client! {
			self,
			client,
			{
				client.requires_full_sync()
			}
		}
	}
}

impl polkadot_sdk::sc_client_api::StorageProvider<Block, crate::FullBackend> for Client {
	fn storage(
		&self,
		hash: <Block as BlockT>::Hash,
		key: &StorageKey,
	) -> polkadot_sdk::sp_blockchain::Result<Option<StorageData>> {
		with_client! {
			self,
			client,
			{
				client.storage(hash, key)
			}
		}
	}

	fn storage_hash(
		&self,
		hash: <Block as BlockT>::Hash,
		key: &StorageKey,
	) -> polkadot_sdk::sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		with_client! {
			self,
			client,
			{
				client.storage_hash(hash, key)
			}
		}
	}

	fn storage_pairs(
		&self,
		hash: <Block as BlockT>::Hash,
		key_prefix: Option<&StorageKey>,
		start_key: Option<&StorageKey>,
	) -> polkadot_sdk::sp_blockchain::Result<
		PairsIter<<crate::FullBackend as polkadot_sdk::sc_client_api::Backend<Block>>::State, Block>,
	> {
		with_client! {
			self,
			client,
			{
				client.storage_pairs(hash, key_prefix, start_key)
			}
		}
	}

	fn storage_keys(
		&self,
		hash: <Block as BlockT>::Hash,
		prefix: Option<&StorageKey>,
		start_key: Option<&StorageKey>,
	) -> polkadot_sdk::sp_blockchain::Result<
		KeysIter<<crate::FullBackend as polkadot_sdk::sc_client_api::Backend<Block>>::State, Block>,
	> {
		with_client! {
			self,
			client,
			{
				client.storage_keys(hash, prefix, start_key)
			}
		}
	}

	fn child_storage(
		&self,
		hash: <Block as BlockT>::Hash,
		child_info: &ChildInfo,
		key: &StorageKey,
	) -> polkadot_sdk::sp_blockchain::Result<Option<StorageData>> {
		with_client! {
			self,
			client,
			{
				client.child_storage(hash, child_info, key)
			}
		}
	}

	fn child_storage_keys(
		&self,
		hash: <Block as BlockT>::Hash,
		child_info: ChildInfo,
		prefix: Option<&StorageKey>,
		start_key: Option<&StorageKey>,
	) -> polkadot_sdk::sp_blockchain::Result<
		KeysIter<<crate::FullBackend as polkadot_sdk::sc_client_api::Backend<Block>>::State, Block>,
	> {
		with_client! {
			self,
			client,
			{
				client.child_storage_keys(hash, child_info, prefix, start_key)
			}
		}
	}

	fn child_storage_hash(
		&self,
		hash: <Block as BlockT>::Hash,
		child_info: &ChildInfo,
		key: &StorageKey,
	) -> polkadot_sdk::sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		with_client! {
			self,
			client,
			{
				client.child_storage_hash(hash, child_info, key)
			}
		}
	}

	/// Given a block's `Hash` and a key, return the closest merkle value.
	fn closest_merkle_value(
		&self,
		hash: <Block as BlockT>::Hash,
		key: &StorageKey,
	) -> polkadot_sdk::sp_blockchain::Result<Option<MerkleValue<<Block as BlockT>::Hash>>> {
		with_client! {
			self,
			client,
			{
				client.closest_merkle_value(hash, key)
			}
		}
	}

	/// Given a block's `Hash`, a key and a child storage key, return the closest merkle value.
	fn child_closest_merkle_value(
		&self,
		hash: <Block as BlockT>::Hash,
		child_info: &ChildInfo,
		key: &StorageKey,
	) -> polkadot_sdk::sp_blockchain::Result<Option<MerkleValue<<Block as BlockT>::Hash>>> {
		with_client! {
			self,
			client,
			{
				client.child_closest_merkle_value(hash, child_info, key)
			}
		}
	}
}

impl polkadot_sdk::sp_blockchain::HeaderBackend<Block> for Client {
	fn header(&self, hash: Hash) -> polkadot_sdk::sp_blockchain::Result<Option<Header>> {
		with_client! {
			self,
			client,
			{
				client.header(hash)
			}
		}
	}

	fn info(&self) -> polkadot_sdk::sp_blockchain::Info<Block> {
		with_client! {
			self,
			client,
			{
				client.info()
			}
		}
	}

	fn status(&self, hash: Hash) -> polkadot_sdk::sp_blockchain::Result<polkadot_sdk::sp_blockchain::BlockStatus> {
		with_client! {
			self,
			client,
			{
				client.status(hash)
			}
		}
	}

	fn number(&self, hash: Hash) -> polkadot_sdk::sp_blockchain::Result<Option<BlockNumber>> {
		with_client! {
			self,
			client,
			{
				client.number(hash)
			}
		}
	}

	fn hash(&self, number: BlockNumber) -> polkadot_sdk::sp_blockchain::Result<Option<Hash>> {
		with_client! {
			self,
			client,
			{
				client.hash(number)
			}
		}
	}
}

#[allow(unused_macros)]
macro_rules! signed_payload {
	(
  $extra:ident, $raw_payload:ident,
  (
    $period:expr,
    $current_block:expr,
    $nonce:expr,
    $tip:expr,
    $call:expr,
    $genesis:expr
  )
  ) => {
		let $extra: runtime::SignedExtra = (
			polkadot_sdk::frame_system::CheckNonZeroSender::<runtime::Runtime>::new(),
			polkadot_sdk::frame_system::CheckSpecVersion::<runtime::Runtime>::new(),
			polkadot_sdk::frame_system::CheckTxVersion::<runtime::Runtime>::new(),
			polkadot_sdk::frame_system::CheckGenesis::<runtime::Runtime>::new(),
			polkadot_sdk::frame_system::CheckMortality::<runtime::Runtime>::from(
				polkadot_sdk::sp_runtime::generic::Era::mortal($period, $current_block),
			),
			polkadot_sdk::frame_system::CheckNonce::<runtime::Runtime>::from($nonce),
			polkadot_sdk::frame_system::CheckWeight::<runtime::Runtime>::new(),
			polkadot_sdk::pallet_transaction_payment::ChargeTransactionPayment::<runtime::Runtime>::from($tip),
		);

		let $raw_payload = runtime::SignedPayload::from_raw(
			$call.clone(),
			$extra.clone(),
			(
				(),
				runtime::VERSION.spec_version,
				runtime::VERSION.transaction_version,
				$genesis.clone(),
				$genesis,
				(),
				(),
				(),
			),
		);
	};
}

pub trait RuntimeApiCollection:
	polkadot_sdk::sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
	+ polkadot_sdk::sp_api::ApiExt<Block>
	+ polkadot_sdk::sp_consensus_babe::BabeApi<Block>
	+ polkadot_sdk::sp_consensus_grandpa::GrandpaApi<Block>
	+ polkadot_sdk::sp_block_builder::BlockBuilder<Block>
	+ polkadot_sdk::frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
	+ polkadot_sdk::pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
	+ pallet_ismp_runtime_api::IsmpRuntimeApi<Block, H256>
	+ polkadot_sdk::sp_api::Metadata<Block>
	+ polkadot_sdk::sp_offchain::OffchainWorkerApi<Block>
	+ polkadot_sdk::sp_session::SessionKeys<Block>
	+ polkadot_sdk::sp_authority_discovery::AuthorityDiscoveryApi<Block>
{
}

impl<Api> RuntimeApiCollection for Api where
	Api: polkadot_sdk::sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ polkadot_sdk::sp_api::ApiExt<Block>
		+ polkadot_sdk::sp_consensus_babe::BabeApi<Block>
		+ polkadot_sdk::sp_consensus_grandpa::GrandpaApi<Block>
		+ polkadot_sdk::sp_block_builder::BlockBuilder<Block>
		+ polkadot_sdk::frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
		+ polkadot_sdk::pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
		+ pallet_ismp_runtime_api::IsmpRuntimeApi<Block, H256>
		+ polkadot_sdk::sp_api::Metadata<Block>
		+ polkadot_sdk::sp_offchain::OffchainWorkerApi<Block>
		+ polkadot_sdk::sp_session::SessionKeys<Block>
		+ polkadot_sdk::sp_authority_discovery::AuthorityDiscoveryApi<Block>
{
}

pub fn benchmark_inherent_data() -> Result<polkadot_sdk::sp_inherents::InherentData, polkadot_sdk::sp_inherents::Error> {
	use polkadot_sdk::sp_inherents::InherentDataProvider;

	let mut inherent_data = polkadot_sdk::sp_inherents::InherentData::new();
	let d = std::time::Duration::from_millis(0);
	let timestamp = polkadot_sdk::sp_timestamp::InherentDataProvider::new(d.into());

	futures::executor::block_on(timestamp.provide_inherent_data(&mut inherent_data))?;

	Ok(inherent_data)
}
