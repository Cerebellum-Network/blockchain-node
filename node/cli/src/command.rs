use cere_service::IdentifyVariant;
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use sc_cli::{Error, SubstrateCli};
use sc_service::error::Error as ServiceError;

use crate::cli::{Cli, Subcommand};

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

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"cere-mainnet" => Box::new(cere_service::chain_spec::cere_mainnet_config()?),
			"cere-testnet" => Box::new(cere_service::chain_spec::cere_testnet_config()?),
			"cere-qanet" => Box::new(cere_service::chain_spec::cere_qanet_config()?),
			#[cfg(feature = "cere-native")]
			"cere-thirdparty" => Box::new(cere_service::chain_spec::cere_thirdparty_config()?),
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

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let mut cli = Cli::from_args();

	cli.run.base.offchain_worker_params.indexing_enabled = true;

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
							.into());
					}

					if chain_spec.is_cere_dev() {
						#[cfg(feature = "cere-dev-native")]
						return runner.sync_run(|config| {
							cmd.run_with_spec::<sp_runtime::traits::HashingFor<cere_service::cere_dev_runtime::Block>, ()>(Some(config.chain_spec))
						});

						#[cfg(not(feature = "cere-dev-native"))]
						Err(Error::Service(ServiceError::Other(
							"No runtime feature (cere-dev-native) is enabled".to_string(),
						)))
					} else {
						#[cfg(feature = "cere-native")]
						return runner.sync_run(|config| {
							cmd.run_with_spec::<sp_runtime::traits::HashingFor<cere_service::cere_runtime::Block>, ()>(Some(config.chain_spec))
						});

						#[cfg(not(feature = "cere-native"))]
						Err(Error::Service(ServiceError::Other(
							"No runtime feature (cere-native) is enabled".to_string(),
						)))
					}
				},
				BenchmarkCmd::Block(cmd) => runner.sync_run(|config| {
					let (client, _, _, _) = cere_service::new_chain_ops(&config)?;
					unwrap_client!(client, cmd.run(client.clone()))
				}),
				#[cfg(not(feature = "runtime-benchmarks"))]
				BenchmarkCmd::Storage(_) => {
					Err("Storage benchmarking can be enabled with `--features runtime-benchmarks`."
						.into())
				},
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
				BenchmarkCmd::Machine(cmd) => {
					runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()))
				},
			}
		},
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<cere_service::Block>(&config))
		},
		None => {
			let runner = cli.create_runner(&cli.run.base)?;
			runner.run_node_until_exit(|config| async move {
				cere_service::build_full::<sc_network::Litep2pNetworkBackend>(
					config,
					cli.run.no_hardware_benchmarks,
				)
				.map(|full| full.task_manager)
				.map_err(Error::Service)
			})
		},
	}
}
