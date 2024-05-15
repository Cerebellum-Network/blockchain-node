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
	pub use node_primitives::Balance;

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
	use node_primitives::{BlockNumber, Moment};

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
	//! Track configurations for governance.xw
	use node_primitives::{Balance, BlockNumber};

	use super::{currency::*, time::*};

	const fn percent(x: i32) -> sp_arithmetic::FixedI64 {
		sp_arithmetic::FixedI64::from_rational(x as u128, 100)
	}
	use pallet_referenda::Curve;
	const APP_ROOT: Curve = Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
	const SUP_ROOT: Curve = Curve::make_linear(28, 28, percent(0), percent(50));
	const APP_STAKING_ADMIN: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
	const SUP_STAKING_ADMIN: Curve =
		Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50));
	const APP_TREASURER: Curve =
		Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
	const SUP_TREASURER: Curve = Curve::make_linear(28, 28, percent(0), percent(50));
	const APP_FELLOWSHIP_ADMIN: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
	const SUP_FELLOWSHIP_ADMIN: Curve =
		Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50));
	const APP_GENERAL_ADMIN: Curve =
		Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
	const SUP_GENERAL_ADMIN: Curve =
		Curve::make_reciprocal(7, 28, percent(10), percent(0), percent(50));
	const APP_REFERENDUM_CANCELLER: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
	const SUP_REFERENDUM_CANCELLER: Curve =
		Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50));
	const APP_REFERENDUM_KILLER: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
	const SUP_REFERENDUM_KILLER: Curve =
		Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50));
	const APP_SMALL_TIPPER: Curve = Curve::make_linear(10, 28, percent(50), percent(100));
	const SUP_SMALL_TIPPER: Curve =
		Curve::make_reciprocal(1, 28, percent(4), percent(0), percent(50));
	const APP_BIG_TIPPER: Curve = Curve::make_linear(10, 28, percent(50), percent(100));
	const SUP_BIG_TIPPER: Curve =
		Curve::make_reciprocal(8, 28, percent(1), percent(0), percent(50));
	const APP_SMALL_SPENDER: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
	const SUP_SMALL_SPENDER: Curve =
		Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50));
	const APP_MEDIUM_SPENDER: Curve = Curve::make_linear(23, 28, percent(50), percent(100));
	const SUP_MEDIUM_SPENDER: Curve =
		Curve::make_reciprocal(16, 28, percent(1), percent(0), percent(50));
	const APP_BIG_SPENDER: Curve = Curve::make_linear(28, 28, percent(50), percent(100));
	const SUP_BIG_SPENDER: Curve =
		Curve::make_reciprocal(20, 28, percent(1), percent(0), percent(50));
	const APP_WHITELISTED_CALLER: Curve =
		Curve::make_reciprocal(16, 28 * 24, percent(96), percent(50), percent(100));
	const SUP_WHITELISTED_CALLER: Curve =
		Curve::make_reciprocal(1, 28, percent(20), percent(5), percent(50));

	const APP_CLUSTER_ACTIVATOR: Curve = Curve::make_linear(10, 28, percent(0), percent(10));
	const SUP_CLUSTER_ACTIVATOR: Curve =
		Curve::make_reciprocal(1, 28, percent(4), percent(0), percent(10));

	const APP_CLUSTER_ECONOMICS_UPDATER: Curve =
		Curve::make_linear(10, 28, percent(0), percent(10));
	const SUP_CLUSTER_ECONOMICS_UPDATER: Curve =
		Curve::make_reciprocal(1, 28, percent(4), percent(0), percent(10));

	pub const CLUSTER_ACTIVATOR_TRACK_ID: u16 = 100;
	pub const CLUSTER_ECONOMICS_UPDATER_TRACK_ID: u16 = 101;

	pub const TRACKS_DATA: [(u16, pallet_referenda::TrackInfo<Balance, BlockNumber>); 15] = [
		(
			0,
			pallet_referenda::TrackInfo {
				name: "root",
				max_deciding: 1,
				decision_deposit: 100 * GRAND,
				prepare_period: 2 * HOURS,
				decision_period: 28 * DAYS,
				confirm_period: 24 * HOURS,
				min_enactment_period: 24 * HOURS,
				min_approval: APP_ROOT,
				min_support: SUP_ROOT,
			},
		),
		(
			1,
			pallet_referenda::TrackInfo {
				name: "whitelisted_caller",
				max_deciding: 100,
				decision_deposit: 10 * GRAND,
				prepare_period: 30 * MINUTES,
				decision_period: 28 * DAYS,
				confirm_period: 10 * MINUTES,
				min_enactment_period: 10 * MINUTES,
				min_approval: APP_WHITELISTED_CALLER,
				min_support: SUP_WHITELISTED_CALLER,
			},
		),
		(
			10,
			pallet_referenda::TrackInfo {
				name: "staking_admin",
				max_deciding: 10,
				decision_deposit: 5 * GRAND,
				prepare_period: 2 * HOURS,
				decision_period: 28 * DAYS,
				confirm_period: 3 * HOURS,
				min_enactment_period: 10 * MINUTES,
				min_approval: APP_STAKING_ADMIN,
				min_support: SUP_STAKING_ADMIN,
			},
		),
		(
			11,
			pallet_referenda::TrackInfo {
				name: "treasurer",
				max_deciding: 10,
				decision_deposit: GRAND,
				prepare_period: 2 * HOURS,
				decision_period: 28 * DAYS,
				confirm_period: 3 * HOURS,
				min_enactment_period: 24 * HOURS,
				min_approval: APP_TREASURER,
				min_support: SUP_TREASURER,
			},
		),
		(
			13,
			pallet_referenda::TrackInfo {
				name: "fellowship_admin",
				max_deciding: 10,
				decision_deposit: 5 * GRAND,
				prepare_period: 2 * HOURS,
				decision_period: 28 * DAYS,
				confirm_period: 3 * HOURS,
				min_enactment_period: 10 * MINUTES,
				min_approval: APP_FELLOWSHIP_ADMIN,
				min_support: SUP_FELLOWSHIP_ADMIN,
			},
		),
		(
			14,
			pallet_referenda::TrackInfo {
				name: "general_admin",
				max_deciding: 10,
				decision_deposit: 5 * GRAND,
				prepare_period: 2 * HOURS,
				decision_period: 28 * DAYS,
				confirm_period: 3 * HOURS,
				min_enactment_period: 10 * MINUTES,
				min_approval: APP_GENERAL_ADMIN,
				min_support: SUP_GENERAL_ADMIN,
			},
		),
		(
			20,
			pallet_referenda::TrackInfo {
				name: "referendum_canceller",
				max_deciding: 1_000,
				decision_deposit: 10 * GRAND,
				prepare_period: 2 * HOURS,
				decision_period: 7 * DAYS,
				confirm_period: 3 * HOURS,
				min_enactment_period: 10 * MINUTES,
				min_approval: APP_REFERENDUM_CANCELLER,
				min_support: SUP_REFERENDUM_CANCELLER,
			},
		),
		(
			21,
			pallet_referenda::TrackInfo {
				name: "referendum_killer",
				max_deciding: 1_000,
				decision_deposit: 50 * GRAND,
				prepare_period: 2 * HOURS,
				decision_period: 28 * DAYS,
				confirm_period: 3 * HOURS,
				min_enactment_period: 10 * MINUTES,
				min_approval: APP_REFERENDUM_KILLER,
				min_support: SUP_REFERENDUM_KILLER,
			},
		),
		(
			30,
			pallet_referenda::TrackInfo {
				name: "small_tipper",
				max_deciding: 200,
				decision_deposit: DOLLARS,
				prepare_period: MINUTES,
				decision_period: 7 * DAYS,
				confirm_period: 10 * MINUTES,
				min_enactment_period: MINUTES,
				min_approval: APP_SMALL_TIPPER,
				min_support: SUP_SMALL_TIPPER,
			},
		),
		(
			31,
			pallet_referenda::TrackInfo {
				name: "big_tipper",
				max_deciding: 100,
				decision_deposit: 10 * DOLLARS,
				prepare_period: 10 * MINUTES,
				decision_period: 7 * DAYS,
				confirm_period: HOURS,
				min_enactment_period: 10 * MINUTES,
				min_approval: APP_BIG_TIPPER,
				min_support: SUP_BIG_TIPPER,
			},
		),
		(
			32,
			pallet_referenda::TrackInfo {
				name: "small_spender",
				max_deciding: 50,
				decision_deposit: 100 * DOLLARS,
				prepare_period: 4 * HOURS,
				decision_period: 28 * DAYS,
				confirm_period: 12 * HOURS,
				min_enactment_period: 24 * HOURS,
				min_approval: APP_SMALL_SPENDER,
				min_support: SUP_SMALL_SPENDER,
			},
		),
		(
			33,
			pallet_referenda::TrackInfo {
				name: "medium_spender",
				max_deciding: 50,
				decision_deposit: 200 * DOLLARS,
				prepare_period: 4 * HOURS,
				decision_period: 28 * DAYS,
				confirm_period: 24 * HOURS,
				min_enactment_period: 24 * HOURS,
				min_approval: APP_MEDIUM_SPENDER,
				min_support: SUP_MEDIUM_SPENDER,
			},
		),
		(
			34,
			pallet_referenda::TrackInfo {
				name: "big_spender",
				max_deciding: 50,
				decision_deposit: 400 * DOLLARS,
				prepare_period: 4 * HOURS,
				decision_period: 28 * DAYS,
				confirm_period: 48 * HOURS,
				min_enactment_period: 24 * HOURS,
				min_approval: APP_BIG_SPENDER,
				min_support: SUP_BIG_SPENDER,
			},
		),
		(
			100,
			pallet_referenda::TrackInfo {
				name: "cluster_activator",
				max_deciding: 50,
				decision_deposit: 0 * DOLLARS,
				prepare_period: 0 * HOURS,
				decision_period: 1 * MINUTES,
				confirm_period: MINUTES / 2,
				min_enactment_period: 0 * HOURS,
				min_approval: APP_CLUSTER_ACTIVATOR,
				min_support: SUP_CLUSTER_ACTIVATOR,
			},
		),
		(
			101,
			pallet_referenda::TrackInfo {
				name: "cluster_economics_updater",
				max_deciding: 50,
				decision_deposit: 0 * DOLLARS,
				prepare_period: 0 * HOURS,
				decision_period: 1 * MINUTES,
				confirm_period: MINUTES / 2,
				min_enactment_period: 0 * HOURS,
				min_approval: APP_CLUSTER_ECONOMICS_UPDATER,
				min_support: SUP_CLUSTER_ECONOMICS_UPDATER,
			},
		),
	];
}
