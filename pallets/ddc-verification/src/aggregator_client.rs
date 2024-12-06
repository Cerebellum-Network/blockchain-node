#![allow(dead_code)]

use ddc_primitives::{AggregatorInfo, BucketId, DdcEra};
use prost::Message;
use serde_with::{base64::Base64, serde_as};
use sp_io::offchain::timestamp;
use sp_runtime::offchain::{http, Duration};

use super::*;
use crate::signature::Verify;

pub struct AggregatorClient<'a> {
	pub base_url: &'a str,
	timeout: Duration,
	retries: u32,
	verify_sig: bool,
}

impl<'a> AggregatorClient<'a> {
	pub fn new(base_url: &'a str, timeout: Duration, retries: u32, verify_sig: bool) -> Self {
		Self { base_url, timeout, retries, verify_sig }
	}

	pub fn buckets_aggregates(
		&self,
		era_id: DdcEra,
		limit: Option<u32>,
		prev_token: Option<BucketId>,
	) -> Result<Vec<json::BucketAggregateResponse>, http::Error> {
		let mut url = format!("{}/activity/buckets?eraId={}", self.base_url, era_id);
		if let Some(limit) = limit {
			url = format!("{}&limit={}", url, limit);
		}
		if let Some(prev_token) = prev_token {
			url = format!("{}&prevToken={}", url, prev_token);
		}
		if self.verify_sig {
			url = format!("{}&sign=true", url);
		}
		let response = self.get(&url, Accept::Any)?;

		let body = response.body().collect::<Vec<u8>>();
		if self.verify_sig {
			let json_response: json::SignedJsonResponse<Vec<json::BucketAggregateResponse>> =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;

			if !json_response.verify() {
				log::debug!("bad signature, req: {:?}, resp: {:?}", url, json_response);
				return Err(http::Error::Unknown); // TODO (khssnv): more specific error.
			}

			Ok(json_response.payload)
		} else {
			let json_response: Vec<json::BucketAggregateResponse> =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;

			Ok(json_response)
		}
	}

	pub fn nodes_aggregates(
		&self,
		era_id: DdcEra,
		limit: Option<u32>,
		prev_token: Option<String>, // node_id hex string
	) -> Result<Vec<json::NodeAggregateResponse>, http::Error> {
		let mut url = format!("{}/activity/nodes?eraId={}", self.base_url, era_id);
		if let Some(limit) = limit {
			url = format!("{}&limit={}", url, limit);
		}
		if let Some(prev_token) = prev_token {
			url = format!("{}&prevToken={}", url, prev_token);
		}
		if self.verify_sig {
			url = format!("{}&sign=true", url);
		}
		let response = self.get(&url, Accept::Any)?;

		let body = response.body().collect::<Vec<u8>>();
		if self.verify_sig {
			let json_response: json::SignedJsonResponse<Vec<json::NodeAggregateResponse>> =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;

			if !json_response.verify() {
				log::debug!("bad signature, req: {:?}, resp: {:?}", url, json_response);
				return Err(http::Error::Unknown); // TODO (khssnv): more specific error.
			}

			Ok(json_response.payload)
		} else {
			let json_response: Vec<json::NodeAggregateResponse> =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;

			Ok(json_response)
		}
	}

	pub fn challenge_bucket_sub_aggregate(
		&self,
		era_id: DdcEra,
		bucket_id: BucketId,
		node_id: &str,
		merkle_tree_node_id: Vec<u32>,
	) -> Result<proto::ChallengeResponse, http::Error> {
		let url = format!(
			"{}/activity/buckets/{}/challenge?eraId={}&nodeId={}&merkleTreeNodeId={}",
			self.base_url,
			bucket_id,
			era_id,
			node_id,
			Self::merkle_tree_node_id_param(merkle_tree_node_id.as_slice()),
		);
		let response = self.get(&url, Accept::Protobuf)?;
		let body = response.body().collect::<Vec<u8>>();
		let proto_response =
			proto::ChallengeResponse::decode(body.as_slice()).map_err(|_| http::Error::Unknown)?;

		Ok(proto_response)
	}

	pub fn challenge_node_aggregate(
		&self,
		era_id: DdcEra,
		node_id: &str,
		merkle_tree_node_id: Vec<u32>,
	) -> Result<proto::ChallengeResponse, http::Error> {
		let url = format!(
			"{}/activity/nodes/{}/challenge?eraId={}&merkleTreeNodeId={}",
			self.base_url,
			node_id,
			era_id,
			Self::merkle_tree_node_id_param(merkle_tree_node_id.as_slice()),
		);
		let response = self.get(&url, Accept::Protobuf)?;
		let body = response.body().collect::<Vec<u8>>();
		let proto_response =
			proto::ChallengeResponse::decode(body.as_slice()).map_err(|_| http::Error::Unknown)?;

		Ok(proto_response)
	}

	pub fn eras(&self) -> Result<Vec<json::AggregationEraResponse>, http::Error> {
		let mut url = format!("{}/activity/eras", self.base_url);
		if self.verify_sig {
			url = format!("{}&sign=true", url);
		}
		let response = self.get(&url, Accept::Any)?;

		let body = response.body().collect::<Vec<u8>>();
		if self.verify_sig {
			let json_response: json::SignedJsonResponse<Vec<json::AggregationEraResponse>> =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;

			if !json_response.verify() {
				log::debug!("bad signature, req: {:?}, resp: {:?}", url, json_response);
				return Err(http::Error::Unknown); // TODO (khssnv): more specific error.
			}

			Ok(json_response.payload)
		} else {
			let json_response: Vec<json::AggregationEraResponse> =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;

			Ok(json_response)
		}
	}

	pub fn traverse_bucket_sub_aggregate(
		&self,
		era_id: DdcEra,
		bucket_id: BucketId,
		node_id: &str,
		merkle_tree_node_id: u32,
		levels: u16,
	) -> Result<json::MerkleTreeNodeResponse, http::Error> {
		let mut url = format!(
			"{}/activity/buckets/{}/traverse?eraId={}&nodeId={}&merkleTreeNodeId={}&levels={}",
			self.base_url, bucket_id, era_id, node_id, merkle_tree_node_id, levels,
		);
		if self.verify_sig {
			url = format!("{}&sign=true", url);
		}

		let response = self.get(&url, Accept::Any)?;

		let body = response.body().collect::<Vec<u8>>();
		if self.verify_sig {
			let json_response: json::SignedJsonResponse<json::MerkleTreeNodeResponse> =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;

			if !json_response.verify() {
				log::debug!("bad signature, req: {:?}, resp: {:?}", url, json_response);
				return Err(http::Error::Unknown); // TODO (khssnv): more specific error.
			}

			Ok(json_response.payload)
		} else {
			let json_response: json::MerkleTreeNodeResponse =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;

			Ok(json_response)
		}
	}

	pub fn traverse_node_aggregate(
		&self,
		era_id: DdcEra,
		node_id: &str,
		merkle_tree_node_id: u32,
		levels: u16,
	) -> Result<json::MerkleTreeNodeResponse, http::Error> {
		let mut url = format!(
			"{}/activity/nodes/{}/traverse?eraId={}&merkleTreeNodeId={}&levels={}",
			self.base_url, node_id, era_id, merkle_tree_node_id, levels,
		);
		if self.verify_sig {
			url = format!("{}&sign=true", url);
		}
		let response = self.get(&url, Accept::Any)?;

		let body = response.body().collect::<Vec<u8>>();
		if self.verify_sig {
			let json_response: json::SignedJsonResponse<json::MerkleTreeNodeResponse> =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;

			if !json_response.verify() {
				log::debug!("bad signature, req: {:?}, resp: {:?}", url, json_response);
				return Err(http::Error::Unknown); // TODO (khssnv): more specific error.
			}

			Ok(json_response.payload)
		} else {
			let json_response: json::MerkleTreeNodeResponse =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;

			Ok(json_response)
		}
	}

	fn merkle_tree_node_id_param(merkle_tree_node_id: &[u32]) -> String {
		merkle_tree_node_id
			.iter()
			.map(|x| format!("{}", x.clone()))
			.collect::<Vec<_>>()
			.join(",")
	}

	fn get(&self, url: &str, accept: Accept) -> Result<http::Response, http::Error> {
		let mut maybe_response = None;

		let deadline = timestamp().add(self.timeout);
		let mut error = None;

		for _ in 0..self.retries {
			let mut request = http::Request::get(url).deadline(deadline);
			request = match accept {
				Accept::Any => request,
				Accept::Protobuf => request.add_header("Accept", "application/protobuf"),
			};

			let pending = match request.send() {
				Ok(p) => p,
				Err(_) => {
					error = Some(http::Error::IoError);
					continue;
				},
			};

			match pending.try_wait(deadline) {
				Ok(Ok(r)) => {
					maybe_response = Some(r);
					error = None;
					break;
				},
				Ok(Err(_)) | Err(_) => {
					error = Some(http::Error::DeadlineReached);
					continue;
				},
			}
		}

		if let Some(e) = error {
			return Err(e);
		}

		let response = match maybe_response {
			Some(r) => r,
			None => return Err(http::Error::Unknown),
		};

		if response.code != 200 {
			return Err(http::Error::Unknown);
		}

		Ok(response)
	}
}

enum Accept {
	Any,
	Protobuf,
}

pub(crate) mod json {
	use super::*;

	/// Node aggregate response from aggregator.
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct NodeAggregateResponse {
		/// Node id.
		pub node_id: String,
		/// Total amount of stored bytes.
		pub stored_bytes: i64,
		/// Total amount of transferred bytes.
		pub transferred_bytes: u64,
		/// Total number of puts.
		pub number_of_puts: u64,
		/// Total number of gets.
		pub number_of_gets: u64,
	}

	/// DDC aggregation era
	#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Encode, Decode)]
	pub struct AggregationEraResponse {
		pub id: DdcEra,
		pub status: String,
		pub start: i64,
		pub end: i64,
		pub processing_time: i64,
		pub nodes_total: u32,
		pub nodes_processed: u32,
		pub records_processed: u32,
		pub records_applied: u32,
		pub records_discarded: u32,
		pub attempt: u32,
	}

	/// Bucket aggregate response from aggregator.
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct BucketAggregateResponse {
		/// Bucket id
		pub bucket_id: BucketId,
		/// Total amount of stored bytes.
		pub stored_bytes: i64,
		/// Total amount of transferred bytes.
		pub transferred_bytes: u64,
		/// Total number of puts.
		pub number_of_puts: u64,
		/// Total number of gets.
		pub number_of_gets: u64,
		/// Bucket sub aggregates.
		pub sub_aggregates: Vec<BucketSubAggregateResponse>,
	}

	/// Sub aggregates of a bucket.
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	#[allow(non_snake_case)]
	pub struct BucketSubAggregateResponse {
		/// Node id.
		pub NodeID: String,
		/// Total amount of stored bytes.
		pub stored_bytes: i64,
		/// Total amount of transferred bytes.
		pub transferred_bytes: u64,
		/// Total number of puts.
		pub number_of_puts: u64,
		/// Total number of gets.
		pub number_of_gets: u64,
	}

	/// Bucket activity per a DDC node.
	#[derive(
		Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct BucketSubAggregate {
		/// Bucket id
		pub bucket_id: BucketId,
		/// Node id.
		pub node_id: String,
		/// Total amount of stored bytes.
		pub stored_bytes: i64,
		/// Total amount of transferred bytes.
		pub transferred_bytes: u64,
		/// Total number of puts.
		pub number_of_puts: u64,
		/// Total number of gets.
		pub number_of_gets: u64,
		/// Aggregator data.
		pub aggregator: AggregatorInfo,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct NodeAggregate {
		/// Node id.
		pub node_id: String,
		/// Total amount of stored bytes.
		pub stored_bytes: i64,
		/// Total amount of transferred bytes.
		pub transferred_bytes: u64,
		/// Total number of puts.
		pub number_of_puts: u64,
		/// Total number of gets.
		pub number_of_gets: u64,
		/// Node data.
		pub aggregator: AggregatorInfo,
	}

	/// Challenge Response
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct ChallengeAggregateResponse {
		/// proofs
		pub proofs: Vec<Proof>, //todo! add optional fields
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct Proof {
		pub merkle_tree_node_id: u32,
		pub usage: Usage,
		pub path: Vec<String>, //todo! add base64 deserialization
		pub leafs: Vec<Leaf>,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct Usage {
		/// Total amount of stored bytes.
		pub stored_bytes: i64,
		/// Total amount of transferred bytes.
		pub transferred_bytes: u64,
		/// Total number of puts.
		pub number_of_puts: u64,
		/// Total number of gets.
		pub number_of_gets: u64,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct Leaf {
		pub record: Record,
		pub transferred_bytes: u64,
		pub stored_bytes: i64,
		// todo! add links if there is no record
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	#[allow(non_snake_case)]
	pub struct Record {
		pub id: String,
		pub upstream: Upstream,
		pub downstream: Vec<Downstream>,
		pub timestamp: String,
		pub signature: Signature,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct Upstream {
		pub request: Request,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct Downstream {
		pub request: Request,
	}
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	#[allow(non_snake_case)]
	pub struct Request {
		pub requestId: String,
		pub requestType: String,
		pub contentType: String,
		pub bucketId: String,
		pub pieceCid: String,
		pub offset: String,
		pub size: String,
		pub timestamp: String,
		pub signature: Signature,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct Signature {
		pub algorithm: String,
		pub signer: String,
		pub value: String,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct MerkleTreeNodeResponse {
		pub merkle_tree_node_id: u32,
		pub hash: String,
		pub stored_bytes: i64,
		pub transferred_bytes: u64,
		pub number_of_puts: u64,
		pub number_of_gets: u64,
	}

	/// Json response wrapped with a signature.
	#[serde_as]
	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct SignedJsonResponse<T> {
		pub payload: T,
		#[serde_as(as = "Base64")]
		pub signer: Vec<u8>,
		#[serde_as(as = "Base64")]
		pub signature: Vec<u8>,
	}
}
