//! Provisional weights for `orml_oracle`.
//!
//! These are **not** benchmarked. They are hand-set, deliberately conservative
//! estimates so the pallet does not run on upstream's defaults, which were
//! benchmarked on Acala hardware in 2021.
//!
//! The storage access counts are read off the extrinsic rather than guessed:
//! `feed_values` reads the membership set and `HasDispatched`, writes
//! `HasDispatched`, and for each fed value reads the key's `RawValues` (to
//! combine) and writes both `RawValues` and `Values`. `on_finalize` clears
//! `HasDispatched`.
//!
//! Regenerate with the benchmark CLI before mainnet:
//!
//! ```text
//! cere benchmark pallet --chain=dev --pallet=orml_oracle --extrinsic='*' \
//!   --wasm-execution=compiled --steps=50 --repeat=20 \
//!   --output=runtime/cere-dev/src/weights/orml_oracle.rs
//! ```

#![allow(unused_parens)]
#![allow(clippy::unnecessary_cast)]

use core::marker::PhantomData;
use polkadot_sdk::frame_support::{traits::Get, weights::Weight};

/// Provisional weights for `orml_oracle`, parameterised by the runtime's
/// configured DB weights.
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: polkadot_sdk::frame_system::Config> orml_oracle::WeightInfo for WeightInfo<T> {
	/// `c` is the number of values fed in one call, bounded by `MaxFeedValues`.
	fn feed_values(c: u32) -> Weight {
		// Roughly 2.5x upstream's 2021 base to stay conservative on unknown
		// validator hardware; over-estimating costs block space, under-
		// estimating is a liveness risk.
		Weight::from_parts(40_000_000, 0)
			.saturating_add(Weight::from_parts(10_000_000, 0).saturating_mul(c as u64))
			// membership set + HasDispatched + the previous aggregate
			.saturating_add(T::DbWeight::get().reads(3_u64))
			// one RawValues read per fed key, to recombine
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(c as u64)))
			// HasDispatched
			.saturating_add(T::DbWeight::get().writes(1_u64))
			// RawValues + Values per fed key
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(c as u64)))
	}

	fn on_finalize() -> Weight {
		Weight::from_parts(10_000_000, 0).saturating_add(T::DbWeight::get().writes(1_u64))
	}
}
