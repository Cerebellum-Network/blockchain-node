use sp_runtime::{DispatchError, DispatchResult};

use crate::{BucketId, ClusterId, CustomerUsage};

pub trait BucketManager<T: frame_system::Config> {
	fn inc_total_customer_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: T::AccountId,
		customer_usage: &CustomerUsage,
	) -> DispatchResult;
}

pub trait BucketVisitor<T: frame_system::Config> {
	fn get_bucket_owner_id(bucket_id: BucketId) -> Result<T::AccountId, DispatchError>;

	fn get_total_customer_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: &T::AccountId,
	) -> Result<Option<CustomerUsage>, DispatchError>;
}
