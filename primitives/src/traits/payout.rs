use scale_info::prelude::string::String;
use sp_runtime::DispatchResult;

use crate::{
	BatchIndex, BucketId, ClusterId, CustomerUsage, DdcEra, MMRProof, NodePubKey, NodeUsage,
	PayoutError, PayoutState,
};
pub trait PayoutProcessor<T: frame_system::Config> {}

pub trait PayoutVisitor<T: frame_system::Config> {
	// todo! factor out into PayoutProcessor
	fn begin_billing_report(
		origin: T::AccountId,
		cluster_id: ClusterId,
		era_id: DdcEra,
		start_era: i64,
		end_era: i64,
	) -> DispatchResult;

	// todo! factor out into PayoutProcessor
	fn begin_charging_customers(
		origin: T::AccountId,
		cluster_id: ClusterId,
		era_id: DdcEra,
		max_batch_index: BatchIndex,
	) -> DispatchResult;

	// todo! factor out into PayoutProcessor
	fn send_charging_customers_batch(
		origin: T::AccountId,
		cluster_id: ClusterId,
		era_id: DdcEra,
		batch_index: BatchIndex,
		payers: &[(T::AccountId, String, BucketId, CustomerUsage)],
		batch_proof: MMRProof,
	) -> DispatchResult;

	// todo! factor out into PayoutProcessor
	fn end_charging_customers(
		origin: T::AccountId,
		cluster_id: ClusterId,
		era_id: DdcEra,
	) -> DispatchResult;

	// todo! factor out into PayoutProcessor
	fn begin_rewarding_providers(
		origin: T::AccountId,
		cluster_id: ClusterId,
		era_id: DdcEra,
		max_batch_index: BatchIndex,
		total_node_usage: NodeUsage,
	) -> DispatchResult;

	// todo! factor out into PayoutProcessor
	fn send_rewarding_providers_batch(
		origin: T::AccountId,
		cluster_id: ClusterId,
		era_id: DdcEra,
		batch_index: BatchIndex,
		payees: &[(NodePubKey, NodeUsage)],
		batch_proof: MMRProof,
	) -> DispatchResult;

	// todo! factor out into PayoutProcessor
	fn end_rewarding_providers(
		origin: T::AccountId,
		cluster_id: ClusterId,
		era_id: DdcEra,
	) -> DispatchResult;

	// todo! factor out into PayoutProcessor
	fn end_billing_report(
		origin: T::AccountId,
		cluster_id: ClusterId,
		era_id: DdcEra,
	) -> DispatchResult;

	fn get_billing_report_status(cluster_id: &ClusterId, era: DdcEra) -> PayoutState;

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
}
