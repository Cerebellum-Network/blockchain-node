//! Validators' "shared memory" module.

use alloc::{format, string::String}; // ToDo: remove String usage
use base64::prelude::*;
use lite_json::json::JsonValue;
use sp_runtime::offchain::{http, Duration};
use sp_staking::EraIndex;
use sp_std::prelude::*;

const HTTP_TIMEOUT_MS: u64 = 30_000;

/// Encodes a vector of bytes into a vector of characters using base64 encoding.
pub fn base64_encode(input: &Vec<u8>) -> Vec<char> {
	let mut buf = Vec::with_capacity(1024); // ToDo: calculate capacity
	buf.resize(1024, 0);
	BASE64_STANDARD.encode_slice(input, &mut buf).unwrap(); // ToDo: handle error
	buf.iter().map(|&byte| byte as char).collect()
}

/// Publish intermediate validation result to redis.
pub fn share_intermediate_validation_result(
	shared_memory_webdis_url: &String,
	era: EraIndex,
	validator: &String,
	cdn_node: &String,
	validation_result: bool,
	validation_decision_encoded: &String,
) -> Result<JsonValue, http::Error> {
	let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(HTTP_TIMEOUT_MS));
	let validation_result_string = String::from(if validation_result { "true" } else { "false" });
	let validation_decision_string = String::from(validation_decision_encoded);
	let url = format!(
		"{}/FCALL/save_validation_result_by_node/1/{}:{}:{}/{{\"result\":{},\"data\":{}}}",
		shared_memory_webdis_url,
		validator,
		cdn_node,
		era,
		validation_result_string,
		validation_decision_string,
	);
	let request = http::Request::get(url.as_str());
	let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
	let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
	if response.code != 200 {
		log::warn!("Unexpected status code: {}", response.code);
		return Err(http::Error::Unknown)
	}
	let body = response.body().collect::<Vec<u8>>();
	let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
		log::warn!("No UTF-8 body");
		http::Error::Unknown
	})?;
	let json = lite_json::parse_json(body_str).map_err(|_| {
		log::warn!("No JSON body");
		http::Error::Unknown
	})?;
	Ok(json)
}
