#![allow(dead_code)]
#![allow(clippy::from_over_into)]

use ddc_primitives::{BucketId, EHDId, EhdEra, PHDId, TcaEra};
use prost::Message;
use scale_info::prelude::{collections::BTreeMap, format, string::String, vec::Vec};
use sp_io::offchain::timestamp;
use sp_runtime::offchain::{http, Duration};
use sp_std::vec;

use super::*;
use crate::{json, signature::Verify};

pub struct DdcClient<'a> {
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

impl<'a> DdcClient<'a> {
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

	pub fn get_inspection_state(
		&self,
		era: EhdEra,
	) -> Result<proto::EndpointItmGetPathsState, http::Error> {
		let url = format!("{}/itm/state?eraId={}", self.base_url, era);
		let response = self.get(&url, Accept::Protobuf)?;
		let body = response.body().collect::<Vec<u8>>();
		let proto_response =
			proto::EndpointItmGetPathsState::decode(body.as_slice()).map_err(|e| {
				log::info!("Decode ITM Path Report protobuf error: {:?}", e);
				http::Error::Unknown
			})?;

		Ok(proto_response)
	}

	pub fn submit_inspection_report(
		&self,
		report_json_str: String, /* todo(yahortsaryk): add .proto definition for `InspEraReport`
		                          * type */
	) -> Result<proto::EndpointItmPostPath, http::Error> {
		let url = format!("{}/itm/path", self.base_url);
		let body = report_json_str;

		let response = self.post(&url, body.into(), Accept::Protobuf)?;
		let body = response.body().collect::<Vec<u8>>();

		let proto_response = proto::EndpointItmPostPath::decode(body.as_slice())
			.map_err(|_| http::Error::Unknown)?;
		Ok(proto_response)
	}

	pub fn fetch_inspection_exceptions(
		&self,
		era: EhdEra,
	) -> Result<BTreeMap<String, BTreeMap<String, json::InspPathException>>, http::Error> {
		let mut url = format!("{}/itm/exception?eraId={}", self.base_url, era);

		fetch_and_parse!(self, url, BTreeMap<String, BTreeMap<String, json::InspPathException>>, BTreeMap<String, BTreeMap<String, json::InspPathException>>)
	}

	pub fn check_grouping_collector(&self) -> Result<json::IsGCollectorResponse, http::Error> {
		let mut url = format!("{}/activity/is-grouping-collector", self.base_url);
		fetch_and_parse!(self, url, json::IsGCollectorResponse, json::IsGCollectorResponse)
	}

	pub fn submit_assignments_table(
		&self,
		era: EhdEra,
		table_json_str: String, /* todo(yahortsaryk): add .proto definition for
		                         * `InspAssignmentsTable` type */
		inspector_hex: String,
	) -> Result<proto::EndpointItmSubmit, http::Error> {
		let url =
			format!("{}/itm/submit?eraId={}&inspectorKey={}", self.base_url, era, inspector_hex);
		let body = table_json_str;

		let response = self.post(&url, body.into(), Accept::Protobuf)?;
		let body = response.body().collect::<Vec<u8>>();

		let proto_response =
			proto::EndpointItmSubmit::decode(body.as_slice()).map_err(|_| http::Error::Unknown)?;
		Ok(proto_response)
	}

	pub fn get_assignments_table(
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
