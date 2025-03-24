use core::str;

use codec::{Decode, Encode};
use ddc_primitives::{
	traits::{ClusterManager, NodeManager},
	BucketId, ClusterId, EHDId, EhdEra, NodeParams, NodePubKey, PHDId, StorageNodeParams, TcaEra,
	VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
};
use scale_info::{
	prelude::{format, string::String},
	TypeInfo,
};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as, TryFromInto};
use sp_runtime::offchain::{http, Duration};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use crate::{
	aggregator as aggregator_client,
	aggregator::json::{PHDBucketsTCAs, PHDNodesTCAs},
	proto,
};

pub const RESPONSE_TIMEOUT: u64 = 20000;
pub const MAX_RETRIES_COUNT: u32 = 3;
pub const BUCKETS_AGGREGATES_FETCH_BATCH_SIZE: usize = 100;

#[allow(dead_code)]
pub const NODES_AGGREGATES_FETCH_BATCH_SIZE: usize = 10;

#[derive(Debug, Encode, Decode, Clone, TypeInfo, PartialEq)]
pub enum ApiError {
	NodeRetrievalError,
	FailedToFetchCollectors { cluster_id: ClusterId },
	FailedToFetchBucketChallenge,
	FailedToFetchNodeChallenge,
	FailedToFetchBucketAggregate,
	FailedToFetchTraversedEHD,
	FailedToFetchTraversedPHD,
	FailedToFetchPaymentEra,
	FailedToFetchGCollectors { cluster_id: ClusterId },
	FailedToFetchInspectionReceipt,
	FailedToSaveInspectionReceipt,
}

/// Fetch grouping collectors nodes of a cluster.
/// Parameters:
/// - `cluster_id`: Cluster id of a cluster.
pub fn get_g_collectors_nodes<
	AccountId,
	BlockNumber,
	CM: ClusterManager<AccountId, BlockNumber>,
	NM: NodeManager<AccountId>,
>(
	cluster_id: &ClusterId,
) -> Result<Vec<(NodePubKey, StorageNodeParams)>, ApiError> {
	let mut g_collectors = Vec::new();

	let collectors = get_collectors_nodes::<AccountId, BlockNumber, CM, NM>(cluster_id)?;
	for (node_key, node_params) in collectors {
		if check_grouping_collector(&node_params).map_err(|_| ApiError::NodeRetrievalError)? {
			g_collectors.push((node_key, node_params))
		}
	}

	Ok(g_collectors)
}

/// Fetch customer usage.
///
/// Parameters:
/// - `node_params`: Requesting DDC node
pub fn check_grouping_collector(node_params: &StorageNodeParams) -> Result<bool, http::Error> {
	let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
	let base_url = format!("http://{}:{}", host, node_params.http_port);
	let client = aggregator_client::AggregatorClient::new(
		&base_url,
		Duration::from_millis(RESPONSE_TIMEOUT),
		MAX_RETRIES_COUNT,
		VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
	);

	let response = client.check_grouping_collector()?;
	Ok(response.is_g_collector)
}

/// Fetch collectors nodes of a cluster.
/// Parameters:
/// - `cluster_id`: Cluster id of a cluster.
pub fn get_collectors_nodes<
	AccountId,
	BlockNumber,
	CM: ClusterManager<AccountId, BlockNumber>,
	NM: NodeManager<AccountId>,
>(
	cluster_id: &ClusterId,
) -> Result<Vec<(NodePubKey, StorageNodeParams)>, ApiError> {
	let mut collectors = Vec::new();

	let nodes = CM::get_nodes(cluster_id).map_err(|_| ApiError::NodeRetrievalError)?;

	for node_pub_key in nodes {
		if let Ok(NodeParams::StorageParams(storage_params)) = NM::get_node_params(&node_pub_key) {
			collectors.push((node_pub_key, storage_params));
		}
	}

	Ok(collectors)
}

pub fn fetch_bucket_challenge_response<
	AccountId,
	BlockNum,
	CM: ClusterManager<AccountId, BlockNum>,
	NM: NodeManager<AccountId>,
>(
	cluster_id: &ClusterId,
	tcaa_id: TcaEra,
	collector_key: NodePubKey,
	node_key: NodePubKey,
	bucket_id: BucketId,
	tree_node_ids: Vec<u64>,
) -> Result<proto::ChallengeResponse, ApiError> {
	let collectors = get_collectors_nodes::<AccountId, BlockNum, CM, NM>(cluster_id)?;

	for (key, collector_params) in collectors {
		if key != collector_key {
			continue;
		};

		if let Ok(host) = str::from_utf8(&collector_params.host) {
			let base_url = format!("http://{}:{}", host, collector_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				MAX_RETRIES_COUNT,
				false, // no response signature verification for now
			);

			if let Ok(node_challenge_res) = client.challenge_bucket_sub_aggregate(
				tcaa_id,
				bucket_id,
				Into::<String>::into(node_key.clone()).as_str(),
				tree_node_ids.clone(),
			) {
				return Ok(node_challenge_res);
			} else {
				log::warn!(
                    "Collector from cluster {:?} is unavailable while challenging bucket sub-aggregate or responded with unexpected body. Key: {:?} Host: {:?}",
                    cluster_id,
                    collector_key,
                    String::from_utf8(collector_params.host)
                );
			}
		}
	}

	Err(ApiError::FailedToFetchBucketChallenge)
}

pub fn fetch_node_challenge_response<
	AccountId,
	BlockNumber,
	CM: ClusterManager<AccountId, BlockNumber>,
	NM: NodeManager<AccountId>,
>(
	cluster_id: &ClusterId,
	tcaa_id: TcaEra,
	collector_key: NodePubKey,
	node_key: NodePubKey,
	tree_node_ids: Vec<u64>,
) -> Result<proto::ChallengeResponse, ApiError> {
	let collectors = get_collectors_nodes::<AccountId, BlockNumber, CM, NM>(cluster_id)?;

	for (key, collector_params) in collectors {
		if key != collector_key {
			continue;
		};

		if let Ok(host) = str::from_utf8(&collector_params.host) {
			let base_url = format!("http://{}:{}", host, collector_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				MAX_RETRIES_COUNT,
				false, // no response signature verification for now
			);

			if let Ok(node_challenge_res) = client.challenge_node_aggregate(
				tcaa_id,
				Into::<String>::into(node_key.clone()).as_str(),
				tree_node_ids.clone(),
			) {
				return Ok(node_challenge_res);
			} else {
				log::warn!(
							"Collector from cluster {:?} is unavailable while challenging node aggregate or responded with unexpected body. Key: {:?} Host: {:?}",
							cluster_id,
							collector_key,
							String::from_utf8(collector_params.host)
						);
			}
		}
	}

	Err(ApiError::FailedToFetchNodeChallenge)
}

/// Fetch customer usage.
///
/// Parameters:
/// - `cluster_id`: cluster id of a cluster
/// - `tcaa_id`: time capsule era
/// - `collector_key`: collector to fetch Bucket aggregates from
pub fn fetch_bucket_aggregates<
	AccountId,
	BlockNumber,
	CM: ClusterManager<AccountId, BlockNumber>,
	NM: NodeManager<AccountId>,
>(
	cluster_id: &ClusterId,
	tcaa_id: TcaEra,
	collector_key: NodePubKey,
) -> Result<Vec<aggregator_client::json::BucketAggregateResponse>, ApiError> {
	let collectors = get_collectors_nodes::<AccountId, BlockNumber, CM, NM>(cluster_id)?;

	for (key, collector_params) in collectors {
		if key != collector_key {
			continue;
		};

		if let Ok(host) = str::from_utf8(&collector_params.host) {
			let base_url = format!("http://{}:{}", host, collector_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				MAX_RETRIES_COUNT,
				VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
			);

			let mut buckets_aggregates = Vec::new();
			let mut prev_token = None;

			loop {
				let response = client
					.buckets_aggregates(
						tcaa_id,
						Some(BUCKETS_AGGREGATES_FETCH_BATCH_SIZE as u32),
						prev_token,
					)
					.map_err(|_| ApiError::FailedToFetchBucketAggregate)?;

				let response_len = response.len();

				prev_token = response.last().map(|a| a.bucket_id);

				buckets_aggregates.extend(response);

				if response_len < BUCKETS_AGGREGATES_FETCH_BATCH_SIZE {
					break;
				}
			}

			return Ok(buckets_aggregates);
		}
	}

	Err(ApiError::FailedToFetchBucketAggregate)
}

/// Traverse EHD record.
///
/// Parameters:
/// - `cluster_id`: cluster id of a cluster
/// - `ehd_id`: EHDId is a concatenated representation of:
///     1) A 32-byte node public key in hex
///     2) Starting TCA id
///     3) Ending TCA id
/// - `tree_node_id` - merkle tree node identifier
/// - `tree_levels_count` - merkle tree levels to request
pub fn fetch_traversed_era_historical_document<
	AccountId,
	BlockNumber,
	CM: ClusterManager<AccountId, BlockNumber>,
	NM: NodeManager<AccountId>,
>(
	cluster_id: &ClusterId,
	ehd_id: EHDId,
	tree_node_id: u32,
	tree_levels_count: u32,
) -> Result<Vec<aggregator_client::json::EHDTreeNode>, ApiError> {
	let collectors = get_collectors_nodes::<AccountId, BlockNumber, CM, NM>(cluster_id)?;

	for (collector_key, collector_params) in collectors {
		if collector_key != ehd_id.1 {
			continue;
		};

		if let Ok(host) = str::from_utf8(&collector_params.host) {
			let base_url = format!("http://{}:{}", host, collector_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				MAX_RETRIES_COUNT,
				false, // no response signature verification for now
			);

			if let Ok(traversed_ehd) = client.traverse_era_historical_document(
				ehd_id.clone(),
				tree_node_id,
				tree_levels_count,
			) {
				// proceed with the first available EHD record for the prototype
				return Ok(traversed_ehd);
			} else {
				log::warn!(
							"⚠️  Collector from cluster {:?} is unavailable while fetching EHD record or responded with unexpected body. Key: {:?} Host: {:?}",
							cluster_id,
							collector_key,
							String::from_utf8(collector_params.host)
						);
			}
		}
	}

	Err(ApiError::FailedToFetchTraversedEHD)
}

/// Traverse PHD record.
///
/// Parameters:
/// - `cluster_id`: cluster id of a cluster
/// - `phd_id`: PHDId is a concatenated representation of:
///     1) A 32-byte node public key in hex
///     2) Starting TCAA id
///     3) Ending TCAA id
/// - `tree_node_id` - merkle tree node identifier
/// - `tree_levels_count` - merkle tree levels to request
pub fn fetch_traversed_partial_historical_document<
	AccountId,
	BlockNumber,
	CM: ClusterManager<AccountId, BlockNumber>,
	NM: NodeManager<AccountId>,
>(
	cluster_id: &ClusterId,
	phd_id: PHDId,
	tree_node_id: u32,
	tree_levels_count: u32,
) -> Result<Vec<aggregator_client::json::PHDTreeNode>, ApiError> {
	let collectors = get_collectors_nodes::<AccountId, BlockNumber, CM, NM>(cluster_id)?;

	for (collector_key, collector_params) in collectors {
		if collector_key != phd_id.0 {
			continue;
		};

		if let Ok(host) = str::from_utf8(&collector_params.host) {
			let base_url = format!("http://{}:{}", host, collector_params.http_port);
			let client = aggregator_client::AggregatorClient::new(
				&base_url,
				Duration::from_millis(RESPONSE_TIMEOUT),
				MAX_RETRIES_COUNT,
				false, // no response signature verification for now
			);

			if let Ok(traversed_phd) = client.traverse_partial_historical_document(
				phd_id.clone(),
				tree_node_id,
				tree_levels_count,
			) {
				// proceed with the first available EHD record for the prototype
				return Ok(traversed_phd);
			} else {
				log::warn!(
							"⚠️  Collector from cluster {:?} is unavailable while fetching PHD record or responded with unexpected body. Key: {:?} Host: {:?}",
							cluster_id,
							collector_key,
							String::from_utf8(collector_params.host)
						);
			}
		}
	}

	Err(ApiError::FailedToFetchTraversedPHD)
}

/// Fetch EHD merkle root node.
///
/// Parameters:
/// - `cluster_id`: Cluster Id
/// - `phd_id`: EHD id
pub fn get_ehd_root<
	AccountId,
	BlockNumber,
	CM: ClusterManager<AccountId, BlockNumber>,
	NM: NodeManager<AccountId>,
>(
	cluster_id: &ClusterId,
	ehd_id: EHDId,
) -> Result<aggregator_client::json::EHDTreeNode, ApiError> {
	fetch_traversed_era_historical_document::<AccountId, BlockNumber, CM, NM>(
		cluster_id, ehd_id, 1, 1,
	)?
	.first()
	.ok_or(ApiError::FailedToFetchTraversedEHD)
	.cloned()
}

/// Fetch PHD merkle root node.
///
/// Parameters:
/// - `cluster_id`: Cluster Id
/// - `phd_id`: PHD id
pub fn get_phd_root<
	AccountId,
	BlockNumber,
	CM: ClusterManager<AccountId, BlockNumber>,
	NM: NodeManager<AccountId>,
>(
	cluster_id: &ClusterId,
	phd_id: PHDId,
) -> Result<aggregator_client::json::PHDTreeNode, ApiError> {
	fetch_traversed_partial_historical_document::<AccountId, BlockNumber, CM, NM>(
		cluster_id, phd_id, 1, 1,
	)?
	.first()
	.ok_or(ApiError::FailedToFetchTraversedPHD)
	.cloned()
}

/// Fetch processed EHD eras.
///
/// Parameters:
/// - `node_params`: DAC node parameters
#[allow(dead_code)]
pub fn fetch_processed_ehd_eras(
	node_params: &StorageNodeParams,
) -> Result<Vec<aggregator_client::json::EHDEra>, http::Error> {
	let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
	let base_url = format!("http://{}:{}", host, node_params.http_port);
	let client = aggregator_client::AggregatorClient::new(
		&base_url,
		Duration::from_millis(RESPONSE_TIMEOUT),
		MAX_RETRIES_COUNT,
		VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
	);

	let response = client.payment_eras()?;

	Ok(response.into_iter().filter(|e| e.status == "PROCESSED").collect::<Vec<_>>())
}

/// Fetch processed payment era for across all nodes.
///
/// Parameters:
/// - `cluster_id`: Cluster Id
/// - `g_collector_key`: G-collector node key to fetch the payment eras from
/// - `node_params`: DAC node parameters
pub fn fetch_processed_eras(
	cluster_id: &ClusterId,
	g_collectors: &[(NodePubKey, StorageNodeParams)],
) -> Result<Vec<Vec<aggregator_client::json::EHDEra>>, ApiError> {
	let mut processed_eras_by_nodes: Vec<Vec<aggregator_client::json::EHDEra>> = Vec::new();

	for (collector_key, node_params) in g_collectors {
		let processed_payment_eras = fetch_processed_ehd_eras(node_params);
		if processed_payment_eras.is_err() {
			log::warn!(
						"Aggregator from cluster {:?} is unavailable while fetching processed eras. Key: {:?} Host: {:?}",
						cluster_id,
						collector_key,
						String::from_utf8(node_params.host.clone())
					);
			// Skip unavailable aggregators and continue with available ones
			continue;
		} else {
			let eras = processed_payment_eras.map_err(|_| ApiError::FailedToFetchPaymentEra)?;
			if !eras.is_empty() {
				processed_eras_by_nodes.push(eras.into_iter().collect::<Vec<_>>());
			}
		}
	}

	Ok(processed_eras_by_nodes)
}

/// Fetch processed payment era by its id.
///
/// Parameters:
/// - `cluster_id`: Cluster Id
/// - `era`: EHD era id to process
/// - `node_params`: DAC node parameters
pub fn fetch_processed_era(
	cluster_id: &ClusterId,
	era: EhdEra,
	g_collector: &(NodePubKey, StorageNodeParams),
) -> Result<aggregator_client::json::EHDEra, ApiError> {
	let ehd_eras = fetch_processed_eras(cluster_id, vec![g_collector.clone()].as_slice())?;

	let era = ehd_eras
		.iter()
		.flat_map(|eras| eras.iter())
		.find(|ehd| ehd.id == era)
		.ok_or(ApiError::FailedToFetchPaymentEra)?;

	Ok(era.clone())
}

pub fn fetch_inspection_receipts<
	AccountId,
	BlockNumber,
	CM: ClusterManager<AccountId, BlockNumber>,
	NM: NodeManager<AccountId>,
>(
	cluster_id: &ClusterId,
	ehd_id: EHDId,
) -> Result<BTreeMap<String, aggregator_client::json::GroupedInspectionReceipt>, ApiError> {
	// todo(yahortsaryk): infer the G-collector deterministically
	let g_collector = get_g_collectors_nodes::<AccountId, BlockNumber, CM, NM>(cluster_id)?
		.first()
		.cloned()
		.ok_or(ApiError::FailedToFetchGCollectors { cluster_id: *cluster_id })?;

	if let Ok(host) = str::from_utf8(&g_collector.1.host) {
		let base_url = format!("http://{}:{}", host, g_collector.1.http_port);
		let client = aggregator_client::AggregatorClient::new(
			&base_url,
			Duration::from_millis(RESPONSE_TIMEOUT),
			MAX_RETRIES_COUNT,
			false, // no response signature verification for now
		);

		if let Ok(res) = client.fetch_grouped_inspection_receipts(ehd_id) {
			return Ok(res);
		}
	}

	Err(ApiError::FailedToFetchInspectionReceipt)
}

/// Send Inspection Receipt.
///
/// Parameters:
/// - `cluster_id`: cluster id of a cluster
/// - `g_collector`: grouping collector node to save the receipt
/// - `receipt`: inspection receipt
pub fn send_inspection_receipt(
	cluster_id: &ClusterId,
	g_collector: &(NodePubKey, StorageNodeParams),
	receipt: aggregator_client::json::InspectionReceipt,
) -> Result<(), ApiError> {
	if let Ok(host) = str::from_utf8(&g_collector.1.host) {
		let base_url = format!("http://{}:{}", host, g_collector.1.http_port);
		let client = aggregator_client::AggregatorClient::new(
			&base_url,
			Duration::from_millis(RESPONSE_TIMEOUT),
			MAX_RETRIES_COUNT,
			false, // no response signature verification for now
		);

		if client.send_inspection_receipt(receipt.clone()).is_ok() {
			// proceed with the first available EHD record for the prototype
			return Ok(());
		} else {
			log::warn!(
						"⚠️  Collector from cluster {:?} is unavailable while fetching EHD record or responded with unexpected body. Key: {:?} Host: {:?}",
						cluster_id,
						g_collector.0,
						String::from_utf8(g_collector.1.host.clone())
					);
		}
	}
	Err(ApiError::FailedToSaveInspectionReceipt)
}

#[allow(dead_code)]
pub fn get_inspection_assignment_table(
	_cluster_id: &ClusterId,
	_sync_node: &(NodePubKey, StorageNodeParams),
) -> Result<(), ()> {
	// todo(yahortsaryk): request DDC Sync node for the inspection assignments table
	Ok(())
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
