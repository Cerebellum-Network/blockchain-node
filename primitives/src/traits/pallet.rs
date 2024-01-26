use frame_support::traits::OriginTrait;
use frame_system::Config;

pub type PalletsOriginOf<T> =
	<<T as frame_system::Config>::RuntimeOrigin as OriginTrait>::PalletsOrigin;

pub trait GetDdcOrigin<T: Config> {
	fn get() -> T::RuntimeOrigin;
}

pub trait PalletVisitor<T: Config> {
	fn get_account_id() -> T::AccountId;
}
