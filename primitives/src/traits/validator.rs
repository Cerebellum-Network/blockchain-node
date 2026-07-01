use polkadot_sdk::frame_system::Config;
use polkadot_sdk::sp_std::prelude::*;

pub trait ValidatorVisitor<T: Config> {
	fn get_active_validators() -> Vec<T::AccountId>;
}
