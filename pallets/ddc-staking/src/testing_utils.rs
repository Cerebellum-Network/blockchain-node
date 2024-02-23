//! Testing utils for ddc-staking.

use ddc_primitives::{
	ClusterGovParams, ClusterId, ClusterParams, NodeParams, StorageNodeMode, StorageNodeParams,
	StorageNodePubKey,
};
use frame_benchmarking::account;
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_runtime::{traits::StaticLookup, Perquintill};
use sp_std::prelude::*;

use crate::{Pallet as DdcStaking, *};

const SEED: u32 = 0;

/// This function removes all storage and Storages nodes from storage.
pub fn clear_activated_nodes<T: Config>() {
	#[allow(unused_must_use)]
	{
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
	balance_factor: u128,
) -> T::AccountId {
	let user = account(string, n, SEED);
	let balance = T::Currency::minimum_balance() * balance_factor.saturated_into::<BalanceOf<T>>();
	let _ = T::Currency::make_free_balance_be(&user, balance);
	user
}

/// Create a stash and controller pair.
pub fn create_stash_controller_node<T: Config>(
	n: u32,
	balance_factor: u32,
) -> Result<(T::AccountId, T::AccountId, NodePubKey), &'static str> {
	let stash = create_funded_user::<T>("stash", n, balance_factor);
	let controller = create_funded_user::<T>("controller", n, balance_factor);
	let controller_lookup: <T::Lookup as StaticLookup>::Source =
		T::Lookup::unlookup(controller.clone());
	let node = NodePubKey::StoragePubKey(StorageNodePubKey::new([0; 32]));

	T::NodeCreator::create_node(
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
		}),
	)?;
	let amount = T::Currency::minimum_balance() * (balance_factor / 10).max(1).into();
	DdcStaking::<T>::bond(
		RawOrigin::Signed(stash.clone()).into(),
		controller_lookup,
		node.clone(),
		amount,
	)?;
	Ok((stash, controller, node))
}

/// Create a stash and controller pair with fixed balance.
pub fn create_stash_controller_node_with_balance<T: Config>(
	n: u32,
	balance_factor: u128,
	node_pub_key: NodePubKey,
) -> Result<(T::AccountId, T::AccountId, NodePubKey), &'static str> {
	let stash = create_funded_user_with_balance::<T>("stash", n, balance_factor);
	let controller = create_funded_user_with_balance::<T>("controller", n, balance_factor);
	let controller_lookup: <T::Lookup as StaticLookup>::Source =
		T::Lookup::unlookup(controller.clone());

	let node_pub = node_pub_key.clone();
	match node_pub_key {
		NodePubKey::StoragePubKey(node_pub_key) => {
			T::NodeCreator::create_node(
				ddc_primitives::NodePubKey::StoragePubKey(node_pub_key),
				stash.clone(),
				NodeParams::StorageParams(StorageNodeParams {
					mode: StorageNodeMode::Storage,
					host: vec![1u8; 255],
					domain: vec![2u8; 256],
					ssl: true,
					http_port: 35000u16,
					grpc_port: 25000u16,
					p2p_port: 15000u16,
				}),
			)?;
		},
	}

	let cluster_id = ClusterId::from([1; 20]);
	let cluster_params = ClusterParams { node_provider_auth_contract: Some(stash.clone()) };
	let cluster_gov_params: ClusterGovParams<BalanceOf<T>, BlockNumberFor<T>> = ClusterGovParams {
		treasury_share: Perquintill::default(),
		validators_share: Perquintill::default(),
		cluster_reserve_share: Perquintill::default(),
		storage_bond_size: 10u32.into(),
		storage_chill_delay: 50u32.into(),
		storage_unbonding_delay: 50u32.into(),
		unit_per_mb_stored: 10,
		unit_per_mb_streamed: 10,
		unit_per_put_request: 10,
		unit_per_get_request: 10,
	};
	T::ClusterCreator::create_new_cluster(
		cluster_id,
		stash.clone(),
		stash.clone(),
		cluster_params,
		cluster_gov_params,
	)?;

	DdcStaking::<T>::bond(
		RawOrigin::Signed(stash.clone()).into(),
		controller_lookup,
		node_pub.clone(),
		T::Currency::minimum_balance() * balance_factor.saturated_into::<BalanceOf<T>>(),
	)?;

	Ok((stash, controller, node_pub))
}
