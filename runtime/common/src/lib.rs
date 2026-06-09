#![cfg_attr(not(feature = "std"), no_std)]

pub mod constants;
pub mod migrations;

use ddc_primitives::Balance;

/// The type used for currency conversion.
///
/// This must only be used as long as the balance type is `u128`.
pub type CurrencyToVote = polkadot_sdk::sp_staking::currency_to_vote::U128CurrencyToVote;

/// Convert a balance to an unsigned 256-bit number, use in nomination pools.
pub struct BalanceToU256;
impl polkadot_sdk::sp_runtime::traits::Convert<Balance, polkadot_sdk::sp_core::U256> for BalanceToU256 {
	fn convert(n: Balance) -> polkadot_sdk::sp_core::U256 {
		n.into()
	}
}

/// Convert an unsigned 256-bit number to balance, use in nomination pools.
pub struct U256ToBalance;
impl polkadot_sdk::sp_runtime::traits::Convert<polkadot_sdk::sp_core::U256, Balance> for U256ToBalance {
	fn convert(n: polkadot_sdk::sp_core::U256) -> Balance {
		use polkadot_sdk::frame_support::traits::Defensive;
		n.try_into().defensive_unwrap_or(Balance::MAX)
	}
}
