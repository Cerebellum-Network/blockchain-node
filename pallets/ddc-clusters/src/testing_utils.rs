//! DdcStaking pallet benchmarking.

use ddc_primitives::{
	CDNNodeParams, ClusterGovParams, ClusterId, ClusterParams, NodeParams, NodePubKey,
};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
	BenchmarkError,
};
use frame_system::RawOrigin;
use pallet_contracts::chain_extension::UncheckedFrom;
use pallet_ddc_nodes::Node;
use sp_runtime::Perbill;
use sp_std::prelude::*;

use crate::{Pallet as DdcClusters, *};

pub fn config_cluster<T: Config>(user: T::AccountId, cluster_id: ClusterId)
where
	T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
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

	let _ = DdcClusters::<T>::create_cluster(
		RawOrigin::Root.into(),
		cluster_id,
		user.clone(),
		user,
		cluster_params,
		cluster_gov_params,
	);
}

pub fn config_cluster_and_node<T: Config>(
	user: T::AccountId,
	node_pub_key: NodePubKey,
	cluster_id: ClusterId,
) -> Result<(), Box<BenchmarkError>>
where
	T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
	let cluster_params = ClusterParams { node_provider_auth_contract: Some(user.clone()) };
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

	let _ = DdcClusters::<T>::create_cluster(
		RawOrigin::Root.into(),
		cluster_id,
		user.clone(),
		user.clone(),
		cluster_params,
		cluster_gov_params,
	);

	if let Ok(new_node) =
		Node::<T>::new(node_pub_key.clone(), user.clone(), NodeParams::CDNParams(cdn_node_params))
	{
		let _ = T::NodeRepository::create(new_node);
	}

	T::StakerCreator::bond_stake_and_participate(
		user.clone(),
		user.clone(),
		node_pub_key.clone(),
		10_000u32.into(),
		cluster_id,
	)
	.unwrap();

	let mut auth_contract = NodeProviderAuthContract::<T>::new(user.clone(), user.clone());
	auth_contract = auth_contract.deploy_contract(user.clone())?;
	auth_contract.authorize_node(node_pub_key)?;

	let updated_cluster_params =
		ClusterParams { node_provider_auth_contract: Some(auth_contract.contract_id) };

	// Register auth contract
	let _ = DdcClusters::<T>::set_cluster_params(
		RawOrigin::Signed(user).into(),
		cluster_id,
		updated_cluster_params,
	);

	Ok(())
}

impl From<NodeProviderAuthContractError> for Box<BenchmarkError> {
	fn from(error: NodeProviderAuthContractError) -> Self {
		match error {
			NodeProviderAuthContractError::ContractCallFailed =>
				Box::new(BenchmarkError::Stop("NodeAuthContractCallFailed")),
			NodeProviderAuthContractError::ContractDeployFailed =>
				Box::new(BenchmarkError::Stop("NodeAuthContractDeployFailed")),
			NodeProviderAuthContractError::NodeAuthorizationNotSuccessful =>
				Box::new(BenchmarkError::Stop("NodeAuthNodeAuthorizationNotSuccessful")),
		}
	}
}
