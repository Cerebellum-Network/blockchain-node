<<<<<<< HEAD
<<<<<<< HEAD
use sp_runtime::{DispatchError, DispatchResult};

<<<<<<< HEAD
<<<<<<< HEAD
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
=======
use sp_runtime::DispatchResult;
=======
use sp_runtime::{DispatchError, DispatchResult};
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)

use crate::{BucketId, BucketVisitorError, ClusterId, CustomerUsage};
=======
use crate::{BucketId, ClusterId, CustomerUsage};
>>>>>>> c3a18c28 (chore: using generic type for bucket visitor)

=======
#[cfg(feature = "runtime-benchmarks")]
use crate::BucketParams;
<<<<<<< HEAD
use crate::{BucketId, ClusterId, CustomerUsage};
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
=======
use crate::{BucketId, BucketUsage, ClusterId};
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
pub trait BucketManager<T: frame_system::Config> {
	fn get_bucket_owner_id(bucket_id: BucketId) -> Result<T::AccountId, DispatchError>;

	fn get_total_bucket_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: &T::AccountId,
	) -> Result<Option<BucketUsage>, DispatchError>;

	fn inc_total_bucket_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: T::AccountId,
		customer_usage: &BucketUsage,
	) -> DispatchResult;

	#[cfg(feature = "runtime-benchmarks")]
	fn create_bucket(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
<<<<<<< HEAD
		content_owner: &T::AccountId,
<<<<<<< HEAD
	) -> Result<Option<CustomerUsage>, BucketVisitorError>;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	) -> Result<Option<CustomerUsage>, DispatchError>;
>>>>>>> c3a18c28 (chore: using generic type for bucket visitor)
=======
		owner_id: T::AccountId,
		bucket_params: BucketParams,
	) -> Result<(), DispatchError>;
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
}
