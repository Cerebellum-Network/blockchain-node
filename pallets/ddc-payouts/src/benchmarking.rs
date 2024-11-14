//! DdcPayouts pallet benchmarking.

use ddc_primitives::{
	traits::ValidatorVisitor, ActivityHash, BucketParams, ClusterId, ClusterParams,
	ClusterProtocolParams, EraValidation, EraValidationStatus, MergeActivityHash, NodeParams,
	NodePubKey, StorageNodeMode, StorageNodeParams,
};
pub use frame_benchmarking::{account, benchmarks, whitelist_account};
use frame_system::RawOrigin;
use polkadot_ckb_merkle_mountain_range::{
	util::{MemMMR, MemStore},
	MMR,
};
use scale_info::prelude::{collections::BTreeMap, format};
use sp_runtime::{AccountId32, Perquintill};
use sp_std::prelude::*;

use super::*;
use crate::Pallet as DdcPayouts;

const CERE: u128 = 10000000000;
const AVG_SECONDS_MONTH: i64 = 2630016; // 30.44 * 24.0 * 3600.0;

fn create_dac_account<T: Config>() -> T::AccountId {
	let dac_account = create_account::<T>("dac_account", 0, 0);
	T::ValidatorVisitor::setup_validators(vec![(dac_account.clone(), dac_account.clone())]);
	dac_account
}

fn create_account<T: Config>(name: &'static str, idx: u32, seed: u32) -> T::AccountId {
	account::<T::AccountId>(name, idx, seed)
}

fn endow_account<T: Config>(account: &T::AccountId, amount: u128) {
	let balance = amount.saturated_into::<BalanceOf<T>>();
	let _ = T::Currency::make_free_balance_be(account, balance);
}

fn endow_customer<T: Config>(customer: &T::AccountId, amount: u128) {
	endow_account::<T>(customer, amount);
	T::CustomerDepositor::deposit(
		customer.clone(),
		// we need to keep min existensial deposit
		amount - T::Currency::minimum_balance().saturated_into::<u128>(),
	)
	.expect("Customer deposit failed");
}

fn create_cluster<T: Config>(
	cluster_id: ClusterId,
	cluster_manager_id: T::AccountId,
	cluster_reserve_id: T::AccountId,
	cluster_params: ClusterParams<T::AccountId>,
	cluster_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>,
) {
	T::ClusterCreator::create_cluster(
		cluster_id,
		cluster_manager_id,
		cluster_reserve_id,
		cluster_params,
		cluster_protocol_params,
	)
	.expect("Cluster is not created");
}

fn create_default_cluster<T: Config>(cluster_id: ClusterId) {
	let cluster_manager = create_account::<T>("cm", 0, 0);
	let cluster_reserve = create_account::<T>("cr", 0, 0);
	let cluster_params = ClusterParams {
		node_provider_auth_contract: Default::default(),
		erasure_coding_required: 4,
		erasure_coding_total: 6,
		replication_total: 3,
	};
	let cluster_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>> =
		ClusterProtocolParams {
			treasury_share: Perquintill::from_percent(5),
			validators_share: Perquintill::from_percent(10),
			cluster_reserve_share: Perquintill::from_percent(15),
			unit_per_mb_stored: CERE,
			unit_per_mb_streamed: CERE,
			unit_per_put_request: CERE,
			unit_per_get_request: CERE,
			..Default::default()
		};

	create_cluster::<T>(
		cluster_id,
		cluster_manager,
		cluster_reserve,
		cluster_params,
		cluster_protocol_params,
	);
}

struct BillingReportParams {
	cluster_id: ClusterId,
	era: DdcEra,
	state: PayoutState,
	total_customer_charge: CustomerCharge,
	total_distributed_reward: u128,
	total_node_usage: NodeUsage,
	charging_max_batch_index: BatchIndex,
	charging_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
	rewarding_max_batch_index: BatchIndex,
	rewarding_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
}

fn create_billing_report<T: Config>(params: BillingReportParams) {
	let vault = DdcPayouts::<T>::sub_account_id(params.cluster_id, params.era);
	let start_era: i64 = 1_000_000_000;
	let end_era: i64 = start_era + AVG_SECONDS_MONTH;

	let billing_report = BillingReport::<T> {
		vault,
		start_era,
		end_era,
		state: params.state,
		total_customer_charge: params.total_customer_charge,
		total_distributed_reward: params.total_distributed_reward,
		total_node_usage: params.total_node_usage,
		charging_max_batch_index: params.charging_max_batch_index,
		charging_processed_batches: params.charging_processed_batches,
		rewarding_max_batch_index: params.rewarding_max_batch_index,
		rewarding_processed_batches: params.rewarding_processed_batches,
	};

	ActiveBillingReports::<T>::insert(params.cluster_id, params.era, billing_report);
}

fn set_validation_era<T: Config>(
	cluster_id: ClusterId,
	era_id: DdcEra,
	era_validation: EraValidation<T>,
) {
	T::ValidatorVisitor::setup_validation_era(cluster_id, era_id, era_validation);
}

benchmarks! {

	begin_billing_report {

		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		create_default_cluster::<T>(cluster_id);

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era, start_era, end_era)
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, PayoutState::Initialized);
	}

	begin_charging_customers {
		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = PayoutState::Initialized;
		let total_customer_charge = CustomerCharge::default();
		let total_distributed_reward : u128= 0;
		let total_node_usage = NodeUsage::default();
		let charging_max_batch_index = BatchIndex::default();
		let charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		let rewarding_max_batch_index = BatchIndex::default();
		let rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge,
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

		let max_batch_index: BatchIndex = 10;

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era, max_batch_index)
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, PayoutState::ChargingCustomers);
		assert_eq!(billing_report.charging_max_batch_index, max_batch_index);
	}

	send_charging_customers_batch {
		let b in 1 .. MaxBatchSize::get() as u32;

		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = PayoutState::ChargingCustomers;
		let total_customer_charge = CustomerCharge::default();
		let total_distributed_reward : u128 = 0;
		let total_node_usage = NodeUsage::default();
		let charging_max_batch_index = 0;
		let charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		let rewarding_max_batch_index = BatchIndex::default();
		let rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge,
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let batch_index: BatchIndex = 0;
		let mut payers_batch: Vec<(NodePubKey, BucketId, CustomerUsage)> = vec![];
		for i in 0..b {
			let customer = create_account::<T>("customer", i, i);

			if b % 2 == 0 {
				// no customer debt path
				endow_customer::<T>(&customer, 1_000_000 * CERE);
			} else {
				// customer debt path
				endow_customer::<T>(&customer, 10 * CERE);
			}

			let customer_usage = CustomerUsage {
				transferred_bytes: 200000000, // 200 mb
				stored_bytes: 100000000, // 100 mb
				number_of_gets: 10, // 10 gets
				number_of_puts: 5, // 5 puts
			};
			let bucket_id: BucketId = (i + 1).into();
			let node_key = NodePubKey::StoragePubKey(AccountId32::from([
				48, 47, 147, 125, 243, 160, 236, 76, 101, 142, 129, 34, 67, 158, 116, 141, 34, 116, 66,
				235, 212, 147, 206, 245, 33, 161, 225, 73, 67, 132, 67, 149,
			]));

			T::BucketManager::create_bucket(
				&cluster_id,
				bucket_id,
				customer,
				BucketParams { is_public: true }
			).expect("Bucket to be created");

			payers_batch.push((node_key, bucket_id, customer_usage));
		}

		let activity_hashes = payers_batch.clone().into_iter().map(|(node_key, bucket_id, usage)| {
			let mut data = bucket_id.encode();
			let node_id = format!("0x{}", node_key.get_hex());
			data.extend_from_slice(&node_id.encode());
			data.extend_from_slice(&usage.stored_bytes.encode());
			data.extend_from_slice(&usage.transferred_bytes.encode());
			data.extend_from_slice(&usage.number_of_puts.encode());
			data.extend_from_slice(&usage.number_of_gets.encode());
			sp_io::hashing::blake2_256(&data)
		}).collect::<Vec<_>>();

		let store1 = MemStore::default();
		let mut mmr1: MMR<ActivityHash, MergeActivityHash, &MemStore<ActivityHash>> = MemMMR::<ActivityHash, MergeActivityHash>::new(0, &store1);
		for activity_hash in activity_hashes {
			let _pos: u64 = mmr1.push(activity_hash).unwrap();
		}

		let batch_root = mmr1.get_root().unwrap();

		let store2 = MemStore::default();
		let mut mmr2: MMR<ActivityHash, MergeActivityHash, &MemStore<ActivityHash>> = MemMMR::<ActivityHash, MergeActivityHash>::new(0, &store2);
		let pos = mmr2.push(batch_root).unwrap();
		let payers_merkle_root_hash = mmr2.get_root().unwrap();

		let proof = mmr2
			.gen_proof(vec![pos])
			.unwrap()
			.proof_items()
			.to_vec();

		let payees_merkle_root_hash = ActivityHash::default();

		let mut validators = BTreeMap::new();
		validators.insert(
			(payers_merkle_root_hash, payees_merkle_root_hash),
			vec![dac_account.clone()],
		);
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;
		let era_validation = EraValidation::<T> {
			validators,
			start_era,
			end_era,
			payers_merkle_root_hash,
			payees_merkle_root_hash,
			status: EraValidationStatus::PayoutInProgress,
		};

		set_validation_era::<T>(cluster_id, era, era_validation);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era, batch_index, payers_batch, MMRProof { proof })
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, PayoutState::ChargingCustomers);
		assert!(billing_report.charging_processed_batches.contains(&batch_index));
	}

	end_charging_customers {
		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = PayoutState::ChargingCustomers;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE, // price for 100 mb
			gets: 10 * CERE, // price for 10 gets
			puts: 5 * CERE, // price for 5 puts
		};
		let total_distributed_reward : u128 = 0;
		let total_node_usage = NodeUsage::default();
		let charging_max_batch_index = 0;
		let mut charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		charging_processed_batches.try_insert(0).unwrap();
		let rewarding_max_batch_index = BatchIndex::default();
		let rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge: total_customer_charge.clone(),
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let vault = DdcPayouts::<T>::sub_account_id(cluster_id, era);
		let total_customer_charge_amount = total_customer_charge.transfer + total_customer_charge.storage + total_customer_charge.gets + total_customer_charge.puts;
		endow_account::<T>(&vault, total_customer_charge_amount);

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era)
	verify {
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, PayoutState::CustomersChargedWithFees);
		assert!(billing_report.charging_processed_batches.contains(&charging_max_batch_index));
	}

	begin_rewarding_providers {
		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = PayoutState::CustomersChargedWithFees;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE, // price for 100 mb
			gets: 10 * CERE, // price for 10 gets
			puts: 5 * CERE, // price for 5 puts
		};
		let total_distributed_reward : u128 = 0;
		let total_node_usage = NodeUsage::default();
		let charging_max_batch_index = 0;
		let mut charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		charging_processed_batches.try_insert(0).unwrap();
		let rewarding_max_batch_index = BatchIndex::default();
		let rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge,
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let max_batch_index: BatchIndex = 10;
		let total_node_usage = NodeUsage {
			transferred_bytes: 200000000, // 200 mb
			stored_bytes: 100000000, // 100 mb
			number_of_gets: 10, // 10 gets
			number_of_puts: 5, // 5 puts
		};

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era, max_batch_index, total_node_usage)
	verify {
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, PayoutState::RewardingProviders);
		assert_eq!(billing_report.rewarding_max_batch_index, max_batch_index);
	}

	send_rewarding_providers_batch {
		let b in 1 .. MaxBatchSize::get() as u32;

		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = PayoutState::RewardingProviders;
		let total_customer_charge = CustomerCharge {
			transfer: (200 * CERE).saturating_mul(b.into()), // price for 200 mb per customer
			storage: (100 * CERE).saturating_mul(b.into()), // price for 100 mb per customer
			gets: (10 * CERE).saturating_mul(b.into()), // price for 10 gets per customer
			puts: (5 * CERE).saturating_mul(b.into()), // price for 5 puts per customer
		};
		let total_distributed_reward : u128 = 0;
		let total_node_usage = NodeUsage {
			transferred_bytes: 200000000u64.saturating_mul(b.into()), // 200 mb per provider
			stored_bytes: 100000000i64.saturating_mul(b.into()), // 100 mb per provider
			number_of_gets: 10u64.saturating_mul(b.into()), // 10 gets per provider
			number_of_puts: 10u64.saturating_mul(b.into()), // 5 puts per provider
		};
		let charging_max_batch_index = 0;
		let mut charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		charging_processed_batches.try_insert(0).unwrap();
		let rewarding_max_batch_index = 0;
		let rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge: total_customer_charge.clone(),
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let vault = DdcPayouts::<T>::sub_account_id(cluster_id, era);
		let total_customer_charge_amount = total_customer_charge.transfer + total_customer_charge.storage + total_customer_charge.gets + total_customer_charge.puts;
		endow_account::<T>(&vault, total_customer_charge_amount + T::Currency::minimum_balance().saturated_into::<u128>());

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

		let batch_index: BatchIndex = 0;
		let mut payees_batch: Vec<(NodePubKey, NodeUsage)> = vec![];

		for i in 0..b {
			let provider = create_account::<T>("provider", i, i);

			endow_account::<T>(&provider, T::Currency::minimum_balance().saturated_into());

			let usage = NodeUsage {
				transferred_bytes: 200000000, // 200 mb
				stored_bytes: 100000000, // 100 mb
				number_of_gets: 10, // 10 gets
				number_of_puts: 5, // 5 puts
			};

			let key = sp_io::hashing::blake2_256(&i.encode());
			let node_key = NodePubKey::StoragePubKey(AccountId32::from(key));

			T::NodeManager::create_node(
				node_key.clone(),
				provider,
				NodeParams::StorageParams(StorageNodeParams {
					mode: StorageNodeMode::Storage,
					host: vec![1u8; 255],
					domain: vec![2u8; 255],
					ssl: true,
					http_port: 35000u16,
					grpc_port: 25000u16,
					p2p_port: 15000u16,
				}),
			).expect("Node to be created");

			payees_batch.push((node_key, usage));
		}

		let activity_hashes = payees_batch.clone().into_iter().map(|(node_key, usage)| {
			let mut data = format!("0x{}", node_key.get_hex()).encode();
			data.extend_from_slice(&usage.stored_bytes.encode());
			data.extend_from_slice(&usage.transferred_bytes.encode());
			data.extend_from_slice(&usage.number_of_puts.encode());
			data.extend_from_slice(&usage.number_of_gets.encode());
			sp_io::hashing::blake2_256(&data)
		}).collect::<Vec<_>>();

		let store1 = MemStore::default();
		let mut mmr1: MMR<ActivityHash, MergeActivityHash, &MemStore<ActivityHash>> = MemMMR::<ActivityHash, MergeActivityHash>::new(0, &store1);
		for activity_hash in activity_hashes {
			let _pos: u64 = mmr1.push(activity_hash).unwrap();
		}

		let batch_root = mmr1.get_root().unwrap();

		let store2 = MemStore::default();
		let mut mmr2: MMR<ActivityHash, MergeActivityHash, &MemStore<ActivityHash>> = MemMMR::<ActivityHash, MergeActivityHash>::new(0, &store2);
		let pos = mmr2.push(batch_root).unwrap();
		let payees_merkle_root_hash = mmr2.get_root().unwrap();

		let payers_merkle_root_hash = ActivityHash::default();

		let proof = mmr2
			.gen_proof(vec![pos])
			.unwrap()
			.proof_items()
			.to_vec();

		let mut validators = BTreeMap::new();
		validators.insert(
			(payers_merkle_root_hash, payees_merkle_root_hash),
			vec![dac_account.clone()],
		);
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;
		let era_validation = EraValidation::<T> {
			validators,
			start_era,
			end_era,
			payers_merkle_root_hash,
			payees_merkle_root_hash,
			status: EraValidationStatus::PayoutInProgress,
		};

		set_validation_era::<T>(cluster_id, era, era_validation);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era, batch_index, payees_batch, MMRProof { proof })
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, PayoutState::RewardingProviders);
		assert!(billing_report.rewarding_processed_batches.contains(&batch_index));
	}

	end_rewarding_providers {
		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = PayoutState::RewardingProviders;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE, // price for 100 mb
			gets: 10 * CERE, // price for 10 gets
			puts: 5 * CERE, // price for 5 puts
		};
		let total_distributed_reward : u128 = total_customer_charge.transfer + total_customer_charge.storage + total_customer_charge.gets + total_customer_charge.puts;
		let total_node_usage = NodeUsage {
			transferred_bytes: 200000000, // 200 mb
			stored_bytes: 100000000, // 100 mb
			number_of_gets: 10, // 10 gets
			number_of_puts: 5, // 5 puts
		};
		let charging_max_batch_index = 0;
		let mut charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		charging_processed_batches.try_insert(0).unwrap();
		let rewarding_max_batch_index = 0;
		let mut rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		rewarding_processed_batches.try_insert(0).unwrap();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge,
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era)
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, PayoutState::ProvidersRewarded);
	}

	end_billing_report {
		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = PayoutState::ProvidersRewarded;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE, // price for 100 mb
			gets: 10 * CERE, // price for 10 gets
			puts: 5 * CERE, // price for 5 puts
		};
		let total_distributed_reward : u128 = total_customer_charge.transfer + total_customer_charge.storage + total_customer_charge.gets + total_customer_charge.puts;
		let total_node_usage = NodeUsage {
			transferred_bytes: 200000000, // 200 mb
			stored_bytes: 100000000, // 100 mb
			number_of_gets: 10, // 10 gets
			number_of_puts: 5, // 5 puts
		};
		let charging_max_batch_index = 0;
		let mut charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		charging_processed_batches.try_insert(0).unwrap();
		let rewarding_max_batch_index = 0;
		let mut rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		rewarding_processed_batches.try_insert(0).unwrap();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge,
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era)
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, PayoutState::Finalized);
	}

}
