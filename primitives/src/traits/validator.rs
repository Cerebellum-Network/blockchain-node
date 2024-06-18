use frame_system::Config;
use sp_std::prelude::*;

use crate::{ActivityHash, BatchIndex, ClusterId, CustomerUsage, DdcEra, NodeUsage};

pub trait ValidatorVisitor<T: Config> {
	fn setup_validators(validators: Vec<T::AccountId>);
	fn is_ocw_validator(caller: T::AccountId) -> bool;
	fn is_customers_batch_valid(
		cluster_id: ClusterId,
		era: DdcEra,
		batch_index: BatchIndex,
		payers: &[(T::AccountId, CustomerUsage)],
		adjacent_hashes: &[ActivityHash],
	) -> bool;
	fn is_providers_batch_valid(
		cluster_id: ClusterId,
		era: DdcEra,
		batch_index: BatchIndex,
		payees: &[(T::AccountId, NodeUsage)],
		adjacent_hashes: &[ActivityHash],
	) -> bool;
}
