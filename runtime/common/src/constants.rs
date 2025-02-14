// This file is part of Substrate.

// Copyright (C) 2019-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A set of constant values used in substrate runtime.

/// Money matters.
pub mod currency {
	pub use ddc_primitives::Balance;

	pub const MILLICENTS: Balance = 100_000;
	pub const CENTS: Balance = 1_000 * MILLICENTS; // assume this is worth about a cent.
	pub const DOLLARS: Balance = 100 * CENTS;
	pub const GRAND: Balance = DOLLARS * 1_000;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
	}
}

/// Time.
pub mod time {
	pub use ddc_primitives::{BlockNumber, Moment};

	/// Since BABE is probabilistic this is the average expected block time that
	/// we are targeting. Blocks will be produced at a minimum duration defined
	/// by `SLOT_DURATION`, but some slots will not be allocated to any
	/// authority and hence no block will be produced. We expect to have this
	/// block time on average following the defined slot duration and the value
	/// of `c` configured for BABE (where `1 - c` represents the probability of
	/// a slot being empty).
	/// This value is only used indirectly to define the unit constants below
	/// that are expressed in blocks. The rest of the code should use
	/// `SLOT_DURATION` instead (like the Timestamp pallet for calculating the
	/// minimum period).
	///
	/// If using BABE with secondary slots (default) then all of the slots will
	/// always be assigned, in which case `MILLISECS_PER_BLOCK` and
	/// `SLOT_DURATION` should have the same value.
	///
	/// <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
	pub const MILLISECS_PER_BLOCK: Moment = 6000;
	pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;

	// NOTE: Currently it is not possible to change the slot duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

	// 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

	pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 4 * HOURS;
	pub const EPOCH_DURATION_IN_SLOTS: u64 = {
		const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

		(EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
	};

	// These time units are defined in number of blocks.
	pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
}

pub mod tracks {
	pub const fn percent(x: i32) -> sp_arithmetic::FixedI64 {
		sp_arithmetic::FixedI64::from_rational(x as u128, 100)
	}
	pub const fn percent_perbill(x: Perbill) -> sp_arithmetic::FixedI64 {
		sp_arithmetic::FixedI64::from_perbill(x)
	}
	pub use pallet_referenda::Curve;
	use sp_runtime::Perbill;

	pub const APP_ROOT: Curve =
		Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
	pub const SUP_ROOT: Curve = Curve::make_linear(28, 28, percent(20), percent(50));
	pub const APP_STAKING_ADMIN: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
	pub const SUP_STAKING_ADMIN: Curve =
		Curve::make_reciprocal(12, 28, percent(11), percent(10), percent(50));
	pub const APP_TREASURER: Curve =
		Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
	pub const SUP_TREASURER: Curve = Curve::make_linear(28, 28, percent(10), percent(50));
	pub const APP_GENERAL_ADMIN: Curve =
		Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
	pub const SUP_GENERAL_ADMIN: Curve =
		Curve::make_reciprocal(7, 28, percent(20), percent(10), percent(50));
	pub const APP_REFERENDUM_CANCELLER: Curve =
		Curve::make_linear(17, 28, percent(50), percent(100));
	pub const SUP_REFERENDUM_CANCELLER: Curve =
		Curve::make_reciprocal(12, 28, percent(11), percent(10), percent(50));
	pub const APP_REFERENDUM_KILLER: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
	pub const SUP_REFERENDUM_KILLER: Curve =
		Curve::make_reciprocal(12, 28, percent(11), percent(10), percent(50));
	pub const APP_SMALL_TIPPER: Curve = Curve::make_linear(10, 28, percent(50), percent(100));
	pub const SUP_SMALL_TIPPER: Curve =
		Curve::make_reciprocal(1, 28, percent(14), percent(10), percent(50));
	pub const APP_BIG_TIPPER: Curve = Curve::make_linear(10, 28, percent(50), percent(100));
	pub const SUP_BIG_TIPPER: Curve =
		Curve::make_reciprocal(8, 28, percent(11), percent(10), percent(50));
	pub const APP_SMALL_SPENDER: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
	pub const SUP_SMALL_SPENDER: Curve =
		Curve::make_reciprocal(12, 28, percent(11), percent(10), percent(50));
	pub const APP_MEDIUM_SPENDER: Curve = Curve::make_linear(23, 28, percent(50), percent(100));
	pub const SUP_MEDIUM_SPENDER: Curve =
		Curve::make_reciprocal(16, 28, percent(11), percent(10), percent(50));
	pub const APP_BIG_SPENDER: Curve = Curve::make_linear(28, 28, percent(50), percent(100));
	pub const SUP_BIG_SPENDER: Curve =
		Curve::make_reciprocal(20, 28, percent(11), percent(10), percent(50));
	pub const APP_WHITELISTED_CALLER: Curve =
		Curve::make_reciprocal(16, 28 * 24, percent(96), percent(50), percent(100));
	pub const SUP_WHITELISTED_CALLER: Curve = Curve::make_reciprocal(
		1,
		28,
		percent_perbill(Perbill::from_parts(1_000_000)), // 0.1 %
		percent_perbill(Perbill::from_parts(250_000)),   // 0.025 %
		percent(50),
	);
	pub const APP_CLUSTER_PROTOCOL_ACTIVATOR: Curve =
		Curve::make_reciprocal(16, 28 * 24, percent(96), percent(50), percent(100));
	pub const SUP_CLUSTER_PROTOCOL_ACTIVATOR: Curve = Curve::make_reciprocal(
		1,
		28,
		percent_perbill(Perbill::from_parts(100_000)), // 0.01 %
		percent_perbill(Perbill::from_parts(25_000)),  // 0.0025 %
		percent(50),
	);
	pub const APP_CLUSTER_PROTOCOL_UPDATER: Curve =
		Curve::make_reciprocal(16, 28 * 24, percent(96), percent(50), percent(100));
	pub const SUP_CLUSTER_PROTOCOL_UPDATER: Curve = Curve::make_reciprocal(
		1,
		28,
		percent_perbill(Perbill::from_parts(100_000)), // 0.01 %
		percent_perbill(Perbill::from_parts(25_000)),  // 0.0025 %
		percent(50),
	);

	// Root track
	pub const ROOT_TRACK_ID: u16 = 0;
	// Whitelister track
	pub const WHITELISTED_CALLER_TRACK_ID: u16 = 1;
	// General admin tracks
	pub const STAKING_ADMIN_TRACK_ID: u16 = 10;
	pub const TREASURER_TRACK_ID: u16 = 11;
	pub const GENERAL_ADMIN_TRACK_ID: u16 = 14;
	// Referendum admins tracks
	pub const REFERENDUM_CANCELER_TRACK_ID: u16 = 20;
	pub const REFERENDUM_KILLER_TRACK_ID: u16 = 21;
	// Limited treasury spenders tracks
	pub const SMALL_TIPPER_TRACK_ID: u16 = 30;
	pub const BIG_TIPPER_TRACK_ID: u16 = 31;
	pub const SMALL_SPENDER_TRACK_ID: u16 = 32;
	pub const MEDIUM_SPENDER_TRACK_ID: u16 = 33;
	pub const BIG_SPENDER_TRACK_ID: u16 = 34;
	// DDC admins tracks
	pub const CLUSTER_PROTOCOL_ACTIVATOR_TRACK_ID: u16 = 100;
	pub const CLUSTER_PROTOCOL_UPDATER_TRACK_ID: u16 = 101;
}
