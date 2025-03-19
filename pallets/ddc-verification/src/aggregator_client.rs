#![allow(dead_code)]
#![allow(clippy::from_over_into)]

use ddc_primitives::{AccountId32Hex, AggregatorInfo, BucketId, TcaEra};
use insp_task_manager::{InspAssignmentsTable, InspEraReport, InspPathException};
use prost::Message;
use scale_info::prelude::{collections::BTreeMap, string::String, vec::Vec};
use serde_with::{base64::Base64, serde_as, TryFromInto};
use sp_io::offchain::timestamp;
use sp_runtime::offchain::{http, Duration};

use super::*;
use crate::{signature::Verify, Config};

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
		era_id: TcaEra,
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
		era_id: TcaEra,
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
		era_id: TcaEra,
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
		era_id: TcaEra,
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

	pub fn traverse_era_historical_document(
		&self,
		ehd_id: EHDId,
		tree_node_id: u32,
		tree_levels_count: u32,
	) -> Result<Vec<json::EHDTreeNode>, http::Error> {
		let mut url = format!(
			"{}/activity/ehds/{}/traverse?merkleTreeNodeId={}&levels={}",
			self.base_url,
			<EHDId as Into<String>>::into(ehd_id),
			tree_node_id,
			tree_levels_count
		);
		fetch_and_parse!(self, url, Vec<json::EHDTreeNode>, Vec<json::EHDTreeNode>)
	}

	pub fn traverse_partial_historical_document(
		&self,
		phd_id: PHDId,
		tree_node_id: u32,
		tree_levels_count: u32,
	) -> Result<Vec<json::PHDTreeNode>, http::Error> {
		let mut url = format!(
			"{}/activity/phds/{}/traverse?merkleTreeNodeId={}&levels={}",
			self.base_url,
			<PHDId as Into<String>>::into(phd_id),
			tree_node_id,
			tree_levels_count
		);
		fetch_and_parse!(self, url, Vec<json::PHDTreeNode>, Vec<json::PHDTreeNode>)
	}

	pub fn traverse_bucket_sub_aggregate(
		&self,
		era_id: TcaEra,
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
		era_id: TcaEra,
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

	pub fn get_inspection_report(
		&self,
		era: EhdEra,
	) -> Result<proto::EndpointItmGetPath, http::Error> {
		let url = format!("{}/itm/path?eraId={}", self.base_url, era);
		let response = self.get(&url, Accept::Protobuf)?;
		let body = response.body().collect::<Vec<u8>>();
		let proto_response = proto::EndpointItmGetPath::decode(body.as_slice()).map_err(|e| {
			log::info!("Decode ITM Path Report protobuf error: {:?}", e);
			http::Error::Unknown
		})?;

		Ok(proto_response)
	}

	pub fn submit_inspection_report(
		&self,
		report: InspEraReport,
	) -> Result<proto::EndpointItmPostPath, http::Error> {
		let url = format!("{}/itm/path", self.base_url);
		let body = serde_json::to_string(&report).expect("Assignments table to be encoded");

		let response = self.post(&url, body.into(), Accept::Protobuf)?;
		let body = response.body().collect::<Vec<u8>>();

		let proto_response = proto::EndpointItmPostPath::decode(body.as_slice())
			.map_err(|_| http::Error::Unknown)?;
		Ok(proto_response)
	}

	pub fn fetch_inspection_exceptions(
		&self,
		era: EhdEra,
	) -> Result<BTreeMap<String, BTreeMap<String, InspPathException>>, http::Error> {
		let mut url = format!("{}/itm/exception?eraId={}", self.base_url, era);

		fetch_and_parse!(self, url, BTreeMap<String, BTreeMap<String, InspPathException>>, BTreeMap<String, BTreeMap<String, InspPathException>>)
	}

	pub fn check_grouping_collector(&self) -> Result<json::IsGCollectorResponse, http::Error> {
		let mut url = format!("{}/activity/is-grouping-collector", self.base_url);
		fetch_and_parse!(self, url, json::IsGCollectorResponse, json::IsGCollectorResponse)
	}

	pub fn submit_assignments_table<T: Config>(
		&self,
		table: InspAssignmentsTable<T::AccountId>,
		inspector_hex: String,
	) -> Result<proto::EndpointItmSubmit, http::Error> {
		let url = format!(
			"{}/itm/submit?eraId={}&inspectorKey={}",
			self.base_url, table.era, inspector_hex
		);
		let body = serde_json::to_string(&table).expect("Assignments table to be encoded");

		let response = self.post(&url, body.into(), Accept::Protobuf)?;
		let body = response.body().collect::<Vec<u8>>();

		let proto_response =
			proto::EndpointItmSubmit::decode(body.as_slice()).map_err(|_| http::Error::Unknown)?;
		Ok(proto_response)
	}

	pub fn get_assignments_table<T: Config>(
		&self,
		era: EhdEra,
	) -> Result<proto::EndpointItmTable, http::Error> {
		let url = format!("{}/itm/table?eraId={}", self.base_url, era);
		let response = self.get(&url, Accept::Protobuf)?;
		let body = response.body().collect::<Vec<u8>>();

		let proto_response =
			proto::EndpointItmTable::decode(body.as_slice()).map_err(|_| http::Error::Unknown)?;
		Ok(proto_response)
	}

	pub fn post_itm_lease(
		&self,
		era: EhdEra,
		inspector_hex: String,
	) -> Result<proto::EndpointItmLease, http::Error> {
		let url =
			format!("{}/itm/lease?eraId={}&inspectorKey={}", self.base_url, era, inspector_hex);

		let json = serde_json::json!({ "inspector_key": inspector_hex });
		let body = serde_json::to_string(&json).expect("Assignments table to be encoded");

		let response = self.post(&url, body.into(), Accept::Protobuf)?;
		let body = response.body().collect::<Vec<u8>>();

		let proto_response =
			proto::EndpointItmLease::decode(body.as_slice()).map_err(|_| http::Error::Unknown)?;

		Ok(proto_response)
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

		if response.code >= 500 {
			return Err(http::Error::Unknown);
		}

		Ok(response)
	}

	fn post(
		&self,
		url: &str,
		request_body: Vec<u8>,
		accept: Accept,
	) -> Result<http::Response, http::Error> {
		let mut maybe_response = None;

		let deadline = timestamp().add(self.timeout);
		let mut error = None;

		for _ in 0..self.retries {
			let mut request = http::Request::post(url, vec![request_body.clone()])
				.add_header("content-type", "application/json");

			request = match accept {
				Accept::Any => request,
				Accept::Protobuf => request.add_header("Accept", "application/protobuf"),
			};

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

		if response.code >= 500 {
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
		pub id: TcaEra,
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

	#[serde_as]
	#[derive(
		Debug, Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Encode, Decode,
	)]
	pub struct EHDTreeNode {
		#[serde(rename = "ehdId")]
		#[serde_as(as = "TryFromInto<String>")]
		pub ehd_id: EHDId,

		#[serde(rename = "groupingCollector")]
		#[serde_as(as = "TryFromInto<String>")]
		pub g_collector: NodePubKey,

		#[serde(rename = "merkleTreeNodeId")]
		pub tree_node_id: u32,

		#[serde(rename = "merkleTreeNodeHash")]
		#[serde_as(as = "Base64")]
		pub tree_node_hash: Vec<u8>,

		#[serde(rename = "pdhIds")]
		#[serde_as(as = "Vec<TryFromInto<String>>")]
		pub pdh_ids: Vec<PHDId>,

		pub status: String,
		pub customers: Vec<EHDCustomer>,
		pub providers: Vec<EHDProvider>,
	}

	impl EHDTreeNode {
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

	#[serde_as]
	#[derive(
		Debug, Serialize, Deserialize, PartialOrd, Ord, Clone, Hash, PartialEq, Eq, Encode, Decode,
	)]
	pub struct EHDCustomer {
		#[serde(rename = "customerId")]
		#[serde_as(as = "TryFromInto<String>")]
		pub customer_id: AccountId32Hex,

		#[serde(rename = "consumedUsage")]
		pub consumed_usage: EHDUsage,
	}

	#[serde_as]
	#[derive(
		Debug, Serialize, Deserialize, PartialOrd, Ord, Hash, Clone, PartialEq, Eq, Encode, Decode,
	)]
	pub struct EHDProvider {
		#[serde(rename = "providerId")]
		#[serde_as(as = "TryFromInto<String>")]
		pub provider_id: AccountId32Hex,

		#[serde(rename = "providedUsage")]
		pub provided_usage: EHDUsage,
		pub nodes: Vec<EHDProviderUsage>,
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Encode, Decode,
	)]
	pub struct EHDProviderUsage {
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

	#[serde_as]
	#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Encode, Decode)]
	pub struct PHDTreeNode {
		#[serde(rename = "phdId")]
		#[serde_as(as = "TryFromInto<String>")]
		pub phd_id: PHDId,

		#[serde(rename = "collectorId")]
		#[serde_as(as = "TryFromInto<String>")]
		pub collector: NodePubKey,

		#[serde(rename = "merkleTreeNodeId")]
		pub tree_node_id: u32,

		#[serde(rename = "merkleTreeNodeHash")]
		#[serde_as(as = "Base64")]
		pub tree_node_hash: Vec<u8>,

		#[serde(rename = "nodesAggregates")]
		#[serde_as(as = "BTreeMap<TryFromInto<String>, _>")]
		pub nodes_aggregates: PHDNodesTCAs,

		#[serde(rename = "bucketsAggregates")]
		pub buckets_aggregates: PHDBucketsTCAs,
	}

	pub type PHDNodesTCAs = BTreeMap<NodePubKey, Vec<PHDNodeTCA>>;

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct PHDNodeTCA {
		#[serde(rename = "tcaaId")]
		pub tca_id: TcaEra,
		#[serde(rename = "tcaaRootHash")]
		pub tca_hash: String,
		#[serde(rename = "storedBytes")]
		pub stored_bytes: i64,
		#[serde(rename = "transferredBytes")]
		pub transferred_bytes: u64,
		#[serde(rename = "numberOfPuts")]
		pub number_of_puts: u64,
		#[serde(rename = "numberOfGets")]
		pub number_of_gets: u64,
	}

	impl Into<NodeUsage> for PHDNodeTCA {
		fn into(self) -> NodeUsage {
			NodeUsage {
				transferred_bytes: self.transferred_bytes,
				stored_bytes: self.stored_bytes,
				number_of_gets: self.number_of_gets,
				number_of_puts: self.number_of_puts,
			}
		}
	}

	pub type PHDBucketsTCAs = BTreeMap<BucketId, Vec<PHDBucketTCA>>;

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, PartialEq, Eq, Encode, Decode,
	)]
	pub struct PHDBucketTCA {
		#[serde(rename = "tcaaId")]
		pub tca_id: TcaEra,
		#[serde(rename = "tcaaRootHash")]
		pub tca_hash: String,
		#[serde(rename = "storedBytes")]
		pub stored_bytes: i64,
		#[serde(rename = "transferredBytes")]
		pub transferred_bytes: u64,
		#[serde(rename = "numberOfPuts")]
		pub number_of_puts: u64,
		#[serde(rename = "numberOfGets")]
		pub number_of_gets: u64,
	}

	impl Into<BucketUsage> for PHDBucketTCA {
		fn into(self) -> BucketUsage {
			BucketUsage {
				transferred_bytes: self.transferred_bytes,
				stored_bytes: self.stored_bytes,
				number_of_gets: self.number_of_gets,
				number_of_puts: self.number_of_puts,
			}
		}
	}

	#[derive(
		Debug, Serialize, Deserialize, Clone, Hash, Encode, Decode, Ord, PartialOrd, PartialEq, Eq,
	)]
	pub struct EHDEra {
		pub id: u32,
		pub status: String,
		pub era_start: Option<TcaEra>,
		pub era_end: Option<TcaEra>,
		pub time_start: Option<i64>,
		pub time_end: Option<i64>,
	}

	#[derive(Debug, Serialize, Deserialize, Clone, Hash, Encode, Decode)]
	pub struct IsGCollectorResponse {
		#[serde(rename = "isGroupingCollector")]
		pub is_g_collector: bool,
	}
}
