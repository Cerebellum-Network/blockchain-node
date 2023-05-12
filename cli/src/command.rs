use crate::cli::{Cli, Subcommand};

use cere_service::{self, IdentifyVariant};
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use sc_cli::{Error, RuntimeVersion, SubstrateCli};
use sc_service::error::Error as ServiceError;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Substrate Node".into()
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

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		let id = if id == "" {
			let n = get_exec_name().unwrap_or_default();
			["cere", "cere-dev"]
				.iter()
				.cloned()
				.find(|&chain| n.starts_with(chain))
				.unwrap_or("cere")
		} else {
			id
		};

		Ok(match id {
			"cere-mainnet" => Box::new(cere_service::chain_spec::cere_mainnet_config()?),
			"cere-testnet" => Box::new(cere_service::chain_spec::cere_testnet_config()?),
			"cere-qanet" => Box::new(cere_service::chain_spec::cere_qanet_config()?),
			#[cfg(feature = "cere-dev-native")]
			"cere-devnet" | "devnet" => Box::new(cere_service::chain_spec::cere_devnet_config()?),
			path => {
				let path = std::path::PathBuf::from(path);

				if self.run.force_cere_dev {
					Box::new(cere_service::CereDevChainSpec::from_json_file(path)?)
				} else {
					Box::new(cere_service::CereChainSpec::from_json_file(path.clone())?)
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
			return &cere_service::cere_runtime::VERSION
		}

		#[cfg(not(feature = "cere-native"))]
		panic!("No runtime feature (cere, cere-dev) is enabled")
	}
}

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
			_ => Err(Error::Service(ServiceError::Other(
				"No runtime feature (cere-native, cere-dev-native) is enabled".to_string(),
			))),
		}
	};
}

fn get_exec_name() -> Option<String> {
	std::env::current_exe()
		.ok()
		.and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
		.and_then(|s| s.into_string().ok())
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let (client, _, import_queue, task_manager) = cere_service::new_chain_ops(&config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let (client, _, _, task_manager) = cere_service::new_chain_ops(&config)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let (client, _, _, task_manager) = cere_service::new_chain_ops(&config)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let (client, _, import_queue, task_manager) = cere_service::new_chain_ops(&config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let (client, backend, _, task_manager) = cere_service::new_chain_ops(&config)?;
				let aux_revert = Box::new(|client, backend, blocks| {
					cere_service::revert_backend(client, backend, blocks)?;
					Ok(())
				});
				Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
			})
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			match cmd {
				BenchmarkCmd::Pallet(cmd) => {
					if !cfg!(feature = "runtime-benchmarks") {
						return Err("Runtime benchmarking wasn't enabled when building the node. \
            You can enable it with `--features runtime-benchmarks`."
							.into())
					}

					#[cfg(feature = "cere-dev-native")]
					if chain_spec.is_cere_dev() {
						return Ok(runner.sync_run(|config| {
							cmd.run::<cere_service::cere_dev_runtime::Block, cere_client::CereDevExecutorDispatch>(config)
						})?)
					}

					// else we assume it is Cere
					#[cfg(feature = "cere-native")]
					{
						return Ok(runner.sync_run(|config| {
							cmd.run::<cere_service::cere_runtime::Block, cere_client::CereExecutorDispatch>(config)
						})?)
					}

					#[cfg(not(feature = "cere-native"))]
					#[allow(unreachable_code)]
					Error::Service(ServiceError::Other(
						"No runtime feature (cere-native, cere-dev-native) is enabled".to_string(),
					))
				},
				BenchmarkCmd::Block(cmd) => runner.sync_run(|config| {
					let (client, _, _, _) = cere_service::new_chain_ops(&config)?;
					unwrap_client!(client, cmd.run(client.clone()))
				}),
				BenchmarkCmd::Storage(cmd) => runner.sync_run(|config| {
					let (client, backend, _, _) = cere_service::new_chain_ops(&config)?;
					let db = backend.expose_db();
					let storage = backend.expose_storage();
					unwrap_client!(client, cmd.run(config, client.clone(), db, storage))
				}),
				BenchmarkCmd::Overhead(cmd) => runner.sync_run(|config| {
					use cere_client::benchmark_inherent_data;

					let (client, _, _, _) = cere_service::new_chain_ops(&config)?;
					let wrapped = client.clone();

					let inherent_data = benchmark_inherent_data()
						.map_err(|e| format!("generating inherent data: {:?}", e))?;

					unwrap_client!(client, cmd.run(config, client.clone(), inherent_data, wrapped))
				}),
				BenchmarkCmd::Machine(cmd) =>
					runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())),
			}
		},
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			let registry = &runner.config().prometheus_config.as_ref().map(|cfg| &cfg.registry);
			let task_manager =
				sc_service::TaskManager::new(runner.config().tokio_handle.clone(), *registry)
					.map_err(|e| Error::Service(ServiceError::Prometheus(e)))?;

			#[cfg(feature = "cere-dev-native")]
			if chain_spec.is_cere_dev() {
				return runner.async_run(|config| {
            Ok((cmd.run::<cere_service::cere_dev_runtime::Block, cere_service::CereDevExecutorDispatch>(config), task_manager))
        })
			}

			#[cfg(feature = "cere-native")]
			{
				return runner.async_run(|config| {
          Ok((cmd.run::<cere_service::cere_runtime::Block, cere_service::CereExecutorDispatch>(config), task_manager))
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
			runner.sync_run(|config| cmd.run::<cere_service::Block>(&config))
		},
		None => {
			let runner = cli.create_runner(&cli.run.base)?;
			runner.run_node_until_exit(|config| async move {
				cere_service::build_full(config, cli.run.no_hardware_benchmarks)
					.map(|full| full.task_manager)
					.map_err(Error::Service)
			})
		},
	}
}
