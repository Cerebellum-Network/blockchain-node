use core::u128;

use sp_runtime::DispatchError;

use crate::{BucketId, ClusterId, CustomerUsage};

pub trait CustomerCharger<T: frame_system::Config> {
	fn charge_content_owner(
		cluster_id: &ClusterId,
		bucket_id: BucketId,
		content_owner: T::AccountId,
		billing_vault: T::AccountId,
		customer_usage: &CustomerUsage,
		amount: u128,
	) -> Result<u128, DispatchError>;
}

pub trait CustomerDepositor<T: frame_system::Config> {
	fn deposit(customer: T::AccountId, amount: u128) -> Result<(), DispatchError>;
	fn deposit_extra(customer: T::AccountId, amount: u128) -> Result<(), DispatchError>;
}
