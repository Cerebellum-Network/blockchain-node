#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::{env::Environment, prelude::vec::Vec};

type AccountId = <ink::env::DefaultEnvironment as Environment>::AccountId;

#[ink::trait_definition]
pub trait DdcPayouts {
	#[ink(message)]
	fn charge(
		&mut self,
		vault: AccountId,
		batch: Vec<(AccountId, u128)>,
	) -> Result<Vec<(AccountId, u128)>, ()>;
}
