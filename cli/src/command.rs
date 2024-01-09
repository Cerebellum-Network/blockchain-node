use crate::cli::{Cli, Subcommand};

use cere_service::{self, IdentifyVariant};
use frame_benchmarking_cli::{BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE};
use sc_cli::{RuntimeVersion, SubstrateCli};
use sc_service::error::Error as ServiceError;
use futures::future::TryFutureExt;
use log::info;
// use cere_client::benchmarking::{
// 	benchmark_inherent_data, ExistentialDepositProvider, RemarkBuilder, TransferKeepAliveBuilder,
// };
use sp_core::crypto::Ss58AddressFormatRegistry;
use sp_keyring::Sr25519Keyring;
use std::net::ToSocketAddrs;

pub use crate::error::Error;

impl From<String> for Error {
	fn from(s: String) -> Self {
		Self::Other(s)
	}
}

type Result<T> = std::result::Result<T, Error>;

fn get_exec_name() -> Option<String> {
	std::env::current_exe()
		.ok()
		.and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
		.and_then(|s| s.into_string().ok())
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Cere Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://cere.network/discord".into()
	}

	fn copyright_start_year() -> i32 {
		2017
	}

	fn executable_name() -> String {
		"cere".into()
	}
	
	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"cere-mainnet" => Box::new(cere_service::chain_spec::cere_mainnet_config()?),
			"cere-testnet" => Box::new(cere_service::chain_spec::cere_testnet_config()?),
			"cere-qanet" => Box::new(cere_service::chain_spec::cere_qanet_config()?),
			#[cfg(feature = "cere-dev-native")]
			"cere-devnet" => Box::new(cere_service::chain_spec::cere_devnet_config()?),
			#[cfg(feature = "cere-dev-native")]
			"dev" => Box::new(cere_service::chain_spec::cere_dev_development_config()?),
			#[cfg(feature = "cere-dev-native")]
			"local" => Box::new(cere_service::chain_spec::cere_dev_local_testnet_config()?),
			path => {
				let path = std::path::PathBuf::from(path);
				let chain_spec =
					Box::new(cere_service::CereChainSpec::from_json_file(path.clone())?)
						as Box<dyn cere_service::ChainSpec>;

				if self.run.force_cere_dev || chain_spec.is_cere_dev() {
					Box::new(cere_service::CereDevChainSpec::from_json_file(path)?)
				} else {
					chain_spec
				}
			},
		})
	}

	fn native_runtime_version(spec: &Box<dyn cere_service::ChainSpec>) -> &'static RuntimeVersion {
		#[cfg(feature = "cere-dev-native")]
		if spec.is_cere_dev() {
			return &cere_service::cere_dev_runtime::VERSION
		}

		#[cfg(not(all(feature = "cere-dev-native")))]
		let _ = spec;

		#[cfg(feature = "cere-native")]
		{
			&cere_service::cere_runtime::VERSION
		}

		#[cfg(not(feature = "cere-native"))]
		panic!("No runtime feature (cere, cere-dev) is enabled")
	}
}

fn set_default_ss58_version(spec: &Box<dyn cere_service::ChainSpec>) {
	let ss58_version = Ss58AddressFormatRegistry::CereAccount.into();

	sp_core::crypto::set_default_ss58_version(ss58_version);
}

// const DEV_ONLY_ERROR_PATTERN: &'static str =
// 	"can only use subcommand with --chain [polkadot-dev, kusama-dev, westend-dev, rococo-dev, wococo-dev], got ";

// fn ensure_dev(spec: &Box<dyn cere_service::ChainSpec>) -> std::result::Result<(), String> {
// 	if spec.is_dev() {
// 		Ok(())
// 	} else {
// 		Err(format!("{}{}", DEV_ONLY_ERROR_PATTERN, spec.id()))
// 	}
// }

/// Unwraps a [`cere_client::Client`] into the concrete runtime client.
macro_rules! unwrap_client {
	(
		$client:ident,
		$code:expr
	) => {
		match $client.as_ref() {
			#[cfg(feature = "cere-native")]
			cere_client::Client::Cere($client) => $code,
			#[cfg(feature = "cere-dev-native")]
			cere_client::Client::CereDev($client) => $code,
			#[allow(unreachable_patterns)]
			_ => Err(Error::CommandNotImplemented),
		}
	};
}

/// Parse and run command line arguments
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => Ok(cmd.run(&cli)?),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd).map_err(Error::SubstrateCli)?;
			Ok(runner.sync_run(|config| cmd.run(config.chain_spec, config.network))?)
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);
			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) =
					cere_service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, import_queue).map_err(Error::SubstrateCli), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, _, task_manager) =
					cere_service::new_chain_ops(&mut config).map_err(Error::SubstrateService)?;
				Ok((cmd.run(client, config.database).map_err(Error::SubstrateCli), task_manager))
			})?)
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, _, task_manager) = cere_service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, config.chain_spec).map_err(Error::SubstrateCli), task_manager))
			})?)
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = cere_service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, import_queue).map_err(Error::SubstrateCli), task_manager))
			})?)
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run(config.database))?)
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, backend, _, task_manager) = cere_service::new_chain_ops(&mut config)?;
				let aux_revert = Box::new(|client, backend, blocks| {
					cere_service::revert_backend(client, backend, blocks).map_err(|err| {
						match err {
							sc_service::Error::Application(err) => err.into(), //fix
							// Generic application-specific error.
							err => sc_cli::Error::Application(err.into()),
						}
					})
				});
				Ok((
					cmd.run(client, backend, Some(aux_revert)).map_err(Error::SubstrateCli),
					task_manager,
				))
			})?)
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			match cmd {
				BenchmarkCmd::Pallet(cmd) => {
					set_default_ss58_version(chain_spec);
					// ensure_dev(chain_spec).map_err(Error::Other)?;

					#[cfg(feature = "cere-dev-native")]
					if chain_spec.is_cere_dev() {
						return runner.sync_run(|config| {
							cmd.run::<cere_service::cere_dev_runtime::Block, cere_client::CereDevExecutorDispatch>(config).map_err(|e| Error::SubstrateCli(e))
						})
					}

					// else we assume it is Cere
					#[cfg(feature = "cere-native")]
					{
						runner.sync_run(|config| {
							cmd.run::<cere_service::cere_runtime::Block, cere_client::CereExecutorDispatch>(config).map_err(|e| Error::SubstrateCli(e))
						})
					}

					#[cfg(not(feature = "cere-native"))]
					#[allow(unreachable_code)]
					Error::Service(ServiceError::Other(
						"No runtime feature (cere-native, cere-dev-native) is enabled".to_string(),
					))
				},
				BenchmarkCmd::Block(cmd) => runner.sync_run(|config| {
					let (client, _, _, _) = cere_service::new_chain_ops(&config)?;
					unwrap_client!(client, cmd.run(client.clone()).map_err(Error::SubstrateCli))
				}),
				#[cfg(not(feature = "runtime-benchmarks"))]
				BenchmarkCmd::Storage(_) =>
					return Err(sc_cli::Error::Input(
						"Compile with --features=runtime-benchmarks \
						to enable storage benchmarks."
							.into(),
					)
					.into()),
				#[cfg(feature = "runtime-benchmarks")]
				BenchmarkCmd::Storage(cmd) => runner.sync_run(|config| {
					let (client, backend, _, _) = cere_service::new_chain_ops(&config)?;
					let db = backend.expose_db();
					let storage = backend.expose_storage();
					unwrap_client!(client, cmd.run(config, client.clone(), db, storage))
				}),
				BenchmarkCmd::Overhead(_cmd) => {
					print!("BenchmarkCmd::Overhead is not supported");
					unimplemented!()
				},
				BenchmarkCmd::Extrinsic(_cmd) => {
					print!("BenchmarkCmd::Extrinsic is not supported");
					unimplemented!()
				},
				BenchmarkCmd::Machine(cmd) =>
				runner.sync_run(|config| {
					cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())
						.map_err(Error::SubstrateCli)
				}),
			}
		},
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			let registry = &runner.config().prometheus_config.as_ref().map(|cfg| &cfg.registry);
			let task_manager =
				sc_service::TaskManager::new(runner.config().tokio_handle.clone(), *registry)
					.map_err(|e| Error::SubstrateService(ServiceError::Prometheus(e)))?;

			#[cfg(feature = "cere-dev-native")]
			if chain_spec.is_cere_dev() {
				return runner.async_run(|config| {
          Ok((
						cmd.run::<cere_service::cere_dev_runtime::Block, cere_service::CereDevExecutorDispatch>(
							config,
						).map_err(Error::SubstrateCli), 
						task_manager,
					))
        })
			}

			#[cfg(feature = "cere-native")]
			{
				return runner.async_run(|config| {
          Ok((
						cmd.run::<cere_service::cere_runtime::Block, cere_service::CereExecutorDispatch>(
							config,
						).map_err(Error::SubstrateCli),
						task_manager,
					))
        })
			}

			#[cfg(not(feature = "cere-native"))]
			panic!("No runtime feature (cere, cere-dev) is enabled")
		},
		#[cfg(not(feature = "try-runtime"))]
		Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
				You can enable it with `--features try-runtime`."
			.into()),
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run::<cere_service::Block>(&config))?)
		},
		None => {
			let runner = cli.create_runner(&cli.run.base)?;
			runner.run_node_until_exit(|config| async move {
				cere_service::build_full(config, cli.run.no_hardware_benchmarks)
					.map(|full| full.task_manager)
					.map_err(Error::SubstrateService)
			})
		},
	}
}
