//! DdcStaking pallet benchmarking.

use ddc_primitives::{
	ClusterId, ClusterNodeKind, ClusterParams, ClusterProtocolParams, NodePubKey,
};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
	BenchmarkError,
};
use frame_system::RawOrigin;
use sp_core::crypto::UncheckedFrom;
use sp_runtime::{AccountId32, Perquintill};
use sp_std::prelude::*;
use testing_utils::*;

use super::*;
use crate::{cluster::ClusterProps, Pallet as DdcClusters};

const USER_SEED: u32 = 999666;
const USER_SEED_2: u32 = 999555;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

benchmarks! {
  where_clause { where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]> }

	create_cluster {
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let cluster_params = ClusterParams {
								node_provider_auth_contract: Some(user.clone()),
								erasure_coding_required: 4,
								erasure_coding_total: 6,
								replication_total: 3
							};
		let cluster_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>> = ClusterProtocolParams {
			treasury_share: Perquintill::default(),
			validators_share: Perquintill::default(),
			cluster_reserve_share: Perquintill::default(),
			storage_bond_size: 100u32.into(),
			storage_chill_delay: 50u32.into(),
			storage_unbonding_delay: 50u32.into(),
			unit_per_mb_stored: 10,
			unit_per_mb_streamed: 10,
			unit_per_put_request: 10,
			unit_per_get_request: 10,
		};
	}: _(RawOrigin::Signed(user.clone()), cluster_id, user.clone(), cluster_params, cluster_protocol_params)
	verify {
		assert!(Clusters::<T>::contains_key(cluster_id));
	}

	add_node {
		let bytes = [0u8; 32];
		let node_pub_key = NodePubKey::StoragePubKey(AccountId32::from(bytes));
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 1_000_000u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let _ = config_cluster_and_node::<T>(user.clone(), node_pub_key.clone(), cluster_id);
	}: _(RawOrigin::Signed(user.clone()), cluster_id, node_pub_key.clone(), ClusterNodeKind::Genesis)
	verify {
		assert!(ClustersNodes::<T>::contains_key(cluster_id, node_pub_key));
	}

	join_cluster {
		let bytes = [0u8; 32];
		let node_pub_key = NodePubKey::StoragePubKey(AccountId32::from(bytes));
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 1_000_000u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let _ = config_cluster_and_node::<T>(user.clone(), node_pub_key.clone(), cluster_id);
	}: _(RawOrigin::Signed(user.clone()), cluster_id, node_pub_key.clone())
	verify {
		assert!(ClustersNodes::<T>::contains_key(cluster_id, node_pub_key));
	}

	remove_node {
		let bytes = [0u8; 32];
		let node_pub_key = NodePubKey::StoragePubKey(AccountId32::from(bytes));
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 1_000_000u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let _ = config_cluster_and_node::<T>(user.clone(), node_pub_key.clone(), cluster_id);
		let _ = DdcClusters::<T>::add_node(
			RawOrigin::Signed(user.clone()).into(),
			cluster_id,
			node_pub_key.clone(),
			ClusterNodeKind::Genesis
		);
	}: _(RawOrigin::Signed(user.clone()), cluster_id, node_pub_key.clone())
	verify {
		assert!(!ClustersNodes::<T>::contains_key(cluster_id, node_pub_key));
	}

	set_cluster_params {
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let user_2 = account::<T::AccountId>("user", USER_SEED_2, 0u32);
		let _ = config_cluster::<T>(user.clone(), cluster_id);
		let new_cluster_params = ClusterParams {
									node_provider_auth_contract: Some(user_2.clone()),
									erasure_coding_required: 4,
									erasure_coding_total: 6,
									replication_total: 3
								};
	}: _(RawOrigin::Signed(user.clone()), cluster_id, new_cluster_params)
	verify {
		assert_eq!(
			Clusters::<T>::try_get(cluster_id).unwrap().props,
			ClusterProps {
				node_provider_auth_contract: Some(user_2),
				erasure_coding_required: 4,
				erasure_coding_total: 6,
				replication_total: 3,
			}
		);
	}

	validate_node {
		let bytes = [0u8; 32];
		let node_pub_key = NodePubKey::StoragePubKey(AccountId32::from(bytes));
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 1_000_000u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let _ = config_cluster_and_node::<T>(user.clone(), node_pub_key.clone(), cluster_id);
		DdcClusters::<T>::add_node(RawOrigin::Signed(user.clone()).into(), cluster_id, node_pub_key.clone(), ClusterNodeKind::Genesis)?;

	}: _(RawOrigin::Signed(user.clone()), cluster_id, node_pub_key.clone(), true)
	verify {
		assert_last_event::<T>(Event::ClusterNodeValidated { cluster_id, node_pub_key, succeeded: true}.into());
	}

	impl_benchmark_test_suite!(
		DdcClusters,
		crate::mock::ExtBuilder.build(),
		crate::mock::Test,
	);
}
