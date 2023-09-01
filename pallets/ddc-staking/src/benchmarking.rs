//! DdcStaking pallet benchmarking.

use super::*;
use crate::Pallet as DdcStaking;
use testing_utils::*;

use frame_support::traits::{Currency, Get};
use sp_runtime::traits::StaticLookup;
use sp_std::prelude::*;

pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;

const USER_SEED: u32 = 999666;

benchmarks! {
	bond {
		let stash = create_funded_user::<T>("stash", USER_SEED, 100);
		let controller = create_funded_user::<T>("controller", USER_SEED, 100);
		let controller_lookup: <T::Lookup as StaticLookup>::Source
			= T::Lookup::unlookup(controller.clone());
		let amount = T::Currency::minimum_balance() * 10u32.into();
		whitelist_account!(stash);
	}: _(RawOrigin::Signed(stash.clone()), controller_lookup, amount)
	verify {
		assert!(Bonded::<T>::contains_key(stash));
		assert!(Ledger::<T>::contains_key(controller));
	}

	unbond {
		// clean up any existing state.
		clear_storages_and_edges::<T>();

		let (stash, controller) = create_stash_controller::<T>(0, 100)?;
		let ledger = Ledger::<T>::get(&controller).ok_or("ledger not created before")?;
		let original_bonded: BalanceOf<T> = ledger.active;
		let amount = T::Currency::minimum_balance() * 5u32.into(); // Half of total

		whitelist_account!(controller);
	}: _(RawOrigin::Signed(controller.clone()), amount)
	verify {
		let ledger = Ledger::<T>::get(&controller).ok_or("ledger not created after")?;
		let new_bonded: BalanceOf<T> = ledger.active;
		assert!(original_bonded > new_bonded);
	}

	withdraw_unbonded {
		let (stash, controller) = create_stash_controller::<T>(0, 100)?;
		let amount = T::Currency::minimum_balance() * 5u32.into(); // Half of total
		DdcStaking::<T>::unbond(RawOrigin::Signed(controller.clone()).into(), amount)?;
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
		let (stash, controller) = create_stash_controller_with_balance::<T>(0, T::DefaultStorageBondSize::get())?;

		whitelist_account!(controller);
	}: _(RawOrigin::Signed(controller), 1)
	verify {
		assert!(Storages::<T>::contains_key(&stash));
	}

	serve {
		let (stash, controller) = create_stash_controller_with_balance::<T>(0, T::DefaultEdgeBondSize::get())?;

		whitelist_account!(controller);
	}: _(RawOrigin::Signed(controller), 1)
	verify {
		assert!(Edges::<T>::contains_key(&stash));
	}

	chill {
		// clean up any existing state.
		clear_storages_and_edges::<T>();

		let (edge_stash, edge_controller) = create_stash_controller_with_balance::<T>(0, T::DefaultEdgeBondSize::get())?;
		DdcStaking::<T>::serve(RawOrigin::Signed(edge_controller.clone()).into(), 1)?;
		assert!(Edges::<T>::contains_key(&edge_stash));
		CurrentEra::<T>::put(1);
		DdcStaking::<T>::chill(RawOrigin::Signed(edge_controller.clone()).into())?;
		CurrentEra::<T>::put(1 + Settings::<T>::get(1).edge_chill_delay);

		whitelist_account!(edge_controller);
	}: _(RawOrigin::Signed(edge_controller))
	verify {
		assert!(!Edges::<T>::contains_key(&edge_stash));
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

	allow_cluster_manager {
		let new_cluster_manager = create_funded_user::<T>("cluster_manager", USER_SEED, 100);
		let new_cluster_manager_lookup = T::Lookup::unlookup(new_cluster_manager.clone());
	}: _(RawOrigin::Root, new_cluster_manager_lookup)
	verify {
		assert!(ClusterManagers::<T>::get().contains(&new_cluster_manager));
	}

	disallow_cluster_manager {
		let new_cluster_manager = create_funded_user::<T>("cluster_manager", USER_SEED, 100);
		let new_cluster_manager_lookup = T::Lookup::unlookup(new_cluster_manager.clone());
		DdcStaking::<T>::allow_cluster_manager(RawOrigin::Root.into(), new_cluster_manager_lookup.clone())?;
		assert!(ClusterManagers::<T>::get().contains(&new_cluster_manager));
	}: _(RawOrigin::Root, new_cluster_manager_lookup)
	verify {
		assert!(!ClusterManagers::<T>::get().contains(&new_cluster_manager));
	}
}
