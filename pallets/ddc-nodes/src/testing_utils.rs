//! Testing utils for ddc-staking.

use crate::{Config, NodePubKey};
use ddc_primitives::{CDNNodeParams, CDNNodePubKey, NodeParams};
use frame_benchmarking::account;
use sp_std::vec;

const SEED: u32 = 0;

/// Grab a funded user.
pub fn create_user_and_config<T: Config>(
	string: &'static str,
	n: u32,
) -> (T::AccountId, NodePubKey, NodeParams, NodeParams) {
	let user = account(string, n, SEED);
	let node = NodePubKey::CDNPubKey(CDNNodePubKey::new([0; 32]));
	let cdn_node_params = NodeParams::CDNParams(CDNNodeParams {
		host: vec![1u8, 255],
		http_port: 35000u16,
		grpc_port: 25000u16,
		p2p_port: 15000u16,
	});

	let cdn_node_params_new = NodeParams::CDNParams(CDNNodeParams {
		host: vec![2u8, 255],
		http_port: 45000u16,
		grpc_port: 55000u16,
		p2p_port: 65000u16,
	});
	(user, node, cdn_node_params, cdn_node_params_new)
}
