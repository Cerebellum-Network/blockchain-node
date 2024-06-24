use sp_runtime::DispatchResult;

use crate::{BucketId, ClusterId, CustomerUsage, NodeUsage};

pub trait BucketManager<T: frame_system::Config> {
	fn inc_total_customer_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: T::AccountId,
		customer_usage: &CustomerUsage,
	) -> DispatchResult;

	fn inc_total_node_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		node_usage: &NodeUsage,
	) -> DispatchResult;
}
