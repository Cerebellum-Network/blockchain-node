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
	decl_event, decl_module, decl_storage,
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
pub use pallet_ddc_staking::{self as ddc_staking};
pub use pallet_session as session;
pub use pallet_staking::{self as staking};
pub use scale_info::TypeInfo;
pub use serde_json::Value;
pub use sp_core::crypto::{AccountId32, KeyTypeId, UncheckedFrom};
pub use sp_io::crypto::sr25519_public_keys;
pub use sp_runtime::offchain::{http, storage::StorageValueRef, Duration, Timestamp};
pub use sp_staking::EraIndex;
pub use sp_std::{collections::btree_map::BTreeMap, prelude::*};
use log::info;

extern crate alloc;

/// The balance type of this pallet.
type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

type ResultStr<T> = Result<T, &'static str>;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"dacv");

pub const TIME_START_MS: u128 = 1_672_531_200_000;
pub const ERA_DURATION_MS: u128 = 120_000;
pub const ERA_IN_BLOCKS: u8 = 20;

/// Webdis in experimental cluster connected to Redis in dev.
// pub const DEFAULT_DATA_PROVIDER_URL: &str = "https://dev-dac-redis.network-dev.aws.cere.io";
pub const DEFAULT_DATA_PROVIDER_URL: &str = "http://161.35.140.182:7379";
pub const DATA_PROVIDER_URL_KEY: &[u8; 32] = b"ddc-validator::data-provider-url";
pub const QUORUM_SIZE: usize = 3;

/// Aggregated values from DAC that describe CDN node's activity during a certain era.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Serialize, Deserialize)]
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
		+ ddc_staking::Config
		+ CreateSignedTransaction<Call<Self>>
	where
		<Self as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<Self::Hash>,
		<BalanceOf<Self> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Something that provides randomness in the runtime. Required by the tasks assignment
		/// procedure.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		/// A dispatchable call.
		type Call: From<Call<Self>>;

		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type TimeProvider: UnixTime;

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

	#[pallet::storage]
	#[pallet::getter(fn assignments)]
	pub(super) type Assignments<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EraIndex, Twox64Concat, T::AccountId, Vec<T::AccountId>>;

	/// A signal to start a process on all the validators.
	#[pallet::storage]
	#[pallet::getter(fn signal)]
	pub(super) type Signal<T: Config> = StorageValue<_, bool>;

	/// The map from the era and CDN participant stash key to the validation decision related.
	#[pallet::storage]
	#[pallet::getter(fn validation_decisions)]
	pub type ValidationDecisions<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EraIndex, Twox64Concat, T::AccountId, ValidationDecision>;

	/// The last era for which the tasks assignment produced.
	#[pallet::storage]
	#[pallet::getter(fn last_managed_era)]
	pub type LastManagedEra<T: Config> = StorageValue<_, EraIndex>;

	#[pallet::error]
	pub enum Error<T> {
		// TBA
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<T::Hash>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode, {}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<T::Hash>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			if block_number <= 1u32.into() {
				return 0
			}

			Signal::<T>::set(Some(false));

			let era = Self::get_current_era();
			log::info!("current era: {:?}", era);

			if let Some(last_managed_era) = <LastManagedEra<T>>::get() {
				log::info!("last_managed_era: {:?}", last_managed_era);
				if last_managed_era >= era {
					return 0
				}
			}
			<LastManagedEra<T>>::put(era);

			Self::assign(3usize);

			0
		}

		fn offchain_worker(block_number: T::BlockNumber) {
			// Skip if not a validator.
			if !sp_io::offchain::is_validator() {
				return
			}

			let current_era = Self::get_current_era();
			let last_managed_era = Self::last_managed_era().unwrap_or(0);
			let data_provider_url = Self::get_data_provider_url();
			log::info!("[DAC Validator] Data provider URL: {:?}", &data_provider_url);

			// `If` commented for testing purposes
			// if current_era > last_managed_era {
			Self::validate_edges();
			//}

			// Print the number of broken sessions per CDN node.
			// let aggregates_value = dac::fetch_aggregates(&data_provider_url, 77436).unwrap(); // 77436 is for a mock data
			// let aggregates_obj = aggregates_value.as_object().unwrap();
			// aggregates_obj
			// 	.into_iter()
			// 	.for_each(|(cdn_node_pubkey, cdn_node_aggregates_value)| {
			// 		// iterate over aggregates for each node
			// 		let cdn_node_aggregates_obj = cdn_node_aggregates_value.as_object().unwrap();
			// 		// Extract `nodeInterruptedSessions` field
			// 		let (_, cdn_node_interrupted_sessions_value) = cdn_node_aggregates_obj
			// 			.into_iter()
			// 			.find(|(key, _)| key.iter().copied().eq("nodeInterruptedSessions".chars()))
			// 			.unwrap();
			// 		let cdn_node_interrupted_sessions_obj =
			// 			cdn_node_interrupted_sessions_value.as_object().unwrap();
			// 		// Prepare CDN pubkey without heap allocated string
			// 		let cdn_node_pubkey_vecu8: Vec<u8> =
			// 			cdn_node_pubkey.iter().map(|c| *c as u8).collect();
			// 		let cdn_node_pubkey_str =
			// 			sp_std::str::from_utf8(&cdn_node_pubkey_vecu8).unwrap();
			// 		log::info!(
			// 			"Broken sessions per CDN node | Node {}: {} sessions broken",
			// 			cdn_node_pubkey_str,
			// 			cdn_node_interrupted_sessions_obj.len(), /* count sessions broken by the
			// 			                                          * node */
			// 		);
			// 	});

			// Wait for signal.
			let signal = Signal::<T>::get().unwrap_or(false);
			if !signal {
				log::info!("🔎 DAC Validator is idle at block {:?}, waiting for a signal, signal state is {:?}", block_number, signal);
				return
			}

			// Read from DAC.
			let response = dac::fetch_data2(&data_provider_url, current_era - 1);
			let (sent_query, sent, received_query, received) = match response {
				Ok(data) => data,
				Err(_) => {
					log::info!("🔎 DAC Validator failed to get bytes sent and received from DAC");
					return
				},
			};
			log::info!(
				"🔎 DAC Validator is fetching data from DAC, current era: {:?}, bytes sent query: {:?}, bytes sent response: {:?}, bytes received query: {:?}, bytes received response: {:?}",
				current_era,
				sent_query,
				sent,
				received_query,
				received,
			);

			// Create intermediate validation decisions
			// ========================================

			// All validators validate all CDN nodes.
			let edges: Vec<T::AccountId> = <ddc_staking::pallet::Edges<T>>::iter_keys().collect();
			for edge in edges.iter() {
				// Get string type CDN node pubkey
				let edge_pubkey: String = utils::account_to_string::<T>(edge.clone());

				// Get bytes sent and received for the CDN node
				let node_sent: &dac::BytesSent = match sent
					.iter()
					.find(|bytes_sent| bytes_sent.node_public_key == edge_pubkey)
				{
					Some(node_sent) => node_sent,
					None => {
						log::warn!("No logs to validate {:?}", edge);
						continue
					},
				};
				let client_received: &dac::BytesReceived = match received
					.iter()
					.find(|bytes_received| bytes_received.node_public_key == edge_pubkey)
				{
					Some(client_received) => client_received,
					None => {
						log::warn!("No acks to validate {:?}", edge);
						continue
					},
				};

				// Proof-of-delivery validation
				let validation_result = Self::validate(node_sent, client_received);

				// Prepare an intermediate validation decision
				let validation_decision = ValidationDecision {
					edge: utils::account_to_string::<T>(edge.clone()),
					result: validation_result,
					payload: [0u8; 32], // ToDo: put a hash of the validated data here
					totals: DacTotalAggregates {
						sent: node_sent.sum as u64,
						received: client_received.sum as u64,
						failed_by_client: 0, // ToDo
						failure_rate: 0,     // ToDo
					},
				};

				// Encode validation decision to base64
				let validation_decision_serialized: Vec<u8> = validation_decision.encode();
				let validation_decision_base64 =
					shm::base64_encode(&validation_decision_serialized);
				log::info!(
					"Intermediate validation decision for CDN node {:?}: , base64 encoded: {:?}",
					validation_decision,
					validation_decision_base64,
				);

				// Prepare values to publish validation decision and publish it
				let validator_id_string = String::from("validator1"); // ToDo: get validator ID
				let edge_id_string = utils::account_to_string::<T>(edge.clone());
				let validation_decision_base64_string =
					validation_decision_base64.iter().cloned().collect::<String>();
				let response = shm::share_intermediate_validation_result(
					&data_provider_url,
					current_era - 1,
					&validator_id_string,
					&edge_id_string,
					validation_result,
					&validation_decision_base64_string,
				);
				match response {
					Ok(response) =>
						log::info!("Shared memory response: {:?}", response.to_string()),
					Err(e) => {
						log::error!("Shared memory error: {:?}", e);
						continue
					},
				}
			}
			log::info!(
				"Intermediate validation results published for {} CDN nodes in era {:?}",
				edges.len(),
				current_era - 1
			);

			// Set CDN nodes' reward points
			// ============================

			// Let's use a mock data until we have a real final validation decisions for all the CDN
			// nodes.
			let mock_final_validation_decisions: Vec<(T::AccountId, ValidationDecision)> = vec![
				(
					utils::string_to_account::<T>(
						"0xd4160f567d7265b9de2c7cbf1a5c931e5b3195efb2224f8706bfb53ea6eaacd1".into(),
					),
					ValidationDecision {
						edge: "test".into(),
						result: true,
						payload: [0u8; 32],
						totals: DacTotalAggregates {
							sent: 100,
							received: 100,
							failed_by_client: 0,
							failure_rate: 0,
						},
					},
				),
				(
					utils::string_to_account::<T>(
						"0xa2d14e71b52e5695e72c0567926bc68b68bda74df5c1ccf1d4ba612c153ff66b".into(),
					),
					ValidationDecision {
						edge: "test".into(),
						result: true,
						payload: [0u8; 32],
						totals: DacTotalAggregates {
							sent: 200,
							received: 200,
							failed_by_client: 0,
							failure_rate: 0,
						},
					},
				),
			];

			// Calculate CDN nodes reward points from validation decision aggregates
			let cdn_nodes_reward_points: Vec<(T::AccountId, u64)> = mock_final_validation_decisions
				.into_iter()
				.filter(|(_, validation_decision)| validation_decision.result) // skip misbehaving
				.map(|(cdn_node, validation_decision)| {
					// ToDo: should we use `sent` or `received` or anything else as a reward point?
					(cdn_node, validation_decision.totals.sent)
				})
				.collect();

			// Store CDN node reward points on-chain
			let signer: Signer<T, T::AuthorityId> = Signer::<_, _>::any_account();
			if !signer.can_sign() {
				log::warn!("No local accounts available to set era reward points. Consider adding one via `author_insertKey` RPC.");
				return
			}
			// ToDo: replace local call by a call from `ddc-staking` pallet
			let _tx_res = signer.send_signed_transaction(|_account| Call::set_era_reward_points {
				era: current_era - 1,
				stakers_points: cdn_nodes_reward_points.clone(),
			});
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
		#[pallet::weight(10_000)]
		pub fn set_validation_decision(
			origin: OriginFor<T>,
			era: EraIndex,
			cdn_node: T::AccountId,
			validation_decision: ValidationDecision,
		) -> DispatchResult {
			ensure_signed(origin)?;

			// ToDo: check if origin is a validator.
			// ToDo: check if the era is current - 1.
			// ToDo: check if the validation decision is not set yet.
			// ToDo: check cdn_node is known to ddc-staking.

			ValidationDecisions::<T>::insert(era, cdn_node, validation_decision);

			// ToDo: emit event.

			Ok(())
		}

		/// Set reward points for CDN participants at the given era.
		///
		/// ToDo: remove it when the off-chain worker will be able to set reward points using the
		/// same call defined in `pallet-ddc-staking`.
		///
		/// `stakers_points` is a vector of (stash account ID, reward points) pairs. The rewards
		/// distribution will be based on total reward points, with each CDN participant receiving a
		/// proportionate reward based on their individual reward points.
		///
		/// See also  [`pallet_ddc_staking::ErasEdgesRewardPoints`].
		#[pallet::weight(100_000)]
		pub fn set_era_reward_points(
			origin: OriginFor<T>,
			era: EraIndex,
			stakers_points: Vec<(T::AccountId, u64)>,
		) -> DispatchResult {
			ensure_signed(origin)?;

			<ddc_staking::pallet::ErasEdgesRewardPoints<T>>::mutate(era, |era_rewards| {
				for (staker, points) in stakers_points.into_iter() {
					*era_rewards.individual.entry(staker).or_default() += points;
					era_rewards.total += points;
				}
			});

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

		fn get_mock_data_url() -> String {
			let data_url = Self::get_data_provider_url();
			let mock_url = "/JSON.GET/testddc:dac:data";
			let url = format!("{}{}", data_url, mock_url);

			url
		}

		fn get_signer() -> ResultStr<Signer<T, T::AuthorityId>> {
			let signer = Signer::<_, _>::any_account();
			if !signer.can_sign() {
				return Err("[DAC Validator] No local accounts available. Consider adding one via `author_insertKey` RPC.");
			}

			Ok(signer)
		}

		// Get the current era; Shall we start era count from 0 or from 1?
		fn get_current_era() -> EraIndex {
			((T::TimeProvider::now().as_millis() - TIME_START_MS) / ERA_DURATION_MS)
				.try_into()
				.unwrap()
		}

		fn validate(bytes_sent: &dac::BytesSent, bytes_received: &dac::BytesReceived) -> bool {
			let percentage_difference = 1f32 - (bytes_received.sum as f32 / bytes_sent.sum as f32);

			return if percentage_difference > 0.0 &&
				(T::ValidationThreshold::get() as f32 - percentage_difference) > 0.0
			{
				true
			} else {
				false
			}
		}

		fn is_valid(bytes_sent: u64, bytes_received: u64) -> bool {
			let percentage_difference = 1f32 - (bytes_received as f32 / bytes_sent as f32);

			return if percentage_difference > 0.0 &&
				(T::ValidationThreshold::get() as f32 - percentage_difference) > 0.0
			{
				true
			} else {
				false
			}
		}

		fn shuffle(mut list: Vec<T::AccountId>) -> Vec<T::AccountId> {
			let len = list.len();
			for i in 1..len {
				let random_index = Self::choose(len as u32).unwrap() as usize;
				list.swap(i, random_index)
			}

			list
		}

		fn split<Item: Clone>(list: Vec<Item>, segment_len: usize) -> Vec<Vec<Item>> {
			let mut result: Vec<Vec<Item>> = Vec::new();

			if segment_len == 0 {
				return result;
			}

			for i in (0..list.len()).step_by(segment_len) {
				let end = usize::min(i + segment_len, list.len());
				let chunk = list[i..end].to_vec();
				result.push(chunk);
			}

			result
		}

		fn assign(quorum_size: usize) {
			let validators: Vec<T::AccountId> = <staking::Validators<T>>::iter_keys().collect();
			let edges: Vec<T::AccountId> = <ddc_staking::pallet::Edges<T>>::iter_keys().collect();

			if edges.len() == 0 {
				return
			}

			let shuffled_validators = Self::shuffle(validators);
			let shuffled_edges = Self::shuffle(edges);

			let validators_keys: Vec<String> = shuffled_validators
				.iter()
				.map(|v| utils::account_to_string::<T>(v.clone()))
				.collect();

			let quorums = Self::split(validators_keys, quorum_size);
			let edges_groups = Self::split(shuffled_edges, quorum_size);

			info!("quorums: {:?}", quorums);
			info!("edges_groups: {:?}", edges_groups);

			let era = Self::get_current_era();

			for (i, quorum) in quorums.iter().enumerate() {
				let edges_group = &edges_groups[i];
				for validator in quorum {
					Assignments::<T>::insert(
						era,
						utils::string_to_account::<T>(validator.clone()),
						edges_group,
					);
				}
			}
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
				None => None
			}
		}

		fn validate_edges() {
			let current_era = Self::get_current_era();
			let mock_data_url = Self::get_mock_data_url();
			let data_provider_url = Self::get_data_provider_url();

			// let signer = Self::get_signer().unwrap();
			// let validator = signer.get_any_account().unwrap().id;
			let validator = Self::get_public_key().unwrap();

			info!("validator: {:?}", validator);

			let assigned_edges = Self::assignments(current_era - 1, validator.clone()).unwrap();

			info!("assigned_edges: {:?}", assigned_edges);

			for assigned_edge in assigned_edges.iter() {
				let file_request = dac::fetch_file_request(&mock_data_url);
				let (bytes_sent, bytes_received) = dac::get_served_bytes_sum(&file_request);
				let is_valid = Self::is_valid(bytes_sent, bytes_received);

				info!("bytes_sent, bytes_received: {:?}, {:?}", bytes_sent, bytes_received);

				let payload = serde_json::to_string(&file_request).unwrap();
				let decision = ValidationDecision {
					edge: utils::account_to_string::<T>(assigned_edge.clone()),
					result: is_valid,
					payload: utils::hash(&payload),
					totals: DacTotalAggregates {
						received: bytes_received,
						sent: bytes_sent,
						failed_by_client: 0,
						failure_rate: 0,
					}
				};

				info!("decision: {:?}", decision);

				let serialized_decision = serde_json::to_string(&decision).unwrap();
				let encoded_decision = shm::base64_encode(&serialized_decision.as_bytes().to_vec());
				let validator_str = utils::account_to_string::<T>(validator.clone());
				let edge_str = utils::account_to_string::<T>(assigned_edge.clone());

				let encoded_decision_str = encoded_decision.iter().cloned().collect::<String>();

				let response = shm::share_intermediate_validation_result(
					&data_provider_url,
					current_era - 1,
					&validator_str,
					&edge_str,
					is_valid,
					&encoded_decision_str,
				);

				if let Err(res) = response.clone() {
					log::error!("share_intermediate_validation_result request failed: {:?}", res);
				}

				if let Ok(res) = response.clone() {
					info!("shm res: {:?}", res.to_string());
				}

				if let Ok(res) = response {
					let edge = utils::account_to_string::<T>(assigned_edge.clone());
					let prev_era = (current_era - 1) as EraIndex;
					let quorum = Self::find_validators_from_quorum(&validator, &prev_era);
					let validations_res = shm::get_intermediate_decisions(&data_provider_url, &edge_str, &prev_era, quorum);

					log::info!("get_intermediate_decisions result: {:?}", validations_res);

					if validations_res.len() == QUORUM_SIZE {
						let final_res = dac::get_final_decision(validations_res);

						let signer = Self::get_signer().unwrap();

						let tx_res = signer.send_signed_transaction(|_acct| Call::set_validation_decision {
							era: current_era,
							cdn_node: utils::string_to_account::<T>(edge.clone()),
							validation_decision: final_res.clone(),
						});

						log::info!("final_res: {:?}", final_res);
					}
				}
			}
		}
	}
}
