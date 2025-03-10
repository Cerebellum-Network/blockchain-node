#![allow(clippy::from_over_into)]

use core::str;

use codec::{Decode, Encode};
use ddc_primitives::{
	AccountId32Hex, AggregateKey, AggregatorInfo, BucketId, BucketUsage, EHDId, NodePubKey,
	NodeUsage, PHDId, TcaEra,
};
use scale_info::prelude::string::String;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as, TryFromInto};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

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
#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, PartialEq, Eq, Encode, Decode)]
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

#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, PartialEq, Eq, Encode, Decode)]
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
#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, Ord, Eq, PartialEq, Encode, Decode)]
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
	pub era_start: Option<TcaEra>,
	pub era_end: Option<TcaEra>,
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

// todo(yahortsaryk): remove after adding .proto for `InspPathException` type
#[derive(Debug, Clone, Deserialize, Serialize, Encode, Decode, PartialOrd, Ord, Eq, PartialEq)]
pub enum InspPathException {
	NodeAR { bad_leaves_ids: Vec<u64> },
	BucketAR { bad_leaves_pos: Vec<u64> },
}
