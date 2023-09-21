//! Validators' "shared memory" module.
//!
//! It implements a step of the DAC and Validation sequence when validators share their intermediate
//! validation results with each other. The design of the "shared memory" is expected to become like
//! transactions pool or peers list in the future, but for now it works on the centralized Redis
//! server which we maintain for DAC DataModel.

use alloc::{format, string::String};
pub use sp_std::collections::btree_map::BTreeMap;
// ToDo: remove String usage
use crate::{dac, utils, ValidationDecision};
use alt_serde::{Deserialize, Serialize};
use base64::prelude::*;
use lite_json::json::JsonValue;
use sp_runtime::offchain::{http, Duration};
use sp_staking::EraIndex;
use sp_std::prelude::*;

const HTTP_TIMEOUT_MS: u64 = 30_000;

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct IntermediateDecisionsWrapper {
	#[serde(rename = "JSON.GET")]
	json: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(crate = "alt_serde")]
pub(crate) struct IntermediateDecisions {
	validators_to_decisions: BTreeMap<String, IntermediateDecision>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(crate = "alt_serde")]
struct IntermediateDecision {
	result: bool,
	data: String,
}

pub fn base64_decode(input: &String) -> Result<Vec<u8>, ()> {
	let mut buf = Vec::with_capacity(1000); // ToDo: calculate capacity
	buf.resize(1000, 0);
	BASE64_STANDARD.decode_slice(input, &mut buf).map_err(|_| ())?;
	Ok(buf.to_vec())
}

/// Encodes a vector of bytes into a vector of characters using base64 encoding.
pub fn base64_encode(input: &Vec<u8>) -> Result<Vec<char>, ()> {
	let mut buf = Vec::with_capacity(1000); // ToDo: calculate capacity
	buf.resize(1000, 0);
	BASE64_STANDARD.encode_slice(input, &mut buf).map_err(|_| ())?;
	Ok(buf.iter().map(|&byte| byte as char).collect())
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
	let validation_decision_string = String::from(validation_decision_encoded);
	let json = serde_json::json!({
		"result": validation_result,
		"data": validation_decision_string,
	});
	let json_str = serde_json::to_string(&json).unwrap();
	let unescaped_json = utils::unescape(&json_str);
	let url_encoded_json = utils::url_encode(&unescaped_json);

	log::debug!("json_str: {:?}", json_str);

	let url = format!(
		"{}/FCALL/save_validation_result_by_node/1/{}:{}:{}/{}",
		shared_memory_webdis_url, validator, cdn_node, era, url_encoded_json,
	);

	log::debug!("share_intermediate_validation_result url: {:?}", url);
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

	log::debug!("body_str: {:?}", body_str);

	let json = lite_json::parse_json(body_str).map_err(|_| {
		log::warn!("No JSON body");
		http::Error::Unknown
	})?;
	Ok(json)
}

pub(crate) fn get_intermediate_decisions(
	data_provider_url: &String,
	edge: &str,
	era: &EraIndex,
	quorum: Vec<String>,
) -> Vec<ValidationDecision> {
	let url = format!("{}/JSON.GET/ddc:dac:shared:nodes:{}", data_provider_url, era);

	let response: IntermediateDecisionsWrapper = dac::http_get_json(url.as_str()).unwrap();
	let mut edges_to_validators_decisions: BTreeMap<
		String,
		BTreeMap<String, IntermediateDecision>,
	> = serde_json::from_str(&response.json).unwrap();
	let decisions_for_edge = IntermediateDecisions {
		validators_to_decisions: edges_to_validators_decisions.remove(edge).unwrap(),
	};

	let quorum_decisions = find_quorum_decisions(decisions_for_edge, quorum);

	decode_intermediate_decisions(quorum_decisions)
}

pub(crate) fn decode_intermediate_decisions(
	decisions: IntermediateDecisions,
) -> Vec<ValidationDecision> {
	let mut decoded_decisions: Vec<ValidationDecision> = Vec::new();

	for (_, decision) in decisions.validators_to_decisions.iter() {
		let data = base64_decode(&decision.data).unwrap();

		let data_str = String::from_utf8_lossy(&data);
		let data_trimmed = data_str.trim_end_matches('\0');

		log::debug!("data_str: {:?}", data_trimmed);

		let decoded_decision: ValidationDecision = serde_json::from_str(data_trimmed).unwrap();

		decoded_decisions.push(decoded_decision);
	}

	decoded_decisions
}

pub(crate) fn find_quorum_decisions(
	all_decisions: IntermediateDecisions,
	quorum: Vec<String>,
) -> IntermediateDecisions {
	let mut quorum_decisions: BTreeMap<String, IntermediateDecision> = BTreeMap::new();
	for (validator_id, decision) in all_decisions.validators_to_decisions.iter() {
		if quorum.contains(validator_id) {
			quorum_decisions.insert(validator_id.clone(), decision.clone());
		}
	}

	IntermediateDecisions { validators_to_decisions: quorum_decisions }
}
