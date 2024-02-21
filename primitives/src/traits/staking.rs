use frame_system::Config;
use sp_staking::EraIndex;

use crate::{ClusterId, NodePubKey, StakingLedger};

pub trait ProtocolStakingVisitor<T: Config, Balance> {
	fn current_era() -> Option<EraIndex>;
	fn get_ledger(node_pub_key: &T::AccountId) -> Option<StakingLedger<T, Balance>>;
}

pub trait DDCStakingVisitor<T: Config> {
	fn has_activated_stake(
		node_pub_key: &NodePubKey,
		cluster_id: &ClusterId,
	) -> Result<bool, StakingVisitorError>;

	fn has_stake(node_pub_key: &NodePubKey) -> bool;

	fn has_chilling_attempt(node_pub_key: &NodePubKey) -> Result<bool, StakingVisitorError>;
}

pub trait DDCStakerCreator<T: Config, Balance> {
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
