use ddc_primitives::{ClusterId, NodePubKey};
use frame_system::Config;

pub trait StakingVisitor<T: Config> {
	fn node_has_stake(
		node_pub_key: &NodePubKey,
		cluster_id: &ClusterId,
	) -> Result<bool, StakingVisitorError>;

	fn node_is_chilling(node_pub_key: &NodePubKey) -> Result<bool, StakingVisitorError>;
}

pub enum StakingVisitorError {
	NodeStakeDoesNotExist,
	NodeStakeIsInBadState,
}
