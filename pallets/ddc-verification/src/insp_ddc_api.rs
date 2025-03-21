use core::str;

use ddc_primitives::{
	traits::{ClusterManager, NodeManager},
	BucketId, ClusterId, EHDId, EhdEra, NodeParams, NodePubKey, PHDId, StorageNodeParams, TcaEra,
};
use proto::{endpoint_itm_table::Variant as ItmTableVariant, ItmTable};
use scale_info::prelude::{format, string::String};
use sp_runtime::offchain::{http, Duration};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use crate::{
	aggregator_client,
	insp_task_manager::{InspAssignmentsTable, InspEraReport, InspPathException},
	proto, Config, Error, OCWError,
};

pub(crate) const RESPONSE_TIMEOUT: u64 = 20000;
pub(crate) const MAX_RETRIES_COUNT: u32 = 3;
pub(crate) const BUCKETS_AGGREGATES_FETCH_BATCH_SIZE: usize = 100;

#[allow(dead_code)]
pub(crate) const NODES_AGGREGATES_FETCH_BATCH_SIZE: usize = 10;

/// Fetch grouping collectors nodes of a cluster.
/// Parameters:
/// - `cluster_id`: Cluster id of a cluster.
pub(crate) fn get_g_collectors_nodes<T: Config>(
	cluster_id: &ClusterId,
) -> Result<Vec<(NodePubKey, StorageNodeParams)>, Error<T>> {
	let mut g_collectors = Vec::new();

	let collectors = get_collectors_nodes(cluster_id)?;
	for (node_key, node_params) in collectors {
		if check_grouping_collector::<T>(&node_params)
			.map_err(|_| Error::<T>::NodeRetrievalError)?
		{
			g_collectors.push((node_key, node_params))
		}
	}

	Ok(g_collectors)
}

/// Fetch customer usage.
///
/// Parameters:
/// - `node_params`: Requesting DDC node
pub(crate) fn check_grouping_collector<T: Config>(
	node_params: &StorageNodeParams,
) -> Result<bool, http::Error> {
	let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
	let base_url = format!("http://{}:{}", host, node_params.http_port);
	let client = aggregator_client::AggregatorClient::new(
		&base_url,
		Duration::from_millis(RESPONSE_TIMEOUT),
		MAX_RETRIES_COUNT,
		T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
	);

	let response = client.check_grouping_collector()?;
	Ok(response.is_g_collector)
}

/// Fetch collectors nodes of a cluster.
/// Parameters:
/// - `cluster_id`: Cluster id of a cluster.
pub(crate) fn get_collectors_nodes<T: Config>(
	cluster_id: &ClusterId,
) -> Result<Vec<(NodePubKey, StorageNodeParams)>, Error<T>> {
	let mut collectors = Vec::new();

	let nodes =
		T::ClusterManager::get_nodes(cluster_id).map_err(|_| Error::<T>::NodeRetrievalError)?;

	for node_pub_key in nodes {
		if let Ok(NodeParams::StorageParams(storage_params)) =
			T::NodeManager::get_node_params(&node_pub_key)
		{
			collectors.push((node_pub_key, storage_params));
		}
	}

	Ok(collectors)
}

pub(crate) fn fetch_bucket_challenge_response<T: Config>(
	cluster_id: &ClusterId,
	tcaa_id: TcaEra,
	collector_key: NodePubKey,
	node_key: NodePubKey,
	bucket_id: BucketId,
	tree_node_ids: Vec<u64>,
) -> Result<proto::ChallengeResponse, OCWError> {
	let collectors = get_collectors_nodes(cluster_id)
		.map_err(|_: Error<T>| OCWError::FailedToFetchCollectors { cluster_id: *cluster_id })?;

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

	Err(OCWError::FailedToFetchBucketChallenge)
}

pub(crate) fn fetch_node_challenge_response<T: Config>(
	cluster_id: &ClusterId,
	tcaa_id: TcaEra,
	collector_key: NodePubKey,
	node_key: NodePubKey,
	tree_node_ids: Vec<u64>,
) -> Result<proto::ChallengeResponse, OCWError> {
	let collectors = get_collectors_nodes(cluster_id)
		.map_err(|_: Error<T>| OCWError::FailedToFetchCollectors { cluster_id: *cluster_id })?;

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

	Err(OCWError::FailedToFetchNodeChallenge)
}

/// Fetch customer usage.
///
/// Parameters:
/// - `cluster_id`: cluster id of a cluster
/// - `tcaa_id`: time capsule era
/// - `collector_key`: collector to fetch Bucket aggregates from
pub(crate) fn fetch_bucket_aggregates<T: Config>(
	cluster_id: &ClusterId,
	tcaa_id: TcaEra,
	collector_key: NodePubKey,
) -> Result<Vec<aggregator_client::json::BucketAggregateResponse>, OCWError> {
	let collectors = get_collectors_nodes(cluster_id)
		.map_err(|_: Error<T>| OCWError::FailedToFetchCollectors { cluster_id: *cluster_id })?;

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
				T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
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
					.map_err(|_| OCWError::FailedToFetchBucketAggregate)?;

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

	Err(OCWError::FailedToFetchBucketAggregate)
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
pub(crate) fn fetch_traversed_era_historical_document<T: Config>(
	cluster_id: &ClusterId,
	ehd_id: EHDId,
	tree_node_id: u32,
	tree_levels_count: u32,
) -> Result<Vec<aggregator_client::json::EHDTreeNode>, OCWError> {
	let collectors = get_collectors_nodes(cluster_id).map_err(|_: Error<T>| {
		log::error!("❌ Error retrieving collectors for cluster {:?}", cluster_id);
		OCWError::FailedToFetchCollectors { cluster_id: *cluster_id }
	})?;

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

	Err(OCWError::FailedToFetchTraversedEHD)
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
pub(crate) fn fetch_traversed_partial_historical_document<T: Config>(
	cluster_id: &ClusterId,
	phd_id: PHDId,
	tree_node_id: u32,
	tree_levels_count: u32,
) -> Result<Vec<aggregator_client::json::PHDTreeNode>, OCWError> {
	let collectors = get_collectors_nodes(cluster_id).map_err(|_: Error<T>| {
		log::error!("❌ Error retrieving collectors for cluster {:?}", cluster_id);
		OCWError::FailedToFetchCollectors { cluster_id: *cluster_id }
	})?;

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

	Err(OCWError::FailedToFetchTraversedPHD)
}

/// Fetch EHD merkle root node.
///
/// Parameters:
/// - `cluster_id`: Cluster Id
/// - `phd_id`: EHD id
pub(crate) fn get_ehd_root<T: Config>(
	cluster_id: &ClusterId,
	ehd_id: EHDId,
) -> Result<aggregator_client::json::EHDTreeNode, OCWError> {
	fetch_traversed_era_historical_document::<T>(cluster_id, ehd_id, 1, 1)?
		.first()
		.ok_or(OCWError::FailedToFetchTraversedEHD)
		.cloned()
}

/// Fetch PHD merkle root node.
///
/// Parameters:
/// - `cluster_id`: Cluster Id
/// - `phd_id`: PHD id
pub(crate) fn get_phd_root<T: Config>(
	cluster_id: &ClusterId,
	phd_id: PHDId,
) -> Result<aggregator_client::json::PHDTreeNode, OCWError> {
	fetch_traversed_partial_historical_document::<T>(cluster_id, phd_id, 1, 1)?
		.first()
		.ok_or(OCWError::FailedToFetchTraversedPHD)
		.cloned()
}

/// Fetch processed EHD eras.
///
/// Parameters:
/// - `node_params`: DAC node parameters
#[allow(dead_code)]
pub(crate) fn fetch_processed_ehd_eras<T: Config>(
	node_params: &StorageNodeParams,
) -> Result<Vec<aggregator_client::json::EHDEra>, http::Error> {
	let host = str::from_utf8(&node_params.host).map_err(|_| http::Error::Unknown)?;
	let base_url = format!("http://{}:{}", host, node_params.http_port);
	let client = aggregator_client::AggregatorClient::new(
		&base_url,
		Duration::from_millis(RESPONSE_TIMEOUT),
		MAX_RETRIES_COUNT,
		T::VERIFY_AGGREGATOR_RESPONSE_SIGNATURE,
	);

	let response = client.payment_eras()?;

	Ok(response.into_iter().filter(|e| e.status == "EHD_PROCESSED").collect::<Vec<_>>())
}

/// Fetch processed payment era for across all nodes.
///
/// Parameters:
/// - `cluster_id`: Cluster Id
/// - `g_collector_key`: G-collector node key to fetch the payment eras from
/// - `node_params`: DAC node parameters
pub(crate) fn fetch_processed_eras<T: Config>(
	cluster_id: &ClusterId,
	g_collectors: &[(NodePubKey, StorageNodeParams)],
) -> Result<Vec<Vec<aggregator_client::json::EHDEra>>, OCWError> {
	let mut processed_eras_by_nodes: Vec<Vec<aggregator_client::json::EHDEra>> = Vec::new();

	for (collector_key, node_params) in g_collectors {
		let processed_payment_eras = fetch_processed_ehd_eras::<T>(node_params);
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
			let eras = processed_payment_eras.map_err(|_| OCWError::FailedToFetchPaymentEra)?;
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
pub(crate) fn fetch_processed_era<T: Config>(
	cluster_id: &ClusterId,
	era: EhdEra,
	g_collector: &(NodePubKey, StorageNodeParams),
) -> Result<aggregator_client::json::EHDEra, OCWError> {
	let ehd_eras = fetch_processed_eras::<T>(cluster_id, vec![g_collector.clone()].as_slice())?;

	let era = ehd_eras
		.iter()
		.flat_map(|eras| eras.iter())
		.find(|ehd| ehd.id == era)
		.ok_or(OCWError::FailedToFetchPaymentEra)?;

	Ok(era.clone())
}

pub(crate) fn fetch_inspection_exceptions<T: Config>(
	cluster_id: &ClusterId,
	era: EhdEra,
) -> Result<BTreeMap<String, BTreeMap<String, InspPathException>>, OCWError> {
	// todo(yahortsaryk): replace with Sync node
	let g_collector = get_g_collectors_nodes(cluster_id)
		.map_err(|_: Error<T>| OCWError::FailedToFetchGCollectors { cluster_id: *cluster_id })?
		.first()
		.cloned()
		.ok_or(OCWError::FailedToFetchGCollectors { cluster_id: *cluster_id })?;

	let host = str::from_utf8(&g_collector.1.host).map_err(|_| OCWError::Unexpected)?;
	let base_url = format!("http://{}:{}", host, g_collector.1.http_port);
	let client = aggregator_client::AggregatorClient::new(
		&base_url,
		Duration::from_millis(RESPONSE_TIMEOUT),
		MAX_RETRIES_COUNT,
		false, // no response signature verification for now
	);

	client
		.fetch_inspection_exceptions(era)
		.map_err(|_| OCWError::FailedToFetchPathsExceptions)
}

pub(crate) fn get_inspection_state<T: Config>(
	cluster_id: &ClusterId,
	era: EhdEra,
) -> Result<proto::EndpointItmGetPathsState, http::Error> {
	// todo(yahortsaryk): replace with Sync node
	let g_collectors =
		get_g_collectors_nodes(cluster_id).map_err(|_: Error<T>| http::Error::Unknown)?;
	let Some(g_collector) = g_collectors.first() else {
		log::warn!("⚠️ No Grouping Collector found in cluster {:?}", cluster_id);
		return Err(http::Error::Unknown);
	};

	let host = str::from_utf8(&g_collector.1.host).map_err(|_| http::Error::Unknown)?;
	let base_url = format!("http://{}:{}", host, g_collector.1.http_port);
	let client = aggregator_client::AggregatorClient::new(
		&base_url,
		Duration::from_millis(RESPONSE_TIMEOUT),
		MAX_RETRIES_COUNT,
		false, // no response signature verification for now
	);

	client.get_inspection_state(era)
}

pub(crate) fn submit_inspection_report<T: Config>(
	cluster_id: &ClusterId,
	report: InspEraReport,
) -> Result<proto::EndpointItmPostPath, http::Error> {
	// todo(yahortsaryk): replace with Sync node
	let g_collectors =
		get_g_collectors_nodes(cluster_id).map_err(|_: Error<T>| http::Error::Unknown)?;
	let Some(g_collector) = g_collectors.first() else {
		log::warn!("⚠️ No Grouping Collector found in cluster {:?}", cluster_id);
		return Err(http::Error::Unknown);
	};

	let host = str::from_utf8(&g_collector.1.host).map_err(|_| http::Error::Unknown)?;
	let base_url = format!("http://{}:{}", host, g_collector.1.http_port);
	let client = aggregator_client::AggregatorClient::new(
		&base_url,
		Duration::from_millis(RESPONSE_TIMEOUT),
		MAX_RETRIES_COUNT,
		false, // no response signature verification for now
	);

	client.submit_inspection_report(report.clone())
}

pub(crate) fn submit_assignments_table<T: Config>(
	cluster_id: &ClusterId,
	table: InspAssignmentsTable<T::AccountId>,
	inspector_hex: String,
) -> Result<proto::EndpointItmSubmit, http::Error> {
	// todo(yahortsaryk): replace with Sync node
	let g_collectors =
		get_g_collectors_nodes(cluster_id).map_err(|_: Error<T>| http::Error::Unknown)?;
	let Some(g_collector) = g_collectors.first() else {
		log::warn!("⚠️ No Grouping Collector found in cluster {:?}", cluster_id);
		return Err(http::Error::Unknown);
	};

	let host = str::from_utf8(&g_collector.1.host).map_err(|_| http::Error::Unknown)?;
	let base_url = format!("http://{}:{}", host, g_collector.1.http_port);
	let client = aggregator_client::AggregatorClient::new(
		&base_url,
		Duration::from_millis(RESPONSE_TIMEOUT),
		MAX_RETRIES_COUNT,
		false, // no response signature verification for now
	);

	client.submit_assignments_table::<T>(table, inspector_hex)
}

pub(crate) fn get_assignments_table<T: Config>(
	cluster_id: &ClusterId,
	era: EhdEra,
) -> Result<InspAssignmentsTable<T::AccountId>, http::Error> {
	// todo(yahortsaryk): replace with Sync node
	let g_collectors =
		get_g_collectors_nodes(cluster_id).map_err(|_: Error<T>| http::Error::Unknown)?;
	let Some(g_collector) = g_collectors.first() else {
		log::warn!("⚠️ No Grouping Collector found in cluster {:?}", cluster_id);
		return Err(http::Error::Unknown);
	};

	let host = str::from_utf8(&g_collector.1.host).map_err(|_| http::Error::Unknown)?;
	let base_url = format!("http://{}:{}", host, g_collector.1.http_port);
	let client = aggregator_client::AggregatorClient::new(
		&base_url,
		Duration::from_millis(RESPONSE_TIMEOUT),
		MAX_RETRIES_COUNT,
		false,
	);

	let table_response: proto::EndpointItmTable =
		client.get_assignments_table::<T>(era).map_err(|_| http::Error::Unknown)?;

	// todo(yahortsaryk): move the below pattern matching to `InspTaskAssigner`
	match table_response.variant {
		Some(ItmTableVariant::Table(ItmTable { json_string, inspector_key: _key })) => {
			let response: InspAssignmentsTable<T::AccountId> = serde_json::from_str(&json_string)
				.map_err(|e| {
				log::error!("ItmTable deserialization error {:?}", e);
				http::Error::Unknown
			})?;
			return Ok(response);
		},
		_ => {
			// todo(yahortsaryk): handle other `EndpointItmTable` variants
			Err(http::Error::Unknown)
		},
	}
}

pub(crate) fn post_itm_lease<T: Config>(
	cluster_id: &ClusterId,
	era: EhdEra,
	inspector_hex: String,
) -> Result<proto::EndpointItmLease, http::Error> {
	let g_collectors =
		get_g_collectors_nodes(cluster_id).map_err(|_: Error<T>| http::Error::Unknown)?;
	let Some(g_collector) = g_collectors.first() else {
		log::warn!("⚠️ No Grouping Collector found in cluster {:?}", cluster_id);
		return Err(http::Error::Unknown);
	};

	let host = str::from_utf8(&g_collector.1.host).map_err(|_| http::Error::Unknown)?;
	let base_url = format!("http://{}:{}", host, g_collector.1.http_port);
	let client = aggregator_client::AggregatorClient::new(
		&base_url,
		Duration::from_millis(RESPONSE_TIMEOUT),
		MAX_RETRIES_COUNT,
		false, // no response signature verification for now
	);

	client.post_itm_lease(era, inspector_hex)
}
