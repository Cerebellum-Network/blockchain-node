use aggregate_tree::{
	calculate_sample_size_fin, calculate_sample_size_inf, get_leaves_ids, D_099, P_001,
};
use aggregator_client::json::{EHDTreeNode, PHDTreeNode};
use ddc_primitives::{
	traits::ClusterValidator, BucketId, BucketUsage, ClusterId, EHDId, EhdEra, NodePubKey,
	NodeUsage, TcaEra,
};
use frame_support::pallet_prelude::{Decode, Encode, TypeInfo};
use frame_system::offchain::Account;
use itertools::Itertools;
use rand::{prelude::*, rngs::SmallRng, SeedableRng};
use scale_info::prelude::{format, string::String};
use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_io::offchain::{local_storage_get, local_storage_set};
use sp_runtime::{
	offchain::StorageKind,
	traits::{Hash, IdentifyAccount},
};
use sp_std::{
	collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	prelude::*,
	rc::Rc,
};

use crate::{
	aggregate_tree, aggregator_client, fetch_last_inspected_ehds,
	insp_ddc_api::{
		fetch_bucket_aggregates, fetch_bucket_challenge_response, fetch_node_challenge_response,
		fetch_processed_eras, get_assignments_table, get_ehd_root, get_g_collectors_nodes,
		get_phd_root, send_assignments_table,
	},
	pallet::{Error, ValidatorSet},
	signature::Verify,
	Config, Hashable,
};

pub(crate) const TCA_INSPECTION_STEP: usize = 0;
pub(crate) const INSPECTION_REDUNDANCY_FACTOR: u8 = 3;
pub(crate) const INSPECTION_BACKUPS_COUNT: u8 = 2;

pub(crate) const ALICE: [u8; 32] = [
	0xd4, 0x35, 0x93, 0xc7, 0x15, 0xfd, 0xd3, 0x1c, 0x61, 0x14, 0x1a, 0xbd, 0x04, 0xa9, 0x9f, 0xd6,
	0x82, 0x2c, 0x85, 0x58, 0x85, 0x4c, 0xcd, 0xe3, 0x9a, 0x56, 0x84, 0xe7, 0xa5, 0x6d, 0xa2, 0x7d,
];

type GCollectorNodeKey = NodePubKey;
type CollectorNodeKey = NodePubKey;
type DataNodeKey = NodePubKey;
type InspPathId = H256;
type InspPathReceiptHash = H256;

#[derive(Debug, Clone, Deserialize, Serialize, TypeInfo)]
pub struct InspAssignmentsTable<AccountId>
where
	AccountId: Ord + Clone,
{
	cluster_id: ClusterId,
	era: EhdEra,
	irf: u8,
	// todo(yahortsaryk): make private after deprecating `InspectionReceipt`
	pub(crate) paths: BTreeMap<InspPathId, InspPath>,
	assignments: BTreeMap<InspPathId, Vec<AccountId>>,
}

impl<AccountId> InspAssignmentsTable<AccountId>
where
	AccountId: Ord + Clone,
{
	#[allow(clippy::map_entry)]
	pub fn build<T: Config>(
		cluster_id: ClusterId,
		era: EhdEra,
		irf: u8,
		backups_count: u8,
		paths: Vec<InspPath>,
		inspectors: Vec<AccountId>,
		// todo(yahortsaryk): build seed from the last paid era hash
		seed: u64,
	) -> Result<Self, InspAssignmentError> {
		let mut paths = paths;
		let mut inspectors = inspectors;

		let mut small_rng = SmallRng::seed_from_u64(seed);
		paths.shuffle(&mut small_rng);
		inspectors.shuffle(&mut small_rng);

		let mut assignments: BTreeMap<AccountId, Vec<InspPathId>> = Default::default();

		// todo(yahortsaryk): define the number of inspectors dynamically based on paths total
		// count
		for inspector in &inspectors {
			assignments.insert(inspector.clone(), Vec::new());
		}

		// assign paths with circular indexing and replication factor
		let mut inspection_paths: BTreeMap<InspPathId, InspPath> = Default::default();

		for (i, path) in paths.into_iter().enumerate() {
			let path_id = path.hash::<T>();
			inspection_paths.insert(path_id, path);

			for j in 0..(irf + backups_count) as usize {
				let assigned_idx = (i + j) % inspectors.len();
				assignments
					.get_mut(&inspectors[assigned_idx])
					.ok_or(InspAssignmentError::NoInspectorAtIdx(assigned_idx as u64))?
					.push(path_id);
			}
		}

		let mut inspectors_assignments: BTreeMap<InspPathId, Vec<AccountId>> = Default::default();
		for (inspector, paths_ids) in assignments {
			for path_id in paths_ids {
				let path_assignments =
					inspectors_assignments.entry(path_id).or_insert_with(Vec::new);
				path_assignments.push(inspector.clone());

				let path_seed: u64 =
					path_id[..8].try_into().map(u64::from_le_bytes).unwrap_or(seed);
				let mut rng = SmallRng::seed_from_u64(path_seed);

				path_assignments.shuffle(&mut rng);
			}
		}

		Ok(InspAssignmentsTable {
			cluster_id,
			era,
			irf,
			paths: inspection_paths,
			assignments: inspectors_assignments,
		})
	}

	pub fn get_assigned_paths(&self, inspector: &AccountId) -> BTreeMap<InspPathId, InspPath> {
		let mut result: BTreeMap<InspPathId, InspPath> = Default::default();
		for (path_id, inspectors) in &self.assignments {
			if inspectors.contains(inspector) {
				if let Some(path) = self.paths.get(path_id) {
					result.insert(*path_id, path.clone());
				}
			}
		}
		result
	}
}

#[derive(Debug, Encode, Decode, Clone, TypeInfo, PartialEq)]
pub enum InspectionError {
	NoGCollectors(ClusterId),
	NoCollectors(TcaEra),
	NoBucketAggregate(BucketId),
	AssignmentsError(InspAssignmentError),
	Unexpected,
}

#[allow(dead_code)]
pub(crate) enum InspSyncStatus<AccountId>
where
	AccountId: Ord + Clone,
{
	NotReadyForInspection,
	AssigningInspectors { era: EhdEra, assigner: AccountId, lease_ttl: u32, salt: u64 },
	ReadyForInspection { assignments_table: InspAssignmentsTable<AccountId> },
}

pub(crate) struct InspTaskAssigner<T: Config> {
	inspector: Rc<Account<T>>,
}
pub(crate) struct InspEraAssignments {
	era: EhdEra,
	assigned_paths: BTreeMap<InspPathId, InspPath>,
}

impl<T: Config> InspTaskAssigner<T> {
	pub(crate) fn new(inspector: Rc<Account<T>>) -> Self {
		InspTaskAssigner { inspector }
	}

	pub(crate) fn try_get_assignments(
		&self,
		cluster_id: &ClusterId,
	) -> Result<Option<InspEraAssignments>, InspAssignmentError> {
		if let Some(assignments_table) = &self.try_get_assignments_table(cluster_id)? {
			let inspector_account = &self.inspector.public.clone().into_account();
			let assigned_paths = assignments_table.get_assigned_paths(inspector_account);
			let era_assignments = InspEraAssignments { era: assignments_table.era, assigned_paths };
			Ok(Some(era_assignments))
		} else {
			Ok(None)
		}
	}

	pub(crate) fn try_get_assignments_table(
		&self,
		cluster_id: &ClusterId,
	) -> Result<Option<InspAssignmentsTable<T::AccountId>>, InspAssignmentError> {
		let Some(era) = Self::try_get_era_to_inspect(cluster_id)? else {
			return Ok(None);
		};

		let alice_account = T::AccountId::decode(&mut &ALICE.encode()[..])
			.map_err(|_| InspAssignmentError::Unexpected)?;
		let inspector_account = &self.inspector.public.clone().into_account();

		let sync_status = if *inspector_account == alice_account {
			InspSyncStatus::<T::AccountId>::AssigningInspectors {
				era,
				assigner: self.inspector.public.clone().into_account(),
				lease_ttl: 60000,
				salt: 0,
			}
		} else {
			if let Ok(assignments_table) = get_assignments_table::<T>(cluster_id, era) {
				InspSyncStatus::ReadyForInspection { assignments_table }
			} else {
				InspSyncStatus::NotReadyForInspection
			}
		};

		match sync_status {
			InspSyncStatus::NotReadyForInspection => {
				log::info!("*** Waiting for assignments table");
				Ok(None)
			},
			InspSyncStatus::AssigningInspectors { era, assigner, lease_ttl: _lease_ttl, salt } => {
				let inspector_account = &self.inspector.public.clone().into_account();
				if *inspector_account == assigner {
					log::info!("*** Building assignments table");
					let assignmens_table = &self.build_assignments_table(cluster_id, &era, salt)?;
					// todo(yahortsaryk): optimize performance by eliminating cloning of the whole
					// table
					let _ = send_assignments_table::<T>(cluster_id, assignmens_table.clone())
						.map_err(|_| InspAssignmentError::Unexpected)?;
					Ok(Some(assignmens_table.clone()))
				} else {
					log::info!("*** Waiting for assignments table to be completed");

					Ok(None)
				}
			},
			InspSyncStatus::ReadyForInspection { assignments_table } => {
				log::info!("*** Obtained assignments table");

				Ok(Some(assignments_table))
			},
		}
	}

	/// Fetch current era across all DAC nodes to validate.
	///
	/// Parameters:
	/// - `cluster_id`: cluster id of a cluster
	/// - `g_collectors`: List of G-Collectors nodes
	fn try_get_era_to_inspect(
		cluster_id: &ClusterId,
	) -> Result<Option<EhdEra>, InspAssignmentError> {
		let g_collectors = get_g_collectors_nodes(cluster_id)
			.map_err(|_: Error<T>| InspAssignmentError::NoGCollectors(*cluster_id))?;

		let last_inspected_ehd_by_this_validator = Self::try_get_last_inspected_ehd(cluster_id);

		let last_inspected_era_by_this_validator: EhdEra =
			if let Some(ehd) = last_inspected_ehd_by_this_validator {
				ehd.2
			} else {
				Default::default()
			};

		let last_paid_era_for_cluster: EhdEra = T::ClusterValidator::get_last_paid_era(cluster_id)
			.map_err(|_| InspAssignmentError::ClusterError(*cluster_id))?;

		log::info!(
				"👁️‍🗨️  The last era inspected by this inspector for cluster_id: {:?} is {:?}. The last paid era for the cluster is {:?}",
				cluster_id,
				last_inspected_era_by_this_validator,
				last_paid_era_for_cluster
			);

		// we want to fetch processed eras from all available G-Collectors
		let available_processed_ehd_eras = fetch_processed_eras::<T>(cluster_id, &g_collectors)
			.map_err(|_| InspAssignmentError::ClusterApiError(*cluster_id))?;

		// we want to let the current inspector to inspect available processed/completed eras
		// that are greater than the last validated era in the cluster
		let processed_eras_to_inspect: Vec<aggregator_client::json::EHDEra> =
			available_processed_ehd_eras
				.iter()
				.flat_map(|eras| {
					eras.iter()
						.filter(|&ids| {
							ids.id > last_inspected_era_by_this_validator &&
								ids.id > last_paid_era_for_cluster
						})
						.cloned()
				})
				.sorted()
				.collect::<Vec<aggregator_client::json::EHDEra>>();

		// we want to process only eras reported by quorum of G-Collector
		let mut processed_eras_with_quorum: Vec<aggregator_client::json::EHDEra> = vec![];

		// todo(yahortsaryk): agree on the result within a quorum of G-Collectors
		// let quorum = T::AggregatorsQuorum::get();
		// let threshold = quorum * collectors_nodes.len();

		let threshold = 1;
		for (era_key, candidates) in
			&processed_eras_to_inspect.into_iter().chunk_by(|elt| elt.clone())
		{
			let count = candidates.count();
			if count >= threshold {
				processed_eras_with_quorum.push(era_key);
			} else {
				log::warn!(
						"⚠️  Era {:?} in cluster_id: {:?} has been reported with unmet quorum. Desired: {:?} Actual: {:?}",
						era_key,
						cluster_id,
						threshold,
						count
					);
			}
		}

		let era_to_inspect =
			processed_eras_with_quorum.iter().cloned().min_by_key(|e| e.id).map(|e| e.id);
		Ok(era_to_inspect)
	}

	fn try_get_last_inspected_ehd(cluster_id: &ClusterId) -> Option<EHDId> {
		if let Some(last_inspected_ehds) = fetch_last_inspected_ehds(cluster_id) {
			last_inspected_ehds.iter().max_by_key(|ehd| ehd.2).cloned()
		} else {
			None
		}
	}

	fn build_assignments_table(
		&self,
		cluster_id: &ClusterId,
		era: &EhdEra,
		salt: u64,
	) -> Result<InspAssignmentsTable<T::AccountId>, InspAssignmentError> {
		let mut inspection_paths: Vec<InspPath> = Vec::new();

		let (ehd_root, ehd_inspection_paths) = build_ehd_inspection_paths::<T>(cluster_id, era)?;
		inspection_paths.extend(ehd_inspection_paths);

		let mut phd_roots = vec![];
		for phd_id in &ehd_root.pdh_ids {
			let phd_root = get_phd_root::<T>(cluster_id, phd_id.clone())
				.map_err(|_| InspAssignmentError::NoPHD(*era, phd_id.clone().0))?;
			phd_roots.push(phd_root.clone());
		}

		// todo(yahortsaryk): build tasks for PHDs and put them to `inspection_paths`

		inspection_paths.extend(build_nodes_inspection_paths::<T>(&phd_roots)?);
		inspection_paths.extend(build_buckets_inspection_paths::<T>(&phd_roots)?);

		let inspectors = <ValidatorSet<T>>::get().clone();
		let assignments_table = InspAssignmentsTable::<T::AccountId>::build::<T>(
			*cluster_id,
			*era,
			INSPECTION_REDUNDANCY_FACTOR,
			INSPECTION_BACKUPS_COUNT,
			inspection_paths,
			inspectors,
			salt,
		)?;

		Ok(assignments_table)
	}
}

#[derive(Debug, Encode, Decode, Clone, TypeInfo, PartialEq)]
pub enum InspAssignmentError {
	NoNodeTCAs(NodePubKey),
	NoNodeTCA(TcaEra, NodePubKey),
	NoNodeTCAVar(TcaEra, NodePubKey),
	NoBucketTCAs(BucketId),
	NoBucketTCA(TcaEra, BucketId),
	NoBucketTCAVar(TcaEra, BucketId),
	NoInspectorAtIdx(u64),
	NoEHDs(EhdEra, ClusterId),
	NoEHD(EhdEra, NodePubKey),
	NoPHD(EhdEra, NodePubKey),
	NoGCollectors(ClusterId),
	NoCollectors(TcaEra),
	NoBucketAggregate(BucketId),
	ClusterError(ClusterId),
	ClusterApiError(ClusterId),
	Unexpected,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialOrd, Ord, TypeInfo, Eq, PartialEq)]
pub enum InspPath {
	NodeAR {
		/// Data Node to inspect.
		node: DataNodeKey,
		/// Time capsule to inspect.
		tca_id: TcaEra,
		/// Merkle tree leaf IDs in NodeAggregate tree.
		leaves_ids: Vec<u64>,
		/// Collectors where the NodeAggregate for Data Node is available.
		collectors: BTreeSet<CollectorNodeKey>,
	},
	BucketAR {
		/// Bucket to inspect.
		bucket: BucketId,
		/// Time capsule to inspect.
		tca_id: TcaEra,
		/// Merkle tree leaf Positions in Cumulative BucketAggregate tree that is composed of
		/// multiple BucketSubAggregates. Position index starts from 0.
		leaves_pos: Vec<u64>,
		/// Collectors where the BucketAggregate for Data Node is available.
		collectors: BTreeSet<CollectorNodeKey>,
	},
}

impl Hashable for InspPath {
	fn hash<T: Config>(&self) -> InspPathId {
		match self {
			InspPath::NodeAR { node, tca_id, leaves_ids, collectors } => {
				let mut data = node.encode();
				data.extend_from_slice(&tca_id.encode());
				data.extend_from_slice(&leaves_ids.encode());
				data.extend_from_slice(&collectors.encode());
				T::Hasher::hash(&data)
			},
			InspPath::BucketAR { bucket, tca_id, leaves_pos, collectors } => {
				let mut data = bucket.encode();
				data.extend_from_slice(&tca_id.encode());
				data.extend_from_slice(&leaves_pos.encode());
				data.extend_from_slice(&collectors.encode());
				T::Hasher::hash(&data)
			},
		}
	}
}

#[derive(
	Debug, Clone, Encode, Decode, Deserialize, Serialize, PartialOrd, Ord, TypeInfo, Eq, PartialEq,
)]
pub struct InspPathReceipt {
	pub path_id: InspPathId,
	pub is_verified: bool,
	pub exception: Option<InspPathException>,
}

impl Hashable for InspPathReceipt {
	fn hash<T: Config>(&self) -> InspPathReceiptHash {
		let mut data = self.path_id.encode();
		data.extend_from_slice(&self.is_verified.encode());
		if let Some(exception) = &self.exception {
			let exception_hash = exception.hash::<T>();
			data.extend_from_slice(&exception_hash.encode());
		}
		T::Hasher::hash(&data)
	}
}

#[derive(
	Debug, Clone, Deserialize, Serialize, Encode, Decode, PartialOrd, Ord, TypeInfo, Eq, PartialEq,
)]
pub enum InspPathException {
	NodeAR { bad_leaves_ids: Vec<u64> },
	BucketAR { bad_leaves_pos: Vec<u64> },
}

impl Hashable for InspPathException {
	fn hash<T: Config>(&self) -> InspPathId {
		match self {
			InspPathException::NodeAR { bad_leaves_ids } => {
				let data = bad_leaves_ids.encode();
				T::Hasher::hash(&data)
			},
			InspPathException::BucketAR { bad_leaves_pos } => {
				let data = bad_leaves_pos.encode();
				T::Hasher::hash(&data)
			},
		}
	}
}

#[allow(clippy::collapsible_else_if)]
fn build_ehd_inspection_paths<T: Config>(
	cluster_id: &ClusterId,
	era: &EhdEra,
) -> Result<(EHDTreeNode, Vec<InspPath>), InspAssignmentError> {
	let inspection_paths: Vec<InspPath> = Default::default();
	let g_collectors = get_g_collectors_nodes(cluster_id)
		.map_err(|_: Error<T>| InspAssignmentError::NoEHDs(*era, *cluster_id))?;

	let mut ehd_variants: BTreeMap<EHDTreeNode, BTreeSet<GCollectorNodeKey>> = Default::default();

	for (g_collector_key, _) in g_collectors {
		let ehd_id = EHDId(*cluster_id, g_collector_key.clone(), *era);
		let ehd_root = get_ehd_root::<T>(cluster_id, ehd_id)
			.map_err(|_| InspAssignmentError::NoEHD(*era, g_collector_key.clone()))?;

		if !ehd_variants.contains_key(&ehd_root) {
			ehd_variants.insert(ehd_root.clone(), BTreeSet::from([g_collector_key.clone()]));
		} else {
			let ehd_reporters =
				ehd_variants.get_mut(&ehd_root.clone()).ok_or(InspAssignmentError::Unexpected)?;
			ehd_reporters.insert(g_collector_key.clone());
		}
	}

	let (ehd_root, _) = if ehd_variants.len() == 1 {
		ehd_variants.first_key_value().ok_or(InspAssignmentError::Unexpected)?
	} else {
		log::debug!(
			"ClusterId {:?} / Era {:?}: {:?} of EHD variants detected.",
			*cluster_id,
			era,
			ehd_variants.len()
		);

		// todo(yahortsaryk): build tasks for in deep inspection of deviating
		// G-Collectors on EHD and put them to `inspection_paths`
		ehd_variants
			.iter()
			.max_by_key(|(_, g_collectors)| g_collectors.len())
			.ok_or(InspAssignmentError::NoEHDs(*era, *cluster_id))?
	};

	// todo(yahortsaryk): build tasks for canonical EHD and put them to `inspection_paths`

	Ok((ehd_root.clone(), inspection_paths))
}

#[allow(clippy::map_entry)]
#[allow(clippy::collapsible_else_if)]
#[allow(clippy::extra_unused_type_parameters)]
fn build_nodes_inspection_paths<T: Config>(
	phd_roots: &Vec<PHDTreeNode>,
) -> Result<Vec<InspPath>, InspAssignmentError> {
	let mut inspection_paths: Vec<InspPath> = Default::default();

	#[allow(clippy::type_complexity)]
	let mut nodes_tcas_map: BTreeMap<
		DataNodeKey,
		BTreeMap<TcaEra, BTreeMap<NodeUsage, BTreeSet<CollectorNodeKey>>>,
	> = BTreeMap::new();

	for phd_root in phd_roots {
		let collector_key = phd_root.phd_id.0.clone();
		for (node_key, node_tcas) in &phd_root.nodes_aggregates {
			for tca_usage in node_tcas {
				if !nodes_tcas_map.contains_key(node_key) {
					nodes_tcas_map.insert(node_key.clone(), Default::default());
				}
				let tcas_map = nodes_tcas_map
					.get_mut(node_key)
					.ok_or(InspAssignmentError::NoNodeTCAs(node_key.clone()))?;

				if !tcas_map.contains_key(&tca_usage.tca_id) {
					tcas_map.insert(tca_usage.tca_id, Default::default());
				}
				let tca_variants = tcas_map
					.get_mut(&tca_usage.tca_id)
					.ok_or(InspAssignmentError::NoNodeTCA(tca_usage.tca_id, node_key.clone()))?;

				if !tca_variants.contains_key(&tca_usage.clone().into()) {
					tca_variants
						.insert(tca_usage.clone().into(), BTreeSet::from([collector_key.clone()]));
				} else {
					let tca_variant = tca_variants.get_mut(&tca_usage.clone().into()).ok_or(
						InspAssignmentError::NoNodeTCAVar(tca_usage.tca_id, node_key.clone()),
					)?;
					tca_variant.insert(collector_key.clone());
				}
			}
		}
	}

	let mut nodes_inspection_timelines: BTreeMap<DataNodeKey, BTreeSet<(TcaEra, NodeUsage)>> =
		Default::default();

	for (node_key, node_tcas) in &nodes_tcas_map {
		let mut node_timeline: BTreeSet<(TcaEra, NodeUsage)> = Default::default();

		for (i, (tca_id, tca_variants)) in node_tcas.iter().enumerate() {
			// todo(yahortsaryk): define optimal step for TCAs dynamically depending on
			// benchmarks and honor TCAs with higher economic value
			if TCA_INSPECTION_STEP != 0 && (i + 1) % TCA_INSPECTION_STEP == 0 {
				continue;
			};

			let (tca_usage, _) = if tca_variants.len() == 1 {
				tca_variants
					.first_key_value()
					.ok_or(InspAssignmentError::NoNodeTCAVar(*tca_id, node_key.clone()))?
			} else {
				log::debug!(
					"Node {:?} / TCAA {:?}: {:?} of aggregation variants detected.",
					node_key.clone(),
					tca_id,
					tca_variants.len()
				);

				// todo(yahortsaryk): build tasks for in deep inspection of deviating
				// Collectors on TCA and put them to `inspection_paths`
				tca_variants
					.iter()
					.max_by_key(|(_, collectors)| collectors.len())
					.ok_or(InspAssignmentError::NoNodeTCAVar(*tca_id, node_key.clone()))?
			};

			node_timeline.insert((*tca_id, tca_usage.clone()));
		}

		nodes_inspection_timelines.insert(node_key.clone(), node_timeline);
	}

	// todo(yahortsaryk): select desired probability of detecting at least one tampered leaf
	// dynamically based on cluster track record and benchmarks
	let n0 = calculate_sample_size_inf(D_099, P_001);

	for (node_key, node_timeline) in nodes_inspection_timelines {
		let timeline_tcas_count = node_timeline.len() as u64;
		let timeline_leaves_count = node_timeline.iter().fold(0u64, |acc, (_, tca_usage)| {
			acc.saturating_add(tca_usage.number_of_puts)
				.saturating_add(tca_usage.number_of_gets)
		});

		let n = match calculate_sample_size_fin(n0, timeline_leaves_count) {
			Ok(n) => n,
			// todo(yahortsaryk): add better error handling and fallback for unsuccessful
			// sample calculation
			Err(_) => continue,
		};

		let n_per_tca = n / timeline_tcas_count;
		if n_per_tca == 0 {
			// todo(yahortsaryk): add better error handling and fallback for unsuccessful
			// sample calculation
			continue;
		}

		let mut remainder = n % timeline_tcas_count;

		for (tca_id, tca_usage) in node_timeline {
			let tca_leaves_count =
				tca_usage.number_of_puts.saturating_add(tca_usage.number_of_gets);
			let tca_leaves_ids = get_leaves_ids(tca_leaves_count);
			let ids_count = tca_leaves_ids.len() as u64;

			let tca_leaves_to_inspect = if n_per_tca < ids_count {
				let sample_size = if remainder > 0 && (n_per_tca + remainder) <= ids_count {
					let size = n_per_tca + remainder;
					remainder = 0;
					size
				} else {
					n_per_tca
				};

				// it should be ok to select leaves randomly (not deterministically) as only
				// one Inspector builds the assignment table at a time
				select_random_leaves(sample_size, tca_leaves_ids, node_key.clone().into())
			} else {
				remainder += n_per_tca - ids_count;
				tca_leaves_ids
			};

			log::debug!(
            "Node {:?} - TCAA {:?}. Selecting {:?} leaves out of {:?} for Node AR inspection path. Selected leaves ids {:?}. Additional reminder is {:?}.",
            node_key.clone(),
            tca_id,
            n_per_tca,
            ids_count,
            tca_leaves_to_inspect,
            remainder
        );

			let collectors = nodes_tcas_map
				.get(&node_key)
				.ok_or(InspAssignmentError::NoNodeTCAs(node_key.clone()))?
				.get(&tca_id)
				.ok_or(InspAssignmentError::NoNodeTCA(tca_id, node_key.clone()))?
				.get(&tca_usage.clone())
				.ok_or(InspAssignmentError::NoNodeTCAVar(tca_id, node_key.clone()))?;

			inspection_paths.push(InspPath::NodeAR {
				node: node_key.clone(),
				tca_id,
				leaves_ids: tca_leaves_to_inspect,
				collectors: collectors.clone(),
			});
		}
	}

	Ok(inspection_paths)
}

#[allow(clippy::map_entry)]
#[allow(clippy::collapsible_else_if)]
#[allow(clippy::extra_unused_type_parameters)]
fn build_buckets_inspection_paths<T: Config>(
	phd_roots: &Vec<PHDTreeNode>,
) -> Result<Vec<InspPath>, InspAssignmentError> {
	let mut inspection_paths: Vec<InspPath> = Default::default();

	#[allow(clippy::type_complexity)]
	let mut buckets_tcas_map: BTreeMap<
		BucketId,
		BTreeMap<TcaEra, BTreeMap<BucketUsage, BTreeSet<CollectorNodeKey>>>,
	> = BTreeMap::new();

	for phd_root in phd_roots {
		let collector_key = phd_root.phd_id.0.clone();
		for (bucket_id, bucket_tcas) in &phd_root.buckets_aggregates {
			for tca_usage in bucket_tcas {
				if !buckets_tcas_map.contains_key(bucket_id) {
					buckets_tcas_map.insert(*bucket_id, Default::default());
				}
				let tcas_map = buckets_tcas_map
					.get_mut(bucket_id)
					.ok_or(InspAssignmentError::NoBucketTCAs(*bucket_id))?;

				if !tcas_map.contains_key(&tca_usage.tca_id) {
					tcas_map.insert(tca_usage.tca_id, Default::default());
				}
				let tca_variants = tcas_map
					.get_mut(&tca_usage.tca_id)
					.ok_or(InspAssignmentError::NoBucketTCA(tca_usage.tca_id, *bucket_id))?;

				if !tca_variants.contains_key(&tca_usage.clone().into()) {
					tca_variants
						.insert(tca_usage.clone().into(), BTreeSet::from([collector_key.clone()]));
				} else {
					let tca_variant = tca_variants
						.get_mut(&tca_usage.clone().into())
						.ok_or(InspAssignmentError::NoBucketTCAVar(tca_usage.tca_id, *bucket_id))?;
					tca_variant.insert(collector_key.clone());
				}
			}
		}
	}

	let mut buckets_inspection_timelines: BTreeMap<BucketId, BTreeSet<(TcaEra, BucketUsage)>> =
		Default::default();

	for (bucket_id, bucket_tcas) in &buckets_tcas_map {
		let mut bucket_timeline: BTreeSet<(TcaEra, BucketUsage)> = Default::default();

		for (i, (tca_id, tca_variants)) in bucket_tcas.iter().enumerate() {
			// todo(yahortsaryk): define optimal step for TCAs dynamically depending on
			// benchmarks and honor TCAs with higher economic value
			if TCA_INSPECTION_STEP != 0 && (i + 1) % TCA_INSPECTION_STEP == 0 {
				continue;
			};

			let (tca_usage, _) = if tca_variants.len() == 1 {
				tca_variants
					.first_key_value()
					.ok_or(InspAssignmentError::NoBucketTCAVar(*tca_id, *bucket_id))?
			} else {
				log::debug!(
					"Node {:?} / TCAA {:?}: {:?} of aggregation variants detected.",
					bucket_id,
					tca_id,
					tca_variants.len()
				);

				// todo(yahortsaryk): build tasks for in deep inspection of deviating
				// Collectors on TCA and put them to `inspection_paths`
				tca_variants
					.iter()
					.max_by_key(|(_, collectors)| collectors.len())
					.ok_or(InspAssignmentError::NoBucketTCAVar(*tca_id, *bucket_id))?
			};

			bucket_timeline.insert((*tca_id, tca_usage.clone()));
		}

		buckets_inspection_timelines.insert(*bucket_id, bucket_timeline);
	}

	// todo(yahortsaryk): select desired probability of detecting at least one tampered leaf
	// dynamically based on cluster track record and benchmarks
	let n0 = calculate_sample_size_inf(D_099, P_001);

	for (bucket_id, bucket_timeline) in buckets_inspection_timelines {
		let timeline_tcas_count = bucket_timeline.len() as u64;
		let timeline_leaves_count = bucket_timeline.iter().fold(0u64, |acc, (_, tca_usage)| {
			acc.saturating_add(tca_usage.number_of_puts)
				.saturating_add(tca_usage.number_of_gets)
		});

		let n = match calculate_sample_size_fin(n0, timeline_leaves_count) {
			Ok(n) => n,
			// todo(yahortsaryk): add better error handling and fallback for unsuccessful
			// sample calculation
			Err(_) => continue,
		};

		let n_per_tca = n / timeline_tcas_count;
		if n_per_tca == 0 {
			// todo(yahortsaryk): add better error handling and fallback for unsuccessful
			// sample calculation
			continue;
		}

		let mut remainder = n % timeline_tcas_count;

		for (tca_id, tca_usage) in bucket_timeline {
			let tca_leaves_count =
				tca_usage.number_of_puts.saturating_add(tca_usage.number_of_gets);
			let tca_leaves_positions: Vec<u64> = (0u64..tca_leaves_count).collect();
			let positions_count = tca_leaves_positions.len() as u64;

			let tca_positions_to_inspect = if n_per_tca < positions_count {
				let sample_size = if remainder > 0 && (n_per_tca + remainder) <= positions_count {
					let size = n_per_tca + remainder;
					remainder = 0;
					size
				} else {
					n_per_tca
				};

				// it should be ok to select leaves randomly (not deterministically) as only
				// one Inspector builds the assignment table at a time
				select_random_leaves(sample_size, tca_leaves_positions, format!("{}", bucket_id))
			} else {
				remainder += n_per_tca - positions_count;
				tca_leaves_positions
			};

			log::debug!(
            "Bucket {:?} - TCAA {:?}. Selecting {:?} leaves out of {:?} for Bucket AR inspection path. Selected leaves positions {:?}. Additional reminder is {:?}.",
            bucket_id,
            tca_id,
            n_per_tca,
            positions_count,
            tca_positions_to_inspect,
            remainder
        );

			let collectors = buckets_tcas_map
				.get(&bucket_id)
				.ok_or(InspAssignmentError::NoBucketTCAs(bucket_id))?
				.get(&tca_id)
				.ok_or(InspAssignmentError::NoBucketTCA(tca_id, bucket_id))?
				.get(&tca_usage.clone())
				.ok_or(InspAssignmentError::NoBucketTCAVar(tca_id, bucket_id))?;

			inspection_paths.push(InspPath::BucketAR {
				bucket: bucket_id,
				tca_id,
				leaves_pos: tca_positions_to_inspect,
				collectors: collectors.clone(),
			});
		}
	}

	Ok(inspection_paths)
}

#[allow(clippy::assign_op_pattern)]
#[allow(clippy::collapsible_else_if)]
fn process_tasks<T: Config>(
	cluster_id: &ClusterId,
	tasks: Vec<&InspTask>,
) -> Result<Vec<InspPathReceipt>, InspectionError> {
	let mut results: Vec<InspPathReceipt> = Default::default();

	let mut cached_bucket_aggregates: BTreeMap<
		(BucketId, TcaEra),
		aggregator_client::json::BucketAggregateResponse,
	> = Default::default();

	for task in tasks {
		let path_id = task.path_id;

		match &task.path {
			InspPath::NodeAR { node: node_key, tca_id, leaves_ids, collectors } => {
				let collector =
					collectors.first().cloned().ok_or(InspectionError::NoCollectors(*tca_id))?;

				// todo(yahortsaryk): in case the request fails due to collector
				// unavailability, re-try with the next one
				if let Ok(challenge_res) = fetch_node_challenge_response::<T>(
					cluster_id,
					*tca_id,
					collector.clone(),
					node_key.clone(),
					leaves_ids.clone(),
				) {
					// todo(yahortsaryk): fix AR signatures
					let is_verified = challenge_res.verify();
					let exception: Option<_> = if is_verified {
						None
					} else {
						// todo(yahortsaryk): add only bad leaves to exceptions
						Some(InspPathException::NodeAR { bad_leaves_ids: leaves_ids.clone() })
					};
					let path_receipt = InspPathReceipt { path_id, is_verified, exception };
					results.push(path_receipt);
				}
			},
			InspPath::BucketAR { bucket: bucket_id, tca_id, leaves_pos, collectors } => {
				let collector =
					collectors.first().cloned().ok_or(InspectionError::NoCollectors(*tca_id))?;

				// todo(yahortsaryk): in case the request fails due to collector
				// unavailability, re-try with the next one
				let bucket_aggregate =
					if let Some(aggregate) = cached_bucket_aggregates.get(&(*bucket_id, *tca_id)) {
						aggregate
					} else {
						let aggregates =
							fetch_bucket_aggregates::<T>(cluster_id, *tca_id, collector.clone())
								.map_err(|_| InspectionError::NoBucketAggregate(*bucket_id))?;

						for mut aggregate in aggregates {
							aggregate.sub_aggregates.sort_by_key(|subagg| subagg.NodeID.clone());
							cached_bucket_aggregates
								.entry((aggregate.bucket_id, *tca_id))
								.or_insert(aggregate.clone());
						}

						cached_bucket_aggregates
							.get(&(*bucket_id, *tca_id))
							.ok_or(InspectionError::NoBucketAggregate(*bucket_id))?
					};

				let mut total_subaggs_leaf_count: u64 = 0;

				for (j, sub_aggregate) in bucket_aggregate.sub_aggregates.iter().enumerate() {
					let subagg_leaves_count =
						sub_aggregate.number_of_puts.saturating_add(sub_aggregate.number_of_gets);

					let subagg_leaves_ids = get_leaves_ids(subagg_leaves_count);

					let mut subagg_leaves_to_inspect: Vec<u64> = Vec::new();

					for (i, subagg_leave_id) in subagg_leaves_ids.iter().enumerate() {
						let leaf_pos =
							if j == 0 { i as u64 } else { i as u64 + total_subaggs_leaf_count };

						if leaves_pos.contains(&leaf_pos) {
							subagg_leaves_to_inspect.push(*subagg_leave_id);
						}
					}

					total_subaggs_leaf_count =
						total_subaggs_leaf_count + subagg_leaves_ids.len() as u64;

					if subagg_leaves_to_inspect.is_empty() {
						// there are no leaves this sub-aggregate to inspect
						continue;
					}

					if let Ok(challenge_res) = fetch_bucket_challenge_response::<T>(
						cluster_id,
						*tca_id,
						collector.clone(),
						// todo(yahortsaryk): fix NodeID field in DTO
						sub_aggregate
							.NodeID
							.clone()
							.try_into()
							.map_err(|_| InspectionError::Unexpected)?,
						*bucket_id,
						subagg_leaves_to_inspect.clone(),
					) {
						// todo(yahortsaryk): fix AR signatures
						let is_verified = challenge_res.verify();
						let exception: Option<_> = if is_verified {
							None
						} else {
							// todo(yahortsaryk): add only bad leaves to exceptions
							Some(InspPathException::BucketAR { bad_leaves_pos: leaves_pos.clone() })
						};
						let path_receipt = InspPathReceipt { path_id, is_verified, exception };
						results.push(path_receipt);
					}
				}
			},
		}
	}

	Ok(results)
}

struct InspTask {
	path_id: InspPathId,
	path: InspPath,
	priority: u8,
}

#[allow(dead_code)]
pub struct InspTaskManager<T: Config> {
	inspector: Rc<Account<T>>,
	assigner: InspTaskAssigner<T>,
	// currently we don't support pending eras in the pool, so there will be always one era per
	// cluster in inspection processing
	tasks_pool: BTreeMap<ClusterId, (EhdEra, Vec<InspTask>)>,
}

impl<T: Config> InspTaskManager<T> {
	pub(crate) fn new(inspector: Account<T>) -> Self {
		let inspector = Rc::new(inspector);
		let assigner = InspTaskAssigner::new(inspector.clone());
		InspTaskManager { inspector, assigner, tasks_pool: Default::default() }
	}

	pub(crate) fn assign_cluster(&mut self, cluster_id: &ClusterId) -> Result<(), InspectionError> {
		if let Some(era_assignments) = self
			.assigner
			.try_get_assignments(cluster_id)
			.map_err(InspectionError::AssignmentsError)?
		{
			self.add_inspection_tasks(
				*cluster_id,
				era_assignments.era,
				era_assignments.assigned_paths,
			);
		}

		Ok(())
	}

	fn add_inspection_tasks(
		&mut self,
		cluster_id: ClusterId,
		era: EhdEra,
		insp_paths: BTreeMap<InspPathId, InspPath>,
	) {
		self.tasks_pool.entry(cluster_id).or_insert((
			era,
			insp_paths
				.into_iter()
				.map(|(path_id, path)| {
					// todo(yahortsaryk): set the priority based on inspector's assignment
					// (main/backup)
					InspTask { path_id, path, priority: 1 }
				})
				.collect::<Vec<_>>(),
		));
	}

	pub(crate) fn inspect_cluster(
		&mut self,
		cluster_id: &ClusterId,
	) -> Result<Option<InspEraResult>, InspectionError> {
		if let Some((era, insp_tasks)) = &self.tasks_pool.remove(cluster_id) {
			let prioritized_tasks =
				insp_tasks.iter().sorted_by_key(|task| task.priority).collect::<Vec<_>>();
			let era_results = process_tasks::<T>(cluster_id, prioritized_tasks)?;

			Ok(Some(InspEraResult { era: *era, receipts: era_results }))
		} else {
			Ok(None)
		}
	}
}

#[derive(
	Debug, Clone, Encode, Decode, Deserialize, Serialize, PartialOrd, Ord, TypeInfo, Eq, PartialEq,
)]
pub(crate) struct InspEraResult {
	pub(crate) era: EhdEra,
	pub(crate) receipts: Vec<InspPathReceipt>,
}

pub(crate) fn select_random_leaves(
	sample_size: u64,
	leaves_ids: Vec<u64>,
	nonce_key: String,
) -> Vec<u64> {
	let nonce = store_and_fetch_nonce(nonce_key);
	let mut small_rng = SmallRng::seed_from_u64(nonce);

	leaves_ids
		.choose_multiple(&mut small_rng, sample_size.try_into().unwrap())
		.cloned()
		.sorted()
		.collect::<Vec<u64>>()
}

pub(crate) fn store_and_fetch_nonce(node_id: String) -> u64 {
	let key = format!("offchain::activities::nonce::{:?}", node_id).into_bytes();
	let encoded_nonce =
		local_storage_get(StorageKind::PERSISTENT, &key).unwrap_or_else(|| 0.encode());

	let nonce_data = match Decode::decode(&mut &encoded_nonce[..]) {
		Ok(nonce) => nonce,
		Err(err) => {
			log::error!("Decoding error while fetching nonce: {:?}", err);
			0
		},
	};

	let new_nonce = nonce_data + 1;

	local_storage_set(StorageKind::PERSISTENT, &key, &new_nonce.encode());
	nonce_data
}
