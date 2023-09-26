//! A module with Data Activity Capture (DAC) interaction.

use crate::{utils, DacTotalAggregates, OpCode, ValidationDecision};
use alloc::string::String; // ToDo: remove String usage
use alt_serde::{de::DeserializeOwned, Deserialize, Serialize};
use codec::{Decode, Encode};
use sp_runtime::offchain::{http, Duration};
use sp_staking::EraIndex;
pub use sp_std::{
	collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	prelude::*,
};

pub type TimestampInSec = u64;
pub const HTTP_TIMEOUT_MS: u64 = 30_000;
pub const FAILED_CONTENT_CONSUMER_THRESHOLD: TimestampInSec = 100;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct RedisFtAggregate {
	#[serde(rename = "FT.AGGREGATE")]
	pub ft_aggregate: Vec<FtAggregate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(crate = "alt_serde")]
#[serde(untagged)]
pub enum FtAggregate {
	Length(u32),
	Node(Vec<String>),
}

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq)]
pub struct BytesSent {
	pub node_public_key: String,
	pub era: EraIndex,
	pub sum: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct FileRequestWrapper {
	#[serde(rename = "JSON.GET")]
	json: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct CDNNodeAggregates {
	aggregate: Vec<CDNNodeAggregate>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct CDNNodeAggregate {
	total_bytes_sent: u64,
	total_queries: u64,
	total_reads: u64,
	total_reads_acked: u64,
	total_queries_acked: u64,
	average_response_time_ms: f64,
	total_bytes_received: u64,
	pub request_ids: Vec<String>,
	total_writes_acked: u64,
	average_response_time_ms_samples: u64,
	total_writes: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct FileRequests {
	requests: Requests,
}

pub type Requests = BTreeMap<String, FileRequest>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct FileRequest {
	pub file_request_id: String,
	file_info: FileInfo,
	bucket_id: u128,
	timestamp: u64,
	chunks: BTreeMap<String, Chunk>,
	user_public_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct Chunk {
	log: Log,
	cid: String,
	ack: Option<Ack>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct Ack {
	user_timestamp: u64,
	nonce: String,
	signature: Option<String>,
	aggregated: u64,
	user_public_key: String,
	bytes_received: u64,
	requested_chunk_cids: Vec<String>,
	node_public_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct Log {
	#[serde(rename = "type")]
	log_type: u64,
	signature: Option<String>,
	aggregated: u64,
	user_public_key: String,
	era: u64,
	bucket_id: u128,
	user_address: String,
	bytes_sent: u64,
	timestamp: u64,
	node_public_key: String,
	session_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
	#[serde(rename = "chunkCids", skip_deserializing)]
	chunk_cids: Option<Vec<String>>,

	#[serde(rename = "requestedChunkCids")]
	requested_chunk_cids: Vec<String>,
}

type EdgeId = String;
type ValidatorId = String;

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "alt_serde")]
pub(crate) struct EdgesToResults(BTreeMap<EdgeId, Vec<ValidationResult>>);

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "alt_serde")]
pub(crate) struct Wrapper {
	#[serde(rename = "HGET")]
	decisions: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(crate = "alt_serde")]
pub(crate) struct ValidationResult {
	validator_id: ValidatorId,
	edge_id: EdgeId,
	result: bool,
	received: u64,
	sent: u64,
	era: EraIndex,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "alt_serde")]
pub(crate) struct FinalDecision {
	result: bool,
	edge_id: EdgeId,
	era: EraIndex,
	received: u64,
	sent: u64,
	results_logs: Vec<ValidationResult>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "alt_serde")]
pub(crate) struct FinalDecisions(BTreeMap<String, FinalDecision>);

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq)]
pub struct BytesReceived {
	pub node_public_key: String,
	pub era: EraIndex,
	pub sum: u32,
}

fn get_timestamps_with_ack(file_requests: &Requests) -> Vec<TimestampInSec> {
	let mut timestamps: Vec<TimestampInSec> = Vec::new();

	for (_, file_request) in file_requests {
		for (_, chunk) in &file_request.chunks {
			if let Some(_ack) = &chunk.ack {
				timestamps.push(chunk.log.timestamp);
			}
		}
	}

	timestamps.sort();

	timestamps
}

pub fn get_acknowledged_bytes_bucket<'a>(
	file_requests: &'a Requests,
	acknowledged_bytes_by_bucket: &'a mut Vec<(u128, u64)>,
) -> &'a Vec<(u128, u64)> {
	let ack_timestamps = get_timestamps_with_ack(file_requests);

	for (_, file_request) in file_requests {
		let mut total_bytes_received = 0u64;
		let bucket_id = file_request.bucket_id;
		for (_, chunk) in &file_request.chunks {
			// Only check reads
			match chunk.log.log_type.try_into() {
				Ok(OpCode::Read) =>
					if let Some(ack) = &chunk.ack {
						total_bytes_received += ack.bytes_received;
					} else {
						total_bytes_received += get_proved_delivered_bytes(chunk, &ack_timestamps);
					},
				_ => (),
			}
		}
		acknowledged_bytes_by_bucket.push((bucket_id, total_bytes_received));
	}

	acknowledged_bytes_by_bucket
}

pub fn get_served_bytes_sum(file_requests: &Requests) -> (u64, u64) {
	let ack_timestamps = get_timestamps_with_ack(file_requests);
	let mut total_bytes_received = 0u64;
	let mut total_bytes_sent = 0u64;

	for (_, file_request) in file_requests {
		for (_, chunk) in &file_request.chunks {
			match chunk.log.log_type.try_into() {
				Ok(OpCode::Read) => {
					total_bytes_sent += chunk.log.bytes_sent;

					if let Some(ack) = &chunk.ack {
						total_bytes_received += ack.bytes_received;
					} else {
						total_bytes_received += get_proved_delivered_bytes(chunk, &ack_timestamps);
					}
				},
				_ => (),
			}
		}
	}

	(total_bytes_sent, total_bytes_received)
}

fn get_proved_delivered_bytes(chunk: &Chunk, ack_timestamps: &Vec<TimestampInSec>) -> u64 {
	let log_timestamp = chunk.log.timestamp;
	let neighbors = get_closer_neighbors(log_timestamp, &ack_timestamps);
	let is_proved =
		is_lies_within_threshold(log_timestamp, neighbors, FAILED_CONTENT_CONSUMER_THRESHOLD);

	if is_proved {
		return chunk.log.bytes_sent
	} else {
		0
	}
}

fn get_closer_neighbors(
	timestamp: TimestampInSec,
	timestamps: &Vec<TimestampInSec>,
) -> (TimestampInSec, TimestampInSec) {
	let mut before = 0;
	let mut after = TimestampInSec::MAX;
	for ts in timestamps {
		if ts < &timestamp {
			before = before.max(*ts);
		} else if ts > &timestamp {
			after = after.min(*ts);
		}
	}

	(before, after)
}

fn is_lies_within_threshold(
	timestamp: TimestampInSec,
	borders: (TimestampInSec, TimestampInSec),
	threshold: TimestampInSec,
) -> bool {
	let left_distance = timestamp - borders.0;
	let right_distance = borders.1 - timestamp;

	if left_distance < threshold || right_distance < threshold {
		return true
	}

	false
}

pub(crate) fn fetch_cdn_node_aggregates_request(url: &String) -> Vec<CDNNodeAggregate> {
	log::debug!("fetch_file_request | url: {:?}", url);
	let response: FileRequestWrapper = http_get_json(&url).unwrap();
	log::debug!("response.json: {:?}", response.json);
	let map: Vec<CDNNodeAggregate> = serde_json::from_str(response.json.as_str()).unwrap();
	// log::debug!("response.json: {:?}", response.json);

	map
}

pub(crate) fn fetch_file_request(url: &String) -> FileRequest {
	log::debug!("fetch_file_request | url: {:?}", url);
	let response: FileRequestWrapper = http_get_json(&url).unwrap();
	log::debug!("response.json: {:?}", response.json);

	let map: FileRequest = serde_json::from_str(response.json.as_str()).unwrap();

	map
}

pub(crate) fn http_get_json<OUT: DeserializeOwned>(url: &str) -> crate::ResultStr<OUT> {
	let body = http_get_request(url).map_err(|err| {
		log::error!("[DAC Validator] Error while getting {}: {:?}", url, err);
		"HTTP GET error"
	})?;

	let parsed = serde_json::from_slice(&body).map_err(|err| {
		log::warn!("[DAC Validator] Error while parsing JSON from {}: {:?}", url, err);
		"HTTP JSON parse error"
	});

	parsed
}

fn http_get_request(http_url: &str) -> Result<Vec<u8>, http::Error> {
	// log::debug!("[DAC Validator] Sending request to: {:?}", http_url);

	// Initiate an external HTTP GET request. This is using high-level wrappers from
	// `sp_runtime`.
	let request = http::Request::get(http_url);

	let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(HTTP_TIMEOUT_MS));

	let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;

	let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

	if response.code != 200 {
		log::warn!("[DAC Validator] http_get_request unexpected status code: {}", response.code);
		return Err(http::Error::Unknown)
	}

	// Next we fully read the response body and collect it to a vector of bytes.
	Ok(response.body().collect::<Vec<u8>>())
}

pub(crate) fn get_final_decision(decisions: Vec<ValidationDecision>) -> ValidationDecision {
	let common_decisions = find_largest_group(decisions).unwrap();
	let decision_example = common_decisions.get(0).unwrap();

	let serialized_decisions = serde_json::to_string(&common_decisions).unwrap();

	let final_decision = ValidationDecision {
		edge: decision_example.edge.clone(),
		result: decision_example.result,
		payload: utils::hash(&serialized_decisions),
		totals: DacTotalAggregates {
			received: decision_example.totals.received,
			sent: decision_example.totals.sent,
			failed_by_client: 0,
			failure_rate: 0,
		},
	};

	final_decision
}

fn find_largest_group(decisions: Vec<ValidationDecision>) -> Option<Vec<ValidationDecision>> {
	let mut groups: Vec<Vec<ValidationDecision>> = Vec::new();
	let half = decisions.len() / 2;

	for decision in decisions {
		let mut found_group = false;

		for group in &mut groups {
			if group.iter().all(|x| {
				x.result == decision.result &&
					x.totals.received == decision.totals.received &&
					x.totals.sent == decision.totals.sent
			}) {
				group.push(decision.clone());
				found_group = true;
				break
			}
		}

		if !found_group {
			groups.push(vec![decision]);
		}
	}

	let largest_group = groups.into_iter().max_by_key(|group| group.len()).unwrap_or(Vec::new());

	if largest_group.len() > half {
		Some(largest_group)
	} else {
		None
	}
}
