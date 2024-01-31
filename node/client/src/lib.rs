use std::sync::Arc;

pub use node_primitives::{AccountId, Balance, Block, BlockNumber, Hash, Header, Index, Signature};
use sc_client_api::{AuxStore, Backend as BackendT, BlockchainEvents, KeyIterator, UsageProvider};
pub use sc_executor::NativeElseWasmExecutor;
use sp_api::{CallApiAt, NumberFor, ProvideRuntimeApi};
use sp_blockchain::{HeaderBackend, HeaderMetadata};
use sp_consensus::BlockStatus;
use sp_runtime::{
	generic::SignedBlock,
	traits::{BlakeTwo256, Block as BlockT},
	Justifications,
};
use sp_storage::{ChildInfo, StorageData, StorageKey};

pub type FullBackend = sc_service::TFullBackend<Block>;
pub type FullClient<RuntimeApi, ExecutorDispatch> =
	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;

#[cfg(not(any(feature = "cere", feature = "cere-dev",)))]
compile_error!("at least one runtime feature must be enabled");

/// The native executor instance for Cere.
#[cfg(feature = "cere")]
pub struct CereExecutorDispatch;

#[cfg(feature = "cere")]
impl sc_executor::NativeExecutionDispatch for CereExecutorDispatch {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		cere_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		cere_runtime::native_version()
	}
}

/// The native executor instance for Cere Dev.
#[cfg(feature = "cere-dev")]
pub struct CereDevExecutorDispatch;

#[cfg(feature = "cere-dev")]
impl sc_executor::NativeExecutionDispatch for CereDevExecutorDispatch {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		cere_dev_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		cere_dev_runtime::native_version()
	}
}

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
	+ HeaderMetadata<Block, Error = sp_blockchain::Error>
where
	Block: BlockT,
	Backend: BackendT<Block>,
	Backend::State: sp_api::StateBackend<BlakeTwo256>,
	Self::Api: RuntimeApiCollection<StateBackend = Backend::State>,
{
}

impl<Block, Backend, Client> AbstractClient<Block, Backend> for Client
where
	Block: BlockT,
	Backend: BackendT<Block>,
	Backend::State: sp_api::StateBackend<BlakeTwo256>,
	Client: BlockchainEvents<Block>
		+ ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ AuxStore
		+ UsageProvider<Block>
		+ Sized
		+ Send
		+ Sync
		+ CallApiAt<Block, StateBackend = Backend::State>
		+ HeaderMetadata<Block, Error = sp_blockchain::Error>,
	Client::Api: RuntimeApiCollection<StateBackend = Backend::State>,
{
}

/// See [`ExecuteWithClient`] for more information.
#[derive(Clone)]
pub enum Client {
	#[cfg(feature = "cere")]
	Cere(Arc<FullClient<cere_runtime::RuntimeApi, CereExecutorDispatch>>),
	#[cfg(feature = "cere-dev")]
	CereDev(Arc<FullClient<cere_dev_runtime::RuntimeApi, CereDevExecutorDispatch>>),
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
		<Api as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
		Backend: sc_client_api::Backend<Block> + 'static,
		Backend::State: sp_api::StateBackend<BlakeTwo256>,
		Api: crate::RuntimeApiCollection<StateBackend = Backend::State>,
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
	fn usage_info(&self) -> sc_client_api::ClientInfo<Block> {
		with_client! {
			self,
			client,
			{
				client.usage_info()
			}
		}
	}
}

impl sc_client_api::BlockBackend<Block> for Client {
	fn block_body(
		&self,
		hash: <Block as BlockT>::Hash,
	) -> sp_blockchain::Result<Option<Vec<<Block as BlockT>::Extrinsic>>> {
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
	) -> sp_blockchain::Result<Option<SignedBlock<Block>>> {
		with_client! {
			self,
			client,
			{
				client.block(hash)
			}
		}
	}

	fn block_status(&self, hash: <Block as BlockT>::Hash) -> sp_blockchain::Result<BlockStatus> {
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
	) -> sp_blockchain::Result<Option<Justifications>> {
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
	) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
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
	) -> sp_blockchain::Result<Option<Vec<u8>>> {
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
	) -> sp_blockchain::Result<Option<Vec<Vec<u8>>>> {
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

impl sc_client_api::StorageProvider<Block, crate::FullBackend> for Client {
	fn storage(
		&self,
		hash: <Block as BlockT>::Hash,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<StorageData>> {
		with_client! {
			self,
			client,
			{
				client.storage(hash, key)
			}
		}
	}

	fn storage_keys(
		&self,
		hash: <Block as BlockT>::Hash,
		key_prefix: &StorageKey,
	) -> sp_blockchain::Result<Vec<StorageKey>> {
		with_client! {
			self,
			client,
			{
				client.storage_keys(hash, key_prefix)
			}
		}
	}

	fn storage_hash(
		&self,
		hash: <Block as BlockT>::Hash,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
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
		key_prefix: &StorageKey,
	) -> sp_blockchain::Result<Vec<(StorageKey, StorageData)>> {
		with_client! {
			self,
			client,
			{
				client.storage_pairs(hash, key_prefix)
			}
		}
	}

	fn storage_keys_iter(
		&self,
		hash: <Block as BlockT>::Hash,
		prefix: Option<&StorageKey>,
		start_key: Option<&StorageKey>,
	) -> sp_blockchain::Result<
		KeyIterator<<crate::FullBackend as sc_client_api::Backend<Block>>::State, Block>,
	> {
		with_client! {
			self,
			client,
			{
				client.storage_keys_iter(hash, prefix, start_key)
			}
		}
	}

	fn child_storage(
		&self,
		hash: <Block as BlockT>::Hash,
		child_info: &ChildInfo,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<StorageData>> {
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
		child_info: &ChildInfo,
		key_prefix: &StorageKey,
	) -> sp_blockchain::Result<Vec<StorageKey>> {
		with_client! {
			self,
			client,
			{
				client.child_storage_keys(hash, child_info, key_prefix)
			}
		}
	}

	fn child_storage_keys_iter(
		&self,
		hash: <Block as BlockT>::Hash,
		child_info: ChildInfo,
		prefix: Option<&StorageKey>,
		start_key: Option<&StorageKey>,
	) -> sp_blockchain::Result<
		KeyIterator<<crate::FullBackend as sc_client_api::Backend<Block>>::State, Block>,
	> {
		with_client! {
			self,
			client,
			{
				client.child_storage_keys_iter(hash, child_info, prefix, start_key)
			}
		}
	}

	fn child_storage_hash(
		&self,
		hash: <Block as BlockT>::Hash,
		child_info: &ChildInfo,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		with_client! {
			self,
			client,
			{
				client.child_storage_hash(hash, child_info, key)
			}
		}
	}
}

impl sp_blockchain::HeaderBackend<Block> for Client {
	fn header(&self, hash: Hash) -> sp_blockchain::Result<Option<Header>> {
		with_client! {
			self,
			client,
			{
				client.header(hash)
			}
		}
	}

	fn info(&self) -> sp_blockchain::Info<Block> {
		with_client! {
			self,
			client,
			{
				client.info()
			}
		}
	}

	fn status(&self, hash: Hash) -> sp_blockchain::Result<sp_blockchain::BlockStatus> {
		with_client! {
			self,
			client,
			{
				client.status(hash)
			}
		}
	}

	fn number(&self, hash: Hash) -> sp_blockchain::Result<Option<BlockNumber>> {
		with_client! {
			self,
			client,
			{
				client.number(hash)
			}
		}
	}

	fn hash(&self, number: BlockNumber) -> sp_blockchain::Result<Option<Hash>> {
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
			frame_system::CheckNonZeroSender::<runtime::Runtime>::new(),
			frame_system::CheckSpecVersion::<runtime::Runtime>::new(),
			frame_system::CheckTxVersion::<runtime::Runtime>::new(),
			frame_system::CheckGenesis::<runtime::Runtime>::new(),
			frame_system::CheckMortality::<runtime::Runtime>::from(
				sp_runtime::generic::Era::mortal($period, $current_block),
			),
			frame_system::CheckNonce::<runtime::Runtime>::from($nonce),
			frame_system::CheckWeight::<runtime::Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<runtime::Runtime>::from($tip),
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
	sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
	+ sp_api::ApiExt<Block>
	+ sp_consensus_babe::BabeApi<Block>
	+ sp_finality_grandpa::GrandpaApi<Block>
	+ sp_block_builder::BlockBuilder<Block>
	+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index>
	+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
	+ sp_api::Metadata<Block>
	+ sp_offchain::OffchainWorkerApi<Block>
	+ sp_session::SessionKeys<Block>
	+ sp_authority_discovery::AuthorityDiscoveryApi<Block>
where
	<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

impl<Api> RuntimeApiCollection for Api
where
	Api: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::ApiExt<Block>
		+ sp_consensus_babe::BabeApi<Block>
		+ sp_finality_grandpa::GrandpaApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index>
		+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
		+ sp_api::Metadata<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_authority_discovery::AuthorityDiscoveryApi<Block>,
	<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

pub fn benchmark_inherent_data() -> Result<sp_inherents::InherentData, sp_inherents::Error> {
	use sp_inherents::InherentDataProvider;

	let mut inherent_data = sp_inherents::InherentData::new();
	let d = std::time::Duration::from_millis(0);
	let timestamp = sp_timestamp::InherentDataProvider::new(d.into());

	futures::executor::block_on(timestamp.provide_inherent_data(&mut inherent_data))?;

	Ok(inherent_data)
}
