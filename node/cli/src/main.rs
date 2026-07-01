//! Substrate Node Template CLI library.
#![warn(missing_docs)]

#[allow(clippy::result_large_err)]
fn main() -> polkadot_sdk::sc_cli::Result<()> {
	cere_cli::run()
}
