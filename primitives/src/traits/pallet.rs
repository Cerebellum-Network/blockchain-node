use polkadot_sdk::frame_support::traits::OriginTrait;
use polkadot_sdk::frame_system::Config;

pub type PalletsOriginOf<T> =
	<<T as polkadot_sdk::frame_system::Config>::RuntimeOrigin as OriginTrait>::PalletsOrigin;

pub trait GetDdcOrigin<T: Config> {
	fn get() -> T::RuntimeOrigin;
}

pub trait PalletVisitor<T: Config> {
	fn get_account_id() -> T::AccountId;
}
