//! DdcStaking pallet benchmarking.

use ddc_primitives::{ClusterGovParams, ClusterId, ClusterParams, NodePubKey};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
	BenchmarkError,
};
use frame_system::RawOrigin;
use pallet_contracts::chain_extension::UncheckedFrom;
use sp_runtime::{AccountId32, Perbill};
use sp_std::prelude::*;
use testing_utils::*;

use super::*;
use crate::{cluster::ClusterProps, Pallet as DdcClusters};

const USER_SEED: u32 = 999666;
const USER_SEED_2: u32 = 999555;

benchmarks! {
  where_clause { where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]> }

	create_cluster {
	  let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
	  let cluster_params = ClusterParams { node_provider_auth_contract: Some(user.clone()) };
	let cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber> = ClusterGovParams {
	  treasury_share: Perbill::default(),
	  validators_share: Perbill::default(),
	  cluster_reserve_share: Perbill::default(),
	  cdn_bond_size: 100u32.into(),
	  cdn_chill_delay: 50u32.into(),
	  cdn_unbonding_delay: 50u32.into(),
	  storage_bond_size: 100u32.into(),
	  storage_chill_delay: 50u32.into(),
	  storage_unbonding_delay: 50u32.into(),
	  unit_per_mb_stored: 10,
	  unit_per_mb_streamed: 10,
	  unit_per_put_request: 10,
	  unit_per_get_request: 10,
	};
	}: _(RawOrigin::Root, cluster_id, user.clone(), user, cluster_params, cluster_gov_params)
	verify {
		assert!(Clusters::<T>::contains_key(cluster_id));
	}

  add_node {
	let bytes = [0u8; 32];
	let node_pub_key = NodePubKey::CDNPubKey(AccountId32::from(bytes));
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
	let node_pub_key = NodePubKey::CDNPubKey(AccountId32::from(bytes));
	let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
	let balance = <T as pallet::Config>::Currency::minimum_balance() * 1_000_000u32.into();
	let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
	let _ = config_cluster_and_node::<T>(user.clone(), node_pub_key.clone(), cluster_id);
	let _ = DdcClusters::<T>::add_node(
	  RawOrigin::Signed(user.clone()).into(),
	  cluster_id,
	  node_pub_key.clone()
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
	  let new_cluster_params = ClusterParams { node_provider_auth_contract: Some(user_2.clone()) };
	}: _(RawOrigin::Signed(user.clone()), cluster_id, new_cluster_params)
	verify {
	  assert_eq!(Clusters::<T>::try_get(cluster_id).unwrap().props, ClusterProps { node_provider_auth_contract: Some(user_2) });
	}

	set_cluster_gov_params {
	  let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
	  let _ = config_cluster::<T>(user, cluster_id);
	let new_cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber> = ClusterGovParams {
	  treasury_share: Perbill::default(),
	  validators_share: Perbill::default(),
	  cluster_reserve_share: Perbill::default(),
	  cdn_bond_size: 10u32.into(),
	  cdn_chill_delay: 5u32.into(),
	  cdn_unbonding_delay: 5u32.into(),
	  storage_bond_size: 10u32.into(),
	  storage_chill_delay: 5u32.into(),
	  storage_unbonding_delay: 5u32.into(),
	  unit_per_mb_stored: 1,
	  unit_per_mb_streamed: 1,
	  unit_per_put_request: 1,
	  unit_per_get_request: 1,
	};
	}: _(RawOrigin::Root, cluster_id, new_cluster_gov_params.clone())
	verify {
	  assert_eq!(ClustersGovParams::<T>::try_get(cluster_id).unwrap(), new_cluster_gov_params);
	}

	impl_benchmark_test_suite!(
		DdcClusters,
		crate::mock::ExtBuilder.build(),
		crate::mock::Test,
	);
}
