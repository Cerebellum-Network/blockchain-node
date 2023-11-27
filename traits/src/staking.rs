use ddc_primitives::{ClusterId, NodePubKey};
use frame_system::Config;

pub trait StakingVisitor<T: Config> {
	fn has_activated_stake(
		node_pub_key: &NodePubKey,
		cluster_id: &ClusterId,
	) -> Result<bool, StakingVisitorError>;

	fn has_stake(node_pub_key: &NodePubKey) -> bool;

	fn has_chilling_attempt(node_pub_key: &NodePubKey) -> Result<bool, StakingVisitorError>;
}

pub enum StakingVisitorError {
	NodeStakeDoesNotExist,
	NodeStakeIsInBadState,
}
