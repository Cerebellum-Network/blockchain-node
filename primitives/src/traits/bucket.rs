use sp_runtime::DispatchResult;

use crate::{BucketId, BucketVisitorError, ClusterId, CustomerUsage};

pub trait BucketManager<T: frame_system::Config> {
	fn inc_total_customer_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: T::AccountId,
		customer_usage: &CustomerUsage,
	) -> DispatchResult;
}

pub trait BucketVisitor<T: frame_system::Config> {
	fn get_total_customer_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: &T::AccountId,
	) -> Result<Option<CustomerUsage>, BucketVisitorError>;
}
