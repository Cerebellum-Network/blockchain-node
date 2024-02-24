use sp_runtime::DispatchError;

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
use crate::{BucketId, BucketUsage, ClusterId};
=======
use crate::{BucketId, ClusterId, CustomerUsage};
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
use crate::{BucketId, BucketUsage, ClusterId};
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
=======
use crate::BucketId;
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)

pub trait CustomerCharger<T: frame_system::Config> {
	fn charge_bucket_owner(
		content_owner: T::AccountId,
		billing_vault: T::AccountId,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		customer_usage: &BucketUsage,
=======
		customer_usage: &CustomerUsage,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		customer_usage: &BucketUsage,
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
=======
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
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
