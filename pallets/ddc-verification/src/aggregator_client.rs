use ddc_primitives::{BucketId, DdcEra};
use prost::Message;
use sp_io::offchain::timestamp;
use sp_runtime::offchain::{http, Duration};

use super::*;

pub struct AggregatorClient<'a> {
	pub base_url: &'a str,
	timeout: Duration,
}

impl<'a> AggregatorClient<'a> {
	pub fn new(base_url: &'a str, timeout: Duration) -> Self {
		Self { base_url, timeout }
	}

	pub fn challenge_bucket_sub_aggregate(
		&self,
		era_id: DdcEra,
		bucket_id: BucketId,
		node_id: &str,
		merkle_tree_node_id: Vec<u32>,
	) -> Result<proto::ChallengeResponse, http::Error> {
		let merkle_tree_nodes_ids = merkle_tree_node_id
			.iter()
			.map(|x| format!("{}", x.clone()))
			.collect::<Vec<_>>()
			.join(",");
		let url = format!(
			"{}/activity/buckets/{}/challenge?eraId={}&merkleTreeNodeId={}&nodeId={}",
			self.base_url, bucket_id, era_id, merkle_tree_nodes_ids, node_id,
		);
		let response = self.get_proto(&url)?;
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
		let merkle_tree_nodes_ids = merkle_tree_node_id
			.iter()
			.map(|x| format!("{}", x.clone()))
			.collect::<Vec<_>>()
			.join(",");
		let url = format!(
			"{}/activity/nodes/{}/challenge?eraId={}&merkleTreeNodeId={}",
			self.base_url, node_id, era_id, merkle_tree_nodes_ids,
		);
		let response = self.get_proto(&url)?;
		let body = response.body().collect::<Vec<u8>>();
		let proto_response =
			proto::ChallengeResponse::decode(body.as_slice()).map_err(|_| http::Error::Unknown)?;

		Ok(proto_response)
	}

	fn get_proto(&self, url: &str) -> Result<http::Response, http::Error> {
		let deadline = timestamp().add(self.timeout);
		let response = http::Request::get(url)
			.add_header("Accept", "application/protobuf")
			.deadline(deadline)
			.send()
			.map_err(|_| http::Error::IoError)?
			.try_wait(deadline)
			.map_err(|_| http::Error::DeadlineReached)??;

		if response.code != 200 {
			return Err(http::Error::Unknown);
		}

		Ok(response)
	}
}
