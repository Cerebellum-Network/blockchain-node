//! # DDC Validator pallet
//!
//! The DDC Validator pallet is responsible for producing validation decisions based on activity
//! data from DAC DataModel. It is expected to work on validators nodes only.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//! - [`Hooks`]
//!
//! ## Responsibility
//!
//! 1. Assign validation tasks on DAC Validators in the beginning of each era,
//! 2. Spin the offchain worker which tries to execute the validation tasks each era,
//! 3. Fetch the data required for validation from DAC DataModel,
//! 4. Execute validation method on this data,
//! 5. Produce validation decision and submit it to the chain.
//!
//! ## Usage
//!
//! 1. Run the node with `--validator` flag,
//! 2. Setup validator key with `author_insertKey` RPC call. Use `dacv` validator key type and the
//!    same private key as the one used to generate the validator's session keys,
//!	3. Proceed a regular validator setup,
//! 4. Tasks assignment will assign you a task in the beginning of the era which has your account in
//!    validators set.
//!
//!	## Notes
//!
//! - Era definition in this pallet is different than in the `pallet-staking`. In this pallet era is
//!   a period of time during which the validator is expected to produce a validation decision.
//!   Means staking era and DAC era are different and are not related to each other,
//! - You can set DAC Validators quorum size by specifying `DdcValidatorsQuorumSize` parameter,

#![cfg_attr(not(feature = "std"), no_std)]

pub use alloc::{format, string::String};
pub use alt_serde::{de::DeserializeOwned, Deserialize, Serialize};
pub use codec::{Decode, Encode, HasCompact, MaxEncodedLen};
pub use core::fmt::Debug;
pub use frame_support::{
	decl_event, decl_module, decl_storage,
	dispatch::DispatchResult,
	log::{error, info, warn},
	pallet_prelude::*,
	parameter_types,
	traits::{Currency, Randomness, UnixTime},
	weights::Weight,
	BoundedVec, RuntimeDebug,
};
pub use frame_system::{
	ensure_signed,
	offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer, SigningTypes},
	pallet_prelude::*,
};
pub use pallet::*;
pub use pallet_ddc_staking::{self as ddc_staking};
pub use pallet_session as session;
pub use pallet_staking::{self as staking};
pub use scale_info::TypeInfo;
pub use sp_core::crypto::{KeyTypeId, UncheckedFrom};
pub use sp_io::crypto::sr25519_public_keys;
pub use sp_runtime::offchain::{http, Duration, Timestamp};
pub use sp_staking::EraIndex;
pub use sp_std::prelude::*;

extern crate alloc;

/// The balance type of this pallet.
type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

type ResultStr<T> = Result<T, &'static str>;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"dacv");

pub const HTTP_TIMEOUT_MS: u64 = 30_000;

const TIME_START_MS: u128 = 1_672_531_200_000;
const ERA_DURATION_MS: u128 = 120_000;
const ERA_IN_BLOCKS: u8 = 20;

/// Webdis in experimental cluster connected to Redis in dev.
const DATA_PROVIDER_URL: &str = "http://161.35.140.182:7379/";

/// DAC Validation methods.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ValidationMethodKind {
	/// Compare amount of served content with amount of content consumed.
	ProofOfDelivery,
}

/// Associates validation decision with the validator and the method used to produce it.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Decision<AccountId> {
	/// Individual validator's decision. Can be `None` if the validator did not produce a decision
	/// (yet).
	pub decision: Option<bool>,
	/// The method used to produce the decision.
	pub method: ValidationMethodKind,
	/// The validator who produced the decision.
	pub validator: AccountId,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug, TypeInfo, Default)]
pub struct ValidationResult<AccountId> {
	era: EraIndex,
	signer: AccountId,
	val_res: bool,
	cdn_node_pub_key: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(crate = "alt_serde")]
#[serde(rename_all = "camelCase")]
pub struct RedisFtAggregate {
	#[serde(rename = "FT.AGGREGATE")]
	pub ft_aggregate: Vec<FtAggregate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(crate = "alt_serde")]
#[serde(untagged)]
pub enum FtAggregate {
	Length(u32),
	Node(Vec<String>),
}

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq)]
pub struct BytesSent {
	node_public_key: String,
	era: EraIndex,
	sum: u32,
}

impl BytesSent {
	pub fn new(aggregate: RedisFtAggregate) -> BytesSent {
		let data = aggregate.ft_aggregate[1].clone();

		match data {
			FtAggregate::Node(node) =>
				return BytesSent {
					node_public_key: node[1].clone(),
					era: node[3].clone().parse::<u32>().expect("era must be convertible u32")
						as EraIndex,
					sum: node[5].parse::<u32>().expect("bytesSentSum must be convertible to u32"),
				},
			FtAggregate::Length(_) => panic!("[DAC Validator] Not a Node"),
		}
	}

	pub fn get_all(aggregation: RedisFtAggregate) -> Vec<BytesSent> {
		let mut res: Vec<BytesSent> = vec![];
		for i in 1..aggregation.ft_aggregate.len() {
			let data = aggregation.ft_aggregate[i].clone();
			match data {
				FtAggregate::Node(node) => {
					let node = BytesSent {
						node_public_key: node[1].clone(),
						era: node[3].clone().parse::<u32>().expect("era must be convertible u32")
							as EraIndex,
						sum: node[5]
							.parse::<u32>()
							.expect("bytesSentSum must be convertible to u32"),
					};

					res.push(node);
				},
				FtAggregate::Length(_) => panic!("[DAC Validator] Not a Node"),
			}
		}

		return res
	}
}

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq)]
pub struct BytesReceived {
	node_public_key: String,
	era: EraIndex,
	sum: u32,
}

impl BytesReceived {
	pub fn new(aggregate: RedisFtAggregate) -> BytesReceived {
		let data = aggregate.ft_aggregate[1].clone();

		match data {
			FtAggregate::Node(node) =>
				return BytesReceived {
					node_public_key: node[1].clone(),
					era: node[3].clone().parse::<u32>().expect("era must be convertible u32")
						as EraIndex,
					sum: node[5]
						.parse::<u32>()
						.expect("bytesReceivedSum must be convertible to u32"),
				},
			FtAggregate::Length(_) => panic!("[DAC Validator] Not a Node"),
		}
	}

	pub fn get_all(aggregation: RedisFtAggregate) -> Vec<BytesReceived> {
		let mut res: Vec<BytesReceived> = vec![];
		for i in 1..aggregation.ft_aggregate.len() {
			let data = aggregation.ft_aggregate[i].clone();
			match data {
				FtAggregate::Node(node) => {
					let node = BytesReceived {
						node_public_key: node[1].clone(),
						era: node[3].clone().parse::<u32>().expect("era must be convertible u32")
							as EraIndex,
						sum: node[5]
							.parse::<u32>()
							.expect("bytesReceivedSum must be convertible to u32"),
					};

					res.push(node);
				},
				FtAggregate::Length(_) => panic!("[DAC Validator] Not a Node"),
			}
		}

		return res
	}
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

		/// Proof-of-Delivery parameter specifies an allowed deviation between bytes sent and bytes
		/// received. The deviation is expressed as a percentage. For example, if the value is 10,
		/// then the difference between bytes sent and bytes received is allowed to be up to 10%.
		/// The value must be in range [0, 100].
		#[pallet::constant]
		type ValidationThreshold: Get<u32>;
	}

	/// A signal to start a process on all the validators.
	#[pallet::storage]
	#[pallet::getter(fn signal)]
	pub(super) type Signal<T: Config> = StorageValue<_, bool>;

	/// The map from the era and CDN participant stash key to the validation decisions related.
	#[pallet::storage]
	#[pallet::getter(fn tasks)]
	pub type Tasks<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		T::AccountId,
		BoundedVec<Decision<T::AccountId>, T::DdcValidatorsQuorumSize>,
	>;

	/// The last era for which the tasks assignment produced.
	#[pallet::storage]
	#[pallet::getter(fn last_managed_era)]
	pub type LastManagedEra<T: Config> = StorageValue<_, EraIndex>;

	#[pallet::storage]
	#[pallet::getter(fn validation_results)]
	pub(super) type ValidationResults<T: Config> =
		StorageValue<_, Vec<ValidationResult<T::AccountId>>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<T::Hash>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		/// DAC Validator successfully published the validation decision.
		ValidationDecisionSubmitted,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Validation decision attempts to submit the result for the wrong era (not the current
		/// one).
		BadEra,
		/// Can't submit the validation decision twice.
		DecisionAlreadySubmitted,
		/// Task does not exist for a given era, CDN participant, and DAC validator.
		TaskNotFound,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<T::Hash>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			// Reset the signal in the beginning of the block to keep it reset until an incoming
			// transaction sets it to true.
			if Signal::<T>::get().unwrap_or(false) {
				Signal::<T>::set(Some(false));
			}

			// Old task manager.
			//
			// if block_number <= 1u32.into() {
			// 	return 0
			// }

			// let era = Self::get_current_era();
			// if let Some(last_managed_era) = <LastManagedEra<T>>::get() {
			// 	if last_managed_era >= era {
			// 		return 0
			// 	}
			// }
			// <LastManagedEra<T>>::put(era);

			// let validators: Vec<T::AccountId> = <staking::Validators<T>>::iter_keys().collect();
			// let validators_count = validators.len() as u32;
			// let edges: Vec<T::AccountId> =
			// <ddc_staking::pallet::Edges<T>>::iter_keys().collect(); log::info!(
			// 	"Block number: {:?}, global era: {:?}, last era: {:?}, validators_count: {:?},
			// validators: {:?}, edges: {:?}", 	block_number,
			// 	era,
			// 	<LastManagedEra<T>>::get(),
			// 	validators_count,
			// 	validators,
			// 	edges,
			// );

			// // A naive approach assigns random validators for each edge.
			// for edge in edges {
			// 	let mut decisions: BoundedVec<Decision<T::AccountId>, T::DdcValidatorsQuorumSize> =
			// 		Default::default();
			// 	while !decisions.is_full() {
			// 		let validator_idx = Self::choose(validators_count).unwrap_or(0) as usize;
			// 		let validator: T::AccountId = validators[validator_idx].clone();
			// 		let assignment = Decision {
			// 			validator,
			// 			method: ValidationMethodKind::ProofOfDelivery,
			// 			decision: None,
			// 		};
			// 		decisions.try_push(assignment).unwrap();
			// 	}
			// 	Tasks::<T>::insert(era, edge, decisions);
			// }

			0
		}

		/// Offchain worker entry point.
		///
		/// 1. Listen to a signal,
		/// 2. Run a process at the same time,
		/// 3. Read data from DAC.
		fn offchain_worker(block_number: T::BlockNumber) {
			// Skip if not a validator.
			if !sp_io::offchain::is_validator() {
				return
			}

			// Wait for signal.
			let signal = Signal::<T>::get().unwrap_or(false);
			if !signal {
				log::info!("🔎 DAC Validator is idle at block {:?}, waiting for a signal, signal state is {:?}", block_number, signal);
				return
			}

			// Read from DAC.
			let current_era = Self::get_current_era();
			let (sent_query, sent, received_query, received) = Self::fetch_data2(current_era - 1);
			log::info!(
				"🔎 DAC Validator is fetching data from DAC, current era: {:?}, bytes sent query: {:?}, bytes sent response: {:?}, bytes received query: {:?}, bytes received response: {:?}",
				current_era,
				sent_query,
				sent,
				received_query,
				received,
			);

			// Old off-chain worker.
			//
			// let pubkeys = sr25519_public_keys(KEY_TYPE);
			// if pubkeys.is_empty() {
			// 	log::info!("No local sr25519 accounts available to offchain worker.");
			// 	return
			// }
			// log::info!(
			// 	"Local sr25519 accounts available to offchain worker: {:?}, first pubilc key: {:?}",
			// 	pubkeys,
			// 	pubkeys.first().unwrap()
			// );

			// let res = Self::offchain_worker_main(block_number);

			// match res {
			// 	Ok(()) => info!("[DAC Validator] DAC Validator is suspended."),
			// 	Err(err) => error!("[DAC Validator] Error in Offchain Worker: {}", err),
			// };
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

		#[pallet::weight(10_000)]
		pub fn reset_signal(origin: OriginFor<T>) -> DispatchResult {
			ensure_signed(origin)?;

			Signal::<T>::set(Some(false));

			Ok(())
		}

		#[pallet::weight(10000)]
		pub fn save_validated_data(
			origin: OriginFor<T>,
			val_res: bool,
			cdn_node_pub_key: String,
			era: EraIndex,
		) -> DispatchResult {
			let signer: T::AccountId = ensure_signed(origin)?;

			info!("[DAC Validator] author: {:?}", signer);
			let mut v_results = ValidationResults::<T>::get();

			let cur_validation =
				ValidationResult::<T::AccountId> { era, val_res, cdn_node_pub_key, signer };

			v_results.push(cur_validation);

			ValidationResults::<T>::set(v_results);

			Ok(())
		}

		/// Set validation decision in tasks assignment.
		///
		/// `origin` must be a DAC Validator assigned to the task.
		/// `era` must be a current era, otherwise the decision will be rejected.
		/// `subject` is a CDN participant stash.
		///
		/// Emits `ValidationDecisionSubmitted` event.
		#[pallet::weight(100_000)]
		pub fn submit_validation_decision(
			origin: OriginFor<T>,
			era: EraIndex,
			subject: T::AccountId,
			method: ValidationMethodKind,
			decision: bool,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;

			ensure!(Self::get_current_era() == era, Error::<T>::BadEra);

			Tasks::<T>::try_mutate_exists(era, &subject, |maybe_tasks| {
				let mut tasks = maybe_tasks.take().ok_or(Error::<T>::TaskNotFound)?;
				let mut task = tasks
					.iter_mut()
					.find(|task| task.validator == account && task.method == method)
					.ok_or(Error::<T>::TaskNotFound)?;
				ensure!(task.decision.is_none(), Error::<T>::DecisionAlreadySubmitted);
				task.decision = Some(decision);

				Self::deposit_event(Event::ValidationDecisionSubmitted);

				Ok(())
			})
		}

		#[pallet::weight(10000)]
		pub fn proof_of_delivery(
			origin: OriginFor<T>,
			s: Vec<BytesSent>,
			r: Vec<BytesReceived>,
		) -> DispatchResult {
			info!("[DAC Validator] processing proof_of_delivery");
			let signer: T::AccountId = ensure_signed(origin)?;

			info!("signer: {:?}", Self::account_to_string(signer.clone()));

			let era = Self::get_current_era();
			let cdn_nodes_to_validate = Self::fetch_tasks(era, &signer);

			info!("[DAC Validator] cdn_nodes_to_validate: {:?}", cdn_nodes_to_validate);

			for cdn_node_id in cdn_nodes_to_validate {
				let (bytes_sent, bytes_received) = Self::filter_data(&s, &r, &cdn_node_id);
				let val_res = Self::validate(bytes_sent.clone(), bytes_received.clone());

				<Tasks<T>>::mutate(era, &cdn_node_id, |decisions_for_cdn| {
					let decisions =
						decisions_for_cdn.as_mut().expect("unexpected empty tasks assignment");
					let mut decision = decisions
						.iter_mut()
						.find(|decision| decision.validator == signer)
						.expect("unexpected validators set in tasks assignment");
					decision.decision = Some(val_res);
				});

				info!("[DAC Validator] decisions_for_cdn: {:?}", <Tasks<T>>::get(era, cdn_node_id));
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<T::Hash>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		fn offchain_worker_main(block_number: T::BlockNumber) -> ResultStr<()> {
			if block_number % ERA_IN_BLOCKS.into() != 0u32.into() {
				return Ok(())
			}

			let signer = match Self::get_signer() {
				Err(e) => {
					warn!("{:?}", e);
					return Ok(())
				},
				Ok(signer) => signer,
			};

			// Read data from DataModel and do dumb validation
			let current_era = Self::get_current_era() - 1;
			let (s, r) = Self::fetch_data1(current_era);

			let tx_res = signer.send_signed_transaction(|_acct| Call::proof_of_delivery {
				s: s.clone(),
				r: r.clone(),
			});

			match &tx_res {
				None => return Err("Error while submitting proof of delivery TX"),
				Some((_, Err(e))) => {
					info!("Error while submitting proof of delivery TX: {:?}", e);
					return Err("Error while submitting proof of delivery TX")
				},
				Some((_, Ok(()))) => {},
			}

			Ok(())
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

		fn fetch_data(era: EraIndex, cdn_node: &T::AccountId) -> (BytesSent, BytesReceived) {
			info!("[DAC Validator] DAC Validator is running. Current era is {}", era);
			// Todo: handle the error
			let bytes_sent_query = Self::get_bytes_sent_query_url(era);
			let bytes_sent_res: RedisFtAggregate = Self::http_get_json(&bytes_sent_query).unwrap();
			info!("[DAC Validator] Bytes sent sum is fetched: {:?}", bytes_sent_res);
			let bytes_sent = BytesSent::new(bytes_sent_res);

			// Todo: handle the error
			let bytes_received_query = Self::get_bytes_received_query_url(era);
			let bytes_received_res: RedisFtAggregate =
				Self::http_get_json(&bytes_received_query).unwrap();
			info!("[DAC Validator] Bytes received sum is fetched:: {:?}", bytes_received_res);
			let bytes_received = BytesReceived::new(bytes_received_res);

			(bytes_sent, bytes_received)
		}

		fn account_to_string(account: T::AccountId) -> String {
			let to32 = T::AccountId::encode(&account);
			let pub_key_str = array_bytes::bytes2hex("", to32);

			pub_key_str
		}

		fn filter_data(
			s: &Vec<BytesSent>,
			r: &Vec<BytesReceived>,
			a: &T::AccountId,
		) -> (BytesSent, BytesReceived) {
			let ac = Self::account_to_string(a.clone());

			let filtered_s = &*s.into_iter().find(|bs| bs.node_public_key == ac).unwrap();
			let filtered_r = &*r.into_iter().find(|br| br.node_public_key == ac).unwrap();

			(filtered_s.clone(), filtered_r.clone())
		}

		fn fetch_data1(era: EraIndex) -> (Vec<BytesSent>, Vec<BytesReceived>) {
			info!("[DAC Validator] DAC Validator is running. Current era is {}", era);
			// Todo: handle the error
			let bytes_sent_query = Self::get_bytes_sent_query_url(era);
			let bytes_sent_res: RedisFtAggregate = Self::http_get_json(&bytes_sent_query).unwrap();
			info!("[DAC Validator] Bytes sent sum is fetched: {:?}", bytes_sent_res);
			let bytes_sent = BytesSent::get_all(bytes_sent_res);

			// Todo: handle the error
			let bytes_received_query = Self::get_bytes_received_query_url(era);
			let bytes_received_res: RedisFtAggregate =
				Self::http_get_json(&bytes_received_query).unwrap();
			info!("[DAC Validator] Bytes received sum is fetched:: {:?}", bytes_received_res);
			let bytes_received = BytesReceived::get_all(bytes_received_res);

			(bytes_sent, bytes_received)
		}

		fn fetch_data2(era: EraIndex) -> (String, Vec<BytesSent>, String, Vec<BytesReceived>) {
			let bytes_sent_query = Self::get_bytes_sent_query_url(era);
			let bytes_sent_res: RedisFtAggregate = Self::http_get_json(&bytes_sent_query).unwrap();
			let bytes_sent = BytesSent::get_all(bytes_sent_res);

			let bytes_received_query = Self::get_bytes_received_query_url(era);
			let bytes_received_res: RedisFtAggregate =
				Self::http_get_json(&bytes_received_query).unwrap();
			let bytes_received = BytesReceived::get_all(bytes_received_res);

			(bytes_sent_query, bytes_sent, bytes_received_query, bytes_received)
		}

		fn get_bytes_sent_query_url(era: EraIndex) -> String {
			format!("{}FT.AGGREGATE/ddc:dac:searchCommonIndex/@era:[{}%20{}]/GROUPBY/2/@nodePublicKey/@era/REDUCE/SUM/1/@bytesSent/AS/bytesSentSum", DATA_PROVIDER_URL, era, era)
		}

		fn get_bytes_received_query_url(era: EraIndex) -> String {
			format!("{}FT.AGGREGATE/ddc:dac:searchCommonIndex/@era:[{}%20{}]/GROUPBY/2/@nodePublicKey/@era/REDUCE/SUM/1/@bytesReceived/AS/bytesReceivedSum", DATA_PROVIDER_URL, era, era)
		}

		fn http_get_json<OUT: DeserializeOwned>(url: &str) -> ResultStr<OUT> {
			let body = Self::http_get_request(url).map_err(|err| {
				error!("[DAC Validator] Error while getting {}: {:?}", url, err);
				"HTTP GET error"
			})?;

			let parsed = serde_json::from_slice(&body).map_err(|err| {
				warn!("[DAC Validator] Error while parsing JSON from {}: {:?}", url, err);
				"HTTP JSON parse error"
			});

			parsed
		}

		fn http_get_request(http_url: &str) -> Result<Vec<u8>, http::Error> {
			// info!("[DAC Validator] Sending request to: {:?}", http_url);

			// Initiate an external HTTP GET request. This is using high-level wrappers from
			// `sp_runtime`.
			let request = http::Request::get(http_url);

			let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(HTTP_TIMEOUT_MS));

			let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;

			let response =
				pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

			if response.code != 200 {
				warn!("[DAC Validator] http_get_request unexpected status code: {}", response.code);
				return Err(http::Error::Unknown)
			}

			// Next we fully read the response body and collect it to a vector of bytes.
			Ok(response.body().collect::<Vec<u8>>())
		}

		fn validate(bytes_sent: BytesSent, bytes_received: BytesReceived) -> bool {
			let percentage_difference = 1f32 - (bytes_received.sum as f32 / bytes_sent.sum as f32);

			return if percentage_difference > 0.0 &&
				(T::ValidationThreshold::get() as f32 - percentage_difference) > 0.0
			{
				true
			} else {
				false
			}
		}

		/// Fetch the tasks related to current validator
		fn fetch_tasks(era: EraIndex, validator: &T::AccountId) -> Vec<T::AccountId> {
			let mut cdn_nodes: Vec<T::AccountId> = vec![];
			for (cdn_id, cdn_tasks) in <Tasks<T>>::iter_prefix(era) {
				info!("[DAC Validator] tasks assigned to {:?}: {:?}", cdn_id, cdn_tasks);

				for decision in cdn_tasks.iter() {
					if decision.validator == *validator {
						cdn_nodes.push(cdn_id);
						break
					}
				}
			}
			cdn_nodes
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
	}
}
