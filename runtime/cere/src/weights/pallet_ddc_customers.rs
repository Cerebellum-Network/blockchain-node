//! Autogenerated weights for pallet_ddc_customers
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-12-20, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `bench`, CPU: `DO-Premium-AMD`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Interpreted, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/cere
// benchmark
// pallet
// --chain=dev
// --execution=wasm
// --pallet=pallet-ddc-customers
// --extrinsic=*
// --steps=50
// --repeat=20
// --template=./.maintain/frame-weight-template.hbs
// --output=pallets/ddc-customers/src/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weights for pallet_ddc_customers using the Substrate node and recommended hardware.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_ddc_customers::WeightInfo for WeightInfo<T> {
	// Storage: DdcCustomers BucketsCount (r:1 w:1)
	// Storage: DdcClusters Clusters (r:1 w:0)
	// Storage: DdcCustomers Buckets (r:0 w:1)
	fn create_bucket() -> Weight {
		Weight::from_parts(156_518_000_u64, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	// Storage: DdcCustomers Ledger (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	fn deposit() -> Weight {
		Weight::from_parts(439_562_000_u64, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	// Storage: DdcCustomers Ledger (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	fn deposit_extra() -> Weight {
		Weight::from_parts(582_699_000_u64, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	// Storage: DdcCustomers Ledger (r:1 w:1)
	fn unlock_deposit() -> Weight {
		Weight::from_parts(208_316_000_u64, 0)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: DdcCustomers Ledger (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	fn withdraw_unlocked_deposit_update() -> Weight {
		Weight::from_parts(451_983_000_u64, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	// Storage: DdcCustomers Ledger (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	fn withdraw_unlocked_deposit_kill() -> Weight {
		Weight::from_parts(599_908_000_u64, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	// Storage: DdcCustomers Buckets (r:1 w:1)
	fn set_bucket_params() -> Weight {
		Weight::from_parts(155_437_000_u64, 0)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}