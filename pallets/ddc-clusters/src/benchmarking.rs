//! DdcStaking pallet benchmarking.

use super::*;
use crate::{cluster::{ClusterParams, ClusterGovParams}, Pallet as DdcClusters};
use ddc_primitives::{ClusterId, ClusterPricingParams, NodePubKey, NodeType};

use sp_std::prelude::*;
use sp_runtime::{Perbill, AccountId32};
use pallet_contracts::chain_extension::UncheckedFrom;
use pallet_ddc_nodes::{Node, NodeParams, CDNNodeParams};

pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;

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
    let cluster_params = ClusterParams { node_provider_auth_contract: user.clone() };
		let cdn_node_params = CDNNodeParams {
			host: vec![1u8, 255],
			http_port: 35000u16,
			grpc_port: 25000u16,
			p2p_port: 15000u16,
		};

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

    DdcClusters::<T>::create_cluster(
      RawOrigin::Root.into(), 
      cluster_id.clone(), 
      user.clone(), 
      user.clone(), 
      cluster_params, 
      cluster_gov_params
    );

    if let Ok(new_node) = Node::<T>::new(node_pub_key.clone(), user.clone(), pallet_ddc_nodes::NodeParams::CDNParams(cdn_node_params)) {
      T::NodeRepository::create(new_node);
    } 
    
    let auth_contract = NodeProviderAuthContract::<T>::new(
      user.clone(),
      user.clone(),
    );
    auth_contract.deploy_contract(user.clone());
    auth_contract.authorize_node(node_pub_key.clone());


  }: _(RawOrigin::Signed(user.clone()), cluster_id.clone(), node_pub_key) 
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
