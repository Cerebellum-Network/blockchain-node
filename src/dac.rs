//! A module with Data Activity Capture (DAC) interaction.

use alloc::{format, string::String}; // ToDo: remove String usage
use alt_serde::{de::DeserializeOwned, Deserialize, Serialize};
use codec::{Decode, Encode};
use frame_support::log::{error, info, warn};
use serde_json::Value;
use sp_runtime::offchain::{http, Duration};
use sp_staking::EraIndex;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use crate::utils;

pub const HTTP_TIMEOUT_MS: u64 = 30_000;

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
	node_public_key: String,
	era: EraIndex,
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
	bucket_id: i64,
	timestamp: i64,
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
	bytes_received: i64,
	user_timestamp: i64,
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
	log_type: i64,
	session_id: String,
	user_public_key: String,
	era: i64,
	user_address: String,
	bytes_sent: i64,
	timestamp: i64,
	node_public_key: String,
	signature: String,
	bucket_id: i64,
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
	node_public_key: String,
	era: EraIndex,
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
	info!("[DAC Validator] DAC Validator is running. Current era is {}", era);
	// Todo: handle the error
	let bytes_sent_query = get_bytes_sent_query_url(data_provider_url, era);
	let bytes_sent_res: RedisFtAggregate = http_get_json(&bytes_sent_query).unwrap();
	info!("[DAC Validator] Bytes sent sum is fetched: {:?}", bytes_sent_res);
	let bytes_sent = BytesSent::new(bytes_sent_res);

	// Todo: handle the error
	let bytes_received_query = get_bytes_received_query_url(data_provider_url, era);
	let bytes_received_res: RedisFtAggregate = http_get_json(&bytes_received_query).unwrap();
	info!("[DAC Validator] Bytes received sum is fetched:: {:?}", bytes_received_res);
	let bytes_received = BytesReceived::new(bytes_received_res);

	(bytes_sent, bytes_received)
}

pub(crate) fn fetch_data1(
	data_provider_url: &String,
	era: EraIndex,
) -> (Vec<BytesSent>, Vec<BytesReceived>) {
	info!("[DAC Validator] DAC Validator is running. Current era is {}", era);
	// Todo: handle the error
	let bytes_sent_query = get_bytes_sent_query_url(data_provider_url, era);
	let bytes_sent_res: RedisFtAggregate = http_get_json(&bytes_sent_query).unwrap();
	info!("[DAC Validator] Bytes sent sum is fetched: {:?}", bytes_sent_res);
	let bytes_sent = BytesSent::get_all(bytes_sent_res);

	// Todo: handle the error
	let bytes_received_query = get_bytes_received_query_url(data_provider_url, era);
	let bytes_received_res: RedisFtAggregate = http_get_json(&bytes_received_query).unwrap();
	info!("[DAC Validator] Bytes received sum is fetched:: {:?}", bytes_received_res);
	let bytes_received = BytesReceived::get_all(bytes_received_res);

	(bytes_sent, bytes_received)
}

pub(crate) fn fetch_data2(
	data_provider_url: &String,
	era: EraIndex,
) -> (String, Vec<BytesSent>, String, Vec<BytesReceived>) {
	let bytes_sent_query = get_bytes_sent_query_url(data_provider_url, era);
	let bytes_sent_res: RedisFtAggregate = http_get_json(&bytes_sent_query).unwrap();
	let bytes_sent = BytesSent::get_all(bytes_sent_res);

	let bytes_received_query = get_bytes_received_query_url(data_provider_url, era);
	let bytes_received_res: RedisFtAggregate = http_get_json(&bytes_received_query).unwrap();
	let bytes_received = BytesReceived::get_all(bytes_received_res);

	(bytes_sent_query, bytes_sent, bytes_received_query, bytes_received)
}

fn get_bytes_received_query_url(data_provider_url: &String, era: EraIndex) -> String {
	format!("{}/FT.AGGREGATE/ddc:dac:searchCommonIndex/@era:[{}%20{}]/GROUPBY/2/@nodePublicKey/@era/REDUCE/SUM/1/@bytesReceived/AS/bytesReceivedSum", data_provider_url, era, era)
}

fn http_get_json<OUT: DeserializeOwned>(url: &str) -> crate::ResultStr<OUT> {
	let body = http_get_request(url).map_err(|err| {
		error!("[DAC Validator] Error while getting {}: {:?}", url, err);
		"HTTP GET error"
	})?;

	let parsed = serde_json::from_slice(&body).map_err(|err| {
		warn!("[DAC Validator] Error while parsing JSON from {}: {:?}", url, err);
		"HTTP JSON parse error"
	});

	parsed
}

fn http_get_request(http_url: &str) -> Result<Vec<u8>, http::Error> {
	// info!("[DAC Validator] Sending request to: {:?}", http_url);

	// Initiate an external HTTP GET request. This is using high-level wrappers from
	// `sp_runtime`.
	let request = http::Request::get(http_url);

	let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(HTTP_TIMEOUT_MS));

	let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;

	let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

	if response.code != 200 {
		warn!("[DAC Validator] http_get_request unexpected status code: {}", response.code);
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
