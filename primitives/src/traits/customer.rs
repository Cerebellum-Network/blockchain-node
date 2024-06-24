use core::u128;

use sp_runtime::{DispatchError, DispatchResult};

use crate::{BucketId, ClusterId, CustomerUsage, NodeUsage};

pub trait CustomerCharger<T: frame_system::Config> {
	fn charge_content_owner(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: T::AccountId,
		billing_vault: T::AccountId,
		customer_usage: &CustomerUsage,
		amount: u128,
	) -> Result<u128, DispatchError>;

	// todo! deprecate in favor of BucketManager trait
	fn inc_total_node_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		node_usage: &NodeUsage,
	) -> DispatchResult;

	fn inc_total_customer_usage(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: T::AccountId,
		customer_usage: &CustomerUsage,
	) -> DispatchResult;
}

pub trait CustomerDepositor<T: frame_system::Config> {
	fn deposit(customer: T::AccountId, amount: u128) -> Result<(), DispatchError>;
	fn deposit_extra(customer: T::AccountId, amount: u128) -> Result<(), DispatchError>;
}
