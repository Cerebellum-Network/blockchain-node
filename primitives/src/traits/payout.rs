use scale_info::prelude::vec::Vec;
use sp_runtime::DispatchResult;

use crate::{
	ActivityHash, BatchIndex, BucketId, ClusterId, CustomerUsage, DdcEra, NodeUsage, PayoutState,
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
	// todo! remove clippy::too_many_arguments
	#[allow(clippy::too_many_arguments)]
	fn send_charging_customers_batch(
		origin: T::AccountId,
		cluster_id: ClusterId,
		era_id: DdcEra,
		batch_index: BatchIndex,
		payers: Vec<(T::AccountId, BucketId, CustomerUsage)>,
		mmr_size: u64,
		proof: Vec<ActivityHash>,
		leaf_with_position: (u64, ActivityHash),
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
	// todo! remove clippy::too_many_arguments
	#[allow(clippy::too_many_arguments)]
	fn send_rewarding_providers_batch(
		origin: T::AccountId,
		cluster_id: ClusterId,
		era_id: DdcEra,
		batch_index: BatchIndex,
		payees: Vec<(T::AccountId, BucketId, NodeUsage)>,
		mmr_size: u64,
		proof: Vec<ActivityHash>,
		leaf_with_position: (u64, ActivityHash),
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
}
