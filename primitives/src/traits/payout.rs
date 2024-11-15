use sp_runtime::DispatchResult;

#[cfg(feature = "runtime-benchmarks")]
use crate::BillingReportParams;
use crate::{
	BatchIndex, BucketId, ClusterId, CustomerUsage, DdcEra, MMRProof, NodePubKey, NodeUsage,
	PayoutError, PayoutState,
};

pub trait PayoutProcessor<T: frame_system::Config> {
	fn begin_billing_report(
		cluster_id: ClusterId,
		era_id: DdcEra,
		start_era: i64,
		end_era: i64,
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
		payers: &[(NodePubKey, BucketId, CustomerUsage)],
		batch_proof: MMRProof,
	) -> DispatchResult;

	fn end_charging_customers(cluster_id: ClusterId, era: DdcEra) -> DispatchResult;

	fn begin_rewarding_providers(
		cluster_id: ClusterId,
		era_id: DdcEra,
		max_batch_index: BatchIndex,
		total_node_usage: NodeUsage,
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

	#[cfg(feature = "runtime-benchmarks")]
	fn create_billing_report(vault: T::AccountId, params: BillingReportParams);
}
