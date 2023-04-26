use alloc::string::String;
use codec::{Decode, Encode};
use sp_core::crypto::AccountId32;

pub fn account_to_string<T: frame_system::Config>(account: T::AccountId) -> String {
	let to32 = T::AccountId::encode(&account);
	let pub_key_str = array_bytes::bytes2hex("", to32);

	pub_key_str
}

pub fn string_to_account<T: frame_system::Config>(pub_key_str: String) -> T::AccountId {
	let acc32: sp_core::crypto::AccountId32 =
		array_bytes::hex2array::<_, 32>(pub_key_str).unwrap().into();
	let mut to32 = AccountId32::as_ref(&acc32);
	let address: T::AccountId = T::AccountId::decode(&mut to32).unwrap();
	address
}
