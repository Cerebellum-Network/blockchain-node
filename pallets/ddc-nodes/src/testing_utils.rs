//! Testing utils for ddc-staking.

use ddc_primitives::{NodeParams, StorageNodeParams, StorageNodePubKey};
use frame_benchmarking::account;
use sp_std::vec;

use crate::{Config, NodePubKey};

const SEED: u32 = 0;

/// Grab a funded user.
pub fn create_user_and_config<T: Config>(
	string: &'static str,
	n: u32,
) -> (T::AccountId, NodePubKey, NodeParams, NodeParams) {
	let user = account(string, n, SEED);
	let node = NodePubKey::StoragePubKey(StorageNodePubKey::new([0; 32]));
	let storage_node_params = NodeParams::StorageParams(StorageNodeParams {
		host: vec![1u8, 255],
		http_port: 35000u16,
		grpc_port: 25000u16,
		p2p_port: 15000u16,
	});

	let new_storage_node_params = NodeParams::StorageParams(StorageNodeParams {
		host: vec![2u8, 255],
		http_port: 45000u16,
		grpc_port: 55000u16,
		p2p_port: 65000u16,
	});
	(user, node, storage_node_params, new_storage_node_params)
}
