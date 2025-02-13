#![allow(dead_code)]

use ddc_primitives::{AggregatorInfo, BucketId, DdcEra};
use prost::Message;
use serde_with::{base64::Base64, serde_as};
use sp_core::crypto::AccountId32;
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

macro_rules! fetch_and_parse {
	(
        // Self reference (the aggregator client)
        $self:expr,
        // URL string variable (mutable)
        $url:expr,
        // The type of the JSON response if not signed
        $unsigned_ty:ty,
        // The type of the JSON response if signed
        $signed_ty:ty
    ) => {{
		if $self.verify_sig {
			if $url.contains('?') {
				$url = format!("{}&sign=true", $url);
			} else {
				$url = format!("{}?sign=true", $url);
			}
		}

		let response = $self.get(&$url, Accept::Any)?;

		let body = response.body().collect::<Vec<u8>>();

		if $self.verify_sig {
			let json_response: json::SignedJsonResponse<$signed_ty> =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;

			if !json_response.verify() {
				log::debug!("Bad signature, req: {:?}, resp: {:?}", $url, json_response);
				return Err(http::Error::Unknown);
			}

			Ok(json_response.payload)
		} else {
			let json_response: $unsigned_ty =
				serde_json::from_slice(&body).map_err(|_| http::Error::Unknown)?;
			Ok(json_response)
		}
	}};
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

		// Now let the macro do the rest
		fetch_and_parse!(
			self,
			url,
			Vec<json::BucketAggregateResponse>,
			Vec<json::BucketAggregateResponse>
		)
	}

	pub fn nodes_aggregates(
		&self,
		era_id: DdcEra,
		limit: Option<u32>,
		prev_token: Option<String>,
	) -> Result<Vec<json::NodeAggregateResponse>, http::Error> {
		let mut url = format!("{}/activity/nodes?eraId={}", self.base_url, era_id);

		if let Some(limit) = limit {
			url = format!("{}&limit={}", url, limit);
		}
		if let Some(prev_token) = prev_token {
			url = format!("{}&prevToken={}", url, prev_token);
		}

		fetch_and_parse!(
			self,
			url,
			Vec<json::NodeAggregateResponse>,
			Vec<json::NodeAggregateResponse>
		)
	}

	pub fn challenge_bucket_sub_aggregate(
		&self,
		era_id: DdcEra,
		bucket_id: BucketId,
		node_id: &str,
		merkle_tree_node_id: Vec<u64>,
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
		merkle_tree_node_id: Vec<u64>,
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
		fetch_and_parse!(
			self,
			url,
			Vec<json::AggregationEraResponse>,
			Vec<json::AggregationEraResponse>
		)
	}

	pub fn payment_eras(&self) -> Result<Vec<json::EHDEra>, http::Error> {
		let mut url = format!("{}/activity/payment-eras", self.base_url);
		fetch_and_parse!(self, url, Vec<json::EHDEra>, Vec<json::EHDEra>)
	}

	pub fn era_historical_document(
		&self,
		era_id: DdcEra,
	) -> Result<json::EHDResponse, http::Error> {
		let mut url = format!("{}/activity/ehd/{}", self.base_url, era_id);
		fetch_and_parse!(self, url, json::EHDResponse, json::EHDResponse)
	}

	pub fn partial_historical_document(
		&self,
		phd_id: String,
	) -> Result<json::PHDResponse, http::Error> {
		let mut url = format!("{}/activity/phd/{}", self.base_url, phd_id);
		fetch_and_parse!(self, url, json::PHDResponse, json::PHDResponse)
	}

	pub fn traverse_era_historical_document(
		&self,
		ehd_id: EHDId,
		tree_node_id: u32,
		tree_levels_count: u32,
	) -> Result<Vec<json::TraversedEHDResponse>, http::Error> {
		let mut url = format!(
			"{}/activity/ehds/{}/traverse?merkleTreeNodeId={}&levels={}",
			self.base_url,
			<EHDId as Into<String>>::into(ehd_id),
			tree_node_id,
			tree_levels_count
		);
		fetch_and_parse!(
			self,
			url,
			Vec<json::TraversedEHDResponse>,
			Vec<json::TraversedEHDResponse>
		)
	}

	pub fn traverse_partial_historical_document(
		&self,
		phd_id: PHDId,
		tree_node_id: u32,
		tree_levels_count: u32,
	) -> Result<Vec<json::TraversedPHDResponse>, http::Error> {
		let mut url = format!(
			"{}/activity/phds/{}/traverse?merkleTreeNodeId={}&levels={}",
			self.base_url,
			<PHDId as Into<String>>::into(phd_id),
			tree_node_id,
			tree_levels_count
		);
		fetch_and_parse!(
			self,
			url,
			Vec<json::TraversedPHDResponse>,
			Vec<json::TraversedPHDResponse>
		)
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
		fetch_and_parse!(self, url, json::MerkleTreeNodeResponse, json::MerkleTreeNodeResponse)
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
		fetch_and_parse!(self, url, json::MerkleTreeNodeResponse, json::MerkleTreeNodeResponse)
	}

	fn merkle_tree_node_id_param(merkle_tree_node_id: &[u64]) -> String {
		merkle_tree_node_id
			.iter()
			.map(|x| format!("{}", x.clone()))
			.collect::<Vec<_>>()
			.join(",")
	}

	pub fn send_inspection_receipt(
		&self,
		receipt: json::InspectionReceipt,
	) -> Result<http::Response, http::Error> {
		let url = format!("{}/activity/inspection-receipts", self.base_url);
		let body = serde_json::to_vec(&receipt).expect("Inspection receipt to be encoded");
		self.post(&url, body)
	}

	pub fn fetch_grouped_inspection_receipts(
		&self,
		ehd_id: EHDId,
	) -> Result<BTreeMap<String, json::GroupedInspectionReceipt>, http::Error> {
		let ehd_param: String = ehd_id.into();
		let mut url = format!("{}/activity/inspection-receipts?ehdId={}", self.base_url, ehd_param);

		fetch_and_parse!(self, url, BTreeMap<String, json::GroupedInspectionReceipt>, BTreeMap<String, json::GroupedInspectionReceipt>)
	}

	pub fn check_grouping_collector(&self) -> Result<json::IsGCollectorResponse, http::Error> {
		let mut url = format!("{}/activity/is-grouping-collector", self.base_url);
		fetch_and_parse!(self, url, json::IsGCollectorResponse, json::IsGCollectorResponse)
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

	fn post(&self, url: &str, request_body: Vec<u8>) -> Result<http::Response, http::Error> {
		let mut maybe_response = None;

		let deadline = timestamp().add(self.timeout);
		let mut error = None;

		for _ in 0..self.retries {
			let request = http::Request::post(url, vec![request_body.clone()])
				.add_header("content-type", "application/json");

			let pending = request
				.deadline(deadline)
				.body(vec![request_body.clone()])
				.send()
				.map_err(|_| http::Error::IoError)?;

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

		if response.code != 201 {
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

	#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Encode, Decode)]
	pub struct EHDResponse {
		#[serde(rename = "ehdId")]
		pub ehd_id: String,
		#[serde(rename = "clusterId")]
		pub cluster_id: String,
		#[serde(rename = "eraId")]
		pub era_id: DdcEra,
		#[serde(rename = "gCollector")]
		pub g_collector: String,
		#[serde(rename = "pdhIds")]
		pub pdh_ids: Vec<String>,
		pub hash: String,
		pub status: String,
		pub customers: Vec<EHDCustomer>,
		pub providers: Vec<EHDProvider>,
	}

	#[derive(
		Default,
		Debug,
		Serialize,
		Deserialize,
		Clone,
		Hash,
		Ord,
		PartialOrd,
		PartialEq,
		Eq,
		Encode,
		Decode,
	)]
	pub struct EHDUsage {
		#[serde(rename = "storedBytes")]
		pub stored_bytes: i64,
		#[serde(rename = "transferredBytes")]
		pub transferred_bytes: u64,
		#[serde(rename = "puts")]
		pub number_of_puts: u64,
		#[serde(rename = "gets")]
		pub number_of_gets: u64,
	}

	#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Encode, Decode)]
	pub struct EHDUsagePercent {
		#[serde(rename = "storedBytesPercent")]
		pub stored_bytes: f64,
		#[serde(rename = "transferredBytesPercent")]
		pub transferred_bytes: f64,
		#[serde(rename = "putsPercent")]
		pub number_of_puts: f64,
		#[serde(rename = "getsPercent")]
		pub number_of_gets: f64,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct EHDCustomer {
		#[serde(rename = "customerId")]
		pub customer_id: String,
		#[serde(rename = "consumedUsage")]
		pub consumed_usage: EHDUsage,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct EHDProvider {
		#[serde(rename = "providerId")]
		pub provider_id: String,
		#[serde(rename = "providedUsage")]
		pub provided_usage: EHDUsage,
		pub nodes: Vec<EHDNodeResources>,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct EHDNodeResources {
		#[serde(rename = "nodeId")]
		pub node_key: String,
		#[serde(rename = "storedBytes")]
		pub stored_bytes: i64,
		#[serde(rename = "transferredBytes")]
		pub transferred_bytes: u64,
		#[serde(rename = "puts")]
		pub number_of_puts: u64,
		#[serde(rename = "gets")]
		pub number_of_gets: u64,
		#[serde(rename = "avgLatency")]
		pub avg_latency: u64,
		#[serde(rename = "avgBandwidth")]
		pub avg_bandwidth: u64,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct PHDNodeAggregate {
		#[serde(rename = "nodeId")]
		pub node_key: String,
		#[serde(rename = "storedBytes")]
		pub stored_bytes: i64,
		#[serde(rename = "transferredBytes")]
		pub transferred_bytes: u64,
		#[serde(rename = "numberOfPuts")]
		pub number_of_puts: u64,
		#[serde(rename = "numberOfGets")]
		pub number_of_gets: u64,
		pub metrics: PHDNodeMetrics,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct PHDNodeMetrics {
		latency: LatencyMetrics,
		availability: AvailabilityMetrics,
		bandwidth: BandwidthMetrics,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct LatencyMetrics {
		#[serde(rename = "p50")]
		p_50: u64,
		#[serde(rename = "p95")]
		p_95: u64,
		#[serde(rename = "p99")]
		p_99: u64,
		#[serde(rename = "p999")]
		p_999: u64,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct AvailabilityMetrics {
		#[serde(rename = "totalRequests")]
		total_requests: u64,
		#[serde(rename = "failedRequests")]
		failed_requests: u64,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct BandwidthMetrics {
		#[serde(rename = "100kbps")]
		kbps_100: u64,
		#[serde(rename = "1Mbps")]
		mbps_1: u64,
		#[serde(rename = "10Mbps")]
		mbps_10: u64,
		#[serde(rename = "100Mbps")]
		mbps_100: u64,
		inf: u64,
	}

	#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Encode, Decode)]
	pub struct PHDResponse {
		#[serde(rename = "phdId")]
		pub phd_id: String,
		#[serde(rename = "merkleTreeNodeId")]
		pub tree_node_id: u32,
		#[serde(rename = "NodesAggregates")]
		pub nodes_aggregates: Vec<PHDNodeAggregate>,
		#[serde(rename = "BucketsAggregates")]
		pub buckets_aggregates: Vec<PHDBucketAggregate>,
		pub hash: String,
		pub collector: String,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct PHDBucketAggregate {
		#[serde(rename = "bucketId")]
		pub bucket_id: BucketId,
		#[serde(rename = "storedBytes")]
		pub stored_bytes: i64,
		#[serde(rename = "transferredBytes")]
		pub transferred_bytes: u64,
		#[serde(rename = "numberOfPuts")]
		pub number_of_puts: u64,
		#[serde(rename = "numberOfGets")]
		pub number_of_gets: u64,
		#[serde(rename = "subAggregates")]
		pub sub_aggregates: Vec<PHDBucketSubAggregate>,

		// todo: remove from top-level bucket aggregation
		#[serde(rename = "nodeId")]
		pub node_key: String,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct PHDBucketSubAggregate {
		#[serde(rename = "bucketId")]
		pub bucket_id: BucketId,
		#[serde(rename = "nodeId")]
		pub node_key: String,
		#[serde(rename = "storedBytes")]
		pub stored_bytes: i64,
		#[serde(rename = "transferredBytes")]
		pub transferred_bytes: u64,
		#[serde(rename = "numberOfPuts")]
		pub number_of_puts: u64,
		#[serde(rename = "numberOfGets")]
		pub number_of_gets: u64,
	}

	#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Encode, Decode)]
	pub struct TraversedEHDResponse {
		#[serde(rename = "ehdId")]
		pub ehd_id: String,
		#[serde(rename = "groupingCollector")]
		pub g_collector: String,
		#[serde(rename = "merkleTreeNodeId")]
		pub tree_node_id: u32,
		#[serde(rename = "merkleTreeNodeHash")]
		pub tree_node_hash: String,
		#[serde(rename = "pdhIds")]
		pub pdh_ids: Vec<String>,
		pub status: String,
		pub customers: Vec<TraversedEHDCustomer>,
		pub providers: Vec<TraversedEHDProvider>,
	}

	impl TraversedEHDResponse {
		pub fn get_cluster_usage(&self) -> EHDUsage {
			self.providers.iter().fold(
				EHDUsage {
					stored_bytes: 0,
					transferred_bytes: 0,
					number_of_puts: 0,
					number_of_gets: 0,
				},
				|mut acc, provider| {
					acc.stored_bytes += provider.provided_usage.stored_bytes;
					acc.transferred_bytes += provider.provided_usage.transferred_bytes;
					acc.number_of_puts += provider.provided_usage.number_of_puts;
					acc.number_of_gets += provider.provided_usage.number_of_gets;
					acc
				},
			)
		}
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct TraversedEHDCustomer {
		#[serde(rename = "customerId")]
		pub customer_id: String,
		#[serde(rename = "consumedUsage")]
		pub consumed_usage: EHDUsage,
	}

	impl TraversedEHDCustomer {
		pub fn parse_customer_id(&self) -> Result<AccountId32, ()> {
			if !self.customer_id.starts_with("0x") || self.customer_id.len() != 66 {
				return Err(());
			}

			let hex_str = &self.customer_id[2..]; // skip '0x'
			let hex_bytes = match hex::decode(hex_str) {
				Ok(bytes) => bytes,
				Err(_) => return Err(()),
			};
			if hex_bytes.len() != 32 {
				return Err(());
			}
			let mut pub_key = [0u8; 32];
			pub_key.copy_from_slice(&hex_bytes[..32]);

			Ok(AccountId32::from(pub_key))
		}
	}

	#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Encode, Decode)]
	pub struct TraversedEHDProvider {
		#[serde(rename = "providerId")]
		pub provider_id: String,
		#[serde(rename = "providedUsage")]
		pub provided_usage: EHDUsage,
		pub nodes: Vec<TraversedEHDNodeResources>,
	}

	impl TraversedEHDProvider {
		pub fn parse_provider_id(&self) -> Result<AccountId32, ()> {
			if !self.provider_id.starts_with("0x") || self.provider_id.len() != 66 {
				return Err(());
			}

			let hex_str = &self.provider_id[2..]; // skip '0x'
			let hex_bytes = match hex::decode(hex_str) {
				Ok(bytes) => bytes,
				Err(_) => return Err(()),
			};
			if hex_bytes.len() != 32 {
				return Err(());
			}
			let mut pub_key = [0u8; 32];
			pub_key.copy_from_slice(&hex_bytes[..32]);

			Ok(AccountId32::from(pub_key))
		}
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct TraversedEHDNodeResources {
		#[serde(rename = "nodeId")]
		pub node_key: String,
		#[serde(rename = "storedBytes")]
		pub stored_bytes: i64,
		#[serde(rename = "transferredBytes")]
		pub transferred_bytes: u64,
		#[serde(rename = "puts")]
		pub number_of_puts: u64,
		#[serde(rename = "gets")]
		pub number_of_gets: u64,
	}

	#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Encode, Decode)]
	pub struct TraversedPHDResponse {
		#[serde(rename = "phdId")]
		pub phd_id: String,
		#[serde(rename = "collectorId")]
		pub collector: String,
		#[serde(rename = "merkleTreeNodeId")]
		pub tree_node_id: u32,
		#[serde(rename = "merkleTreeNodeHash")]
		pub tree_node_hash: String,
		#[serde(rename = "nodesAggregates")]
		pub nodes_aggregates: Vec<TraversedPHDNodeAggregate>,
		#[serde(rename = "bucketsAggregates")]
		pub buckets_aggregates: Vec<TraversedPHDBucketAggregate>,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct TraversedPHDNodeAggregate {
		#[serde(rename = "nodeId")]
		pub node_key: String,
		#[serde(rename = "storedBytes")]
		pub stored_bytes: i64,
		#[serde(rename = "transferredBytes")]
		pub transferred_bytes: u64,
		#[serde(rename = "numberOfPuts")]
		pub number_of_puts: u64,
		#[serde(rename = "numberOfGets")]
		pub number_of_gets: u64,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct TraversedPHDBucketAggregate {
		#[serde(rename = "bucketId")]
		pub bucket_id: BucketId,
		#[serde(rename = "storedBytes")]
		pub stored_bytes: i64,
		#[serde(rename = "transferredBytes")]
		pub transferred_bytes: u64,
		#[serde(rename = "numberOfPuts")]
		pub number_of_puts: u64,
		#[serde(rename = "numberOfGets")]
		pub number_of_gets: u64,
	}

	#[derive(Debug, Serialize, Deserialize, Clone, Hash, Encode, Decode)]
	pub struct InspectionReceipt {
		#[serde(rename = "ehdId")]
		pub ehd_id: String,
		pub inspector: String,
		pub signature: String,
		#[serde(rename = "nodesInspection")]
		pub nodes_inspection: InspectionResult,
		#[serde(rename = "bucketsInspection")]
		pub buckets_inspection: InspectionResult,
	}

	#[derive(Debug, Serialize, Deserialize, Clone, Hash, Encode, Decode)]
	pub struct InspectionResult {
		#[serde(rename = "unverifiedBranches")]
		pub unverified_branches: Vec<InspectedTreePart>,
		#[serde(rename = "verifiedBranches")]
		pub verified_branches: Vec<InspectedTreePart>,
	}

	#[derive(Debug, Serialize, Deserialize, Clone, Hash, Encode, Decode)]
	pub struct InspectedTreePart {
		pub collector: String,
		pub aggregate: AggregateKey,
		pub nodes: Vec<u64>,
		pub leafs: BTreeMap<u32, Vec<u64>>,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Encode, Decode, Ord, PartialOrd, PartialEq, Eq,
	)]
	pub struct EHDEra {
		pub id: u32,
		pub status: String,
		pub era_start: Option<DdcEra>,
		pub era_end: Option<DdcEra>,
		pub time_start: Option<i64>,
		pub time_end: Option<i64>,
	}

	#[derive(Debug, Serialize, Deserialize, Clone, Hash, Encode, Decode)]
	pub struct GroupedInspectionReceipt {
		#[serde(rename = "ehdId")]
		pub ehd_id: String,
		pub inspector: String,
		pub signature: String,
		#[serde(rename = "nodesInspection")]
		pub nodes_inspection: InspectionResult, /* todo(yahortsaryk): remove optional
		                                         * after fix in DDC api */
		#[serde(rename = "bucketsInspection")]
		pub buckets_inspection: InspectionResult, /* todo(yahortsaryk): remove optional
		                                           * after fix in DDC api */
	}

	#[derive(Debug, Serialize, Deserialize, Clone, Hash, Encode, Decode)]
	pub struct IsGCollectorResponse {
		#[serde(rename = "isGroupingCollector")]
		pub is_g_collector: bool,
	}
}
