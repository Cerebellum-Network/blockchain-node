//! Validators' "shared memory" module.

use base64::prelude::*;
use sp_std::prelude::*;

/// Encodes a vector of bytes into a vector of characters using base64 encoding.
pub fn base64_encode(input: &Vec<u8>) -> Vec<char> {
	let mut buf = Vec::with_capacity(1024); // ToDo: calculate capacity
	buf.resize(1024, 0);
	BASE64_STANDARD.encode_slice(input, &mut buf).unwrap(); // ToDo: handle error
	buf.iter().map(|&byte| byte as char).collect()
}
