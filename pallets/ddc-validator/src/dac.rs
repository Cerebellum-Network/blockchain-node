//! A module with Data Activity Capture (DAC) interaction.

use crate::{utils, DacTotalAggregates, ValidationDecision};
use alloc::{format, string::String}; // ToDo: remove String usage
use alt_serde::{de::DeserializeOwned, Deserialize, Serialize};
use codec::{Decode, Encode};
use lite_json::json::JsonValue;
use log::info;
use serde_json::Value;
use sp_runtime::{
	generic::Era,
	offchain::{
		http,
		http::{Method, Request},
		Duration,
	},
};
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct FileRequestWrapper {
	#[serde(rename = "JSON.GET")]
	json: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct FileRequests {
	requests: Requests,
}

pub type Requests = BTreeMap<String, FileRequest>;

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct FileRequest {
	file_request_id: String,
	file_info: FileInfo,
	bucket_id: u64,
	timestamp: u64,
	chunks: BTreeMap<String, Chunk>,
	user_public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct Chunk {
	log: Log,
	cid: String,
	ack: Option<Ack>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct Ack {
	bytes_received: u64,
	user_timestamp: u64,
	nonce: String,
	node_public_key: String,
	user_public_key: String,
	signature: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct Log {
	#[serde(rename = "type")]
	log_type: u64,
	session_id: String,
	user_public_key: String,
	era: u64,
	user_address: String,
	bytes_sent: u64,
	timestamp: u64,
	node_public_key: String,
	signature: String,
	bucket_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
	#[serde(rename = "chunkCids")]
	chunk_cids: Vec<String>,

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

impl BytesSent {
	pub fn new(aggregate: RedisFtAggregate) -> BytesSent {
		let data = aggregate.ft_aggregate[1].clone();

		match data {
			FtAggregate::Node(node) =>
				return BytesSent {
					node_public_key: node[1].clone(),
					era: node[3].clone().parse::<u32>().expect("era must be convertible u32")
						as EraIndex,
					sum: node[5].parse::<u32>().expect("bytesSentSum must be convertible to u32"),
				},
			FtAggregate::Length(_) => panic!("[DAC Validator] Not a Node"),
		}
	}

	pub fn get_all(aggregation: RedisFtAggregate) -> Vec<BytesSent> {
		let mut res: Vec<BytesSent> = vec![];
		for i in 1..aggregation.ft_aggregate.len() {
			let data = aggregation.ft_aggregate[i].clone();
			match data {
				FtAggregate::Node(node) => {
					let node = BytesSent {
						node_public_key: node[1].clone(),
						era: node[3].clone().parse::<u32>().expect("era must be convertible u32")
							as EraIndex,
						sum: node[5]
							.parse::<u32>()
							.expect("bytesSentSum must be convertible to u32"),
					};

					res.push(node);
				},
				FtAggregate::Length(_) => panic!("[DAC Validator] Not a Node"),
			}
		}

		return res
	}
}

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq)]
pub struct BytesReceived {
	pub node_public_key: String,
	pub era: EraIndex,
	pub sum: u32,
}

impl BytesReceived {
	pub fn new(aggregate: RedisFtAggregate) -> BytesReceived {
		let data = aggregate.ft_aggregate[1].clone();

		match data {
			FtAggregate::Node(node) =>
				return BytesReceived {
					node_public_key: node[1].clone(),
					era: node[3].clone().parse::<u32>().expect("era must be convertible u32")
						as EraIndex,
					sum: node[5]
						.parse::<u32>()
						.expect("bytesReceivedSum must be convertible to u32"),
				},
			FtAggregate::Length(_) => panic!("[DAC Validator] Not a Node"),
		}
	}

	pub fn get_all(aggregation: RedisFtAggregate) -> Vec<BytesReceived> {
		let mut res: Vec<BytesReceived> = vec![];
		for i in 1..aggregation.ft_aggregate.len() {
			let data = aggregation.ft_aggregate[i].clone();
			match data {
				FtAggregate::Node(node) => {
					let node = BytesReceived {
						node_public_key: node[1].clone(),
						era: node[3].clone().parse::<u32>().expect("era must be convertible u32")
							as EraIndex,
						sum: node[5]
							.parse::<u32>()
							.expect("bytesReceivedSum must be convertible to u32"),
					};

					res.push(node);
				},
				FtAggregate::Length(_) => panic!("[DAC Validator] Not a Node"),
			}
		}

		return res
	}
}

fn get_timestamps_with_ack(file_requests: &Requests) -> Vec<TimestampInSec> {
	let mut timestamps: Vec<TimestampInSec> = Vec::new();

	for (_, file_request) in file_requests {
		for (_, chunk) in &file_request.chunks {
			if let Some(ack) = &chunk.ack {
				timestamps.push(chunk.log.timestamp);
			}
		}
	}

	timestamps.sort();

	timestamps
}

pub fn get_proved_delivered_bytes_sum(file_requests: &Requests) -> u64 {
	let ack_timestamps = get_timestamps_with_ack(file_requests);
	let mut total_bytes_received = 0u64;

	for (_, file_request) in file_requests {
		for (_, chunk) in &file_request.chunks {
			if let Some(ack) = &chunk.ack {
				total_bytes_received += &chunk.log.bytes_sent;
			} else {
				total_bytes_received += get_proved_delivered_bytes(chunk, &ack_timestamps);
			}
		}
	}

	total_bytes_received
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

fn get_file_request_url(data_provider_url: &String) -> String {
	let res = format!("{}/JSON.GET/testddc:dac:data", data_provider_url);

	res
}

pub(crate) fn fetch_file_request(url: &String) -> Requests {
	let response: FileRequestWrapper = http_get_json(&url).unwrap();
	let value: Value = serde_json::from_str(response.json.as_str()).unwrap();
	let map: Requests = serde_json::from_value(value).unwrap();

	map
}

pub(crate) fn fetch_data<T: frame_system::Config>(
	data_provider_url: &String,
	era: EraIndex,
	cdn_node: &T::AccountId,
) -> (BytesSent, BytesReceived) {
	log::info!("[DAC Validator] DAC Validator is running. Current era is {}", era);
	// Todo: handle the error
	let bytes_sent_query = get_bytes_sent_query_url(data_provider_url, era);
	let bytes_sent_res: RedisFtAggregate = http_get_json(&bytes_sent_query).unwrap();
	log::info!("[DAC Validator] Bytes sent sum is fetched: {:?}", bytes_sent_res);
	let bytes_sent = BytesSent::new(bytes_sent_res);

	// Todo: handle the error
	let bytes_received_query = get_bytes_received_query_url(data_provider_url, era);
	let bytes_received_res: RedisFtAggregate = http_get_json(&bytes_received_query).unwrap();
	log::info!("[DAC Validator] Bytes received sum is fetched:: {:?}", bytes_received_res);
	let bytes_received = BytesReceived::new(bytes_received_res);

	(bytes_sent, bytes_received)
}

pub(crate) fn fetch_data1(
	data_provider_url: &String,
	era: EraIndex,
) -> (Vec<BytesSent>, Vec<BytesReceived>) {
	log::info!("[DAC Validator] DAC Validator is running. Current era is {}", era);
	// Todo: handle the error
	let bytes_sent_query = get_bytes_sent_query_url(data_provider_url, era);
	let bytes_sent_res: RedisFtAggregate = http_get_json(&bytes_sent_query).unwrap();
	log::info!("[DAC Validator] Bytes sent sum is fetched: {:?}", bytes_sent_res);
	let bytes_sent = BytesSent::get_all(bytes_sent_res);

	// Todo: handle the error
	let bytes_received_query = get_bytes_received_query_url(data_provider_url, era);
	let bytes_received_res: RedisFtAggregate = http_get_json(&bytes_received_query).unwrap();
	log::info!("[DAC Validator] Bytes received sum is fetched:: {:?}", bytes_received_res);
	let bytes_received = BytesReceived::get_all(bytes_received_res);

	(bytes_sent, bytes_received)
}

pub(crate) fn fetch_data2(
	data_provider_url: &String,
	era: EraIndex,
) -> Result<(String, Vec<BytesSent>, String, Vec<BytesReceived>), ()> {
	let bytes_sent_query = get_bytes_sent_query_url(data_provider_url, era);
	let bytes_sent_res: RedisFtAggregate = http_get_json(&bytes_sent_query).map_err(|_| ())?;
	let bytes_sent = BytesSent::get_all(bytes_sent_res);

	let bytes_received_query = get_bytes_received_query_url(data_provider_url, era);
	let bytes_received_res: RedisFtAggregate =
		http_get_json(&bytes_received_query).map_err(|_| ())?;
	let bytes_received = BytesReceived::get_all(bytes_received_res);

	Ok((bytes_sent_query, bytes_sent, bytes_received_query, bytes_received))
}

fn get_bytes_received_query_url(data_provider_url: &String, era: EraIndex) -> String {
	format!("{}/FT.AGGREGATE/ddc:dac:searchCommonIndex/@era:[{}%20{}]/GROUPBY/2/@nodePublicKey/@era/REDUCE/SUM/1/@bytesReceived/AS/bytesReceivedSum", data_provider_url, era, era)
}

fn http_get_json<OUT: DeserializeOwned>(url: &str) -> crate::ResultStr<OUT> {
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
	// log::info!("[DAC Validator] Sending request to: {:?}", http_url);

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

fn filter_data<T: frame_system::Config>(
	s: &Vec<BytesSent>,
	r: &Vec<BytesReceived>,
	a: &T::AccountId,
) -> (BytesSent, BytesReceived) {
	let ac = utils::account_to_string::<T>(a.clone());

	let filtered_s = &*s.into_iter().find(|bs| bs.node_public_key == ac).unwrap();
	let filtered_r = &*r.into_iter().find(|br| br.node_public_key == ac).unwrap();

	(filtered_s.clone(), filtered_r.clone())
}

fn get_bytes_sent_query_url(data_provider_url: &String, era: EraIndex) -> String {
	format!("{}/FT.AGGREGATE/ddc:dac:searchCommonIndex/@era:[{}%20{}]/GROUPBY/2/@nodePublicKey/@era/REDUCE/SUM/1/@bytesSent/AS/bytesSentSum", data_provider_url, era, era)
}

pub(crate) fn fetch_aggregates(
	data_provider_url: &String,
	era: EraIndex,
) -> Result<JsonValue, http::Error> {
	let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(HTTP_TIMEOUT_MS));
	let url =
		format!("{}/JSON.GET/ddc:dac:aggregation:nodes:{}?type=query", data_provider_url, era);
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

pub(crate) fn make_http_put(url: &String, payload: &String) -> Result<(), http::Error> {
	let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(HTTP_TIMEOUT_MS));

	let request = http::Request::new(url.as_str())
		.method(Method::Put)
		.body(vec![payload.as_bytes()]);

	let pending_req = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
	let response = pending_req.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

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

	Ok(())
}

pub(crate) fn fetch_validators_decisions(
	data_provider_url: &String,
	era: EraIndex,
) -> Result<Wrapper, http::Error> {
	let url = format!("{}/HGET/mock:ddc:dac:decisions_to_eras/{}", data_provider_url, era);
	let wrapper: Wrapper = http_get_json(&url).unwrap();

	Ok(wrapper)
}

pub(crate) fn post_final_decision(
	data_provider_url: &String,
	era: EraIndex,
	decision: FinalDecision,
) -> Result<(), http::Error> {
	let url = format!("{}/HSET/mock:ddc:dac:final_decision_to_era/{}", data_provider_url, era);

	let payload_str: String = serde_json::to_string(&decision).unwrap();
	let res = make_http_put(&url, &payload_str);

	res
}

pub(crate) fn get_final_decision(decisions: Vec<ValidationResult>) -> ValidationDecision {
	let common_decisions = find_largest_group(decisions).unwrap();
	let decision_example = common_decisions.get(0).unwrap();

	let final_decision = ValidationDecision {
		result: decision_example.result,
		payload: utils::get_hashed(&common_decisions),
		totals: DacTotalAggregates {
			received: decision_example.received,
			sent: decision_example.sent,
			failed_by_client: 0,
			failure_rate: 0,
		},
	};

	final_decision
}

pub(crate) fn get_validation_results(
	data_provider_url: &String,
	era: EraIndex,
	edge: &String,
) -> Result<Vec<ValidationResult>, http::Error> {
	let wrapper = fetch_validators_decisions(data_provider_url, 5 as EraIndex).unwrap(); // Era is mocked for now
	let mut edges: EdgesToResults = serde_json::from_str(wrapper.decisions.as_str()).unwrap();
	let results = edges.0.remove(edge).unwrap();

	Ok(results)
}

fn find_largest_group(decisions: Vec<ValidationResult>) -> Option<Vec<ValidationResult>> {
	let mut groups: Vec<Vec<ValidationResult>> = Vec::new();
	let half = decisions.len() / 2;

	for decision in decisions {
		let mut found_group = false;

		for group in &mut groups {
			if group.iter().all(|x| {
				x.result == decision.result &&
					x.received == decision.received &&
					x.sent == decision.sent
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
