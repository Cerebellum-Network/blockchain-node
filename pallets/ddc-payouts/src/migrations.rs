use frame_support::{migration, storage::unhashed, storage_alias, traits::OnRuntimeUpgrade};
use log;
use sp_runtime::Saturating;

use super::*;

const LOG_TARGET: &str = "ddc-payouts";

pub mod v1 {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct BillingReport<T: Config> {
		pub state: PayoutState,
		pub vault: T::AccountId,
		pub start_era: i64, // removed field
		pub end_era: i64,   // removed field
		pub total_customer_charge: CustomerCharge,
		pub total_distributed_reward: u128,
		pub total_node_usage: NodeUsage, // removed field
		pub charging_max_batch_index: BatchIndex,
		pub charging_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
		pub rewarding_max_batch_index: BatchIndex,
		pub rewarding_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
	}

	#[storage_alias]
	pub type ActiveBillingReports<T: Config> = StorageDoubleMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		DdcEra,
		BillingReport<T>,
	>;

	pub fn migrate_to_v1<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		log::info!(
			target: LOG_TARGET,
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 0 && current_version == 1 {
			log::info!(target: LOG_TARGET, "Running migration to v1.");

			let res = migration::clear_storage_prefix(
				<Pallet<T>>::name().as_bytes(),
				b"AuthorisedCaller",
				b"",
				None,
				None,
			);

			log::info!(
				target: LOG_TARGET,
				"Cleared '{}' entries from 'AuthorisedCaller' storage prefix.",
				res.unique
			);

			if res.maybe_cursor.is_some() {
				log::error!(
					target: LOG_TARGET,
					"Storage prefix 'AuthorisedCaller' is not completely cleared."
				);
			}

			// Update storage version.
			StorageVersion::new(1).put::<Pallet<T>>();
			log::info!(
				target: LOG_TARGET,
				"Storage migrated to version {:?}",
				current_version
			);

			T::DbWeight::get().reads_writes(1, res.unique.into())
		} else {
			log::info!(target: LOG_TARGET, " >>> Unused migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v1::<T>()
		}
	}
}

pub mod v2 {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct BillingReport<T: Config> {
		pub state: PayoutState,
		pub vault: T::AccountId,
		pub fingerprint: Fingerprint,
		pub total_customer_charge: CustomerCharge, // removed field
		pub total_distributed_reward: u128,
		pub charging_max_batch_index: BatchIndex,
		pub charging_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
		pub rewarding_max_batch_index: BatchIndex,
		pub rewarding_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
	}

	#[storage_alias]
	pub type ActiveBillingReports<T: Config> = StorageDoubleMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		DdcEra,
		BillingReport<T>,
	>;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct PayoutFingerprint<AccountId> {
		pub cluster_id: ClusterId,
		pub era_id: DdcEra, // removed field
		pub start_era: i64, // removed field
		pub end_era: i64,   // removed field
		pub payers_merkle_root: PayableUsageHash,
		pub payees_merkle_root: PayableUsageHash,
		pub cluster_usage: NodeUsage,
		pub validators: BTreeSet<AccountId>,
	}

	impl<AccountId> PayoutFingerprint<AccountId> {
		pub fn selective_hash<T: Config>(&self) -> Fingerprint {
			let mut data = self.cluster_id.encode();
			data.extend_from_slice(&self.era_id.encode());
			data.extend_from_slice(&self.start_era.encode());
			data.extend_from_slice(&self.end_era.encode());
			data.extend_from_slice(&self.payers_merkle_root.encode());
			data.extend_from_slice(&self.payees_merkle_root.encode());
			data.extend_from_slice(&self.cluster_usage.encode());
			// we truncate the `validators` field on purpose as it's appendable collection that is
			// used for reaching the quorum on the billing fingerprint
			T::Hasher::hash(&data)
		}
	}

	#[storage_alias]
	pub type BillingFingerprints<T: Config> = StorageMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		Fingerprint,
		PayoutFingerprint<<T as frame_system::Config>::AccountId>,
	>;

	pub fn migrate_to_v2<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		log::info!(
			target: LOG_TARGET,
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 1 && current_version == 2 {
			let mut translated = 0u64;
			let count = v1::ActiveBillingReports::<T>::iter().count();
			log::info!(
				target: LOG_TARGET,
				" >>> Updating Billing Reports. Migrating {} reports...", count
			);

			let mut billing_fingerprints: Vec<PayoutFingerprint<T::AccountId>> = Vec::new();

			v2::ActiveBillingReports::<T>::translate::<v1::BillingReport<T>, _>(
				|cluster_id: ClusterId, era_id: DdcEra, payout_receipt: v1::BillingReport<T>| {
					log::info!(target: LOG_TARGET, "Migrating Billing Report for cluster_id {:?} era_id {:?}", cluster_id, era_id);
					translated.saturating_inc();

					let payout_fingerprint = v2::PayoutFingerprint {
						cluster_id,
						era_id,
						start_era: payout_receipt.start_era,
						end_era: payout_receipt.end_era,
						payers_merkle_root: Default::default(),
						payees_merkle_root: Default::default(),
						cluster_usage: payout_receipt.total_node_usage,
						validators: Default::default(),
					};

					let fingerprint = payout_fingerprint.selective_hash::<T>();

					billing_fingerprints.push(payout_fingerprint);

					Some(v2::BillingReport {
						state: payout_receipt.state,
						vault: payout_receipt.vault,
						fingerprint,
						total_customer_charge: payout_receipt.total_customer_charge,
						total_distributed_reward: payout_receipt.total_distributed_reward,
						charging_max_batch_index: payout_receipt.charging_max_batch_index,
						charging_processed_batches: payout_receipt.charging_processed_batches,
						rewarding_max_batch_index: payout_receipt.rewarding_max_batch_index,
						rewarding_processed_batches: payout_receipt.rewarding_processed_batches,
					})
				},
			);

			let fingerprints_count: u64 = billing_fingerprints.len() as u64;
			for payout_fingerprint in billing_fingerprints {
				let fingerprint = payout_fingerprint.selective_hash::<T>();
				v2::BillingFingerprints::<T>::insert(fingerprint, payout_fingerprint);
			}

			StorageVersion::new(2).put::<Pallet<T>>();
			log::info!(
				target: LOG_TARGET,
				"Upgraded {} records, storage to version {:?}",
				translated,
				current_version
			);

			T::DbWeight::get().reads_writes(translated + 1, fingerprints_count + translated + 1)
		} else {
			log::info!(target: LOG_TARGET, " >>> Unused migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v2::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
			let prev_count = v1::ActiveBillingReports::<T>::iter().count();
			let pre_fingerprints_count = v2::BillingFingerprints::<T>::iter().count() as u64;

			log::info!(
				target: LOG_TARGET,
				"BEFORE the migration - prev_count: {:?} pre_fingerprints_count: {:?}",
				prev_count,
				pre_fingerprints_count
			);

			ensure!(
				pre_fingerprints_count == 0,
				"Billing report fingerprints count before the migration should be equal to zero"
			);
			Ok((prev_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
			let prev_count: u64 = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			let post_count = v2::ActiveBillingReports::<T>::iter().count() as u64;
			let post_fingerprints_count = v2::BillingFingerprints::<T>::iter().count() as u64;

			log::info!(
				target: LOG_TARGET,
				"AFTER the migration - post_count: {:?} post_fingerprints_count: {:?}",
				post_count,
				post_fingerprints_count
			);

			ensure!(
				prev_count == post_count,
				"Billing report count before and after the migration should be the same"
			);
			ensure!(
				post_fingerprints_count == post_count,
				"Billing report fingerprints count after the migration should be equal to billing reports count"
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			frame_support::ensure!(current_version == 2, "must_upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same"
			);
			Ok(())
		}
	}
}

pub mod v3 {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct BillingReport<T: Config> {
		pub state: PayoutState,
		pub vault: T::AccountId,
		pub fingerprint: Fingerprint,
		pub total_collected_charges: u128,
		pub total_distributed_rewards: u128,
		pub total_settled_fees: u128,
		pub charging_max_batch_index: BatchIndex,
		pub charging_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
		pub rewarding_max_batch_index: BatchIndex,
		pub rewarding_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
	}

	#[storage_alias]
	pub type ActiveBillingReports<T: Config> = StorageDoubleMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		DdcEra,
		BillingReport<T>,
	>;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct PayoutFingerprint<AccountId> {
		pub ehd_id: String,
		pub cluster_id: ClusterId,
		pub payers_merkle_root: PayableUsageHash,
		pub payees_merkle_root: PayableUsageHash,
		pub validators: BTreeSet<AccountId>,
	}

	impl<AccountId> PayoutFingerprint<AccountId> {
		pub fn selective_hash<T: Config>(&self) -> Fingerprint {
			let mut data = self.cluster_id.encode();
			data.extend_from_slice(&self.ehd_id.encode());
			data.extend_from_slice(&self.payers_merkle_root.encode());
			data.extend_from_slice(&self.payees_merkle_root.encode());
			// we truncate the `validators` field on purpose as it's appendable collection that is
			// used for reaching the quorum on the billing fingerprint
			T::Hasher::hash(&data)
		}
	}

	#[storage_alias]
	pub type BillingFingerprints<T: Config> = StorageMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		Fingerprint,
		PayoutFingerprint<<T as frame_system::Config>::AccountId>,
	>;

	pub fn migrate_to_v3<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		log::info!(
			target: LOG_TARGET,
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 2 && current_version == 3 {
			let mut translated = 0u64;
			let reports_count = v2::ActiveBillingReports::<T>::iter().count();
			log::info!(
				target: LOG_TARGET,
				" >>> Updating Billing Reports. Migrating {} reports...", reports_count
			);

			v3::ActiveBillingReports::<T>::translate::<v2::BillingReport<T>, _>(
				|cluster_id: ClusterId, era_id: EhdEra, payout_receipt: v2::BillingReport<T>| {
					log::info!(target: LOG_TARGET, "Migrating Billing Report for cluster_id {:?} era_id {:?}", cluster_id, era_id);
					translated.saturating_inc();

					let total_collected_charges = payout_receipt
						.total_customer_charge
						.transfer
						.saturating_add(payout_receipt.total_customer_charge.storage)
						.saturating_add(payout_receipt.total_customer_charge.gets)
						.saturating_add(payout_receipt.total_customer_charge.puts);

					let total_distributed_rewards = payout_receipt.total_distributed_reward;
					let total_settled_fees =
						total_collected_charges.saturating_sub(total_distributed_rewards);

					Some(v3::BillingReport {
						state: payout_receipt.state,
						vault: payout_receipt.vault,
						fingerprint: payout_receipt.fingerprint,
						total_collected_charges,
						total_distributed_rewards,
						total_settled_fees,
						charging_max_batch_index: payout_receipt.charging_max_batch_index,
						charging_processed_batches: payout_receipt.charging_processed_batches,
						rewarding_max_batch_index: payout_receipt.rewarding_max_batch_index,
						rewarding_processed_batches: payout_receipt.rewarding_processed_batches,
					})
				},
			);

			let fingerprints_count = v2::BillingFingerprints::<T>::iter().count();
			log::info!(
				target: LOG_TARGET,
				" >>> Updating Billing Fingerprints. Migrating {} fingerprints...", fingerprints_count
			);

			v3::BillingFingerprints::<T>::translate::<
				v2::PayoutFingerprint<<T as frame_system::Config>::AccountId>,
				_,
			>(
				|_key: Fingerprint,
				 payout_fingerprint: v2::PayoutFingerprint<
					<T as frame_system::Config>::AccountId,
				>| {
					log::info!(target: LOG_TARGET, "Migrating Billing Fingerprint for cluster_id {:?} era_id {:?}", payout_fingerprint.cluster_id, payout_fingerprint.era_id);
					translated.saturating_inc();

					Some(v3::PayoutFingerprint {
						cluster_id: payout_fingerprint.cluster_id,
						ehd_id: EHDId(
							payout_fingerprint.cluster_id,
							NodePubKey::StoragePubKey(sp_runtime::AccountId32::from([0u8; 32])),
							payout_fingerprint.era_id,
						)
						.into(),
						payers_merkle_root: payout_fingerprint.payers_merkle_root,
						payees_merkle_root: payout_fingerprint.payees_merkle_root,
						validators: Default::default(),
					})
				},
			);

			let payouts_old_prefix =
				storage::storage_prefix(<Pallet<T>>::name().as_bytes(), b"ActiveBillingReports");
			let payouts_new_prefix =
				storage::storage_prefix(<Pallet<T>>::name().as_bytes(), b"PayoutReceipts");
			migration::move_prefix(&payouts_old_prefix, &payouts_new_prefix);

			if let Some(value) = unhashed::get_raw(&payouts_old_prefix) {
				unhashed::put_raw(&payouts_new_prefix, &value);
				unhashed::kill(&payouts_old_prefix);
			}

			let fingerprints_old_prefix =
				storage::storage_prefix(<Pallet<T>>::name().as_bytes(), b"BillingFingerprints");
			let fingerprints_new_prefix =
				storage::storage_prefix(<Pallet<T>>::name().as_bytes(), b"PayoutFingerprints");
			migration::move_prefix(&fingerprints_old_prefix, &fingerprints_new_prefix);

			if let Some(value) = unhashed::get_raw(&fingerprints_old_prefix) {
				unhashed::put_raw(&fingerprints_new_prefix, &value);
				unhashed::kill(&fingerprints_old_prefix);
			}

			StorageVersion::new(3).put::<Pallet<T>>();
			log::info!(
				target: LOG_TARGET,
				"Upgraded {} records, storage to version {:?}",
				translated,
				current_version
			);

			T::DbWeight::get().reads_writes(translated * 2 + 1, translated * 2 + 1)
		} else {
			log::info!(target: LOG_TARGET, " >>> Unused migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV3<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v3::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
			let pre_reports_count = v2::ActiveBillingReports::<T>::iter().count();
			let pre_fingerprints_count = v2::BillingFingerprints::<T>::iter().count();

			log::info!(
				target: LOG_TARGET,
				"BEFORE the migration - pre_reports_count: {:?} pre_fingerprints_count: {:?}",
				pre_reports_count,
				pre_fingerprints_count
			);

			Ok((pre_reports_count as u64, pre_fingerprints_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
			let (pre_reports_count, pre_fingerprints_count): (u64, u64) =
				Decode::decode(&mut &prev_state[..])
					.expect("pre_upgrade provides a valid state; qed");

			let post_receipts_count = PayoutReceipts::<T>::iter().count() as u64;
			let post_fingerprints_count = PayoutFingerprints::<T>::iter().count() as u64;

			log::info!(
				target: LOG_TARGET,
				"AFTER the migration - post_receipts_count: {:?} post_fingerprints_count: {:?}",
				post_receipts_count,
				post_fingerprints_count
			);

			ensure!(
				pre_reports_count == post_receipts_count,
				"Billing report count before and after the migration should be the same"
			);
			ensure!(
				pre_fingerprints_count == post_fingerprints_count,
				"Billing report fingerprints count after the migration should be equal to billing reports count."
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			frame_support::ensure!(current_version == 3, "must_upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same"
			);
			Ok(())
		}
	}
}
