use sp_runtime::{DispatchError, DispatchResult};

#[cfg(feature = "runtime-benchmarks")]
use crate::BucketParams;
use crate::{BucketId, BucketUsage, ClusterId};
pub trait BucketManager<T: frame_system::Config> {
	fn get_bucket_owner_id(bucket_id: BucketId) -> Result<T::AccountId, DispatchError>;

	fn get_total_bucket_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		bucket_owner: &T::AccountId,
	) -> Result<Option<BucketUsage>, DispatchError>;

	fn update_total_bucket_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		bucket_owner: T::AccountId,
		payable_usage: &BucketUsage,
	) -> DispatchResult;

	#[cfg(feature = "runtime-benchmarks")]
	fn create_bucket(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		owner_id: T::AccountId,
		bucket_params: BucketParams,
	) -> Result<(), DispatchError>;
}
