//! DdcStaking pallet benchmarking.

use super::*;
use crate::{cdn_node::CDNNodeParams, node::NodeParams, Pallet as DdcNodes};
use ddc_primitives::CDNNodePubKey;

use sp_std::prelude::*;

pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;

const USER_SEED: u32 = 999666;

benchmarks! {
	create_node {
	let user: T::AccountId = account("user", USER_SEED, 0u32);
		let node = NodePubKey::CDNPubKey(CDNNodePubKey::new([0; 32]));
	let cdn_node_params = NodeParams::CDNParams(CDNNodeParams {
	  host: vec![1u8, 255],
	  http_port: 35000u16,
	  grpc_port: 25000u16,
	  p2p_port: 15000u16,
	});

	whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), node, cdn_node_params)
	verify {
		assert!(CDNNodes::<T>::contains_key(CDNNodePubKey::new([0; 32])));
	}

	delete_node {
	let user: T::AccountId = account("user", USER_SEED, 0u32);
		let node = NodePubKey::CDNPubKey(CDNNodePubKey::new([0; 32]));

	let cdn_node_params =  NodeParams::CDNParams(CDNNodeParams {
	  host: vec![1u8, 255],
	  http_port: 35000u16,
	  grpc_port: 25000u16,
	  p2p_port: 15000u16,
	});

	DdcNodes::<T>::create_node(RawOrigin::Signed(user.clone()).into(),node.clone(), cdn_node_params)?;

		whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), node)
	verify {
	assert!(!CDNNodes::<T>::contains_key(CDNNodePubKey::new([0; 32])));
	}

	set_node_params {
	let user: T::AccountId = account("user", USER_SEED, 0u32);
		let node = NodePubKey::CDNPubKey(CDNNodePubKey::new([0; 32]));
	let cdn_node_params =  NodeParams::CDNParams(CDNNodeParams {
	  host: vec![1u8, 255],
	  http_port: 35000u16,
	  grpc_port: 25000u16,
	  p2p_port: 15000u16,
	});

	let cdn_node_params_new =  NodeParams::CDNParams(CDNNodeParams {
	  host: vec![2u8, 255],
	  http_port: 45000u16,
	  grpc_port: 55000u16,
	  p2p_port: 65000u16,
	});
  DdcNodes::<T>::create_node(RawOrigin::Signed(user.clone()).into(),node.clone(), cdn_node_params)?;

  whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), node, cdn_node_params_new)
	verify {
	}
}
