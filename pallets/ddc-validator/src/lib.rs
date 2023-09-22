//! # DDC Validator pallet
//!
//! The DDC Validator pallet defines storage item to store validation results and implements OCW
//! (off-chain worker) to produce these results using the data from Data Activity Capture (DAC).
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//! - [`Hooks`]
//!
//!	## Notes
//!
//! - Era definition in this pallet is different than in the `pallet-staking`. Check DAC
//!   documentation for `era` definition used in this pallet.

#![cfg_attr(not(feature = "std"), no_std)]

mod dac;
mod payments;
mod shm;
mod utils;
mod validation;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use alloc::{format, string::String};
pub use alt_serde::{de::DeserializeOwned, Deserialize, Serialize};
pub use codec::{Decode, Encode, HasCompact, MaxEncodedLen};
pub use core::fmt::Debug;
pub use frame_support::{
	decl_event, decl_module, decl_storage, defensive,
	dispatch::DispatchResult,
	pallet_prelude::*,
	parameter_types, storage,
	traits::{Currency, Randomness, UnixTime},
	weights::Weight,
	BoundedVec, RuntimeDebug,
};
pub use frame_system::{
	ensure_signed,
	offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer, SigningTypes},
	pallet_prelude::*,
};
pub use lite_json::json::JsonValue;
pub use pallet::*;
pub use pallet_ddc_accounts::{self as ddc_accounts, BucketsDetails};
pub use pallet_ddc_staking::{self as ddc_staking};
pub use pallet_session as session;
pub use pallet_staking::{self as staking};
pub use scale_info::TypeInfo;
pub use serde_json::Value;
pub use sp_core::crypto::{AccountId32, KeyTypeId, UncheckedFrom};
pub use sp_io::{crypto::sr25519_public_keys, offchain_index};
pub use sp_runtime::offchain::{
	http, storage::StorageValueRef, storage_lock, storage_lock::StorageLock, Duration, Timestamp,
};
pub use sp_staking::EraIndex;
pub use sp_std::{collections::btree_map::BTreeMap, prelude::*};

extern crate alloc;

/// The balance type of this pallet.
type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

type ResultStr<T> = Result<T, &'static str>;

/// Offchain local storage key that holds the last era in which the validator completed its
/// assignment.
const LAST_VALIDATED_ERA_KEY: &[u8; 40] = b"pallet-ddc-validator::last_validated_era";
/// Offchain local storage that holds the validation lock
const VALIDATION_LOCK: &[u8; 37] = b"pallet-ddc-validator::validation_lock";

/// Local storage key that holds the flag to enable DDC validation. Set it to true (0x01) to enable
/// DDC validation, set it to false (0x00) or delete the key to disable it.
const ENABLE_DDC_VALIDATION_KEY: &[u8; 21] = b"enable-ddc-validation";

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"dacv");

/// Webdis in experimental cluster connected to Redis in dev.
pub const DEFAULT_DATA_PROVIDER_URL: &str = "http://webdis:7379";
// pub const DEFAULT_DATA_PROVIDER_URL: &str = "http://161.35.140.182:7379";
pub const DATA_PROVIDER_URL_KEY: &[u8; 32] = b"ddc-validator::data-provider-url";
pub const QUORUM_SIZE: usize = 1;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum opCode {
	Read = 1,
	Write = 2,
	Search = 3,
}

impl TryFrom<u64> for opCode {
	type Error = &'static str;

	fn try_from(v: u64) -> Result<Self, Self::Error> {
		match v {
			x if x == opCode::Write as u64 => Ok(opCode::Write),
			x if x == opCode::Read as u64 => Ok(opCode::Read),
			x if x == opCode::Query as u64 => Ok(opCode::Query),
			_ => Err("Invalid value to for log type"),
		}
	}
}

#[derive(Debug)]
pub enum AssignmentError {
	NoValidators,
	NotEnoughValidators { requested_quorum: usize, available_validators: usize },
	DefensiveEmptyQuorumsCycle,
}

/// Aggregated values from DAC that describe CDN node's activity during a certain era.
#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Serialize,
	Deserialize,
)]
#[serde(crate = "alt_serde")]
pub struct DacTotalAggregates {
	/// Total bytes received by the client.
	pub received: u64,
	/// Total bytes sent by the CDN node.
	pub sent: u64,
	/// Total bytes sent by the CDN node to the client which interrupts the connection.
	pub failed_by_client: u64,
	/// ToDo: explain.
	pub failure_rate: u64,
}

/// Final DAC Validation decision.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
#[serde(crate = "alt_serde")]
pub struct ValidationDecision {
	/// CDN node public key.
	pub edge: String,
	/// Validation result.
	pub result: bool,
	/// A hash of the data used to produce validation result.
	pub payload: [u8; 32],
	/// Values aggregated from the payload.
	pub totals: DacTotalAggregates,
}

pub mod crypto {
	use super::KEY_TYPE;
	use frame_system::offchain::AppCrypto;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
	};
	app_crypto!(sr25519, KEY_TYPE);

	use sp_runtime::{MultiSignature, MultiSigner};

	pub struct TestAuthId;

	impl AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	impl AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config
		+ pallet_contracts::Config
		+ pallet_session::Config<ValidatorId = <Self as frame_system::Config>::AccountId>
		+ pallet_staking::Config
		+ ddc_accounts::Config
		+ ddc_staking::Config
		+ CreateSignedTransaction<Call<Self>>
	where
		<Self as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<Self::Hash>,
		<BalanceOf<Self> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Something that provides randomness in the runtime. Required by the tasks assignment
		/// procedure.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		/// A dispatchable call.
		type RuntimeCall: From<Call<Self>>;

		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

		/// Number of validators expected to produce an individual validation decision to form a
		/// consensus. Tasks assignment procedure use this value to determine the number of
		/// validators are getting the same task. Must be an odd number.
		#[pallet::constant]
		type DdcValidatorsQuorumSize: Get<u32>;

		type ValidatorsMax: Get<u32>;

		/// Proof-of-Delivery parameter specifies an allowed deviation between bytes sent and bytes
		/// received. The deviation is expressed as a percentage. For example, if the value is 10,
		/// then the difference between bytes sent and bytes received is allowed to be up to 10%.
		/// The value must be in range [0, 100].
		#[pallet::constant]
		type ValidationThreshold: Get<u32>;
	}

	/// The map from the era and validator stash key to the list of CDN nodes to validate.
	#[pallet::storage]
	#[pallet::getter(fn assignments)]
	pub(super) type Assignments<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EraIndex, Identity, T::AccountId, Vec<T::AccountId>>;

	/// Map to from era and account ID to bool indicateing that a particular content owner was
	/// charged for the era
	#[pallet::storage]
	#[pallet::getter(fn content_owners_charged)]
	pub(super) type EraContentOwnersCharged<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EraIndex, Identity, T::AccountId, bool, ValueQuery>;

	/// A signal to start a process on all the validators.
	#[pallet::storage]
	#[pallet::getter(fn signal)]
	pub(super) type Signal<T: Config> = StorageValue<_, bool>;

	/// The map from the era and CDN participant stash key to the validation decision related.
	#[pallet::storage]
	#[pallet::getter(fn validation_decisions)]
	pub type ValidationDecisions<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EraIndex, Identity, T::AccountId, ValidationDecision>;

	// Map to check if validation decision was performed for the era
	#[pallet::storage]
	#[pallet::getter(fn validation_decision_set_for_node)]
	pub(super) type ValidationDecisionSetForNode<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EraIndex, Identity, T::AccountId, bool, ValueQuery>;

	// Map to check if reward points were set for the era
	#[pallet::storage]
	#[pallet::getter(fn reward_points_set_for_node)]
	pub(super) type RewardPointsSetForNode<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EraIndex, Identity, T::AccountId, bool, ValueQuery>;

	/// The last era for which the tasks assignment produced.
	#[pallet::storage]
	#[pallet::getter(fn last_managed_era)]
	pub type LastManagedEra<T: Config> = StorageValue<_, EraIndex>;

	/// The mapping of ddc validator keys to validator stash keys
	///
	/// Keys registered by validators are mapped to validator stash accounts.
	/// The mapping is formed in the way that facilitates fast checking that storage contains key.
	/// Similarly the validator stash is checked if he is still in the list of validators.
	#[pallet::storage]
	#[pallet::getter(fn get_stash_for_ddc_validator)]
	pub type DDCValidatorToStashKeys<T: Config> =
		StorageMap<_, Identity, T::AccountId, T::AccountId>;

	#[pallet::error]
	pub enum Error<T> {
		/// Caller is not controller of validator node
		NotController,
		/// Checked stash is not an active validator
		NotValidatorStash,
		/// OCW key has not been registered by validator
		DDCValidatorKeyNotRegistered,
		/// Attempt to charge content owners twice
		ContentOwnersDoubleSpend,
		/// Validation decision has been already set for CDN node for some era
		ValidationDecisionAlreadySet,
		/// Node is not participating in the network
		NodeNotActive,
		/// Pricing has not been set by sudo
		PricingNotSet,
		/// Current era not set during runtime
		DDCEraNotSet,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<T::Hash>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		// Validator submits decision for an era
		ValidationDecision(EraIndex, T::AccountId, ValidationDecision),
		// Set era reward points
		EraRewardPoints(EraIndex, Vec<(T::AccountId, u64)>),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<T::Hash>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			if block_number <= 1u32.into() {
				return Weight::from_ref_time(0)
			}

			Signal::<T>::set(Some(false));

			let current_ddc_era = match ddc_staking::pallet::Pallet::<T>::current_era() {
				Some(era) => era,
				None => {
					log::debug!("DDC era not set.");
					return Weight::from_ref_time(0)
				},
			};
			log::debug!("Current DDC era: {:?}.", current_ddc_era);

			// Skip assignment if already exists for current era
			match Self::last_managed_era() {
				Some(last_managed_era) if current_ddc_era < last_managed_era =>
					return Weight::from_ref_time(0),
				_ => (),
			}

			match Self::assign(3usize, current_ddc_era + 1) {
				Ok(_) => <LastManagedEra<T>>::put(current_ddc_era + 1),
				Err(e) => log::debug!("DDC validation assignment error: {:?}.", e),
			}

			Weight::from_ref_time(0)
		}

		fn offchain_worker(_block_number: T::BlockNumber) {
			// Skip if not a validator.
			if !sp_io::offchain::is_validator() {
				return
			}

			// Skip if DDC validation is not enabled.
			match StorageValueRef::persistent(ENABLE_DDC_VALIDATION_KEY).get::<bool>() {
				Ok(Some(enabled)) if enabled == true => (),
				_ => return,
			}

			let mut should_validate_because_new_era = true;

			let mut validation_lock = StorageLock::<storage_lock::Time>::new(VALIDATION_LOCK);

			// Skip if the validation is already in progress.
			if validation_lock.try_lock().is_err() {
				should_validate_because_new_era = false;
			}

			let last_validated_era_storage = StorageValueRef::persistent(LAST_VALIDATED_ERA_KEY);
			let last_validated_era = match last_validated_era_storage.get::<EraIndex>() {
				Ok(Some(last_validated_era)) => last_validated_era,
				_ => 0, // let's consider an absent or undecodable data as we never did a validation
			};

			let current_ddc_era = match ddc_staking::pallet::Pallet::<T>::current_era() {
				Some(era) => era,
				None => {
					defensive!("DDC era not set");
					return
				},
			};

			// Skip if the validation is already complete for the era.
			if current_ddc_era <= last_validated_era {
				should_validate_because_new_era = false;
			}

			// Validation start forced externally?
			let should_validate_because_signal = Signal::<T>::get().unwrap_or(false);

			if !should_validate_because_new_era && !should_validate_because_signal {
				return
			}

			if let Err(e) = Self::validate_edges() {
				log::warn!("🔎 DDC validation failed. {}", e);
				return
			}
			last_validated_era_storage.set(&current_ddc_era);
			log::info!("🔎 DDC validation complete for {} era.", current_ddc_era);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<T::Hash>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		/// Run a process at the same time on all the validators.
		#[pallet::weight(10_000)]
		pub fn send_signal(origin: OriginFor<T>) -> DispatchResult {
			ensure_signed(origin)?;

			Signal::<T>::set(Some(true));

			Ok(())
		}

		/// Set validation decision for a given CDN node in an era.
		///
		/// Only registered validator keys can call this exstrinsic.
		/// Validation decision can be set only once per era per CDN node.
		/// CDN node should be active.
		#[pallet::weight(10_000)]
		pub fn set_validation_decision(
			origin: OriginFor<T>,
			era: EraIndex,
			cdn_node: T::AccountId,
			validation_decision: ValidationDecision,
		) -> DispatchResult {
			let ddc_valitor_key = ensure_signed(origin)?;

			ensure!(
				DDCValidatorToStashKeys::<T>::contains_key(&ddc_valitor_key),
				Error::<T>::DDCValidatorKeyNotRegistered
			);

			ensure!(
				staking::Validators::<T>::contains_key(
					Self::get_stash_for_ddc_validator(&ddc_valitor_key).unwrap()
				),
				Error::<T>::NotValidatorStash
			);

			ensure!(
				!Self::validation_decision_set_for_node(era, &cdn_node),
				Error::<T>::ValidationDecisionAlreadySet
			);

			ensure!(
				<ddc_staking::pallet::Edges<T>>::contains_key(&cdn_node),
				Error::<T>::NodeNotActive
			);

			ValidationDecisions::<T>::insert(era, cdn_node.clone(), validation_decision.clone());

			ValidationDecisionSetForNode::<T>::insert(era, cdn_node.clone(), true);

			Self::deposit_event(Event::<T>::ValidationDecision(era, cdn_node, validation_decision));

			Ok(())
		}

		/// Set reward points for CDN participants at the given era.
		///
		/// Only registered validator keys can call this exstrinsic.
		/// Reward points can be set only once per era per CDN node.
		///	CDN node should be active.
		///
		/// `stakers_points` is a vector of (stash account ID, reward points) pairs. The rewards
		/// distribution will be based on total reward points, with each CDN participant receiving a
		/// proportionate reward based on their individual reward points.
		#[pallet::weight(100_000)]
		pub fn set_era_reward_points(
			origin: OriginFor<T>,
			era: EraIndex,
			stakers_points: Vec<(T::AccountId, u64)>,
		) -> DispatchResult {
			let ddc_valitor_key = ensure_signed(origin)?;

			ensure!(
				DDCValidatorToStashKeys::<T>::contains_key(&ddc_valitor_key),
				Error::<T>::DDCValidatorKeyNotRegistered
			);

			ensure!(
				staking::Validators::<T>::contains_key(
					Self::get_stash_for_ddc_validator(&ddc_valitor_key).unwrap()
				),
				Error::<T>::NotValidatorStash
			);

			let mut rewards_counter = 0;

			<ddc_staking::pallet::ErasEdgesRewardPoints<T>>::mutate(era, |era_rewards| {
				for (staker, points) in stakers_points.clone().into_iter() {
					if !Self::reward_points_set_for_node(era, &staker) {
						// ToDo deal with edge case when node is chilling
						if <ddc_staking::pallet::Edges<T>>::contains_key(&staker) {
							*era_rewards.individual.entry(staker.clone()).or_default() += points;
							era_rewards.total += points;
							<ddc_staking::pallet::ErasEdgesRewardPointsPerNode<T>>::mutate(
								&staker,
								|current_reward_points| {
									let rewards =
										ddc_staking::EraRewardPointsPerNode { era, points };
									current_reward_points.push(rewards);
								},
							);
							RewardPointsSetForNode::<T>::insert(era, staker, true);
							rewards_counter += 1;
						}
					}
				}
			});

			if rewards_counter > 0 {
				Self::deposit_event(Event::<T>::EraRewardPoints(era, stakers_points));
			}

			Ok(())
		}

		/// Exstrinsic deducts balances of content owners
		///
		/// Only registered validator keys can call this exstrinsic.
		/// Reward points can be set only once per era per validator.
		#[pallet::weight(100_000)]
		pub fn charge_payments_content_owners(
			origin: OriginFor<T>,
			paying_accounts: Vec<BucketsDetails<ddc_accounts::BalanceOf<T>>>, /* ToDo check if
			                                                                   * bounded vec
			                                                                   * should be used */
		) -> DispatchResult {
			let ddc_valitor_key = ensure_signed(origin)?;
			log::debug!("validator is {:?}", &ddc_valitor_key);

			let current_era =
				ddc_staking::pallet::Pallet::<T>::current_era().ok_or(Error::<T>::DDCEraNotSet)?;

			ensure!(
				DDCValidatorToStashKeys::<T>::contains_key(&ddc_valitor_key),
				Error::<T>::DDCValidatorKeyNotRegistered
			);

			ensure!(
				staking::Validators::<T>::contains_key(
					Self::get_stash_for_ddc_validator(&ddc_valitor_key).unwrap()
				),
				Error::<T>::NotValidatorStash
			);

			ensure!(
				!Self::content_owners_charged(current_era, &ddc_valitor_key),
				Error::<T>::ContentOwnersDoubleSpend
			);

			let pricing: u128 =
				<ddc_staking::pallet::Pallet<T>>::pricing().ok_or(Error::<T>::PricingNotSet)?;
			EraContentOwnersCharged::<T>::insert(current_era, ddc_valitor_key, true);

			<ddc_accounts::pallet::Pallet<T>>::charge_content_owners(paying_accounts, pricing)
		}

		/// Exstrinsic registers a ddc validator key for future use
		///
		/// Only controller of validator can call this exstrinsic
		/// Validator has to be in the active set
		#[pallet::weight(100_000)]
		pub fn set_validator_key(
			origin: OriginFor<T>,
			ddc_validator_pub: T::AccountId,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = staking::Ledger::<T>::get(&controller).ok_or(Error::<T>::NotController)?;

			ensure!(
				staking::Validators::<T>::contains_key(&ledger.stash),
				Error::<T>::NotValidatorStash
			);

			DDCValidatorToStashKeys::<T>::insert(ddc_validator_pub, &ledger.stash);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<T::Hash>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		fn get_data_provider_url() -> String {
			let url_ref = sp_io::offchain::local_storage_get(
				sp_core::offchain::StorageKind::PERSISTENT,
				DATA_PROVIDER_URL_KEY,
			);

			match url_ref {
				Some(url) =>
					String::from_utf8(url).expect("Data provider URL should be valid UTF-8 string"),
				None => String::from(DEFAULT_DATA_PROVIDER_URL),
			}
		}

		fn get_data_url() -> String {
			let data_url = Self::get_data_provider_url();
			let json_getter = "/JSON.GET/";
			let url = format!("{}{}", data_url, json_getter);

			url
		}

		fn get_signer() -> ResultStr<Signer<T, T::AuthorityId>> {
			let signer = Signer::<_, _>::any_account();
			if !signer.can_sign() {
				return Err("[DAC Validator] No local accounts available. Consider adding one via `author_insertKey` RPC.");
			}

			Ok(signer)
		}

		fn is_valid(bytes_sent: u64, bytes_received: u64) -> bool {
			if bytes_sent == bytes_received {
				return true
			}

			let percentage_difference = 1f32 - (bytes_received as f32 / bytes_sent as f32);

			return if percentage_difference >= 0.0 &&
				(T::ValidationThreshold::get() as f32 - percentage_difference) > 0.0
			{
				true
			} else {
				false
			}
		}

		/// Shuffle the `list` swapping it's random elements `list.len()` times.
		fn shuffle(mut list: Vec<T::AccountId>) -> Vec<T::AccountId> {
			let len = list.len();
			for i in 1..len {
				let random_index = Self::choose(len as u32).unwrap() as usize;
				list.swap(i, random_index)
			}

			list
		}

		/// Split the `list` to several chunks `segment_len` length each.
		///
		/// The very last chunk will be shorter than `segment_len` if `list.len()` is not divisible
		/// by `segment_len`.
		fn split<Item: Clone>(list: Vec<Item>, segment_len: usize) -> Vec<Vec<Item>> {
			let mut result: Vec<Vec<Item>> = Vec::new();

			if segment_len == 0 {
				return result
			}

			for i in (0..list.len()).step_by(segment_len) {
				let end = usize::min(i + segment_len, list.len());
				let chunk = list[i..end].to_vec();
				result.push(chunk);
			}

			result
		}

		fn assign(quorum_size: usize, era: EraIndex) -> Result<(), AssignmentError> {
			let validators: Vec<T::AccountId> = DDCValidatorToStashKeys::<T>::iter_keys().collect();
			log::debug!("Current validators: {:?}.", validators);

			if validators.len() == 0 {
				return Err(AssignmentError::NoValidators)
			}

			if validators.len() < quorum_size {
				return Err(AssignmentError::NotEnoughValidators {
					requested_quorum: quorum_size,
					available_validators: validators.len(),
				})
			}

			let edges: Vec<T::AccountId> = <ddc_staking::pallet::Edges<T>>::iter_keys().collect();
			log::debug!("Current edges: {:?}.", edges);

			if edges.len() == 0 {
				return Ok(())
			}

			let shuffled_validators = Self::shuffle(validators);
			let shuffled_edges = Self::shuffle(edges);

			let validators_keys: Vec<String> = shuffled_validators
				.iter()
				.map(|v| utils::account_to_string::<T>(v.clone()))
				.collect();

			// Create several groups of validators `quorum_size` length each.
			let quorums = Self::split(validators_keys, quorum_size);

			// Write an assignment to each validator in each quorum. The difference between the
			// number of edges assigned to each validator is not higher then 1. If the number of
			// edges is less then the number of quorums, some quorums will not have any edges
			// assigned.
			let mut quorums_cycle = quorums.iter().cycle();
			for edge in shuffled_edges {
				let Some(quorum_validators) = quorums_cycle.next() else {
					return Err(AssignmentError::DefensiveEmptyQuorumsCycle);
				};
				quorum_validators.iter().for_each(|validator| {
					Assignments::<T>::append(
						era,
						utils::string_to_account::<T>(validator.clone()),
						edge.clone(),
					);
				});
			}

			return Ok(())
		}

		/// Randomly choose a number in range `[0, total)`.
		/// Returns `None` for zero input.
		/// Modification of `choose_ticket` from `pallet-lottery` version `4.0.0-dev`.
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

		/// Generate a random number from a given seed.
		/// Note that there is potential bias introduced by using modulus operator.
		/// You should call this function with different seed values until the random
		/// number lies within `u32::MAX - u32::MAX % n`.
		/// Modification of `generate_random_number` from `pallet-lottery` version `4.0.0-dev`.
		fn generate_random_number(seed: u32) -> u32 {
			let (random_seed, _) =
				<T as pallet::Config>::Randomness::random(&(b"ddc-validator", seed).encode());
			let random_number = <u32>::decode(&mut random_seed.as_ref())
				.expect("secure hashes should always be bigger than u32; qed");

			random_number
		}

		fn find_validators_from_quorum(validator_id: &T::AccountId, era: &EraIndex) -> Vec<String> {
			let validator_edges = Self::assignments(era, &validator_id).unwrap();
			let mut quorum_members: Vec<String> = Vec::new();

			<Assignments<T>>::iter_prefix(era).for_each(|(candidate_id, edges)| {
				if validator_edges == edges {
					let candidate_id_str = utils::account_to_string::<T>(candidate_id);
					quorum_members.push(candidate_id_str);
				}
			});

			quorum_members
		}

		fn get_public_key() -> Option<T::AccountId> {
			match sr25519_public_keys(KEY_TYPE).first() {
				Some(pubkey) => Some(T::AccountId::decode(&mut &pubkey.encode()[..]).unwrap()),
				None => None,
			}
		}

		fn validate_edges() -> Result<(), &'static str> {
			let current_ddc_era =
				ddc_staking::pallet::Pallet::<T>::current_era().ok_or("DDC era not set")?;
			let data_url = Self::get_data_url();
			let data_provider_url = Self::get_data_provider_url();
			log::debug!("[DAC Validator] Data provider URL: {:?}", &data_provider_url);

			let validator = match Self::get_public_key() {
				Some(key) => key,
				None => return Err("No validator public key found."),
			};

			log::debug!("validator: {:?}", validator);

			let assigned_edges = match Self::assignments(current_ddc_era - 1, validator.clone()) {
				Some(edges) => edges,
				None => return Err("No assignments for the previous era."),
			};

			log::debug!("assigned_edges: {:?}", assigned_edges);

			// Calculate CDN nodes reward points from validation decision aggregates
			let mut cdn_nodes_reward_points: Vec<(T::AccountId, u64)> = vec![];

			for assigned_edge in assigned_edges.iter() {
				log::debug!("assigned edge: {:?}", assigned_edge);

				// form url for each node
				let edge_url = format!(
					"{}{}{}/$.{}",
					data_url,
					"ddc:dac:aggregation:nodes:",
					current_ddc_era - 1,
					utils::account_to_string::<T>(assigned_edge.clone())
				);
				log::debug!("edge url: {:?}", edge_url);

				let node_aggregates = dac::fetch_cdn_node_aggregates_request(&edge_url);
				log::debug!("node aggregates: {:?}", node_aggregates);

				// No data for node
				if node_aggregates.len() == 0 {
					continue
				}

				let request_ids = &node_aggregates[0].request_ids;
				log::debug!("request_ids: {:?}", request_ids);

				// Store bucket payments
				let payments_per_bucket = &mut Vec::new();
				let requests = &mut dac::Requests::new();
				for request_id in request_ids.iter() {
					let request_id_url =
						format!("{}{}{}", data_url, "ddc:dac:data:file:", request_id.clone());
					let file_request = dac::fetch_file_request(&request_id_url);
					requests.insert(file_request.file_request_id.clone(), file_request.clone());
				}
				dac::get_acknowledged_bytes_bucket(&requests, payments_per_bucket);
				let (bytes_sent, bytes_received) = dac::get_served_bytes_sum(&requests);
				let is_valid = Self::is_valid(bytes_sent, bytes_received);

				log::debug!("bytes_sent, bytes_received: {:?}, {:?}", bytes_sent, bytes_received);

				let payload = serde_json::to_string(&requests).unwrap();
				let decision = ValidationDecision {
					edge: utils::account_to_string::<T>(assigned_edge.clone()),
					result: is_valid,
					payload: utils::hash(&payload),
					totals: DacTotalAggregates {
						received: bytes_received,
						sent: bytes_sent,
						failed_by_client: 0,
						failure_rate: 0,
					},
				};

				log::debug!("decision to be encoded: {:?}", decision);

				let serialized_decision = serde_json::to_string(&decision).unwrap();
				let encoded_decision =
					shm::base64_encode(&serialized_decision.as_bytes().to_vec()).unwrap();
				log::debug!("encoded decision: {:?}", encoded_decision);

				let validator_str = utils::account_to_string::<T>(validator.clone());
				let edge_str = utils::account_to_string::<T>(assigned_edge.clone());

				let encoded_decision_str = encoded_decision.iter().cloned().collect::<String>();

				let response = shm::share_intermediate_validation_result(
					&data_provider_url,
					current_ddc_era - 1,
					&validator_str,
					&edge_str,
					is_valid,
					&encoded_decision_str,
				);

				if let Err(res) = response.clone() {
					log::error!("share_intermediate_validation_result request failed: {:?}", res);
				}

				if let Ok(res) = response.clone() {
					log::debug!("shm res: {:?}", res.to_string());
				}

				if let Ok(_res) = response {
					let edge = utils::account_to_string::<T>(assigned_edge.clone());
					let prev_era = (current_ddc_era - 1) as EraIndex;
					let quorum = Self::find_validators_from_quorum(&validator, &prev_era);
					let validations_res = shm::get_intermediate_decisions(
						&data_provider_url,
						&edge_str,
						&prev_era,
						quorum,
					);

					log::debug!("get_intermediate_decisions result: {:?}", validations_res);

					if validations_res.len() == QUORUM_SIZE {
						log::debug!("payments per bucket: {:?}", payments_per_bucket);

						let mut payments: BTreeMap<
							u128,
							BucketsDetails<ddc_accounts::BalanceOf<T>>,
						> = BTreeMap::new();
						for bucket in payments_per_bucket.into_iter() {
							let cere_payment = bucket.1 as u32;
							if payments.contains_key(&bucket.0) {
								payments.entry(bucket.0).and_modify(|bucket_info| {
									bucket_info.amount += cere_payment.into()
								});
							} else {
								let bucket_info = BucketsDetails {
									bucket_id: bucket.0,
									amount: cere_payment.into(),
								};
								payments.insert(bucket.0, bucket_info);
							}
						}
						let mut final_payments = vec![];
						for (_, bucket_info) in payments {
							final_payments.push(bucket_info);
						}
						log::debug!("final payments: {:?}", final_payments);

						// Store CDN node reward points on-chain
						let signer: Signer<T, T::AuthorityId> = Signer::<_, _>::any_account();
						if !signer.can_sign() {
							log::warn!("No local accounts available to charge payments for CDN. Consider adding one via `author_insertKey` RPC.");
							return Err("signing key not set")
						}
						// ToDo: replace local call by a call from `ddc-staking` pallet
						let _tx_res: Option<(frame_system::offchain::Account<T>, Result<(), ()>)> =
							signer.send_signed_transaction(|_account| {
								Call::charge_payments_content_owners {
									paying_accounts: final_payments.clone(),
								}
							});

						let final_res = dac::get_final_decision(validations_res);

						let signer = Self::get_signer().unwrap();

						let _tx_res =
							signer.send_signed_transaction(|_acct| Call::set_validation_decision {
								era: current_ddc_era - 1,
								cdn_node: utils::string_to_account::<T>(edge.clone()),
								validation_decision: final_res.clone(),
							});

						log::debug!("final_res: {:?}", final_res);

						cdn_nodes_reward_points.push((
							utils::string_to_account::<T>(final_res.edge),
							final_res.totals.sent,
						));
					}
				}
			}
			let signer = Self::get_signer().unwrap();

			// ToDo: replace local call by a call from `ddc-staking` pallet
			if cdn_nodes_reward_points.len() > 0 {
				let _tx_res =
					signer.send_signed_transaction(|_account| Call::set_era_reward_points {
						era: current_ddc_era - 1,
						stakers_points: cdn_nodes_reward_points.clone(),
					});
			}

			Ok(())
		}
	}
}
