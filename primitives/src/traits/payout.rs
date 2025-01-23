use scale_info::prelude::string::String;
use sp_runtime::DispatchResult;
use sp_std::boxed::Box;

use crate::{
	BatchIndex, ClusterId, Fingerprint, MMRProof, PayableUsageHash, PaymentEra, PayoutError,
	PayoutFingerprintParams, PayoutReceiptParams, PayoutState,
};

pub trait PayoutProcessor<T: frame_system::Config> {
	#[allow(clippy::too_many_arguments)]
	fn commit_payout_fingerprint(
		validator: T::AccountId,
		cluster_id: ClusterId,
		ehd_id: String,
		payers_merkle_root: PayableUsageHash,
		payees_merkle_root: PayableUsageHash,
	) -> DispatchResult;

	fn begin_payout(
		cluster_id: ClusterId,
		era_id: PaymentEra,
		fingerprint: Fingerprint,
	) -> DispatchResult;

	fn begin_charging_customers(
		cluster_id: ClusterId,
		era_id: PaymentEra,
		max_batch_index: BatchIndex,
	) -> DispatchResult;

	fn send_charging_customers_batch(
		cluster_id: ClusterId,
		era_id: PaymentEra,
		batch_index: BatchIndex,
		payers: &[(T::AccountId, u128)],
		batch_proof: MMRProof,
	) -> DispatchResult;

	fn end_charging_customers(cluster_id: ClusterId, era: PaymentEra) -> DispatchResult;

	fn begin_rewarding_providers(
		cluster_id: ClusterId,
		era_id: PaymentEra,
		max_batch_index: BatchIndex,
	) -> DispatchResult;

	fn send_rewarding_providers_batch(
		cluster_id: ClusterId,
		era_id: PaymentEra,
		batch_index: BatchIndex,
		payees: &[(T::AccountId, u128)],
		batch_proof: MMRProof,
	) -> DispatchResult;

	fn end_rewarding_providers(cluster_id: ClusterId, era_id: PaymentEra) -> DispatchResult;

	fn end_payout(cluster_id: ClusterId, era_id: PaymentEra) -> DispatchResult;

	fn get_payout_state(cluster_id: &ClusterId, era_id: PaymentEra) -> PayoutState;

	fn is_customers_charging_finished(cluster_id: &ClusterId, era_id: PaymentEra) -> bool;

	fn is_providers_rewarding_finished(cluster_id: &ClusterId, era_id: PaymentEra) -> bool;

	fn get_next_customers_batch(
		cluster_id: &ClusterId,
		era_id: PaymentEra,
	) -> Result<Option<BatchIndex>, PayoutError>;

	fn get_next_providers_batch(
		cluster_id: &ClusterId,
		era_id: PaymentEra,
	) -> Result<Option<BatchIndex>, PayoutError>;

	fn create_payout_receipt(vault: T::AccountId, params: PayoutReceiptParams);

	fn create_payout_fingerprint(params: PayoutFingerprintParams<T::AccountId>) -> Fingerprint;
}

pub trait StorageUsageProvider<Key, Item> {
	type Error: sp_std::fmt::Debug;

	fn iter_storage_usage<'a>(cluster_id: &'a ClusterId) -> Box<dyn Iterator<Item = Item> + 'a>;

	fn iter_storage_usage_from<'a>(
		cluster_id: &'a ClusterId,
		from: &'a Key,
	) -> Result<Box<dyn Iterator<Item = Item> + 'a>, Self::Error>;
}
