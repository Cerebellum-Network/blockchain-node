//! DdcPayouts pallet benchmarking.

use ddc_primitives::{ClusterGovParams, ClusterId, ClusterParams};
pub use frame_benchmarking::{account, benchmarks, whitelist_account};
use frame_system::RawOrigin;
use sp_runtime::Perquintill;
use sp_std::prelude::*;

use super::*;
use crate::Pallet as DdcPayouts;

const CERE: u128 = 10000000000;

fn create_dac_account<T: Config>() -> T::AccountId {
	let dac_account = create_account::<T>("dac_account", 0, 0);
	authorize_account::<T>(dac_account.clone());
	dac_account
}

fn create_account<T: Config>(name: &'static str, idx: u32, seed: u32) -> T::AccountId {
	account::<T::AccountId>(name, idx, seed)
}

fn authorize_account<T: Config>(account: T::AccountId) {
	AuthorisedCaller::<T>::put(account);
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
	cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
) {
	T::ClusterCreator::create_new_cluster(
		cluster_id,
		cluster_manager_id,
		cluster_reserve_id,
		cluster_params,
		cluster_gov_params,
	)
	.expect("Cluster is not created");
}

fn create_default_cluster<T: Config>(cluster_id: ClusterId) {
	let cluster_manager = create_account::<T>("cm", 0, 0);
	let cluster_reserve = create_account::<T>("cr", 0, 0);
	let cluster_params = ClusterParams { node_provider_auth_contract: Default::default() };
	let cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber> = ClusterGovParams {
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
		cluster_gov_params,
	);
}

struct BillingReportParams {
	cluster_id: ClusterId,
	era: DdcEra,
	state: State,
	total_customer_charge: CustomerCharge,
	total_distributed_reward: u128,
	total_node_usage: NodeUsage,
	charging_max_batch_index: BatchIndex,
	charging_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
	rewarding_max_batch_index: BatchIndex,
	rewarding_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
}

fn create_billing_report<T: Config>(params: BillingReportParams) {
	let vault = DdcPayouts::<T>::sub_account_id(params.cluster_id, params.era);
	let start_era: i64 = 1_000_000_000;
	let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

	let billing_report = BillingReport::<T> {
		vault,
		start_era,
		end_era,
		state: params.state,
		total_customer_charge: params.total_customer_charge,
		total_distributed_reward: params.total_distributed_reward,
		total_node_usage: params.total_node_usage,
		charging_max_batch_index: params.charging_max_batch_index,
		charging_processed_batches: params.charging_processed_batches,
		rewarding_max_batch_index: params.rewarding_max_batch_index,
		rewarding_processed_batches: params.rewarding_processed_batches,
	};

	ActiveBillingReports::<T>::insert(params.cluster_id, params.era, billing_report);
}

benchmarks! {

	set_authorised_caller {
		 let dac_account = create_account::<T>("dac_account", 0, 0);

	}: _(RawOrigin::Root, dac_account.clone())
	verify {
		assert_eq!(AuthorisedCaller::<T>::get(), Some(dac_account));
	}

	begin_billing_report {

		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let start_era: i64 = 1_000_000_000;
		let end_era: i64 = start_era + (30.44 * 24.0 * 3600.0) as i64;

		create_default_cluster::<T>(cluster_id);

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era, start_era, end_era)
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, State::Initialized);
	}

	begin_charging_customers {
		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = State::Initialized;
		let total_customer_charge = CustomerCharge::default();
		let total_distributed_reward : u128= 0;
		let total_node_usage = NodeUsage::default();
		let charging_max_batch_index = BatchIndex::default();
		let charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		let rewarding_max_batch_index = BatchIndex::default();
		let rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge,
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

		let max_batch_index: BatchIndex = 10;

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era, max_batch_index)
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, State::ChargingCustomers);
		assert_eq!(billing_report.charging_max_batch_index, max_batch_index);
	}

	send_charging_customers_batch {
		let b in 1 .. MaxBatchSize::get() as u32;

		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = State::ChargingCustomers;
		let total_customer_charge = CustomerCharge::default();
		let total_distributed_reward : u128 = 0;
		let total_node_usage = NodeUsage::default();
		let charging_max_batch_index = 0;
		let charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		let rewarding_max_batch_index = BatchIndex::default();
		let rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge,
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let batch_index: BatchIndex = 0;
		let payers: Vec<(T::AccountId, CustomerUsage)> = (0..b).map(|i| {
			let customer = create_account::<T>("customer", i, i);

			if b % 2 == 0 {
				// no customer debt path
				endow_customer::<T>(&customer, 1_000_000 * CERE);
			} else {
				// customer debt path
				endow_customer::<T>(&customer, 10 * CERE);
			}

			let customer_usage = CustomerUsage {
				transferred_bytes: 200000000, // 200 mb
				stored_bytes: 100000000, // 100 mb
				number_of_gets: 10, // 10 gets
				number_of_puts: 5, // 5 puts
			};

			(customer, customer_usage)
		}).collect();

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era, batch_index, payers)
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, State::ChargingCustomers);
		assert!(billing_report.charging_processed_batches.contains(&batch_index));
	}

	end_charging_customers {
		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = State::ChargingCustomers;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE, // price for 100 mb
			gets: 10 * CERE, // price for 10 gets
			puts: 5 * CERE, // price for 5 puts
		};
		let total_distributed_reward : u128 = 0;
		let total_node_usage = NodeUsage::default();
		let charging_max_batch_index = 0;
		let mut charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		charging_processed_batches.try_insert(0).unwrap();
		let rewarding_max_batch_index = BatchIndex::default();
		let rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge: total_customer_charge.clone(),
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let vault = DdcPayouts::<T>::sub_account_id(cluster_id, era);
		let total_customer_charge_amount = total_customer_charge.transfer + total_customer_charge.storage + total_customer_charge.gets + total_customer_charge.puts;
		endow_account::<T>(&vault, total_customer_charge_amount);

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era)
	verify {
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, State::CustomersChargedWithFees);
		assert!(billing_report.charging_processed_batches.contains(&charging_max_batch_index));
	}

	begin_rewarding_providers {
		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = State::CustomersChargedWithFees;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE, // price for 100 mb
			gets: 10 * CERE, // price for 10 gets
			puts: 5 * CERE, // price for 5 puts
		};
		let total_distributed_reward : u128 = 0;
		let total_node_usage = NodeUsage::default();
		let charging_max_batch_index = 0;
		let mut charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		charging_processed_batches.try_insert(0).unwrap();
		let rewarding_max_batch_index = BatchIndex::default();
		let rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge,
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let max_batch_index: BatchIndex = 10;
		let total_node_usage = NodeUsage {
			transferred_bytes: 200000000, // 200 mb
			stored_bytes: 100000000, // 100 mb
			number_of_gets: 10, // 10 gets
			number_of_puts: 5, // 5 puts
		};

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era, max_batch_index, total_node_usage)
	verify {
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, State::RewardingProviders);
		assert_eq!(billing_report.rewarding_max_batch_index, max_batch_index);
	}

	send_rewarding_providers_batch {
		let b in 1 .. MaxBatchSize::get() as u32;

		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = State::RewardingProviders;
		let total_customer_charge = CustomerCharge {
			transfer: (200 * CERE).saturating_mul(b.into()), // price for 200 mb per customer
			storage: (100 * CERE).saturating_mul(b.into()), // price for 100 mb per customer
			gets: (10 * CERE).saturating_mul(b.into()), // price for 10 gets per customer
			puts: (5 * CERE).saturating_mul(b.into()), // price for 5 puts per customer
		};
		let total_distributed_reward : u128 = 0;
		let total_node_usage = NodeUsage {
			transferred_bytes: 200000000u64.saturating_mul(b.into()), // 200 mb per provider
			stored_bytes: 100000000u64.saturating_mul(b.into()), // 100 mb per provider
			number_of_gets: 10u64.saturating_mul(b.into()), // 10 gets per provider
			number_of_puts: 10u64.saturating_mul(b.into()), // 5 puts per provider
		};
		let charging_max_batch_index = 0;
		let mut charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		charging_processed_batches.try_insert(0).unwrap();
		let rewarding_max_batch_index = 0;
		let rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge: total_customer_charge.clone(),
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let vault = DdcPayouts::<T>::sub_account_id(cluster_id, era);
		let total_customer_charge_amount = total_customer_charge.transfer + total_customer_charge.storage + total_customer_charge.gets + total_customer_charge.puts;
		endow_account::<T>(&vault, total_customer_charge_amount + T::Currency::minimum_balance().saturated_into::<u128>());

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

		let batch_index: BatchIndex = 0;
		let payees: Vec<(T::AccountId, NodeUsage)> = (0..b).map(|i| {
			let provider = create_account::<T>("provider", i, i);
			endow_account::<T>(&provider, T::Currency::minimum_balance().saturated_into());
			let node_usage = NodeUsage {
				transferred_bytes: 200000000, // 200 mb
				stored_bytes: 100000000, // 100 mb
				number_of_gets: 10, // 10 gets
				number_of_puts: 5, // 5 puts
			};
			(provider, node_usage)
		}).collect();

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era, batch_index, payees)
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, State::RewardingProviders);
		assert!(billing_report.rewarding_processed_batches.contains(&batch_index));
	}

	end_rewarding_providers {
		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = State::RewardingProviders;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE, // price for 100 mb
			gets: 10 * CERE, // price for 10 gets
			puts: 5 * CERE, // price for 5 puts
		};
		let total_distributed_reward : u128 = total_customer_charge.transfer + total_customer_charge.storage + total_customer_charge.gets + total_customer_charge.puts;
		let total_node_usage = NodeUsage {
			transferred_bytes: 200000000, // 200 mb
			stored_bytes: 100000000, // 100 mb
			number_of_gets: 10, // 10 gets
			number_of_puts: 5, // 5 puts
		};
		let charging_max_batch_index = 0;
		let mut charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		charging_processed_batches.try_insert(0).unwrap();
		let rewarding_max_batch_index = 0;
		let mut rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		rewarding_processed_batches.try_insert(0).unwrap();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge,
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era)
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, State::ProvidersRewarded);
	}

	end_billing_report {
		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let state = State::ProvidersRewarded;
		let total_customer_charge = CustomerCharge {
			transfer: 200 * CERE, // price for 200 mb
			storage: 100 * CERE, // price for 100 mb
			gets: 10 * CERE, // price for 10 gets
			puts: 5 * CERE, // price for 5 puts
		};
		let total_distributed_reward : u128 = total_customer_charge.transfer + total_customer_charge.storage + total_customer_charge.gets + total_customer_charge.puts;
		let total_node_usage = NodeUsage {
			transferred_bytes: 200000000, // 200 mb
			stored_bytes: 100000000, // 100 mb
			number_of_gets: 10, // 10 gets
			number_of_puts: 5, // 5 puts
		};
		let charging_max_batch_index = 0;
		let mut charging_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		charging_processed_batches.try_insert(0).unwrap();
		let rewarding_max_batch_index = 0;
		let mut rewarding_processed_batches : BoundedBTreeSet<BatchIndex, MaxBatchesCount> = BoundedBTreeSet::default();
		rewarding_processed_batches.try_insert(0).unwrap();

		create_default_cluster::<T>(cluster_id);
		create_billing_report::<T>(BillingReportParams {
			cluster_id,
			era,
			state,
			total_customer_charge,
			total_distributed_reward,
			total_node_usage,
			charging_max_batch_index,
			charging_processed_batches,
			rewarding_max_batch_index,
			rewarding_processed_batches,
		});

		let dac_account = create_dac_account::<T>();
		whitelist_account!(dac_account);

	}: _(RawOrigin::Signed(dac_account.clone()), cluster_id, era)
	verify {
		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.state, State::Finalized);
	}

}
