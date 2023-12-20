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

pub mod weights;
use crate::weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

use ddc_primitives::{ClusterId, DdcEra};
use ddc_traits::{
	cluster::{ClusterCreator as ClusterCreatorType, ClusterVisitor as ClusterVisitorType},
	customer::{
		CustomerCharger as CustomerChargerType, CustomerDepositor as CustomerDepositorType,
	},
	pallet::PalletVisitor as PalletVisitorType,
};
use frame_election_provider_support::SortedListProvider;
use frame_support::{
	pallet_prelude::*,
	parameter_types,
	sp_runtime::SaturatedConversion,
	traits::{Currency, ExistenceRequirement, LockableCurrency},
	BoundedBTreeSet,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sp_runtime::{PerThing, Perbill};
use sp_std::prelude::*;

type BatchIndex = u16;

/// Stores usage of customers
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct CustomerUsage {
	pub transferred_bytes: u64,
	pub stored_bytes: u64,
	pub number_of_puts: u128,
	pub number_of_gets: u128,
}

/// Stores usage of node provider
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct NodeUsage {
	pub transferred_bytes: u64,
	pub stored_bytes: u64,
	pub number_of_puts: u128,
	pub number_of_gets: u128,
}

/// Stores reward in tokens(units) of node provider as per NodeUsage
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct NodeReward {
	pub transfer: u128, // reward in tokens for NodeUsage::transferred_bytes
	pub storage: u128,  // reward in tokens for NodeUsage::stored_bytes
	pub puts: u128,     // reward in tokens for NodeUsage::number_of_puts
	pub gets: u128,     // reward in tokens for NodeUsage::number_of_gets
}

#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct BillingReportDebt {
	pub cluster_id: ClusterId,
	pub era: DdcEra,
	pub batch_index: BatchIndex,
	pub amount: u128,
}

/// Stores charge in tokens(units) of customer as per CustomerUsage
#[derive(PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default, Clone)]
pub struct CustomerCharge {
	pub transfer: u128, // charge in tokens for CustomerUsage::transferred_bytes
	pub storage: u128,  // charge in tokens for CustomerUsage::stored_bytes
	pub puts: u128,     // charge in tokens for CustomerUsage::number_of_puts
	pub gets: u128,     // charge in tokens for CustomerUsage::number_of_gets
}

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

parameter_types! {
	pub MaxBatchesCount: u16 = 1000;
	pub MaxBatchSize: u16 = 1000;
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::PalletId;
	use sp_io::hashing::blake2_128;
	use sp_runtime::traits::{AccountIdConversion, Zero};

	use super::*;

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
		type CustomerCharger: CustomerChargerType<Self>;
		type CustomerDepositor: CustomerDepositorType<Self>;
		type TreasuryVisitor: PalletVisitorType<Self>;
		type ClusterVisitor: ClusterVisitorType<Self>;
		type ValidatorList: SortedListProvider<Self::AccountId>;
		type ClusterCreator: ClusterCreatorType<Self, BalanceOf<Self>>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		BillingReportInitialized {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		ChargingStarted {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		Charged {
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			customer_id: T::AccountId,
			amount: u128,
		},
		ChargeFailed {
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			customer_id: T::AccountId,
			amount: u128,
		},
		Indebted {
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			customer_id: T::AccountId,
			amount: u128,
		},
		ChargingFinished {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		TreasuryFeesCollected {
			cluster_id: ClusterId,
			era: DdcEra,
			amount: u128,
		},
		ClusterReserveFeesCollected {
			cluster_id: ClusterId,
			era: DdcEra,
			amount: u128,
		},
		ValidatorFeesCollected {
			cluster_id: ClusterId,
			era: DdcEra,
			amount: u128,
		},
		RewardingStarted {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		Rewarded {
			cluster_id: ClusterId,
			era: DdcEra,
			node_provider_id: T::AccountId,
			amount: u128,
		},
		RewardingFinished {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		BillingReportFinalized {
			cluster_id: ClusterId,
			era: DdcEra,
		},
		AuthorisedCaller {
			authorised_caller: T::AccountId,
		},
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
		NotExpectedClusterState,
		BatchSizeIsOutOfBounds,
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
	>;

	#[pallet::storage]
	#[pallet::getter(fn authorised_caller)]
	pub type AuthorisedCaller<T: Config> = StorageValue<_, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn debtor_customers)]
	pub type DebtorCustomers<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ClusterId, Blake2_128Concat, T::AccountId, u128>;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct BillingReport<T: Config> {
		pub state: State,
		pub vault: T::AccountId,
		pub total_customer_charge: CustomerCharge,
		pub total_distributed_reward: u128,
		pub total_node_usage: NodeUsage,
		// stage 1
		pub charging_max_batch_index: BatchIndex,
		pub charging_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
		// stage 2
		pub rewarding_max_batch_index: BatchIndex,
		pub rewarding_processed_batches: BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
	}

	impl<T: pallet::Config> Default for BillingReport<T> {
		fn default() -> Self {
			Self {
				state: State::default(),
				vault: T::PalletId::get().into_account_truncating(),
				total_customer_charge: CustomerCharge::default(),
				total_distributed_reward: Zero::zero(),
				total_node_usage: NodeUsage::default(),
				charging_max_batch_index: Zero::zero(),
				charging_processed_batches: BoundedBTreeSet::default(),
				rewarding_max_batch_index: Zero::zero(),
				rewarding_processed_batches: BoundedBTreeSet::default(),
			}
		}
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Default)]
	// don't remove or change numbers, if needed add a new state to the end with new number
	// DAC uses the state value for integration!
	pub enum State {
		#[default]
		NotInitialized = 1,
		Initialized = 2,
		ChargingCustomers = 3,
		CustomersChargedWithFees = 4,
		RewardingProviders = 5,
		ProvidersRewarded = 6,
		Finalized = 7,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub authorised_caller: Option<T::AccountId>,
		pub debtor_customers: Vec<(ClusterId, T::AccountId, u128)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				authorised_caller: Default::default(),
				debtor_customers: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			AuthorisedCaller::<T>::set(self.authorised_caller.clone());

			for (cluster_id, customer_id, debt) in &self.debtor_customers {
				DebtorCustomers::<T>::insert(cluster_id, customer_id, debt);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::set_authorised_caller())]
		pub fn set_authorised_caller(
			origin: OriginFor<T>,
			authorised_caller: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?; // requires Governance approval

			AuthorisedCaller::<T>::put(authorised_caller.clone());

			Self::deposit_event(Event::<T>::AuthorisedCaller { authorised_caller });

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::begin_billing_report())]
		pub fn begin_billing_report(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::authorised_caller() == Some(caller), Error::<T>::Unauthorised);

			ensure!(
				ActiveBillingReports::<T>::try_get(cluster_id, era).is_err(),
				Error::<T>::NotExpectedState
			);

			let mut billing_report = BillingReport::<T> {
				vault: Self::sub_account_id(cluster_id, era),
				state: State::Initialized,
				..Default::default()
			};
			billing_report.vault = Self::sub_account_id(cluster_id, era);
			billing_report.state = State::Initialized;
			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);

			Self::deposit_event(Event::<T>::BillingReportInitialized { cluster_id, era });

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::begin_charging_customers())]
		pub fn begin_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			max_batch_index: BatchIndex,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::authorised_caller() == Some(caller), Error::<T>::Unauthorised);

			ensure!(max_batch_index < MaxBatchesCount::get(), Error::<T>::BatchIndexOverflow);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == State::Initialized, Error::<T>::NotExpectedState);

			billing_report.charging_max_batch_index = max_batch_index;
			billing_report.state = State::ChargingCustomers;
			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);

			Self::deposit_event(Event::<T>::ChargingStarted { cluster_id, era });

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::send_charging_customers_batch(payers.len().saturated_into()))]
		pub fn send_charging_customers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			payers: Vec<(T::AccountId, CustomerUsage)>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::authorised_caller() == Some(caller), Error::<T>::Unauthorised);

			ensure!(
				!payers.is_empty() && payers.len() <= MaxBatchSize::get() as usize,
				Error::<T>::BatchSizeIsOutOfBounds
			);

			let billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
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

			let mut updated_billing_report = billing_report;
			for payer in payers {
				let mut customer_charge = get_customer_charge::<T>(cluster_id, &payer.1)?;
				let total_customer_charge = (|| -> Option<u128> {
					customer_charge
						.transfer
						.checked_add(customer_charge.storage)?
						.checked_add(customer_charge.puts)?
						.checked_add(customer_charge.gets)
				})()
				.ok_or(Error::<T>::ArithmeticOverflow)?;

				let customer_id = payer.0.clone();
				let amount_actually_charged = match T::CustomerCharger::charge_content_owner(
					customer_id.clone(),
					updated_billing_report.vault.clone(),
					total_customer_charge,
				) {
					Ok(actually_charged) => actually_charged,
					Err(_e) => 0,
				};

				if amount_actually_charged < total_customer_charge {
					// debt
					let mut customer_debt =
						DebtorCustomers::<T>::try_get(cluster_id, customer_id.clone())
							.unwrap_or_else(|_| Zero::zero());

					let debt = total_customer_charge
						.checked_sub(amount_actually_charged)
						.ok_or(Error::<T>::ArithmeticOverflow)?;

					customer_debt =
						customer_debt.checked_add(debt).ok_or(Error::<T>::ArithmeticOverflow)?;

					DebtorCustomers::<T>::insert(cluster_id, customer_id.clone(), customer_debt);

					Self::deposit_event(Event::<T>::Indebted {
						cluster_id,
						era,
						batch_index,
						customer_id: customer_id.clone(),
						amount: debt,
					});

					Self::deposit_event(Event::<T>::ChargeFailed {
						cluster_id,
						era,
						batch_index,
						customer_id,
						amount: total_customer_charge,
					});

					if amount_actually_charged > 0 {
						// something was charged and should be added
						// calculate ratio
						let ratio =
							Perbill::from_rational(amount_actually_charged, total_customer_charge);

						customer_charge.storage = ratio * customer_charge.storage;
						customer_charge.transfer = ratio * customer_charge.transfer;
						customer_charge.gets = ratio * customer_charge.gets;
						customer_charge.puts = ratio * customer_charge.puts;
					}
				} else {
					Self::deposit_event(Event::<T>::Charged {
						cluster_id,
						era,
						batch_index,
						customer_id,
						amount: total_customer_charge,
					});
				}

				updated_billing_report.total_customer_charge.storage = updated_billing_report
					.total_customer_charge
					.storage
					.checked_add(customer_charge.storage)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				updated_billing_report.total_customer_charge.transfer = updated_billing_report
					.total_customer_charge
					.transfer
					.checked_add(customer_charge.transfer)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				updated_billing_report.total_customer_charge.puts = updated_billing_report
					.total_customer_charge
					.puts
					.checked_add(customer_charge.puts)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				updated_billing_report.total_customer_charge.gets = updated_billing_report
					.total_customer_charge
					.gets
					.checked_add(customer_charge.gets)
					.ok_or(Error::<T>::ArithmeticOverflow)?;
			}

			updated_billing_report
				.charging_processed_batches
				.try_insert(batch_index)
				.map_err(|_| Error::<T>::BoundedVecOverflow)?;

			ActiveBillingReports::<T>::insert(cluster_id, era, updated_billing_report);

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::end_charging_customers())]
		pub fn end_charging_customers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::authorised_caller() == Some(caller), Error::<T>::Unauthorised);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == State::ChargingCustomers, Error::<T>::NotExpectedState);
			validate_batches::<T>(
				&billing_report.charging_processed_batches,
				&billing_report.charging_max_batch_index,
			)?;

			Self::deposit_event(Event::<T>::ChargingFinished { cluster_id, era });

			// deduct fees
			let fees = T::ClusterVisitor::get_fees_params(&cluster_id)
				.map_err(|_| Error::<T>::NotExpectedClusterState)?;

			let total_customer_charge = (|| -> Option<u128> {
				billing_report
					.total_customer_charge
					.transfer
					.checked_add(billing_report.total_customer_charge.storage)?
					.checked_add(billing_report.total_customer_charge.puts)?
					.checked_add(billing_report.total_customer_charge.gets)
			})()
			.ok_or(Error::<T>::ArithmeticOverflow)?;

			let treasury_fee = fees.treasury_share * total_customer_charge;
			let validators_fee = fees.validators_share * total_customer_charge;
			let cluster_reserve_fee = fees.cluster_reserve_share * total_customer_charge;

			if treasury_fee > 0 {
				charge_treasury_fees::<T>(
					treasury_fee,
					&billing_report.vault,
					&T::TreasuryVisitor::get_account_id(),
				)?;

				Self::deposit_event(Event::<T>::TreasuryFeesCollected {
					cluster_id,
					era,
					amount: treasury_fee,
				});
			}

			if cluster_reserve_fee > 0 {
				charge_cluster_reserve_fees::<T>(
					cluster_reserve_fee,
					&billing_report.vault,
					&T::ClusterVisitor::get_reserve_account_id(&cluster_id)
						.map_err(|_| Error::<T>::NotExpectedClusterState)?,
				)?;
				Self::deposit_event(Event::<T>::ClusterReserveFeesCollected {
					cluster_id,
					era,
					amount: cluster_reserve_fee,
				});
			}

			if validators_fee > 0 {
				charge_validator_fees::<T>(validators_fee, &billing_report.vault)?;
				Self::deposit_event(Event::<T>::ValidatorFeesCollected {
					cluster_id,
					era,
					amount: validators_fee,
				});
			}

			// 1 - (X + Y + Z) > 0, 0 < X + Y + Z < 1
			let total_left_from_one =
				(fees.treasury_share + fees.validators_share + fees.cluster_reserve_share)
					.left_from_one();

			if !total_left_from_one.is_zero() {
				// X * Z < X, 0 < Z < 1
				billing_report.total_customer_charge.transfer =
					total_left_from_one * billing_report.total_customer_charge.transfer;
				billing_report.total_customer_charge.storage =
					total_left_from_one * billing_report.total_customer_charge.storage;
				billing_report.total_customer_charge.puts =
					total_left_from_one * billing_report.total_customer_charge.puts;
				billing_report.total_customer_charge.gets =
					total_left_from_one * billing_report.total_customer_charge.gets;
			}

			billing_report.state = State::CustomersChargedWithFees;
			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::begin_rewarding_providers())]
		pub fn begin_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			max_batch_index: BatchIndex,
			total_node_usage: NodeUsage,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::authorised_caller() == Some(caller), Error::<T>::Unauthorised);

			ensure!(max_batch_index < MaxBatchesCount::get(), Error::<T>::BatchIndexOverflow);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(
				billing_report.state == State::CustomersChargedWithFees,
				Error::<T>::NotExpectedState
			);

			billing_report.total_node_usage = total_node_usage;
			billing_report.rewarding_max_batch_index = max_batch_index;
			billing_report.state = State::RewardingProviders;
			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);

			Self::deposit_event(Event::<T>::RewardingStarted { cluster_id, era });

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::send_rewarding_providers_batch(payees.len().saturated_into()))]
		pub fn send_rewarding_providers_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			batch_index: BatchIndex,
			payees: Vec<(T::AccountId, NodeUsage)>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::authorised_caller() == Some(caller), Error::<T>::Unauthorised);

			ensure!(
				!payees.is_empty() && payees.len() <= MaxBatchSize::get() as usize,
				Error::<T>::BatchSizeIsOutOfBounds
			);

			let billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
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
				let node_reward = get_node_reward::<T>(
					&payee.1,
					&billing_report.total_node_usage,
					&billing_report.total_customer_charge,
				)
				.ok_or(Error::<T>::ArithmeticOverflow)?;
				let amount_to_reward = (|| -> Option<u128> {
					node_reward
						.transfer
						.checked_add(node_reward.storage)?
						.checked_add(node_reward.puts)?
						.checked_add(node_reward.gets)
				})()
				.ok_or(Error::<T>::ArithmeticOverflow)?;

				let node_provider_id = payee.0;
				let reward: BalanceOf<T> = amount_to_reward.saturated_into::<BalanceOf<T>>();

				<T as pallet::Config>::Currency::transfer(
					&updated_billing_report.vault,
					&node_provider_id,
					reward,
					ExistenceRequirement::AllowDeath,
				)?;

				updated_billing_report
					.total_distributed_reward
					.checked_add(amount_to_reward)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				Self::deposit_event(Event::<T>::Rewarded {
					cluster_id,
					era,
					node_provider_id,
					amount: amount_to_reward,
				});
			}

			updated_billing_report
				.rewarding_processed_batches
				.try_insert(batch_index)
				.map_err(|_| Error::<T>::BoundedVecOverflow)?;

			ActiveBillingReports::<T>::insert(cluster_id, era, updated_billing_report);

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::end_rewarding_providers())]
		pub fn end_rewarding_providers(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::authorised_caller() == Some(caller), Error::<T>::Unauthorised);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
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
			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);

			Self::deposit_event(Event::<T>::RewardingFinished { cluster_id, era });

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::end_billing_report())]
		pub fn end_billing_report(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::authorised_caller() == Some(caller), Error::<T>::Unauthorised);

			let mut billing_report = ActiveBillingReports::<T>::try_get(cluster_id, era)
				.map_err(|_| Error::<T>::BillingReportDoesNotExist)?;

			ensure!(billing_report.state == State::ProvidersRewarded, Error::<T>::NotExpectedState);
			let expected_amount_to_reward = (|| -> Option<u128> {
				billing_report
					.total_customer_charge
					.transfer
					.checked_add(billing_report.total_customer_charge.storage)?
					.checked_add(billing_report.total_customer_charge.puts)?
					.checked_add(billing_report.total_customer_charge.gets)
			})()
			.ok_or(Error::<T>::ArithmeticOverflow)?;

			ensure!(
				expected_amount_to_reward == billing_report.total_distributed_reward,
				Error::<T>::NotDistributedBalance
			);

			billing_report.charging_processed_batches.clear();
			billing_report.rewarding_processed_batches.clear();
			billing_report.state = State::Finalized;

			ActiveBillingReports::<T>::insert(cluster_id, era, billing_report);
			Self::deposit_event(Event::<T>::BillingReportFinalized { cluster_id, era });

			Ok(())
		}
	}

	fn charge_treasury_fees<T: Config>(
		treasury_fee: u128,
		vault: &T::AccountId,
		treasury_vault: &T::AccountId,
	) -> DispatchResult {
		let amount_to_deduct = treasury_fee.saturated_into::<BalanceOf<T>>();
		<T as pallet::Config>::Currency::transfer(
			vault,
			treasury_vault,
			amount_to_deduct,
			ExistenceRequirement::AllowDeath,
		)
	}

	fn charge_cluster_reserve_fees<T: Config>(
		cluster_reserve_fee: u128,
		vault: &T::AccountId,
		reserve_vault: &T::AccountId,
	) -> DispatchResult {
		let amount_to_deduct = cluster_reserve_fee.saturated_into::<BalanceOf<T>>();
		<T as pallet::Config>::Currency::transfer(
			vault,
			reserve_vault,
			amount_to_deduct,
			ExistenceRequirement::AllowDeath,
		)
	}

	fn charge_validator_fees<T: Config>(
		validators_fee: u128,
		vault: &T::AccountId,
	) -> DispatchResult {
		let amount_to_deduct = validators_fee
			.checked_div(T::ValidatorList::count().try_into().unwrap())
			.ok_or(Error::<T>::ArithmeticOverflow)?
			.saturated_into::<BalanceOf<T>>();

		for validator_account_id in T::ValidatorList::iter() {
			<T as pallet::Config>::Currency::transfer(
				vault,
				&validator_account_id,
				amount_to_deduct,
				ExistenceRequirement::AllowDeath,
			)?;
		}

		Ok(())
	}

	fn get_node_reward<T: Config>(
		node_usage: &NodeUsage,
		total_nodes_usage: &NodeUsage,
		total_customer_charge: &CustomerCharge,
	) -> Option<NodeReward> {
		let mut node_reward = NodeReward::default();

		let mut ratio = Perbill::from_rational(
			node_usage.transferred_bytes,
			total_nodes_usage.transferred_bytes,
		);
		// ratio multiplied by X will be > 0, < X no overflow
		node_reward.transfer = ratio * total_customer_charge.transfer;

		ratio = Perbill::from_rational(node_usage.stored_bytes, total_nodes_usage.stored_bytes);
		node_reward.storage = ratio * total_customer_charge.storage;

		ratio = Perbill::from_rational(node_usage.number_of_puts, total_nodes_usage.number_of_puts);
		node_reward.puts = ratio * total_customer_charge.puts;

		ratio = Perbill::from_rational(node_usage.number_of_gets, total_nodes_usage.number_of_gets);
		node_reward.gets = ratio * total_customer_charge.gets;

		Some(node_reward)
	}

	fn get_customer_charge<T: Config>(
		cluster_id: ClusterId,
		usage: &CustomerUsage,
	) -> Result<CustomerCharge, Error<T>> {
		let mut total = CustomerCharge::default();

		let pricing = T::ClusterVisitor::get_pricing_params(&cluster_id)
			.map_err(|_| Error::<T>::NotExpectedClusterState)?;

		total.transfer = (|| -> Option<u128> {
			(usage.transferred_bytes as u128)
				.checked_mul(pricing.unit_per_mb_streamed)?
				.checked_div(byte_unit::MEBIBYTE)
		})()
		.ok_or(Error::<T>::ArithmeticOverflow)?;

		total.storage = (|| -> Option<u128> {
			(usage.stored_bytes as u128)
				.checked_mul(pricing.unit_per_mb_stored)?
				.checked_div(byte_unit::MEBIBYTE)
		})()
		.ok_or(Error::<T>::ArithmeticOverflow)?;

		total.gets = usage
			.number_of_gets
			.checked_mul(pricing.unit_per_get_request)
			.ok_or(Error::<T>::ArithmeticOverflow)?;

		total.puts = usage
			.number_of_puts
			.checked_mul(pricing.unit_per_put_request)
			.ok_or(Error::<T>::ArithmeticOverflow)?;

		Ok(total)
	}

	fn validate_batches<T: Config>(
		batches: &BoundedBTreeSet<BatchIndex, MaxBatchesCount>,
		max_batch_index: &BatchIndex,
	) -> DispatchResult {
		// Check if the Vec contains all integers between 1 and rewarding_max_batch_index
		ensure!(!batches.is_empty(), Error::<T>::BatchesMissed);

		ensure!((*max_batch_index + 1) as usize == batches.len(), Error::<T>::BatchesMissed);

		for index in 0..*max_batch_index + 1 {
			ensure!(batches.contains(&index), Error::<T>::BatchesMissed);
		}

		Ok(())
	}

	impl<T: Config> Pallet<T> {
		pub fn sub_account_id(cluster_id: ClusterId, era: DdcEra) -> T::AccountId {
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
