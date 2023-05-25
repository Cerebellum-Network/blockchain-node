//! DdcStaking pallet benchmarking.

use super::*;
use crate::Pallet as DddcStaking;
use testing_utils::*;

use frame_support::{ensure, traits::Currency};
use sp_runtime::traits::StaticLookup;
use sp_std::prelude::*;

pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;

const SEED: u32 = 0;

pub fn clear_storages_with_edges<T: Config>(
	n_storages: u32,
	n_edges: u32,
) -> Result<
	(Vec<<T::Lookup as StaticLookup>::Source>, Vec<<T::Lookup as StaticLookup>::Source>),
	&'static str,
> {
	// Clean up any existing state.
	clear_storages_and_edges::<T>();

	// Create new storages
	let mut storages: Vec<<T::Lookup as StaticLookup>::Source> =
		Vec::with_capacity(n_storages as usize);
	for i in 0..n_storages {
		let (stash, controller) = create_stash_controller::<T>(i + SEED, 100)?;
		let storage_prefs = StoragePrefs { foo: true };
		DddcStaking::<T>::store(RawOrigin::Signed(controller).into(), storage_prefs)?;
		let stash_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(stash);
		storages.push(stash_lookup);
	}

	// Create new edges
	let mut edges: Vec<<T::Lookup as StaticLookup>::Source> = Vec::with_capacity(n_edges as usize);
	for i in 0..n_edges {
		let (stash, controller) = create_stash_controller::<T>(i + SEED, 100)?;
		let edge_prefs = EdgePrefs { foo: true };
		DddcStaking::<T>::serve(RawOrigin::Signed(controller).into(), edge_prefs)?;
		let stash_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(stash);
		edges.push(stash_lookup);
	}

	Ok((storages, edges))
}

struct AccountsScenario<T: Config> {
	origin_stash1: T::AccountId,
	origin_controller1: T::AccountId,
}

impl<T: Config> AccountsScenario<T> {
	fn new(origin_balance: BalanceOf<T>) -> Result<Self, &'static str> {
		ensure!(!origin_balance.is_zero(), "origin weight must be greater than 0");

		// burn the entire issuance.
		let i = T::Currency::burn(T::Currency::total_issuance());
		sp_std::mem::forget(i);

		// create accounts with the origin balance
		let (origin_stash1, origin_controller1) =
			create_stash_controller_with_balance::<T>(USER_SEED + 2, origin_balance)?;

		Ok(AccountsScenario { origin_stash1, origin_controller1 })
	}
}

const USER_SEED: u32 = 999666;

benchmarks! {
	bond {
		let stash = create_funded_user::<T>("stash", USER_SEED, 100);
		let controller = create_funded_user::<T>("controller", USER_SEED, 100);
		let controller_lookup: <T::Lookup as StaticLookup>::Source
			= T::Lookup::unlookup(controller.clone());
		whitelist_account!(stash);
	}: _(RawOrigin::Signed(stash.clone()), controller_lookup)
	verify {
		assert!(Bonded::<T>::contains_key(stash));
		assert!(Ledger::<T>::contains_key(controller));
	}

	unbond {
		// clean up any existing state.
		clear_storages_and_edges::<T>();

		let total_issuance = T::Currency::total_issuance();
		
		// Constant taken from original benchmark staking code (/frame/staking/src/benchmarking.rs)
		let origin_balance = BalanceOf::<T>::try_from(952_994_955_240_703u128)
			.map_err(|_| "balance expected to be a u128")
			.unwrap();
		let scenario = AccountsScenario::<T>::new(origin_balance)?;

		let stash = scenario.origin_stash1.clone();
		let controller = scenario.origin_controller1.clone();
		// unbond half of initial balance
		let ledger = Ledger::<T>::get(&controller).ok_or("ledger not created before")?;
		let original_bonded: BalanceOf<T> = ledger.active;

		whitelist_account!(controller);
	}: _(RawOrigin::Signed(controller.clone()))
	verify {
		let ledger = Ledger::<T>::get(&controller).ok_or("ledger not created after")?;
		let new_bonded: BalanceOf<T> = ledger.active;
		assert!(original_bonded > new_bonded);
	}

	withdraw_unbonded {
		let (stash, controller) = create_stash_controller::<T>(0, 100)?;
		DddcStaking::<T>::unbond(RawOrigin::Signed(controller.clone()).into())?;
		CurrentEra::<T>::put(EraIndex::max_value()); 
		let ledger = Ledger::<T>::get(&controller).ok_or("ledger not created before")?;
		let original_total: BalanceOf<T> = ledger.total;
		whitelist_account!(controller);
	}: _(RawOrigin::Signed(controller.clone()))
	verify {
		let ledger = Ledger::<T>::get(&controller).ok_or("ledger not created after")?;
		let new_total: BalanceOf<T> = ledger.total;
		assert!(original_total > new_total);
	}

	store {
		let (stash, controller) = create_stash_controller::<T>(0, 100)?;

		let prefs = StoragePrefs::default();
		whitelist_account!(controller);
	}: _(RawOrigin::Signed(controller), prefs)
	verify {
		assert!(Storages::<T>::contains_key(&stash));
	}

  serve {
		let (stash, controller) = create_stash_controller::<T>(0, 100)?;

		let prefs = EdgePrefs::default();
		whitelist_account!(controller);
	}: _(RawOrigin::Signed(controller), prefs)
	verify {
		assert!(Edges::<T>::contains_key(&stash));
	}

	chill {
		// clean up any existing state.
		clear_storages_and_edges::<T>();

		let origin_balance = BondSize::<T>::get().max(T::Currency::minimum_balance());

		let scenario = AccountsScenario::<T>::new(origin_balance)?;
		let controller = scenario.origin_controller1.clone();
		let stash = scenario.origin_stash1.clone();

		whitelist_account!(controller);
	}: _(RawOrigin::Signed(controller))
	verify {
	}

	set_controller {
		let (stash, _) = create_stash_controller::<T>(USER_SEED, 100)?;
		let new_controller = create_funded_user::<T>("new_controller", USER_SEED, 100);
		let new_controller_lookup = T::Lookup::unlookup(new_controller.clone());
		whitelist_account!(stash);
	}: _(RawOrigin::Signed(stash), new_controller_lookup)
	verify {
		assert!(Ledger::<T>::contains_key(&new_controller));
	}
}
