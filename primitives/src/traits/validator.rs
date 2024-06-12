use frame_system::Config;
use sp_std::prelude::*;

use crate::{BatchIndex, ClusterId, CustomerUsage, DdcEra, NodeUsage};

pub trait ValidatorVisitor<T: Config> {
	fn setup_validators(validators: Vec<T::AccountId>);
	fn is_ocw_validator(caller: T::AccountId) -> bool;
	fn is_customers_batch_valid(
		cluster_id: ClusterId,
		era: DdcEra,
		batch_index: BatchIndex,
		payers: Vec<(T::AccountId, CustomerUsage)>,
	) -> bool;
	fn is_providers_batch_valid(
		cluster_id: ClusterId,
		era: DdcEra,
		batch_index: BatchIndex,
		payees: Vec<(T::AccountId, NodeUsage)>,
	) -> bool;
}
