use alloc::string::String;
use codec::{Decode, Encode};
use sp_core::crypto::AccountId32;
use sp_io::hashing::blake2_256;
use crate::dac::ValidationResult;
pub use sp_std::{
	prelude::*,
};

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

pub(crate) fn hash(data: &String) -> [u8; 32] {
	let hash = blake2_256(data.as_bytes());
	let mut result = [0u8; 32];
	result.copy_from_slice(&hash);

	result
}

pub(crate) fn get_hashed(data: &Vec<ValidationResult>) -> [u8; 256] {
	let results_log = serde_json::to_string(data).unwrap();
	let mut payload:[u8; 256] = [0; 256];
	let hashed_results = hash(&results_log);
	payload[..32].copy_from_slice(&hashed_results);

	payload
}
