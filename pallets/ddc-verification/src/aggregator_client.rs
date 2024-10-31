use ddc_primitives::{BucketId, DdcEra};
use prost::Message;
use sp_io::offchain::timestamp;
use sp_runtime::offchain::{http, Duration};

use super::*;

pub struct AggregatorClient<'a> {
	pub base_url: &'a str,
	timeout: Duration,
	retries: u32,
}

impl<'a> AggregatorClient<'a> {
	pub fn new(base_url: &'a str, timeout: Duration, retries: u32) -> Self {
		Self { base_url, timeout, retries }
	}

	pub fn challenge_bucket_sub_aggregate(
		&self,
		era_id: DdcEra,
		bucket_id: BucketId,
		node_id: &str,
		merkle_tree_node_id: Vec<u32>,
	) -> Result<proto::ChallengeResponse, http::Error> {
		let url = format!(
			"{}/activity/buckets/{}/challenge?eraId={}&merkleTreeNodeId={}&nodeId={}",
			self.base_url,
			bucket_id,
			era_id,
			Self::merkle_tree_node_id_param(merkle_tree_node_id.as_slice()),
			node_id,
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
