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
use frame_support::{
	pallet_prelude::*,
	parameter_types,
	sp_runtime::SaturatedConversion,
	traits::{Currency, ExistenceRequirement, LockableCurrency},
	BoundedBTreeSet,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sp_runtime::Perbill;
use sp_std::prelude::*;

type BatchIndex = u16;

#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct CustomerUsage {
	pub transferred_bytes: u128,
	pub stored_bytes: u128,
	pub number_of_puts: u128,
	pub number_of_gets: u128,
}

#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct NodeUsage {
	pub transferred_bytes: u128,
	pub stored_bytes: u128,
	pub number_of_puts: u128,
	pub number_of_gets: u128,
}

#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct NodeReward {
	pub transfer: u128,
	pub storage: u128,
	pub puts: u128,
	pub gets: u128,
}

#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct CustomerCharge {
	pub transfer: u128,
	pub storage: u128,
	pub puts: u128,
	pub gets: u128,
}

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

parameter_types! {
	pub MaxBatchesCount: u16 = 1000;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::PalletId;
	use sp_io::hashing::blake2_128;
	use sp_runtime::traits::{AccountIdConversion, Zero};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		BillingReportInitialized { cluster_id: ClusterId, era: DdcEra },
		ChargingStarted { cluster_id: ClusterId, era: DdcEra },
		Charged { cluster_id: ClusterId, era: DdcEra, customer_id: T::AccountId, amount: u128 },
		ChargingFinished { cluster_id: ClusterId, era: DdcEra },
		RewardingStarted { cluster_id: ClusterId, era: DdcEra },
		Rewarded { cluster_id: ClusterId, era: DdcEra, node_id: T::AccountId, amount: u128 },
		RewardingFinished { cluster_id: ClusterId, era: DdcEra },
		BillingReportFinalized { cluster_id: ClusterId, era: DdcEra },
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		BillingReportDoesNotExist,
		NotExpectedState,
		Unauthorised,
		BatchIndexAlreadyProcessed,
		BatchIndexIsOutOfRange,
		BatchesMissed,
		NotDistributedBalance,
		BatchIndexOverflow,
		BoundedVecOverflow,
		ArithmeticOverflow,
	}

	#[pallet::storage]
	#[pallet::getter(fn active_billing_reports)]
	pub type ActiveBillingReports<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		DdcEra,
		BillingReport<T>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn dac_account)]
	pub type DACAccount<T: Config> = StorageValue<_, T::AccountId>;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct BillingReport<T: Config> {
		state: State,
		vault: T::AccountId,
		dac_account: Option<T::AccountId>,
		total_charged_balance: u128,
		total_distributed_balance: u128,
		total_node_expected_reward: NodeReward,
		total_node_expected_usage: NodeUsage,
		// stage 1
		charging_max_batch_index: BatchIndex,
		charging_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
		// stage 2
		rewarding_max_batch_index: BatchIndex,
		rewarding_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
	}

	impl<T: pallet::Config> Default for BillingReport<T> {
		fn default() -> Self {
			Self {
				state: State::default(),
				vault: T::PalletId::get().into_account_truncating(),
				dac_account: Option::None,
				total_charged_balance: Zero::zero(),
				total_distributed_balance: Zero::zero(),
				total_node_expected_usage: NodeUsage::default(),
				total_node_expected_reward: NodeReward::default(),
				charging_max_batch_index: Zero::zero(),
				charging_processed_batches: BoundedBTreeSet::default(),
				rewarding_max_batch_index: Zero::zero(),
				rewarding_processed_batches: BoundedBTreeSet::default(),
			}
		}
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
			let caller = ensure_signed(origin)?;
			ensure!(
				Self::dac_account().ok_or(Error::<T>::Unauthorised)? == caller,
				Error::<T>::Unauthorised
			);

			let mut billing_report = BillingReport::default();
			billing_report.vault = Self::sub_account_id(cluster_id.clone(), era);
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
			let caller = ensure_signed(origin)?;
			ensure!(
				Self::dac_account().ok_or(Error::<T>::Unauthorised)? == caller,
				Error::<T>::Unauthorised
			);

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
			payers: Vec<(T::AccountId, CustomerUsage)>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(
				Self::dac_account().ok_or(Error::<T>::Unauthorised)? == caller,
				Error::<T>::Unauthorised
			);

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
				let customer_charge =
					get_customer_charge(&payer.1).ok_or(Error::<T>::ArithmeticOverflow)?;
				let amount = (|| -> Option<u128> {
					customer_charge
						.transfer
						.checked_add(customer_charge.storage)?
						.checked_add(customer_charge.puts)?
						.checked_add(customer_charge.gets)
				})()
				.ok_or(Error::<T>::ArithmeticOverflow)?;

				// todo: charge customer
				let customer_id = payer.0;

				updated_billing_report
					.total_charged_balance
					.checked_add(amount)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				Self::deposit_event(Event::<T>::Charged { cluster_id, era, customer_id, amount });
			}

			updated_billing_report
				.charging_processed_batches
				.try_insert(batch_index)
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
			let caller = ensure_signed(origin)?;
			ensure!(
				Self::dac_account().ok_or(Error::<T>::Unauthorised)? == caller,
				Error::<T>::Unauthorised
			);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id.clone(), era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == State::ChargingCustomers, Error::<T>::NotExpectedState);
			validate_batches::<T>(
				&billing_report.charging_processed_batches,
				&billing_report.charging_max_batch_index,
			)?;

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
			total_node_usage: NodeUsage,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(
				Self::dac_account().ok_or(Error::<T>::Unauthorised)? == caller,
				Error::<T>::Unauthorised
			);

			ensure!(
				max_batch_index > 0 && max_batch_index < MaxBatchesCount::get(),
				Error::<T>::BatchIndexOverflow
			);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id.clone(), era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == State::CustomersCharged, Error::<T>::NotExpectedState);

			let total =
				get_total_usage_reward(&total_node_usage).ok_or(Error::<T>::ArithmeticOverflow)?;

			billing_report.total_node_expected_usage = total_node_usage;
			billing_report.rewarding_max_batch_index = max_batch_index;
			billing_report.total_node_expected_reward = total;
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
			payees: Vec<(T::AccountId, NodeUsage)>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(
				Self::dac_account().ok_or(Error::<T>::Unauthorised)? == caller,
				Error::<T>::Unauthorised
			);

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
				let node_reward = get_node_reward(
					&payee.1,
					&billing_report.total_node_expected_usage,
					&billing_report.total_node_expected_reward,
				);
				let amount = (|| -> Option<u128> {
					node_reward
						.transfer
						.checked_add(node_reward.storage)?
						.checked_add(node_reward.puts)?
						.checked_add(node_reward.gets)
				})()
				.ok_or(Error::<T>::ArithmeticOverflow)?;

				let node_id = payee.0;
				let charge: BalanceOf<T> = amount.saturated_into::<BalanceOf<T>>();

				<T as pallet::Config>::Currency::transfer(
					&updated_billing_report.vault,
					&node_id,
					charge,
					ExistenceRequirement::KeepAlive,
				)?;

				updated_billing_report
					.total_distributed_balance
					.checked_add(amount)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				Self::deposit_event(Event::<T>::Rewarded { cluster_id, era, node_id, amount });
			}

			updated_billing_report
				.rewarding_processed_batches
				.try_insert(batch_index)
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
			let caller = ensure_signed(origin)?;
			ensure!(
				Self::dac_account().ok_or(Error::<T>::Unauthorised)? == caller,
				Error::<T>::Unauthorised
			);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id.clone(), era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(
				billing_report.state == State::RewardingProviders,
				Error::<T>::NotExpectedState
			);

			validate_batches::<T>(
				&billing_report.rewarding_processed_batches,
				&billing_report.rewarding_max_batch_index,
			)?;

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
			let caller = ensure_signed(origin)?;
			ensure!(
				Self::dac_account().ok_or(Error::<T>::Unauthorised)? == caller,
				Error::<T>::Unauthorised
			);

			let billing_report = ActiveBillingReports::<T>::try_get(cluster_id.clone(), era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == State::ProvidersRewarded, Error::<T>::NotExpectedState);
			ensure!(
				billing_report.total_charged_balance == billing_report.total_distributed_balance,
				Error::<T>::NotDistributedBalance
			);

			ActiveBillingReports::<T>::remove(cluster_id.clone(), era);
			Self::deposit_event(Event::<T>::BillingReportFinalized { cluster_id, era });

			Ok(())
		}
	}

	fn get_node_reward(
		node_usage: &NodeUsage,
		total_usage: &NodeUsage,
		total_reward: &NodeReward,
	) -> NodeReward {
		let mut node_reward = NodeReward::default();

		let mut ratio =
			Perbill::from_rational(node_usage.transferred_bytes, total_usage.transferred_bytes);
		node_reward.transfer = (ratio * total_reward.transfer) as u128;

		ratio = Perbill::from_rational(node_usage.stored_bytes, total_usage.stored_bytes);
		node_reward.storage = (ratio * total_reward.storage) as u128;

		ratio = Perbill::from_rational(node_usage.number_of_puts, total_usage.number_of_puts);
		node_reward.puts = (ratio * total_reward.puts) as u128;

		ratio = Perbill::from_rational(node_usage.number_of_gets, total_usage.number_of_gets);
		node_reward.gets = (ratio * total_reward.gets) as u128;

		node_reward
	}

	// todo: to calculate actual charge based on the metrics
	fn get_total_usage_reward(total_usage: &NodeUsage) -> Option<NodeReward> {
		let mut total = NodeReward::default();

		total.transfer = 1;
		total.storage = 2;
		total.puts = 3;
		total.gets = 4;

		Option::Some(total)
	}

	// todo: to calculate actual charge based on the metrics
	fn get_customer_charge(usage: &CustomerUsage) -> Option<CustomerCharge> {
		let mut total = CustomerCharge::default();

		total.transfer = 1;
		total.storage = 2;
		total.puts = 3;
		total.gets = 4;

		Option::Some(total)
	}

	fn validate_batches<T: Config>(
		batches: &BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
		max_batch_index: &BatchIndex,
	) -> DispatchResult {
		// Check if the Vec contains all integers between 1 and rewarding_max_batch_index
		ensure!(!batches.is_empty(), Error::<T>::BatchesMissed);

		ensure!(*max_batch_index as usize == batches.len() - 1usize, Error::<T>::BatchesMissed);

		for index in 0..*max_batch_index {
			ensure!(batches.contains(&index), Error::<T>::BatchesMissed);
		}

		Ok(())
	}

	impl<T: Config> Pallet<T> {
		fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		fn sub_account_id(cluster_id: ClusterId, era: DdcEra) -> T::AccountId {
			let mut bytes = Vec::new();
			bytes.extend_from_slice(&cluster_id[..]);
			bytes.extend_from_slice(&era.encode());
			let hash = blake2_128(&bytes);
			// "modl" + "payouts_" + hash is 28 bytes, the T::AccountId is 32 bytes, so we should be
			// safe from the truncation and possible collisions caused by it. The rest 4 bytes will
			// be fulfilled with trailing zeros.
			T::PalletId::get().into_sub_account_truncating(hash)
		}
	}
}
