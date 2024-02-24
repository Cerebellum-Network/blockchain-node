#[cfg(feature = "runtime-benchmarks")]
use frame_support::dispatch::DispatchResult;
use frame_system::Config;
use sp_runtime::DispatchError;

use crate::{ClusterId, NodeParams, NodePubKey, NodeUsage};

<<<<<<< HEAD
<<<<<<< HEAD
pub trait NodeManager<T: Config> {
	fn get_cluster_id(node_pub_key: &NodePubKey) -> Result<Option<ClusterId>, DispatchError>;
	fn exists(node_pub_key: &NodePubKey) -> bool;
	fn get_node_provider_id(node_pub_key: &NodePubKey) -> Result<T::AccountId, DispatchError>;
	fn get_node_params(node_pub_key: &NodePubKey) -> Result<NodeParams, DispatchError>;
	fn update_total_node_usage(
		node_key: &NodePubKey,
		payable_usage: &NodeUsage,
	) -> Result<(), DispatchError>;
	#[cfg(feature = "runtime-benchmarks")]
=======
pub trait NodeVisitor<T: Config> {
=======
pub trait NodeManager<T: Config> {
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
	fn get_cluster_id(node_pub_key: &NodePubKey) -> Result<Option<ClusterId>, DispatchError>;
	fn exists(node_pub_key: &NodePubKey) -> bool;
	fn get_node_provider_id(node_pub_key: &NodePubKey) -> Result<T::AccountId, DispatchError>;
	fn get_node_params(node_pub_key: &NodePubKey) -> Result<NodeParams, DispatchError>;
	fn get_total_usage(node_pub_key: &NodePubKey) -> Result<Option<NodeUsage>, DispatchError>;
<<<<<<< HEAD
}

pub trait NodeCreator<T: Config> {
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
=======
	#[cfg(feature = "runtime-benchmarks")]
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
	fn create_node(
		node_pub_key: NodePubKey,
		provider_id: T::AccountId,
		node_params: NodeParams,
	) -> DispatchResult;
}
