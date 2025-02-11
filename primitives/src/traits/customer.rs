use sp_runtime::DispatchError;

use crate::BucketId;

pub trait CustomerCharger<T: frame_system::Config> {
	fn charge_customer(
		content_owner: T::AccountId,
		payout_vault: T::AccountId,
		amount: u128,
	) -> Result<u128, DispatchError>;
}

pub trait CustomerDepositor<T: frame_system::Config> {
	fn deposit(customer: T::AccountId, amount: u128) -> Result<(), DispatchError>;
	fn deposit_extra(customer: T::AccountId, amount: u128) -> Result<(), DispatchError>;
}

pub trait CustomerVisitor<T: frame_system::Config> {
	fn get_bucket_owner(bucket_id: &BucketId) -> Result<T::AccountId, DispatchError>;
}
