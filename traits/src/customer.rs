use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

pub trait CustomerCharger<T: frame_system::Config> {
	fn charge_content_owner(
		content_owner: T::AccountId,
		billing_vault: T::AccountId,
		amount: u128,
	) -> Result<u128, CustomerChargerError>;
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum CustomerChargerError {
	NotOwner,
	ArithmeticUnderflow,
	TransferFailed,
	UnlockFailed,
}
