use frame_system::Config;
use scale_info::prelude::string::String;
#[cfg(feature = "runtime-benchmarks")]
use sp_std::prelude::*;

use crate::{BatchIndex, BucketId, ClusterId, CustomerUsage, DdcEra, MMRProof, NodeUsage};

pub trait ValidatorVisitor<T: Config> {
	#[cfg(feature = "runtime-benchmarks")]
	fn setup_validators(validators: Vec<T::AccountId>);
	fn is_ocw_validator(caller: T::AccountId) -> bool;
	fn is_customers_batch_valid(
		cluster_id: ClusterId,
		era: DdcEra,
		batch_index: BatchIndex,
		payers: &[(T::AccountId, String, BucketId, CustomerUsage)],
		batch_proof: &MMRProof,
	) -> bool;
	fn is_providers_batch_valid(
		cluster_id: ClusterId,
		era: DdcEra,
		batch_index: BatchIndex,
		payees: &[(T::AccountId, NodeUsage)],
		batch_proof: &MMRProof,
	) -> bool;
}
