//! DdcStaking pallet benchmarking.

use crate::{cluster::{ClusterParams, ClusterGovParams}, Pallet as DdcClusters, *};
use ddc_primitives::{ClusterId, ClusterPricingParams, NodePubKey, NodeType};

use sp_std::prelude::*;
use sp_runtime::{Perbill, AccountId32};
use pallet_contracts::chain_extension::UncheckedFrom;
use pallet_ddc_nodes::{Node, NodeParams, CDNNodeParams};

pub use frame_benchmarking::{
	account, benchmarks, BenchmarkError, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;

pub fn config_cluster_and_node<T: Config> (
  user: T::AccountId,
  node_pub_key: NodePubKey,
  cluster_id: ClusterId,
) -> Result<(), BenchmarkError> where T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]> {
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

  T::StakingVisitor::bond_stake_and_serve(user.clone(), user.clone(), node_pub_key.clone(), 10_000u32.into(), cluster_id.clone()).unwrap();
  
  let mut auth_contract = NodeProviderAuthContract::<T>::new(
    user.clone(),
    user.clone(),
  );
  auth_contract = auth_contract.deploy_contract(user.clone())?;
  auth_contract.authorize_node(node_pub_key.clone())?;

  let updated_cluster_params = ClusterParams { node_provider_auth_contract: auth_contract.contract_id.clone() };

  // Register auth contract
  DdcClusters::<T>::set_cluster_params(
    RawOrigin::Signed(user.clone()).into(), 
    cluster_id.clone().clone(), 
    updated_cluster_params, 
  );

  Ok(())
}

impl From<NodeProviderAuthContractError> for BenchmarkError {
  fn from(error: NodeProviderAuthContractError) -> Self {
    match error {
      NodeProviderAuthContractError::ContractCallFailed =>
       BenchmarkError::Stop("NodeAuthContractCallFailed"),
      NodeProviderAuthContractError::ContractDeployFailed =>
       BenchmarkError::Stop("NodeAuthContractDeployFailed"),
      NodeProviderAuthContractError::NodeAuthorizationFailed =>
       BenchmarkError::Stop("NodeAuthNodeAuthorizationFailed"),
    }
  }
}