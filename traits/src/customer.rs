pub trait CustomerCharger<T: frame_system::Config> {
	// todo: WIP for decoupling payout and customers
	fn charge_content_owner(
		content_owner: T::AccountId,
		billing_vault: T::AccountId,
		amount: u128,
	) -> sp_runtime::DispatchResult;
}
