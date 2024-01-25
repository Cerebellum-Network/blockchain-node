use frame_system::Config;

use crate::{ClusterId, NodePubKey};

pub trait StakingVisitor<T: Config> {
	fn has_activated_stake(
		node_pub_key: &NodePubKey,
		cluster_id: &ClusterId,
	) -> Result<bool, StakingVisitorError>;

	fn has_stake(node_pub_key: &NodePubKey) -> bool;

	fn has_chilling_attempt(node_pub_key: &NodePubKey) -> Result<bool, StakingVisitorError>;
}

pub trait StakerCreator<T: Config, Balance> {
	fn bond_stake_and_participate(
		stash: T::AccountId,
		controller: T::AccountId,
		node: NodePubKey,
		value: Balance,
		cluster_id: ClusterId,
	) -> sp_runtime::DispatchResult;
}

pub enum StakingVisitorError {
	NodeStakeDoesNotExist,
	NodeStakeIsInBadState,
}
