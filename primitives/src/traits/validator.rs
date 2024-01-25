use frame_system::Config;
use sp_std::prelude::*;

pub trait ValidatorVisitor<T: Config> {
	fn get_active_validators() -> Vec<T::AccountId>;
}
