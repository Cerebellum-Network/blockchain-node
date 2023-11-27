use frame_system::Config;

pub trait PalletVisitor<T: Config> {
	fn get_account_id() -> T::AccountId;
}
