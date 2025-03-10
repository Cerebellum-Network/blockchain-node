use frame_system::Config;
use scale_info::prelude::vec::Vec;
use sp_runtime::Percent;

pub trait ValidatorVisitor<T: Config> {
	fn is_ocw_validator(caller: T::AccountId) -> bool;
	fn is_quorum_reached(quorum: Percent, members_count: usize) -> bool;
}

pub trait InspReceiptsInterceptor {
	type Receipt;
	fn intercept(receipts: Vec<Self::Receipt>) -> Vec<Self::Receipt>;
}
