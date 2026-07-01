use polkadot_sdk::sp_runtime::DispatchError;

pub trait CustomerCharger<T: polkadot_sdk::frame_system::Config> {
	fn charge_content_owner(
		content_owner: T::AccountId,
		billing_vault: T::AccountId,
		amount: u128,
	) -> Result<u128, DispatchError>;
}

pub trait CustomerDepositor<T: polkadot_sdk::frame_system::Config> {
	fn deposit(customer: T::AccountId, amount: u128) -> Result<(), DispatchError>;
	fn deposit_extra(customer: T::AccountId, amount: u128) -> Result<(), DispatchError>;
}
