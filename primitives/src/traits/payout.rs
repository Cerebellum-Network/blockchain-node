use sp_runtime::DispatchResult;
use sp_std::boxed::Box;

use crate::{
	BatchIndex, BillingFingerprintParams, BillingReportParams, BucketId, BucketUsage, ClusterId,
	DdcEra, Fingerprint, MMRProof, NodePubKey, NodeUsage, PayableUsageHash, PayoutError,
	PayoutState,
};

pub trait PayoutProcessor<T: frame_system::Config> {
	#[allow(clippy::too_many_arguments)]
	fn commit_billing_fingerprint(
		validator: T::AccountId,
		cluster_id: ClusterId,
		era_id: DdcEra,
		start_era: i64,
		end_era: i64,
		payers_merkle_root: PayableUsageHash,
		payees_merkle_root: PayableUsageHash,
		cluster_usage: NodeUsage,
	) -> DispatchResult;

	fn begin_billing_report(
		cluster_id: ClusterId,
		era_id: DdcEra,
		fingerprint: Fingerprint,
	) -> DispatchResult;

	fn begin_charging_customers(
		cluster_id: ClusterId,
		era_id: DdcEra,
		max_batch_index: BatchIndex,
	) -> DispatchResult;

	fn send_charging_customers_batch(
		cluster_id: ClusterId,
		era_id: DdcEra,
		batch_index: BatchIndex,
		payers: &[(BucketId, BucketUsage)],
		batch_proof: MMRProof,
	) -> DispatchResult;

	fn end_charging_customers(cluster_id: ClusterId, era: DdcEra) -> DispatchResult;

	fn begin_rewarding_providers(
		cluster_id: ClusterId,
		era_id: DdcEra,
		max_batch_index: BatchIndex,
	) -> DispatchResult;

	fn send_rewarding_providers_batch(
		cluster_id: ClusterId,
		era_id: DdcEra,
		batch_index: BatchIndex,
		payees: &[(NodePubKey, NodeUsage)],
		batch_proof: MMRProof,
	) -> DispatchResult;

	fn end_rewarding_providers(cluster_id: ClusterId, era_id: DdcEra) -> DispatchResult;

	fn end_billing_report(cluster_id: ClusterId, era_id: DdcEra) -> DispatchResult;

	fn get_billing_report_status(cluster_id: &ClusterId, era_id: DdcEra) -> PayoutState;

	fn all_customer_batches_processed(cluster_id: &ClusterId, era_id: DdcEra) -> bool;

	fn all_provider_batches_processed(cluster_id: &ClusterId, era_id: DdcEra) -> bool;

	fn get_next_customer_batch_for_payment(
		cluster_id: &ClusterId,
		era_id: DdcEra,
	) -> Result<Option<BatchIndex>, PayoutError>;

	fn get_next_provider_batch_for_payment(
		cluster_id: &ClusterId,
		era_id: DdcEra,
	) -> Result<Option<BatchIndex>, PayoutError>;

	fn create_billing_report(vault: T::AccountId, params: BillingReportParams);

	fn create_billing_fingerprint(_params: BillingFingerprintParams<T::AccountId>) -> Fingerprint;
}

pub trait StorageUsageProvider<Key, Item> {
	type Error: sp_std::fmt::Debug;

	fn iter_storage_usage<'a>(cluster_id: &'a ClusterId) -> Box<dyn Iterator<Item = Item> + 'a>;

	fn iter_storage_usage_from<'a>(
		cluster_id: &'a ClusterId,
		from: &'a Key,
	) -> Result<Box<dyn Iterator<Item = Item> + 'a>, Self::Error>;
}
