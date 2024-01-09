//! Substrate Node Template CLI library.
#![warn(missing_docs)]

use color_eyre::eyre;

fn main() -> eyre::Result<()> {
	color_eyre::install()?;
	cere_cli::run()?;
	Ok(())
}
