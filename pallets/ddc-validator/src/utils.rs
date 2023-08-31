use alloc::{format, string::String};
use codec::{Decode, Encode};
use sp_core::crypto::AccountId32;
use sp_io::hashing::blake2_256;
pub use sp_std::prelude::*;

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

pub(crate) fn url_encode(input: &str) -> String {
	let mut encoded = String::new();

	for byte in input.bytes() {
		match byte {
			// Unreserved characters (alphanumeric and -_.~)
			b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
				encoded.push(byte as char);
			},
			_ => {
				encoded.push('%');
				encoded.push_str(&format!("{:02X}", byte));
			},
		}
	}

	encoded
}

pub(crate) fn unescape(json: &str) -> String {
	let mut result = String::new();
	let mut chars = json.chars().peekable();

	while let Some(c) = chars.next() {
		if c == '\\' {
			match chars.peek() {
				Some('u') => {
					// Skip over the next 5 characters
					for _ in 0..5 {
						chars.next();
					}
				},
				_ => {
					// Skip over the next character
					chars.next();
				},
			}
		} else {
			result.push(c);
		}
	}

	result
}
