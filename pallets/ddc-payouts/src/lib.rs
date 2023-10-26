//! # DDC Payouts Pallet
//!
//! The DDC Payouts pallet is used to distribute payouts based on DAC validation
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## GenesisConfig
//!
//! The DDC Payouts pallet depends on the [`GenesisConfig`]. The
//! `GenesisConfig` is optional and allow to set some initial nodes in DDC.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use ddc_primitives::{ClusterId, DdcEra};
use frame_support::{pallet_prelude::*, parameter_types, BoundedVec};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sp_runtime::Perbill;
use sp_std::{ops::Mul, prelude::*};

type BatchIndex = u16;

parameter_types! {
	pub MaxBatchesCount: u16 = 1000;
}

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		BillingReportInitialized { cluster_id: ClusterId, era: DdcEra },
		ChargingStarted { cluster_id: ClusterId, era: DdcEra },
		ChargingFinished { cluster_id: ClusterId, era: DdcEra },
		RewardingStarted { cluster_id: ClusterId, era: DdcEra },
		RewardingFinished { cluster_id: ClusterId, era: DdcEra },
		BillingReportFinalized { cluster_id: ClusterId, era: DdcEra },
	}

	#[pallet::error]
	pub enum Error<T> {
		BillingReportDoesNotExist,
		NotExpectedState,
		BatchIndexAlreadyProcessed,
		BatchIndexIsOutOfRange,
		BatchesMissed,
		NotDistributedBalance,
		BatchIndexOverflow,
		BoundedVecOverflow,
	}

	#[pallet::storage]
	#[pallet::getter(fn active_billing_reports)]
	pub type ActiveBillingReports<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		DdcEra,
		BillingReport,
		ValueQuery,
	>;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Default)]
	pub struct BillingReport {
		state: State,
		total_balance: u128,
		distributed_balance: u128,
		// stage 1
		charging_max_batch_index: BatchIndex,
		charging_processed_batches: BoundedVec<BatchIndex, MaxBatchesCount>,
		// stage 2
		rewarding_max_batch_index: BatchIndex,
		rewarding_processed_batches: BoundedVec<BatchIndex, MaxBatchesCount>,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Default)]
	pub enum State {
		#[default]
		NotInitialized,
		Initialized,
		ChargingCustomers,
		CustomersCharged,
		RewardingProviders,
		ProvidersRewarded,
		Finalized,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn begin_billing_report(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
		) -> DispatchResult {
			ensure_signed(origin)?; // todo: check that the caller is DAC account

			let mut billing_report = BillingReport::default();
			billing_report.state = State::Initialized;
			ActiveBillingReports::<T>::insert(cluster_id.clone(), era, billing_report);

			Self::deposit_event(Event::<T>::BillingReportInitialized { cluster_id, era });

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn begin_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			ensure_signed(origin)?;

			ensure!(
				max_batch_index > 0 && max_batch_index < MaxBatchesCount::get(),
				Error::<T>::BatchIndexOverflow
			);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id.clone(), era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == State::Initialized, Error::<T>::NotExpectedState);

			billing_report.charging_max_batch_index = max_batch_index;
			billing_report.state = State::ChargingCustomers;
			ActiveBillingReports::<T>::insert(cluster_id.clone(), era, billing_report);

			Self::deposit_event(Event::<T>::ChargingStarted { cluster_id, era });

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn send_charging_customers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			payers: Vec<(T::AccountId, u128)>,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let billing_report = ActiveBillingReports::<T>::try_get(cluster_id.clone(), era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == State::ChargingCustomers, Error::<T>::NotExpectedState);
			ensure!(
				billing_report.charging_max_batch_index >= batch_index,
				Error::<T>::BatchIndexIsOutOfRange
			);
			ensure!(
				!billing_report.charging_processed_batches.contains(&batch_index),
				Error::<T>::BatchIndexAlreadyProcessed
			);

			let mut updated_billing_report = billing_report.clone();
			for payer in payers {
				let _customer = payer.0; // todo: charge customer
				let amount = payer.1;
				updated_billing_report.total_balance += amount;
			}

			updated_billing_report
				.charging_processed_batches
				.try_push(batch_index)
				.map_err(|_| Error::<T>::BoundedVecOverflow)?;

			ActiveBillingReports::<T>::insert(cluster_id, era, updated_billing_report);

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn end_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id.clone(), era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == State::ChargingCustomers, Error::<T>::NotExpectedState);
			ensure!(
				billing_report.charging_max_batch_index as usize ==
					billing_report.charging_processed_batches.len() - 1usize,
				Error::<T>::BatchesMissed
			);

			billing_report.state = State::CustomersCharged;
			ActiveBillingReports::<T>::insert(cluster_id.clone(), era, billing_report);

			Self::deposit_event(Event::<T>::ChargingFinished { cluster_id, era });

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn begin_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			ensure_signed(origin)?;

			ensure!(
				max_batch_index > 0 && max_batch_index < MaxBatchesCount::get(),
				Error::<T>::BatchIndexOverflow
			);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id.clone(), era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == State::CustomersCharged, Error::<T>::NotExpectedState);

			billing_report.rewarding_max_batch_index = max_batch_index;
			billing_report.state = State::RewardingProviders;
			ActiveBillingReports::<T>::insert(cluster_id.clone(), era, billing_report);

			Self::deposit_event(Event::<T>::RewardingStarted { cluster_id, era });

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn send_rewarding_providers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			payees: Vec<(T::AccountId, Perbill)>,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let billing_report = ActiveBillingReports::<T>::try_get(cluster_id.clone(), era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(
				billing_report.state == State::RewardingProviders,
				Error::<T>::NotExpectedState
			);
			ensure!(
				billing_report.rewarding_max_batch_index >= batch_index,
				Error::<T>::BatchIndexIsOutOfRange
			);
			ensure!(
				!billing_report.rewarding_processed_batches.contains(&batch_index),
				Error::<T>::BatchIndexAlreadyProcessed
			);

			let mut updated_billing_report = billing_report.clone();
			for payee in payees {
				let _provider = payee.0; // todo: reward provider
				let share = payee.1;
				let amount = share.mul(billing_report.total_balance);
				updated_billing_report.distributed_balance += amount;
			}

			updated_billing_report
				.rewarding_processed_batches
				.try_push(batch_index)
				.map_err(|_| Error::<T>::BoundedVecOverflow)?;

			ActiveBillingReports::<T>::insert(cluster_id, era, updated_billing_report);

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn end_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id.clone(), era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(
				billing_report.state == State::RewardingProviders,
				Error::<T>::NotExpectedState
			);
			ensure!(
				billing_report.rewarding_max_batch_index as usize ==
					billing_report.rewarding_processed_batches.len() - 1usize,
				Error::<T>::BatchesMissed
			);

			billing_report.state = State::ProvidersRewarded;
			ActiveBillingReports::<T>::insert(cluster_id.clone(), era, billing_report);

			Self::deposit_event(Event::<T>::RewardingFinished { cluster_id, era });

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn end_billing_report(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id.clone(), era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == State::ProvidersRewarded, Error::<T>::NotExpectedState);
			ensure!(
				billing_report.total_balance == billing_report.distributed_balance,
				Error::<T>::NotDistributedBalance
			);

			billing_report.state = State::Finalized;
			// todo: clear and archive billing_report
			ActiveBillingReports::<T>::insert(cluster_id.clone(), era, billing_report);

			Self::deposit_event(Event::<T>::BillingReportFinalized { cluster_id, era });

			Ok(())
		}
	}
}
