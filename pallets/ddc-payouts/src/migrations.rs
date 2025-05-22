
pub mod v4 {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct V3PayoutReceipt<T: Config> {
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

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct PayoutReceipt<T: Config> {
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
		pub finalized_at: Option<BlockNumberFor<T>>, // new field
	}

	#[storage_alias]
	pub type PayoutReceipts<T: Config> = StorageDoubleMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		EhdEra,
		PayoutReceipt<T>,
	>;

	pub fn migrate_to_v4<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		log::info!(
			target: LOG_TARGET,
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 3 && current_version == 4 {
			let mut translated = 0u64;
			v4::PayoutReceipts::<T>::translate::<V3PayoutReceipt<T>, _>(
				|cluster_id: ClusterId, era_id: EhdEra, payout_receipt: V3PayoutReceipt<T>| {
					log::info!(target: LOG_TARGET, "Migrating Payout Receipt for cluster_id {:?} era_id {:?}", cluster_id, era_id);
					translated.saturating_inc();

					let receipt = if payout_receipt.state == PayoutState::Finalized {
						v4::PayoutReceipt {
							state: payout_receipt.state,
							vault: payout_receipt.vault,
							fingerprint: payout_receipt.fingerprint,
							total_collected_charges: payout_receipt.total_collected_charges,
							total_distributed_rewards: payout_receipt.total_distributed_rewards,
							total_settled_fees: payout_receipt.total_settled_fees,
							charging_max_batch_index: payout_receipt.charging_max_batch_index,
							charging_processed_batches: payout_receipt.charging_processed_batches,
							rewarding_max_batch_index: payout_receipt.rewarding_max_batch_index,
							rewarding_processed_batches: payout_receipt.rewarding_processed_batches,
							finalized_at: Some(frame_system::Pallet::<T>::block_number()),
						}
					} else {
						v4::PayoutReceipt {
							state: payout_receipt.state,
							vault: payout_receipt.vault,
							fingerprint: payout_receipt.fingerprint,
							total_collected_charges: payout_receipt.total_collected_charges,
							total_distributed_rewards: payout_receipt.total_distributed_rewards,
							total_settled_fees: payout_receipt.total_settled_fees,
							charging_max_batch_index: payout_receipt.charging_max_batch_index,
							charging_processed_batches: payout_receipt.charging_processed_batches,
							rewarding_max_batch_index: payout_receipt.rewarding_max_batch_index,
							rewarding_processed_batches: payout_receipt.rewarding_processed_batches,
							finalized_at: None,
						}
					};

					Some(receipt)
				},
			);

			StorageVersion::new(4).put::<Pallet<T>>();
			log::info!(
				target: LOG_TARGET,
				"Upgraded {} records, storage to version {:?}",
				translated,
				current_version
			);

			T::DbWeight::get().reads_writes(translated + 1, translated + 1)
		} else {
			log::info!(target: LOG_TARGET, " >>> Unused migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV4<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV4<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v4::<T>()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
			let pre_receipts_count = crate::PayoutReceipts::<T>::iter().count();
			log::info!(
				target: LOG_TARGET,
				"BEFORE the migration - pre_receipts_count: {:?}",
				pre_receipts_count,
			);
			Ok((pre_receipts_count as u64).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), DispatchError> {
			let pre_receipts_count: u64 = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			let post_receipts_count = PayoutReceipts::<T>::iter().count() as u64;

			log::info!(
				target: LOG_TARGET,
				"AFTER the migration - post_receipts_count: {:?}",
				post_receipts_count,
			);

			ensure!(
				pre_receipts_count == post_receipts_count,
				"Receipts count before and after the migration should be the same"
			);

			let current_version = Pallet::<T>::in_code_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			frame_support::ensure!(current_version == 4, "must_upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same"
			);
			Ok(())
		}
	}
}
