//! Substrate Node Template CLI library.
#![warn(missing_docs)]
use private_pallet::sensitive_function;
fn main() -> sc_cli::Result<()> {

	sensitive_function();
	println!("***************");
	cere_cli::run()
}
