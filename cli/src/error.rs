#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	SubstrateCli(#[from] sc_cli::Error),

	#[error(transparent)]
	SubstrateService(#[from] sc_service::Error),

	#[error(transparent)]
	SubstrateTracing(#[from] sc_tracing::logging::Error),

	#[cfg(not(feature = "pyroscope"))]
	#[error("Binary was not compiled with `--feature=pyroscope`")]
	PyroscopeNotCompiledIn,

	#[cfg(feature = "pyroscope")]
	#[error("Failed to connect to pyroscope agent")]
	PyroscopeError(#[from] pyro::error::PyroscopeError),

	#[error("Failed to resolve provided URL")]
	AddressResolutionFailure(#[from] std::io::Error),

	#[error("URL did not resolve to anything")]
	AddressResolutionMissing,

	#[error("Command is not implemented")]
	CommandNotImplemented,

	#[error("Other: {0}")]
	Other(String),
}