//! DdcStaking pallet benchmarking.

use ddc_primitives::{NodeParams, NodeType, StorageNodeMode, StorageNodeParams, StorageNodePubKey};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_runtime::traits::StaticLookup;
use sp_std::prelude::*;
use testing_utils::*;

use super::*;
use crate::Pallet as DdcStaking;

const USER_SEED: u32 = 999666;

benchmarks! {
	bond {
		let stash = create_funded_user::<T>("stash", USER_SEED, 100);
		let controller = create_funded_user::<T>("controller", USER_SEED, 100);
		let controller_lookup: <T::Lookup as StaticLookup>::Source
			= T::Lookup::unlookup(controller.clone());
		let node = NodePubKey::StoragePubKey(StorageNodePubKey::new([0; 32]));
		let _ = T::NodeCreator::create_node(
			node.clone(),
			stash.clone(),
			NodeParams::StorageParams(StorageNodeParams {
				mode: StorageNodeMode::Storage,
				host: vec![1u8; 255],
				domain: vec![2u8; 256],
				ssl: true,
				http_port: 35000u16,
				grpc_port: 25000u16,
				p2p_port: 15000u16,
			})
		)?;
		let amount = T::Currency::minimum_balance() * 10u32.into();
		whitelist_account!(stash);
	}: _(RawOrigin::Signed(stash.clone()), controller_lookup, node.clone(), amount)
	verify {
		assert!(Bonded::<T>::contains_key(stash));
		assert!(Ledger::<T>::contains_key(controller));
		assert!(Nodes::<T>::contains_key(node));
	}

	unbond {
		// clean up any existing state.
		clear_activated_nodes::<T>();

		let (stash, controller, _) = create_stash_controller_node::<T>(0, 100)?;
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
		let (stash, controller, _) = create_stash_controller_node::<T>(0, 100)?;
		let amount = T::Currency::minimum_balance() * 5u32.into(); // Half of total
		DdcStaking::<T>::unbond(RawOrigin::Signed(controller.clone()).into(), amount)?;
		frame_system::Pallet::<T>::set_block_number(BlockNumberFor::<T>::from(1000u32));
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
		let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([0; 32]));
		let (stash, controller, _) = create_stash_controller_node_with_balance::<T>(0, T::ClusterVisitor::get_bond_size(&ClusterId::from([1; 20]), NodeType::Storage).unwrap_or(100u128), node_pub_key)?;

		whitelist_account!(controller);
	}: _(RawOrigin::Signed(controller), ClusterId::from([1; 20]))
	verify {
		assert!(Storages::<T>::contains_key(&stash));
	}


	chill {
		// clean up any existing state.
		clear_activated_nodes::<T>();

		let node_pub_key = NodePubKey::StoragePubKey(StorageNodePubKey::new([0; 32]));
		let (storage_stash, storage_controller, _) = create_stash_controller_node_with_balance::<T>(0, T::ClusterVisitor::get_bond_size(&ClusterId::from([1; 20]), NodeType::Storage).unwrap_or(10u128), node_pub_key)?;
		DdcStaking::<T>::store(RawOrigin::Signed(storage_controller.clone()).into(), ClusterId::from([1; 20]))?;
		assert!(Storages::<T>::contains_key(&storage_stash));
		frame_system::Pallet::<T>::set_block_number(BlockNumberFor::<T>::from(1u32));
		DdcStaking::<T>::chill(RawOrigin::Signed(storage_controller.clone()).into())?;
		frame_system::Pallet::<T>::set_block_number(BlockNumberFor::<T>::from(1u32) + T::ClusterVisitor::get_chill_delay(&ClusterId::from([1; 20]), NodeType::Storage).unwrap_or_else(|_| BlockNumberFor::<T>::from(10u32)));

		whitelist_account!(storage_controller);
	}: _(RawOrigin::Signed(storage_controller))
	verify {
		assert!(!Storages::<T>::contains_key(&storage_stash));
	}

	set_controller {
		let (stash, _, _) = create_stash_controller_node::<T>(USER_SEED, 100)?;
		let new_controller = create_funded_user::<T>("new_controller", USER_SEED, 100);
		let new_controller_lookup = T::Lookup::unlookup(new_controller.clone());
		whitelist_account!(stash);
	}: _(RawOrigin::Signed(stash), new_controller_lookup)
	verify {
		assert!(Ledger::<T>::contains_key(&new_controller));
	}

	set_node {
		let (stash, _, _) = create_stash_controller_node::<T>(USER_SEED, 100)?;
		let new_node = NodePubKey::StoragePubKey(StorageNodePubKey::new([1; 32]));
		whitelist_account!(stash);
	}: _(RawOrigin::Signed(stash), new_node.clone())
	verify {
		assert!(Nodes::<T>::contains_key(&new_node));
	}

	impl_benchmark_test_suite!(
		DdcStaking,
		crate::mock::ExtBuilder::default().build(),
		crate::mock::Test,
	);
}
