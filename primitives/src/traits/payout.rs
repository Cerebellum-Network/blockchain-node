use sp_runtime::DispatchResult;
use sp_std::boxed::Box;

use crate::{
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
	BatchIndex, BillingReportParams, BucketId, BucketUsage, ClusterId, DdcEra, MMRProof,
	NodePubKey, NodeUsage, PayoutError, PayoutState,
=======
	BatchIndex, BucketId, BucketUsage, ClusterId, DdcEra, MMRProof, NodePubKey, NodeUsage,
	PayoutError, PayoutState,
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
=======
	BatchIndex, BillingReportParams, BucketId, BucketUsage, ClusterId, DdcEra, MMRProof,
	NodePubKey, NodeUsage, PayoutError, PayoutState,
>>>>>>> f870601e (Alow `create_billing_report` at runtime)
};

pub trait PayoutProcessor<T: frame_system::Config> {
	fn begin_billing_report(
=======
	BatchIndex, BucketId, ClusterId, CustomerUsage, DdcEra, MMRProof, NodeUsage, PayoutError,
	PayoutState,
=======
	BatchIndex, BucketId, ClusterId, CustomerUsage, DdcEra, MMRProof, NodePubKey, NodeUsage,
	PayoutError, PayoutState,
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
};

pub trait PayoutProcessor<T: frame_system::Config> {
	fn begin_billing_report(
<<<<<<< HEAD
		origin: T::AccountId,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
=======
	BatchIndex, BillingFingerprintParams, BillingReportParams, BucketId, BucketUsage, ClusterId,
	DdcEra, Fingerprint, MMRProof, NodePubKey, NodeUsage, PayableUsageHash, PayoutError,
	PayoutState,
};

pub trait PayoutProcessor<T: frame_system::Config> {
	#[allow(clippy::too_many_arguments)]
	fn commit_billing_fingerprint(
		validator: T::AccountId,
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
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

<<<<<<< HEAD
<<<<<<< HEAD
	fn begin_charging_customers(
=======
	// todo! factor out into PayoutProcessor
	fn begin_charging_customers(
		origin: T::AccountId,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	fn begin_charging_customers(
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
		cluster_id: ClusterId,
		era_id: DdcEra,
		max_batch_index: BatchIndex,
	) -> DispatchResult;

<<<<<<< HEAD
<<<<<<< HEAD
	fn send_charging_customers_batch(
		cluster_id: ClusterId,
		era_id: DdcEra,
		batch_index: BatchIndex,
		payers: &[(BucketId, BucketUsage)],
		batch_proof: MMRProof,
	) -> DispatchResult;

	fn end_charging_customers(cluster_id: ClusterId, era: DdcEra) -> DispatchResult;

	fn begin_rewarding_providers(
=======
	// todo! factor out into PayoutProcessor
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
	fn send_charging_customers_batch(
		cluster_id: ClusterId,
		era_id: DdcEra,
		batch_index: BatchIndex,
		payers: &[(NodePubKey, BucketId, BucketUsage)],
		batch_proof: MMRProof,
	) -> DispatchResult;

	fn end_charging_customers(cluster_id: ClusterId, era: DdcEra) -> DispatchResult;

	fn begin_rewarding_providers(
<<<<<<< HEAD
		origin: T::AccountId,
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
		cluster_id: ClusterId,
		era_id: DdcEra,
		max_batch_index: BatchIndex,
	) -> DispatchResult;

<<<<<<< HEAD
<<<<<<< HEAD
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
=======
	// todo! factor out into PayoutProcessor
=======
>>>>>>> d3977123 (refactor: ddc-payouts processor and tests)
	fn send_rewarding_providers_batch(
		cluster_id: ClusterId,
		era_id: DdcEra,
		batch_index: BatchIndex,
		payees: &[(NodePubKey, NodeUsage)],
		batch_proof: MMRProof,
	) -> DispatchResult;

	fn end_rewarding_providers(cluster_id: ClusterId, era_id: DdcEra) -> DispatchResult;

	fn end_billing_report(cluster_id: ClusterId, era_id: DdcEra) -> DispatchResult;

<<<<<<< HEAD
	fn get_billing_report_status(cluster_id: &ClusterId, era: DdcEra) -> PayoutState;
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	fn get_billing_report_status(cluster_id: &ClusterId, era_id: DdcEra) -> PayoutState;
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)

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
<<<<<<< HEAD
<<<<<<< HEAD

	fn create_billing_report(vault: T::AccountId, params: BillingReportParams);
<<<<<<< HEAD
=======
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======

	fn create_billing_report(vault: T::AccountId, params: BillingReportParams);
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
=======

	fn create_billing_fingerprint(_params: BillingFingerprintParams<T::AccountId>) -> Fingerprint;
}

pub trait StorageUsageProvider<Key, Item> {
	type Error: sp_std::fmt::Debug;

	fn iter_storage_usage<'a>(cluster_id: &'a ClusterId) -> Box<dyn Iterator<Item = Item> + 'a>;

	fn iter_storage_usage_from<'a>(
		cluster_id: &'a ClusterId,
		from: &'a Key,
	) -> Result<Box<dyn Iterator<Item = Item> + 'a>, Self::Error>;
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
}
