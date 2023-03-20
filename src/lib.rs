#![cfg_attr(not(feature = "std"), no_std)]

pub use frame_support::{
	pallet_prelude::*, parameter_types, traits::Randomness, weights::Weight, BoundedVec,
};
pub use frame_system::pallet_prelude::*;
pub use pallet_ddc_staking::{self as ddc_staking};
pub use pallet_staking::{self as staking};
pub use pallet::*;
pub use sp_std::prelude::*;

parameter_types! {
	pub DdcValidatorsQuorumSize: u32 = 3;
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ValidationMethodKind {
	ProofOfDelivery,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Decision<AccountId> {
	pub decision: Option<bool>,
	pub method: ValidationMethodKind,
	pub validator: AccountId,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_staking::Config + ddc_staking::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
	}

	#[pallet::storage]
	#[pallet::getter(fn tasks)]
	pub type Tasks<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<Decision<T::AccountId>, DdcValidatorsQuorumSize>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn global_era_counter)]
	pub type GlobalEraCounter<T: Config> = StorageValue<_, u32>;

	#[pallet::storage]
	#[pallet::getter(fn last_managed_era)]
	pub type LastManagedEra<T: Config> = StorageValue<_, u32>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100_000)]
		pub fn inc_era(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(era) = <GlobalEraCounter<T>>::get() {
				let new_era = era.checked_add(1).unwrap_or_default();
				<GlobalEraCounter<T>>::put(new_era);
			} else {
				<GlobalEraCounter<T>>::put(1);
			}
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			match (<GlobalEraCounter<T>>::get(), <LastManagedEra<T>>::get()) {
				(Some(global_era_counter), Some(last_managed_era)) => {
					if last_managed_era >= global_era_counter {
						return 0
					}
					<LastManagedEra<T>>::put(global_era_counter);
				},
				(Some(global_era_counter), None) => {
					<LastManagedEra<T>>::put(global_era_counter);
				},
				_ => { return 0 },
			};

			let validators: Vec<T::AccountId> = <staking::Validators<T>>::iter_keys().collect();
			let validators_count = validators.len() as u32;
			let edges: Vec<T::AccountId> = <ddc_staking::pallet::Edges<T>>::iter_keys().collect();
			log::info!(
				"Block number: {:?}, global era: {:?}, last era: {:?}, validators_count: {:?}, validators: {:?}, edges: {:?}",
				block_number,
				<GlobalEraCounter<T>>::get(),
				<LastManagedEra<T>>::get(),
				validators_count,
				validators,
				edges,
			);

			// A naive approach assigns random validators for each edge.
			for edge in edges {
				let mut decisions: BoundedVec<Decision<T::AccountId>, DdcValidatorsQuorumSize> =
					Default::default();
				while !decisions.is_full() {
					let validator_idx = Self::choose(validators_count).unwrap_or(0) as usize;
					let validator: T::AccountId = validators[validator_idx].clone();
					let assignment = Decision {
						validator,
						method: ValidationMethodKind::ProofOfDelivery,
						decision: None,
					};
					decisions.try_push(assignment).unwrap();
				}
				Tasks::<T>::insert(edge, decisions);
			}
			0
		}
		fn offchain_worker(block_number: T::BlockNumber) {
			log::info!("Off-chain worker at block {:?}", block_number);
		}
	}

	impl<T: Config> Pallet<T> {
		/// Fetch the tasks related to current validator
		fn fetch_tasks(validator: T::AccountId) -> Vec<T::AccountId> {
			let mut cdn_nodes: Vec<T::AccountId> = vec![];
			for (cdn_id, cdn_tasks) in <Tasks<T>>::iter() {
				for decision in cdn_tasks.iter() {
					if decision.validator == validator {
						cdn_nodes.push(cdn_id);
						break;
					}
				}
			}
			cdn_nodes
		}

		fn choose(total: u32) -> Option<u32> {
			if total == 0 {
				return None
			}
			let mut random_number = Self::generate_random_number(0);

			// Best effort attempt to remove bias from modulus operator.
			for i in 1..128 {
				if random_number < u32::MAX - u32::MAX % total {
					break
				}

				random_number = Self::generate_random_number(i);
			}

			Some(random_number % total)
		}

		fn generate_random_number(seed: u32) -> u32 {
			let (random_seed, _) = T::Randomness::random(&(b"ddc-validator", seed).encode());
			let random_number = <u32>::decode(&mut random_seed.as_ref())
				.expect("secure hashes should always be bigger than u32; qed");

			random_number
		}
	}
}
