#[allow(missing_docs)]
#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,
	#[clap(flatten)]
	pub run: RunCmd,
}

#[allow(missing_docs)]
#[derive(Debug, clap::Parser)]
#[group(skip)]
pub struct RunCmd {
	#[clap(flatten)]
	pub base: polkadot_sdk::sc_cli::RunCmd,

	/// Force using Cere Dev runtime.
	#[arg(long = "force-cere-dev")]
	pub force_cere_dev: bool,

	/// Disable automatic hardware benchmarks.
	///
	/// By default these benchmarks are automatically ran at startup and measure
	/// the CPU speed, the memory bandwidth and the disk speed.
	///
	/// The results are then printed out in the logs, and also sent as part of
	/// telemetry, if telemetry is enabled.
	#[arg(long)]
	pub no_hardware_benchmarks: bool,

	/// Maximum wasm linear-memory pages (each page is 64 KB) for offchain
	/// worker execution. The default is 8192 (= 512 MB ceiling); memory grows
	/// on demand up to this value. Does not affect on-chain wasm execution.
	#[arg(long)]
	pub ocw_heap_pages: Option<u32>,
}

#[allow(missing_docs, clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Key management cli utilities
	#[command(subcommand)]
	Key(polkadot_sdk::sc_cli::KeySubcommand),

	/// Build a chain specification.
	/// DEPRECATED: `build-spec` command will be removed after 1/04/2026. Use `export-chain-spec`
	/// command instead.
	#[deprecated(
		note = "build-spec command will be removed after 1/04/2026. Use export-chain-spec command instead"
	)]
	BuildSpec(polkadot_sdk::sc_cli::BuildSpecCmd),

	/// Export the chain specification.
	ExportChainSpec(polkadot_sdk::sc_cli::ExportChainSpecCmd),

	/// Validate blocks.
	CheckBlock(polkadot_sdk::sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(polkadot_sdk::sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(polkadot_sdk::sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(polkadot_sdk::sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(polkadot_sdk::sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(polkadot_sdk::sc_cli::RevertCmd),

	/// Sub-commands concerned with benchmarking.
	#[command(subcommand)]
	Benchmark(polkadot_sdk::frame_benchmarking_cli::BenchmarkCmd),

	/// Db meta columns information.
	ChainInfo(polkadot_sdk::sc_cli::ChainInfoCmd),
}
