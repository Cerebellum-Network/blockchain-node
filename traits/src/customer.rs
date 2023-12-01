use sp_runtime::DispatchError;
pub trait CustomerCharger<T: frame_system::Config> {
	fn charge_content_owner(
		content_owner: T::AccountId,
		billing_vault: T::AccountId,
		amount: u128,
	) -> Result<u128, DispatchError>;
}
