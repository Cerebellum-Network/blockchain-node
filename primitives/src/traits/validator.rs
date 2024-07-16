use crate::{BatchIndex, BucketId, ClusterId, CustomerUsage, DdcEra, MMRProof, NodeUsage};
use frame_system::Config;

pub trait ValidatorVisitor<T: Config> {
	fn is_ocw_validator(caller: T::AccountId) -> bool;
	fn is_customers_batch_valid(
		cluster_id: ClusterId,
		era: DdcEra,
		batch_index: BatchIndex,
		payers: &[(T::AccountId, BucketId, CustomerUsage)],
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
