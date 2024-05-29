// Ensure std is enabled
#[cfg(feature = "std")]
pub mod offchain {
	use polars::prelude::*;

	pub fn perform_offchain_work() {
		let data_frame = DataFrame::empty();
	}
}
