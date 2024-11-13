use sp_runtime::{DispatchError, DispatchResult};

#[cfg(feature = "runtime-benchmarks")]
use crate::BucketParams;
use crate::{BucketId, ClusterId, CustomerUsage};
pub trait BucketManager<T: frame_system::Config> {
	fn get_bucket_owner_id(bucket_id: BucketId) -> Result<T::AccountId, DispatchError>;

	fn get_total_bucket_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: &T::AccountId,
	) -> Result<Option<CustomerUsage>, DispatchError>;

	fn inc_total_bucket_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: T::AccountId,
		customer_usage: &CustomerUsage,
	) -> DispatchResult;

	#[cfg(feature = "runtime-benchmarks")]
	fn create_bucket(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		owner_id: T::AccountId,
		bucket_params: BucketParams,
	) -> Result<(), DispatchError>;
}
