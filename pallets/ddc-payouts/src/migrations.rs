use frame_support::{migration, storage_alias, traits::OnRuntimeUpgrade};
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

	pub fn migrate_to_v1<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::current_storage_version();

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
		pub total_customer_charge: CustomerCharge,
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

	pub fn migrate_to_v2<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::current_storage_version();

		log::info!(
			target: LOG_TARGET,
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 1 && current_version == 2 {
			let mut translated = 0u64;
			let count = v2::ActiveBillingReports::<T>::iter().count();
			log::info!(
				target: LOG_TARGET,
				" >>> Updating Billing Reports. Migrating {} reports...", count
			);

			let mut billing_fingerprints: Vec<BillingFingerprint<T::AccountId>> = Vec::new();

			v2::ActiveBillingReports::<T>::translate::<v1::BillingReport<T>, _>(
				|cluster_id: ClusterId, era_id: DdcEra, billing_report: v1::BillingReport<T>| {
					log::info!(target: LOG_TARGET, "Migrating Billing Report for cluster_id {:?} era_id {:?}", cluster_id, era_id);
					translated.saturating_inc();

					let billing_fingerprint = BillingFingerprint {
						cluster_id,
						era_id,
						start_era: billing_report.start_era,
						end_era: billing_report.end_era,
						payers_merkle_root: Default::default(),
						payees_merkle_root: Default::default(),
						cluster_usage: billing_report.total_node_usage,
						validators: Default::default(),
					};

					let fingerprint = billing_fingerprint.selective_hash::<T>();

					billing_fingerprints.push(billing_fingerprint);

					Some(v2::BillingReport {
						state: billing_report.state,
						vault: billing_report.vault,
						fingerprint,
						total_customer_charge: billing_report.total_customer_charge,
						total_distributed_reward: billing_report.total_distributed_reward,
						// stage 1
						charging_max_batch_index: billing_report.charging_max_batch_index,
						charging_processed_batches: billing_report.charging_processed_batches,
						// stage 2
						rewarding_max_batch_index: billing_report.rewarding_max_batch_index,
						rewarding_processed_batches: billing_report.rewarding_processed_batches,
					})
				},
			);

			let fingerprints_count: u64 = billing_fingerprints.len() as u64;
			for billing_fingerprint in billing_fingerprints {
				let fingerprint = billing_fingerprint.selective_hash::<T>();
				v2::BillingFingerprints::<T>::insert(fingerprint, billing_fingerprint);
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
			let prev_count = v2::ActiveBillingReports::<T>::iter().count();
			let pre_fingerprints_count = v2::BillingFingerprints::<T>::iter().count() as u64;
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
			ensure!(
				prev_count == post_count,
				"Billing report count before and after the migration should be the same"
			);

			let post_fingerprints_count = v2::BillingFingerprints::<T>::iter().count() as u64;
			ensure!(
				post_fingerprints_count == post_count,
				"Billing report fingerprints count after the migration should be equal to billing reports count"
			);

			let current_version = Pallet::<T>::current_storage_version();
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
