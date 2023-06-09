//! Testing utils for ddc-staking.

use crate::{Pallet as DdcStaking, *};
use frame_benchmarking::account;
use frame_system::RawOrigin;

use frame_support::traits::Currency;
use sp_runtime::traits::StaticLookup;
use sp_std::prelude::*;

const SEED: u32 = 0;

/// This function removes all storage and edge nodes from storage.
pub fn clear_storages_and_edges<T: Config>() {
	#[allow(unused_must_use)]
	{
		Edges::<T>::clear(u32::MAX, None);
		Storages::<T>::clear(u32::MAX, None);
	}
}

/// Grab a funded user.
pub fn create_funded_user<T: Config>(
	string: &'static str,
	n: u32,
	balance_factor: u32,
) -> T::AccountId {
	let user = account(string, n, SEED);
	let balance = T::Currency::minimum_balance() * balance_factor.into();
	let _ = T::Currency::make_free_balance_be(&user, balance);
	user
}

/// Grab a funded user with max Balance.
pub fn create_funded_user_with_balance<T: Config>(
	string: &'static str,
	n: u32,
	balance: BalanceOf<T>,
) -> T::AccountId {
	let user = account(string, n, SEED);
	let _ = T::Currency::make_free_balance_be(&user, balance);
	user
}

/// Create a stash and controller pair.
pub fn create_stash_controller<T: Config>(
	n: u32,
	balance_factor: u32,
) -> Result<(T::AccountId, T::AccountId), &'static str> {
	let stash = create_funded_user::<T>("stash", n, balance_factor);
	let controller = create_funded_user::<T>("controller", n, balance_factor);
	let controller_lookup: <T::Lookup as StaticLookup>::Source =
		T::Lookup::unlookup(controller.clone());
	let amount = T::Currency::minimum_balance() * (balance_factor / 10).max(1).into();
	DdcStaking::<T>::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, amount)?;
	return Ok((stash, controller))
}

/// Create a stash and controller pair with fixed balance.
pub fn create_stash_controller_with_balance<T: Config>(
	n: u32,
	balance: crate::BalanceOf<T>,
) -> Result<(T::AccountId, T::AccountId), &'static str> {
	let stash = create_funded_user_with_balance::<T>("stash", n, balance);
	let controller = create_funded_user_with_balance::<T>("controller", n, balance);
	let controller_lookup: <T::Lookup as StaticLookup>::Source =
		T::Lookup::unlookup(controller.clone());

	DdcStaking::<T>::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, balance)?;
	Ok((stash, controller))
}
