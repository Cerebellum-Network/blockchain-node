#![cfg(feature = "runtime-benchmarks")]

use ddc_primitives::{
	BillingFingerprintParams, BillingReportParams, BucketId, BucketParams, ClusterId,
	ClusterParams, ClusterProtocolParams, CustomerCharge, DeltaUsageHash, EraValidation,
	EraValidationStatus, MergeMMRHash, NodeParams, NodePubKey, NodeUsage, PayoutState,
	StorageNodeMode, StorageNodeParams, AVG_SECONDS_MONTH, DOLLARS as CERE, MAX_PAYOUT_BATCH_SIZE,
};
use frame_benchmarking::{account, v2::*, whitelist_account};
use frame_system::RawOrigin;
use sp_core::H256;
use sp_io::hashing::{blake2_128, blake2_256};
use sp_runtime::{
	traits::AccountIdConversion, AccountId32, Perquintill, SaturatedConversion, Saturating,
};
use sp_std::{collections::btree_set::BTreeSet, vec};

use super::*;
use crate::EraActivity;
#[allow(unused)]
use crate::Pallet as DdcVerification;

#[benchmarks]
mod benchmarks {
	use super::*;

	fn create_validator_account<T: Config>() -> T::AccountId {
		let validator = create_account::<T>("validator_account", 0, 0);
		ValidatorToStashKey::<T>::insert(&validator.clone(), &validator.clone());
		ValidatorSet::<T>::put(vec![validator.clone()]);
		validator
	}

	fn create_account<T: Config>(name: &'static str, idx: u32, seed: u32) -> T::AccountId {
		account::<T::AccountId>(name, idx, seed)
	}

	fn assert_has_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
		frame_system::Pallet::<T>::assert_has_event(generic_event.into());
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

	fn setup_validation_era<T: Config>(
		cluster_id: ClusterId,
		era_id: DdcEra,
		validators: Vec<T::AccountId>,
		payers_merkle_root_hash: DeltaUsageHash,
		payees_merkle_root_hash: DeltaUsageHash,
		status: EraValidationStatus,
	) {
		let mut validations_map = BTreeMap::new();
		for validator in validators {
			validations_map.insert(
				(payers_merkle_root_hash, payees_merkle_root_hash),
				vec![validator.clone()],
			);
		}

		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;

		let era_validation = EraValidation::<T> {
			validators: validations_map,
			start_era,
			end_era,
			payers_merkle_root_hash,
			payees_merkle_root_hash,
			status,
		};

		<EraValidations<T>>::insert(cluster_id, era_id, era_validation);
	}

	#[allow(clippy::too_many_arguments)]
	fn create_billing_report<T: Config>(
		cluster_id: ClusterId,
		era_id: DdcEra,
		start_era: i64,
		end_era: i64,
		state: PayoutState,
		total_customer_charge: CustomerCharge,
		total_distributed_reward: u128,
		cluster_usage: NodeUsage,
		charging_max_batch_index: BatchIndex,
		charging_processed_batches: Vec<BatchIndex>,
		rewarding_max_batch_index: BatchIndex,
		rewarding_processed_batches: Vec<BatchIndex>,
		payers_merkle_root: H256,
		payees_merkle_root: H256,
		validators: Vec<T::AccountId>,
	) {
		let hash = blake2_128(&0.encode());
		let vault = T::PalletId::get().into_sub_account_truncating(hash);

		let total_customer_charge_amount = total_customer_charge.transfer +
			total_customer_charge.storage +
			total_customer_charge.gets +
			total_customer_charge.puts;

		endow_account::<T>(&vault, total_customer_charge_amount);

		let mut validators_set = BTreeSet::new();
		for validator in validators {
			validators_set.insert(validator);
		}

		let fingerprint =
			T::PayoutProcessor::create_billing_fingerprint(BillingFingerprintParams {
				cluster_id,
				era: era_id,
				start_era,
				end_era,
				payers_merkle_root,
				payees_merkle_root,
				cluster_usage,
				validators: validators_set,
			});

		T::PayoutProcessor::create_billing_report(
			vault.clone(),
			BillingReportParams {
				cluster_id,
				era: era_id,
				state,
				fingerprint,
				total_customer_charge,
				total_distributed_reward,
				charging_max_batch_index,
				charging_processed_batches,
				rewarding_max_batch_index,
				rewarding_processed_batches,
			},
		);
	}

	#[benchmark]
	fn set_prepare_era_for_payout(b: Linear<1, 5>) {
		let validator = create_validator_account::<T>();
		let cluster_id = ClusterId::from([1; 20]);
		let era = EraActivity { id: 1, start: 1000, end: 2000 };

		let payers_merkle_root_hash = H256(blake2_256(&1.encode()));
		let payees_merkle_root_hash = H256(blake2_256(&2.encode()));

		let mut payers_batch_merkle_root_hashes = vec![];
		let mut payees_batch_merkle_root_hashes = vec![];

		for i in 0..b {
			payers_batch_merkle_root_hashes.push(H256(blake2_256(&(i + 10).encode())));
			payees_batch_merkle_root_hashes.push(H256(blake2_256(&(i + 100).encode())))
		}

		#[extrinsic_call]
		set_prepare_era_for_payout(
			RawOrigin::Signed(validator),
			cluster_id,
			era,
			payers_merkle_root_hash,
			payees_merkle_root_hash,
			payers_batch_merkle_root_hashes,
			payees_batch_merkle_root_hashes,
		);

		assert!(<EraValidations<T>>::contains_key(cluster_id, era.id));
		assert_has_event::<T>(Event::EraValidationReady { cluster_id, era_id: era.id }.into());
	}

	#[benchmark]
	fn set_validator_key() {
		let validator = create_account::<T>("new_validator_account", 0, 0);
		endow_account::<T>(&validator, 10000000000000000);

		let min_bond = T::ValidatorStaking::minimum_validator_bond();
		let bond = min_bond.saturating_add(T::Currency::minimum_balance());

		T::ValidatorStaking::bond(&validator, bond, &validator).expect("Bond to be created");

		ValidatorSet::<T>::put(vec![validator.clone()]);

		#[extrinsic_call]
		set_validator_key(RawOrigin::Signed(validator.clone()), validator.clone());

		assert!(<ValidatorToStashKey<T>>::contains_key(validator.clone()));
		assert_has_event::<T>(Event::ValidatorKeySet { validator }.into());
	}

	#[benchmark]
	fn commit_billing_fingerprint() {
		let cluster_id = ClusterId::from([1; 20]);
		let era_id: DdcEra = 1;
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));
		let cluster_usage = NodeUsage::default();

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);
		setup_validation_era::<T>(
			cluster_id,
			era_id,
			vec![validator.clone()],
			H256(blake2_256(&1.encode())),
			H256(blake2_256(&2.encode())),
			EraValidationStatus::ReadyForPayout,
		);

		#[extrinsic_call]
		commit_billing_fingerprint(
			RawOrigin::Signed(validator),
			cluster_id,
			era_id,
			start_era,
			end_era,
			payers_merkle_root,
			payees_merkle_root,
			cluster_usage,
		);

		let status = T::PayoutProcessor::get_billing_report_status(&cluster_id, era_id);
		assert_eq!(status, PayoutState::NotInitialized);
	}

	#[benchmark]
	fn begin_billing_report() {
		let cluster_id = ClusterId::from([1; 20]);
		let era_id: DdcEra = 1;
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);
		setup_validation_era::<T>(
			cluster_id,
			era_id,
			vec![validator.clone()],
			H256(blake2_256(&1.encode())),
			H256(blake2_256(&2.encode())),
			EraValidationStatus::ReadyForPayout,
		);

		let mut validators = BTreeSet::new();
		validators.insert(validator.clone());

		let fingerprint =
			T::PayoutProcessor::create_billing_fingerprint(BillingFingerprintParams {
				cluster_id,
				era: era_id,
				start_era,
				end_era,
				payers_merkle_root: H256(blake2_256(&3.encode())),
				payees_merkle_root: H256(blake2_256(&4.encode())),
				cluster_usage: NodeUsage::default(),
				validators,
			});

		#[extrinsic_call]
		begin_billing_report(RawOrigin::Signed(validator), cluster_id, era_id, fingerprint);

		let status = T::PayoutProcessor::get_billing_report_status(&cluster_id, era_id);
		assert_eq!(status, PayoutState::Initialized);
	}

	#[benchmark]
	fn begin_charging_customers() {
		let cluster_id = ClusterId::from([1; 20]);
		let era_id: DdcEra = 1;
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;
		let state = PayoutState::Initialized;
		let total_customer_charge = Default::default();
		let total_distributed_reward: u128 = 0;
		let cluster_usage = Default::default();
		let charging_max_batch_index = 10;
		let charging_processed_batches = Default::default();
		let rewarding_max_batch_index = Default::default();
		let rewarding_processed_batches = Default::default();
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);

		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		setup_validation_era::<T>(
			cluster_id,
			era_id,
			vec![validator.clone()],
			H256(blake2_256(&1.encode())),
			H256(blake2_256(&2.encode())),
			EraValidationStatus::ReadyForPayout,
		);

		create_billing_report::<T>(
			cluster_id,
			era_id,
			start_era,
			end_era,
			state,
			total_customer_charge,
			total_distributed_reward,
			cluster_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
			payers_merkle_root,
			payees_merkle_root,
			vec![validator.clone()],
		);

		#[extrinsic_call]
		begin_charging_customers(
			RawOrigin::Signed(validator),
			cluster_id,
			era_id,
			charging_max_batch_index,
		);

		let status = T::PayoutProcessor::get_billing_report_status(&cluster_id, era_id);
		assert_eq!(status, PayoutState::ChargingCustomers);
	}

	#[benchmark]
	fn send_charging_customers_batch(b: Linear<1, { MAX_PAYOUT_BATCH_SIZE.into() }>) {
		let cluster_id = ClusterId::from([1; 20]);
		let era_id: DdcEra = 1;
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;
		let state = PayoutState::ChargingCustomers;
		let total_customer_charge = Default::default();
		let total_distributed_reward: u128 = 0;
		let cluster_usage = Default::default();
		let charging_max_batch_index = 0;
		let charging_processed_batches = Default::default();
		let rewarding_max_batch_index = Default::default();
		let rewarding_processed_batches = Default::default();

		create_default_cluster::<T>(cluster_id);

		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		let batch_index: BatchIndex = 0;
		let mut payers_batch: Vec<(BucketId, BucketUsage)> = vec![];
		for i in 0..b {
			let customer = create_account::<T>("customer", i, i);

			if b % 2 == 0 {
				// no customer debt path
				endow_customer::<T>(&customer, 1_000_000 * CERE);
			} else {
				// customer debt path
				endow_customer::<T>(&customer, 10 * CERE);
			}

			let customer_usage = BucketUsage {
				transferred_bytes: 200000000, // 200 mb
				stored_bytes: 100000000,      // 100 mb
				number_of_gets: 10,           // 10 gets
				number_of_puts: 5,            // 5 puts
			};

			let bucket_id: BucketId = (i + 1).into();
			T::BucketManager::create_bucket(
				&cluster_id,
				bucket_id,
				customer,
				BucketParams { is_public: true },
			)
			.expect("Bucket to be created");

			payers_batch.push((bucket_id, customer_usage));
		}

		let activity_hashes = payers_batch
			.clone()
			.into_iter()
			.map(|(bucket_id, usage)| {
				let mut data = bucket_id.encode();
				data.extend_from_slice(&usage.stored_bytes.encode());
				data.extend_from_slice(&usage.transferred_bytes.encode());
				data.extend_from_slice(&usage.number_of_puts.encode());
				data.extend_from_slice(&usage.number_of_gets.encode());
				H256(blake2_256(&data))
			})
			.collect::<Vec<_>>();

		let store1 = MemStore::default();
		let mut mmr1: MMR<DeltaUsageHash, MergeMMRHash, &MemStore<DeltaUsageHash>> =
			MemMMR::<DeltaUsageHash, MergeMMRHash>::new(0, &store1);
		for activity_hash in activity_hashes {
			let _pos: u64 = mmr1.push(activity_hash).unwrap();
		}

		let batch_root = mmr1.get_root().unwrap();

		let store2 = MemStore::default();
		let mut mmr2: MMR<DeltaUsageHash, MergeMMRHash, &MemStore<DeltaUsageHash>> =
			MemMMR::<DeltaUsageHash, MergeMMRHash>::new(0, &store2);
		let pos = mmr2.push(batch_root).unwrap();
		let payers_merkle_root_hash = mmr2.get_root().unwrap();

		let proof = mmr2.gen_proof(vec![pos]).unwrap().proof_items().to_vec();

		let payees_merkle_root_hash = DeltaUsageHash::default();

		setup_validation_era::<T>(
			cluster_id,
			era_id,
			vec![validator.clone()],
			H256(blake2_256(&1.encode())),
			H256(blake2_256(&2.encode())),
			EraValidationStatus::PayoutInProgress,
		);

		create_billing_report::<T>(
			cluster_id,
			era_id,
			start_era,
			end_era,
			state,
			total_customer_charge,
			total_distributed_reward,
			cluster_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
			payers_merkle_root_hash,
			payees_merkle_root_hash,
			vec![validator.clone()],
		);

		#[extrinsic_call]
		send_charging_customers_batch(
			RawOrigin::Signed(validator),
			cluster_id,
			era_id,
			batch_index,
			payers_batch,
			MMRProof { proof },
		);

		let status = T::PayoutProcessor::get_billing_report_status(&cluster_id, era_id);
		assert_eq!(status, PayoutState::ChargingCustomers);
		assert!(T::PayoutProcessor::all_customer_batches_processed(&cluster_id, era_id));
	}

	#[benchmark]
	fn end_charging_customers() {
		let cluster_id = ClusterId::from([1; 20]);
		let era_id: DdcEra = 1;
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;
		let state = PayoutState::ChargingCustomers;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE,  // price for 100 mb
			gets: 10 * CERE,      // price for 10 gets
			puts: 5 * CERE,       // price for 5 puts
		};
		let total_distributed_reward: u128 = 0;
		let cluster_usage = Default::default();
		let charging_max_batch_index = 0;
		let charging_processed_batches = vec![0];
		let rewarding_max_batch_index = Default::default();
		let rewarding_processed_batches = Default::default();
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		setup_validation_era::<T>(
			cluster_id,
			era_id,
			vec![validator.clone()],
			H256(blake2_256(&1.encode())),
			H256(blake2_256(&2.encode())),
			EraValidationStatus::PayoutInProgress,
		);
		create_billing_report::<T>(
			cluster_id,
			era_id,
			start_era,
			end_era,
			state,
			total_customer_charge.clone(),
			total_distributed_reward,
			cluster_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
			payers_merkle_root,
			payees_merkle_root,
			vec![validator.clone()],
		);

		#[extrinsic_call]
		end_charging_customers(RawOrigin::Signed(validator), cluster_id, era_id);

		let status = T::PayoutProcessor::get_billing_report_status(&cluster_id, era_id);
		assert_eq!(status, PayoutState::CustomersChargedWithFees);
		assert!(T::PayoutProcessor::all_customer_batches_processed(&cluster_id, era_id));
	}

	#[benchmark]
	fn begin_rewarding_providers() {
		let cluster_id = ClusterId::from([1; 20]);
		let era_id: DdcEra = 1;
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;
		let state = PayoutState::CustomersChargedWithFees;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE,  // price for 100 mb
			gets: 10 * CERE,      // price for 10 gets
			puts: 5 * CERE,       // price for 5 puts
		};
		let total_distributed_reward: u128 = 0;
		let cluster_usage = Default::default();
		let charging_max_batch_index = 0;
		let charging_processed_batches = vec![0];
		let rewarding_max_batch_index = 10;
		let rewarding_processed_batches = Default::default();
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		setup_validation_era::<T>(
			cluster_id,
			era_id,
			vec![validator.clone()],
			H256(blake2_256(&1.encode())),
			H256(blake2_256(&2.encode())),
			EraValidationStatus::PayoutInProgress,
		);
		create_billing_report::<T>(
			cluster_id,
			era_id,
			start_era,
			end_era,
			state,
			total_customer_charge,
			total_distributed_reward,
			cluster_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
			payers_merkle_root,
			payees_merkle_root,
			vec![validator.clone()],
		);

		#[extrinsic_call]
		begin_rewarding_providers(
			RawOrigin::Signed(validator),
			cluster_id,
			era_id,
			rewarding_max_batch_index,
		);

		let status = T::PayoutProcessor::get_billing_report_status(&cluster_id, era_id);
		assert_eq!(status, PayoutState::RewardingProviders);
	}

	#[benchmark]
	fn send_rewarding_providers_batch(b: Linear<1, { MAX_PAYOUT_BATCH_SIZE.into() }>) {
		let cluster_id = ClusterId::from([1; 20]);
		let era_id: DdcEra = 1;
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;
		let state = PayoutState::RewardingProviders;
		let total_customer_charge = CustomerCharge {
			transfer: (200 * CERE).saturating_mul(b.into()), // price for 200 mb per customer
			storage: (100 * CERE).saturating_mul(b.into()),  // price for 100 mb per customer
			gets: (10 * CERE).saturating_mul(b.into()),      // price for 10 gets per customer
			puts: (5 * CERE).saturating_mul(b.into()),       // price for 5 puts per customer
		};
		let total_distributed_reward: u128 = 0;
		let cluster_usage = NodeUsage {
			transferred_bytes: 200000000u64.saturating_mul(b.into()), // 200 mb per provider
			stored_bytes: 100000000i64.saturating_mul(b.into()),      // 100 mb per provider
			number_of_gets: 10u64.saturating_mul(b.into()),           // 10 gets per provider
			number_of_puts: 10u64.saturating_mul(b.into()),           // 5 puts per provider
		};
		let charging_max_batch_index = 0;
		let charging_processed_batches = vec![0];
		let rewarding_max_batch_index = 0;
		let rewarding_processed_batches = vec![];

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		let batch_index: BatchIndex = 0;
		let mut payees_batch: Vec<(NodePubKey, NodeUsage)> = vec![];

		for i in 0..b {
			let provider = create_account::<T>("provider", i, i);

			endow_account::<T>(&provider, T::Currency::minimum_balance().saturated_into());

			let usage = NodeUsage {
				transferred_bytes: 200000000, // 200 mb
				stored_bytes: 100000000,      // 100 mb
				number_of_gets: 10,           // 10 gets
				number_of_puts: 5,            // 5 puts
			};

			let key = blake2_256(&i.encode());
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
			)
			.expect("Node to be created");

			payees_batch.push((node_key, usage));
		}

		let activity_hashes = payees_batch
			.clone()
			.into_iter()
			.map(|(node_key, usage)| {
				let mut data = node_key.encode();
				data.extend_from_slice(&usage.stored_bytes.encode());
				data.extend_from_slice(&usage.transferred_bytes.encode());
				data.extend_from_slice(&usage.number_of_puts.encode());
				data.extend_from_slice(&usage.number_of_gets.encode());
				H256(blake2_256(&data))
			})
			.collect::<Vec<_>>();

		let store1 = MemStore::default();
		let mut mmr1: MMR<DeltaUsageHash, MergeMMRHash, &MemStore<DeltaUsageHash>> =
			MemMMR::<DeltaUsageHash, MergeMMRHash>::new(0, &store1);
		for activity_hash in activity_hashes {
			let _pos: u64 = mmr1.push(activity_hash).unwrap();
		}

		let batch_root = mmr1.get_root().unwrap();

		let store2 = MemStore::default();
		let mut mmr2: MMR<DeltaUsageHash, MergeMMRHash, &MemStore<DeltaUsageHash>> =
			MemMMR::<DeltaUsageHash, MergeMMRHash>::new(0, &store2);
		let pos = mmr2.push(batch_root).unwrap();
		let payees_merkle_root_hash = mmr2.get_root().unwrap();

		let payers_merkle_root_hash = DeltaUsageHash::default();

		let proof = mmr2.gen_proof(vec![pos]).unwrap().proof_items().to_vec();

		setup_validation_era::<T>(
			cluster_id,
			era_id,
			vec![validator.clone()],
			H256(blake2_256(&1.encode())),
			H256(blake2_256(&2.encode())),
			EraValidationStatus::PayoutInProgress,
		);

		create_billing_report::<T>(
			cluster_id,
			era_id,
			start_era,
			end_era,
			state,
			total_customer_charge,
			total_distributed_reward,
			cluster_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
			payers_merkle_root_hash,
			payees_merkle_root_hash,
			vec![validator.clone()],
		);

		#[extrinsic_call]
		send_rewarding_providers_batch(
			RawOrigin::Signed(validator),
			cluster_id,
			era_id,
			batch_index,
			payees_batch,
			MMRProof { proof },
		);

		let status = T::PayoutProcessor::get_billing_report_status(&cluster_id, era_id);
		assert_eq!(status, PayoutState::RewardingProviders);
		assert!(T::PayoutProcessor::all_provider_batches_processed(&cluster_id, era_id));
	}

	#[benchmark]
	fn end_rewarding_providers() {
		let cluster_id = ClusterId::from([1; 20]);
		let era_id: DdcEra = 1;
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;
		let state = PayoutState::RewardingProviders;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE,  // price for 100 mb
			gets: 10 * CERE,      // price for 10 gets
			puts: 5 * CERE,       // price for 5 puts
		};
		let total_distributed_reward: u128 = total_customer_charge.transfer +
			total_customer_charge.storage +
			total_customer_charge.gets +
			total_customer_charge.puts;
		let cluster_usage = NodeUsage {
			transferred_bytes: 200000000, // 200 mb
			stored_bytes: 100000000,      // 100 mb
			number_of_gets: 10,           // 10 gets
			number_of_puts: 5,            // 5 puts
		};
		let charging_max_batch_index = 0;
		let charging_processed_batches = vec![0];
		let rewarding_max_batch_index = 0;
		let rewarding_processed_batches = vec![0];
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		setup_validation_era::<T>(
			cluster_id,
			era_id,
			vec![validator.clone()],
			H256(blake2_256(&1.encode())),
			H256(blake2_256(&2.encode())),
			EraValidationStatus::PayoutInProgress,
		);
		create_billing_report::<T>(
			cluster_id,
			era_id,
			start_era,
			end_era,
			state,
			total_customer_charge.clone(),
			total_distributed_reward,
			cluster_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
			payers_merkle_root,
			payees_merkle_root,
			vec![validator.clone()],
		);

		#[extrinsic_call]
		end_rewarding_providers(RawOrigin::Signed(validator), cluster_id, era_id);

		let status = T::PayoutProcessor::get_billing_report_status(&cluster_id, era_id);
		assert_eq!(status, PayoutState::ProvidersRewarded);
	}

	#[benchmark]
	fn end_billing_report() {
		let cluster_id = ClusterId::from([1; 20]);
		let era_id: DdcEra = 1;
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + AVG_SECONDS_MONTH;
		let state = PayoutState::ProvidersRewarded;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE,  // price for 100 mb
			gets: 10 * CERE,      // price for 10 gets
			puts: 5 * CERE,       // price for 5 puts
		};
		let total_distributed_reward: u128 = total_customer_charge.transfer +
			total_customer_charge.storage +
			total_customer_charge.gets +
			total_customer_charge.puts;
		let cluster_usage = NodeUsage {
			transferred_bytes: 200000000, // 200 mb
			stored_bytes: 100000000,      // 100 mb
			number_of_gets: 10,           // 10 gets
			number_of_puts: 5,            // 5 puts
		};
		let charging_max_batch_index = 0;
		let charging_processed_batches = vec![0];
		let rewarding_max_batch_index = 0;
		let rewarding_processed_batches = vec![0];
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		setup_validation_era::<T>(
			cluster_id,
			era_id,
			vec![validator.clone()],
			H256(blake2_256(&1.encode())),
			H256(blake2_256(&2.encode())),
			EraValidationStatus::PayoutInProgress,
		);
		create_billing_report::<T>(
			cluster_id,
			era_id,
			start_era,
			end_era,
			state,
			total_customer_charge.clone(),
			total_distributed_reward,
			cluster_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
			payers_merkle_root,
			payees_merkle_root,
			vec![validator.clone()],
		);

		#[extrinsic_call]
		end_billing_report(RawOrigin::Signed(validator), cluster_id, era_id);

		let status = T::PayoutProcessor::get_billing_report_status(&cluster_id, era_id);
		assert_eq!(status, PayoutState::Finalized);
	}

	#[benchmark]
	fn emit_consensus_errors(b: Linear<1, 5>) {
		let validator = create_validator_account::<T>();
		let mut errros = vec![];

		for _i in 0..b {
			errros.push(OCWError::SendChargingCustomersBatchTransactionError {
				cluster_id: ClusterId::from([1; 20]),
				era_id: 1,
				batch_index: 0,
			});
		}

		#[extrinsic_call]
		emit_consensus_errors(RawOrigin::Signed(validator.clone()), errros);
	}

	#[benchmark]
	fn set_era_validations() {
		let cluster_id = ClusterId::from([1; 20]);
		let era_id: DdcEra = 1;

		#[extrinsic_call]
		set_era_validations(RawOrigin::Root, cluster_id, era_id);

		<EraValidations<T>>::contains_key(cluster_id, era_id);
	}
}
