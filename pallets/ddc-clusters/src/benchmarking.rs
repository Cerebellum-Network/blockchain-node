//! DdcStaking pallet benchmarking.

use super::*;
use crate::{cluster::{ClusterParams, ClusterGovParams}, Pallet as DdcClusters};
use ddc_primitives::{ClusterId, ClusterPricingParams, NodePubKey, NodeType};

use sp_std::prelude::*;
use sp_runtime::{Perbill, AccountId32};
use pallet_contracts::chain_extension::UncheckedFrom;
use pallet_ddc_nodes::{Node, NodeParams, CDNNodeParams};

pub use frame_benchmarking::{
	account, benchmarks, BenchmarkError, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;
use testing_utils::config_cluster_and_node;

const USER_SEED: u32 = 999666;

benchmarks! {
  where_clause { where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]> }
  
	create_cluster {
    let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
    let cluster_params = ClusterParams { node_provider_auth_contract: user.clone() };

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

	  whitelist_account!(user);
	}: _(RawOrigin::Root, cluster_id.clone(), user.clone(), user, cluster_params, cluster_gov_params)
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
    let _ = config_cluster_and_node::<T>(user.clone(), node_pub_key.clone(), cluster_id.clone());
  }: _(RawOrigin::Signed(user.clone()), cluster_id, node_pub_key) 
  verify {
    assert!(Clusters::<T>::contains_key(cluster_id));
  }

  remove_node {
    let bytes = [0u8; 32];
    let node_pub_key = NodePubKey::CDNPubKey(AccountId32::from(bytes));
    let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
    let balance = <T as pallet::Config>::Currency::minimum_balance() * 1_000_000u32.into();
    let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
    let _ = config_cluster_and_node::<T>(user.clone(), node_pub_key.clone(), cluster_id.clone());
    let _ = DdcClusters::<T>::add_node(
      RawOrigin::Signed(user.clone()).into(), 
      cluster_id.clone(),
      node_pub_key.clone()
    );
  }: _(RawOrigin::Signed(user.clone()), cluster_id, node_pub_key) 
  verify {
    assert!(Clusters::<T>::contains_key(cluster_id));
  }

	// delete_node {
	// 	let (user, node, cdn_node_params, _) = create_user_and_config::<T>("user", USER_SEED);

	//   DdcNodes::<T>::create_node(RawOrigin::Signed(user.clone()).into(),node.clone(), cdn_node_params)?;

	// 	whitelist_account!(user);
	// }: _(RawOrigin::Signed(user.clone()), node)
	// verify {
	//   assert!(!CDNNodes::<T>::contains_key(CDNNodePubKey::new([0; 32])));
	// }

	// set_node_params {
	//   let (user, node, cdn_node_params, cdn_node_params_new) = create_user_and_config::<T>("user", USER_SEED);

	// DdcNodes::<T>::create_node(RawOrigin::Signed(user.clone()).into(),node.clone(), cdn_node_params)?;

	// whitelist_account!(user);
	// }: _(RawOrigin::Signed(user.clone()), node, cdn_node_params_new)
	// verify {
	//   assert_eq!(CDNNodes::<T>::try_get(
	//   CDNNodePubKey::new([0; 32])).unwrap().props,
	// 	CDNNodeProps {
	// 		host: vec![2u8, 255].try_into().unwrap(),
	// 		http_port: 45000u16,
	// 		grpc_port: 55000u16,
	// 		p2p_port: 65000u16,
	// 	});
	// }
}
