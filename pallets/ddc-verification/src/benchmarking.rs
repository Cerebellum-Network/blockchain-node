use ddc_primitives::{
	ClusterId, ClusterParams, ClusterProtocolParams, DeltaUsageHash, MergeMMRHash, NodePubKey,
	PayoutFingerprintParams, PayoutReceiptParams, PayoutState, DOLLARS as CERE,
	MAX_PAYOUT_BATCH_SIZE,
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
#[allow(unused)]
use crate::Pallet as DdcVerification;

#[benchmarks]
mod benchmarks {
	use super::*;

	fn create_validator_account<T: Config>() -> T::AccountId {
		let validator = create_account::<T>("validator_account", 0, 0);
		ValidatorToStashKey::<T>::insert(validator.clone(), validator.clone());
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

	#[allow(clippy::too_many_arguments)]
	fn create_payout_receipt<T: Config>(
		cluster_id: ClusterId,
		ehd_id: EHDId,
		state: PayoutState,
		total_collected_charges: u128,
		total_distributed_rewards: u128,
		total_settled_fees: u128,
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

		endow_account::<T>(&vault, total_collected_charges);

		let mut validators_set = BTreeSet::new();
		for validator in validators {
			validators_set.insert(validator);
		}

		let fingerprint = T::PayoutProcessor::create_payout_fingerprint(PayoutFingerprintParams {
			cluster_id,
			ehd_id: ehd_id.clone().into(),
			payers_merkle_root,
			payees_merkle_root,
			validators: validators_set,
		});

		T::PayoutProcessor::create_payout_receipt(
			vault.clone(),
			PayoutReceiptParams {
				cluster_id,
				era: ehd_id.2,
				state,
				fingerprint,
				total_collected_charges,
				total_distributed_rewards,
				total_settled_fees,
				charging_max_batch_index,
				charging_processed_batches,
				rewarding_max_batch_index,
				rewarding_processed_batches,
			},
		);
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
	fn commit_payout_fingerprint() {
		let cluster_id = ClusterId::from([1; 20]);
		let payment_era = 1;
		let collector_key = NodePubKey::StoragePubKey(AccountId32::from([0u8; 32]));
		let ehd_id = EHDId(cluster_id, collector_key, payment_era);
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		#[extrinsic_call]
		commit_payout_fingerprint(
			RawOrigin::Signed(validator),
			cluster_id,
			ehd_id.into(),
			payers_merkle_root,
			payees_merkle_root,
		);

		let status = T::PayoutProcessor::get_payout_state(&cluster_id, payment_era);
		assert_eq!(status, PayoutState::NotInitialized);
	}

	#[benchmark]
	fn begin_payout() {
		let cluster_id = ClusterId::from([1; 20]);
		let payment_era = 1;
		let collector_key = NodePubKey::StoragePubKey(AccountId32::from([0u8; 32]));
		let ehd_id = EHDId(cluster_id, collector_key, payment_era);
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		let mut validators = BTreeSet::new();
		validators.insert(validator.clone());

		let fingerprint = T::PayoutProcessor::create_payout_fingerprint(PayoutFingerprintParams {
			cluster_id,
			ehd_id: ehd_id.into(),
			payers_merkle_root,
			payees_merkle_root,
			validators,
		});

		#[extrinsic_call]
		begin_payout(RawOrigin::Signed(validator), cluster_id, payment_era, fingerprint);

		let status = T::PayoutProcessor::get_payout_state(&cluster_id, payment_era);
		assert_eq!(status, PayoutState::Initialized);
	}

	#[benchmark]
	fn begin_charging_customers() {
		let cluster_id = ClusterId::from([1; 20]);
		let payment_era = 1;
		let collector_key = NodePubKey::StoragePubKey(AccountId32::from([0u8; 32]));
		let ehd_id = EHDId(cluster_id, collector_key, payment_era);
		let state = PayoutState::Initialized;
		let total_collected_charges: u128 = 0;
		let total_distributed_rewards: u128 = 0;
		let total_settled_fees: u128 = 0;
		let charging_max_batch_index = 10;
		let charging_processed_batches = Default::default();
		let rewarding_max_batch_index = Default::default();
		let rewarding_processed_batches = Default::default();
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);

		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		create_payout_receipt::<T>(
			cluster_id,
			ehd_id,
			state,
			total_collected_charges,
			total_distributed_rewards,
			total_settled_fees,
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
			payment_era,
			charging_max_batch_index,
		);

		let status = T::PayoutProcessor::get_payout_state(&cluster_id, payment_era);
		assert_eq!(status, PayoutState::ChargingCustomers);
	}

	#[benchmark]
	fn send_charging_customers_batch(b: Linear<1, { MAX_PAYOUT_BATCH_SIZE.into() }>) {
		let cluster_id = ClusterId::from([1; 20]);
		let payment_era: DdcEra = 1;
		let collector_key = NodePubKey::StoragePubKey(AccountId32::from([0u8; 32]));
		let ehd_id = EHDId(cluster_id, collector_key, payment_era);
		let state = PayoutState::ChargingCustomers;
		let total_collected_charges: u128 = 0;
		let total_distributed_rewards: u128 = 0;
		let total_settled_fees: u128 = 0;
		let charging_max_batch_index = 0;
		let charging_processed_batches = Default::default();
		let rewarding_max_batch_index = Default::default();
		let rewarding_processed_batches = Default::default();

		create_default_cluster::<T>(cluster_id);

		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		let batch_index: BatchIndex = 0;
		let mut payers_batch: Vec<(T::AccountId, u128)> = vec![];
		for i in 0..b {
			let customer = create_account::<T>("customer", i, i);

			if b % 2 == 0 {
				// no customer debt path
				endow_customer::<T>(&customer, 1_000_000 * CERE);
			} else {
				// customer debt path
				endow_customer::<T>(&customer, 10 * CERE);
			}

			let charge_amount = if b % 2 == 0 {
				// no customer debt path
				900_000 * CERE
			} else {
				// customer debt path
				20 * CERE
			};

			payers_batch.push((customer, charge_amount));
		}

		let activity_hashes = payers_batch
			.clone()
			.into_iter()
			.map(|(customer_id, amount)| {
				let mut data = customer_id.encode();
				data.extend_from_slice(&amount.encode());
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

		create_payout_receipt::<T>(
			cluster_id,
			ehd_id,
			state,
			total_collected_charges,
			total_distributed_rewards,
			total_settled_fees,
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
			payment_era,
			batch_index,
			payers_batch,
			MMRProof { proof },
		);

		let status = T::PayoutProcessor::get_payout_state(&cluster_id, payment_era);
		assert_eq!(status, PayoutState::ChargingCustomers);
		assert!(T::PayoutProcessor::is_customers_charging_finished(&cluster_id, payment_era));
	}

	#[benchmark]
	fn end_charging_customers() {
		let cluster_id = ClusterId::from([1; 20]);
		let payment_era = 1;
		let collector_key = NodePubKey::StoragePubKey(AccountId32::from([0u8; 32]));
		let ehd_id = EHDId(cluster_id, collector_key, payment_era);
		let state = PayoutState::ChargingCustomers;
		let total_collected_charges: u128 = 315 * CERE;
		let total_distributed_rewards: u128 = 0;
		let total_settled_fees: u128 = 0;
		let charging_max_batch_index = 0;
		let charging_processed_batches = vec![0];
		let rewarding_max_batch_index = Default::default();
		let rewarding_processed_batches = Default::default();
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		create_payout_receipt::<T>(
			cluster_id,
			ehd_id,
			state,
			total_collected_charges,
			total_distributed_rewards,
			total_settled_fees,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
			payers_merkle_root,
			payees_merkle_root,
			vec![validator.clone()],
		);

		#[extrinsic_call]
		end_charging_customers(RawOrigin::Signed(validator), cluster_id, payment_era);

		let status = T::PayoutProcessor::get_payout_state(&cluster_id, payment_era);
		assert_eq!(status, PayoutState::CustomersChargedWithFees);
		assert!(T::PayoutProcessor::is_customers_charging_finished(&cluster_id, payment_era));
	}

	#[benchmark]
	fn begin_rewarding_providers() {
		let cluster_id = ClusterId::from([1; 20]);
		let payment_era = 1;
		let collector_key = NodePubKey::StoragePubKey(AccountId32::from([0u8; 32]));
		let ehd_id = EHDId(cluster_id, collector_key, payment_era);
		let state = PayoutState::CustomersChargedWithFees;
		let total_collected_charges: u128 = 315 * CERE;
		let total_distributed_rewards: u128 = 0;
		let total_settled_fees: u128 = 0;
		let charging_max_batch_index = 0;
		let charging_processed_batches = vec![0];
		let rewarding_max_batch_index = 10;
		let rewarding_processed_batches = Default::default();
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		create_payout_receipt::<T>(
			cluster_id,
			ehd_id,
			state,
			total_collected_charges,
			total_distributed_rewards,
			total_settled_fees,
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
			payment_era,
			rewarding_max_batch_index,
		);

		let status = T::PayoutProcessor::get_payout_state(&cluster_id, payment_era);
		assert_eq!(status, PayoutState::RewardingProviders);
	}

	#[benchmark]
	fn send_rewarding_providers_batch(b: Linear<1, { MAX_PAYOUT_BATCH_SIZE.into() }>) {
		let cluster_id = ClusterId::from([1; 20]);
		let payment_era = 1;
		let collector_key = NodePubKey::StoragePubKey(AccountId32::from([0u8; 32]));
		let ehd_id = EHDId(cluster_id, collector_key, payment_era);
		let state = PayoutState::RewardingProviders;
		let total_collected_charges = (315 * CERE).saturating_mul(b.into());
		let total_distributed_rewards: u128 = 0;
		let total_settled_fees: u128 = 0;
		let charging_max_batch_index = 0;
		let charging_processed_batches = vec![0];
		let rewarding_max_batch_index = 0;
		let rewarding_processed_batches = vec![];

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		let batch_index: BatchIndex = 0;
		let mut payees_batch: Vec<(T::AccountId, u128)> = vec![];

		for i in 0..b {
			let provider = create_account::<T>("provider", i, i);
			endow_account::<T>(&provider, T::Currency::minimum_balance().saturated_into());
			let reward_amount = (10 * CERE).saturating_mul(b.into());
			payees_batch.push((provider, reward_amount));
		}

		let activity_hashes = payees_batch
			.clone()
			.into_iter()
			.map(|(provider_id, amount)| {
				let mut data = provider_id.encode();
				data.extend_from_slice(&amount.encode());
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

		create_payout_receipt::<T>(
			cluster_id,
			ehd_id,
			state,
			total_collected_charges,
			total_distributed_rewards,
			total_settled_fees,
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
			payment_era,
			batch_index,
			payees_batch,
			MMRProof { proof },
		);

		let status = T::PayoutProcessor::get_payout_state(&cluster_id, payment_era);
		assert_eq!(status, PayoutState::RewardingProviders);
		assert!(T::PayoutProcessor::is_providers_rewarding_finished(&cluster_id, payment_era));
	}

	#[benchmark]
	fn end_rewarding_providers() {
		let cluster_id = ClusterId::from([1; 20]);
		let payment_era = 1;
		let collector_key = NodePubKey::StoragePubKey(AccountId32::from([0u8; 32]));
		let ehd_id = EHDId(cluster_id, collector_key, payment_era);
		let state = PayoutState::RewardingProviders;
		let total_collected_charges = 315 * CERE;
		let total_distributed_rewards: u128 = total_collected_charges;
		let total_settled_fees: u128 = 0;
		let charging_max_batch_index = 0;
		let charging_processed_batches = vec![0];
		let rewarding_max_batch_index = 0;
		let rewarding_processed_batches = vec![0];
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		create_payout_receipt::<T>(
			cluster_id,
			ehd_id,
			state,
			total_collected_charges,
			total_distributed_rewards,
			total_settled_fees,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
			payers_merkle_root,
			payees_merkle_root,
			vec![validator.clone()],
		);

		#[extrinsic_call]
		end_rewarding_providers(RawOrigin::Signed(validator), cluster_id, payment_era);

		let status = T::PayoutProcessor::get_payout_state(&cluster_id, payment_era);
		assert_eq!(status, PayoutState::ProvidersRewarded);
	}

	#[benchmark]
	fn end_payout() {
		let cluster_id = ClusterId::from([1; 20]);
		let payment_era = 1;
		let collector_key = NodePubKey::StoragePubKey(AccountId32::from([0u8; 32]));
		let ehd_id = EHDId(cluster_id, collector_key, payment_era);
		let state = PayoutState::ProvidersRewarded;
		let total_collected_charges = 315 * CERE;
		let total_distributed_rewards: u128 = total_collected_charges;
		let total_settled_fees: u128 = 0;
		let charging_max_batch_index = 0;
		let charging_processed_batches = vec![0];
		let rewarding_max_batch_index = 0;
		let rewarding_processed_batches = vec![0];
		let payers_merkle_root = H256(blake2_256(&3.encode()));
		let payees_merkle_root = H256(blake2_256(&4.encode()));

		create_default_cluster::<T>(cluster_id);
		let validator = create_validator_account::<T>();
		whitelist_account!(validator);

		create_payout_receipt::<T>(
			cluster_id,
			ehd_id,
			state,
			total_collected_charges,
			total_distributed_rewards,
			total_settled_fees,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
			payers_merkle_root,
			payees_merkle_root,
			vec![validator.clone()],
		);

		#[extrinsic_call]
		end_payout(RawOrigin::Signed(validator), cluster_id, payment_era);

		let status = T::PayoutProcessor::get_payout_state(&cluster_id, payment_era);
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
	fn force_skip_inspection() {
		let cluster_id = ClusterId::from([1; 20]);
		let payment_era = 1;
		let collector_key = NodePubKey::StoragePubKey(AccountId32::from([0u8; 32]));
		let ehd_id = EHDId(cluster_id, collector_key, payment_era);

		create_default_cluster::<T>(cluster_id);

		#[extrinsic_call]
		force_skip_inspection(RawOrigin::Root, cluster_id, ehd_id.into());
	}
}
