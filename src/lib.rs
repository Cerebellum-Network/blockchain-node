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
	log::{error, info, warn},
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
pub use pallet::*;
pub use pallet_ddc_staking::{self as ddc_staking};
pub use pallet_session as session;
pub use pallet_staking::{self as staking};
pub use scale_info::TypeInfo;
pub use sp_core::crypto::{KeyTypeId, UncheckedFrom};
pub use sp_io::crypto::sr25519_public_keys;
pub use sp_runtime::offchain::{http, storage::StorageValueRef, Duration, Timestamp};
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
const DEFAULT_DATA_PROVIDER_URL: &str = "localhost:7379/";
const DATA_PROVIDER_URL_KEY: &[u8; 32] = b"ddc-validator::data-provider-url";

/// Aggregated values from DAC that describe CDN node's activity during a certain era.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
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
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ValidationDecision {
	/// Validation result.
	pub result: bool,
	/// A hash of the data used to produce validation result.
	pub payload: [u8; 256],
	/// Values aggregated from the payload.
	pub totals: DacTotalAggregates,
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

		/// A dispatchable call.
		type Call: From<Call<Self>>;

		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type TimeProvider: UnixTime;

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

	/// The map from the era and CDN participant stash key to the validation decision related.
	#[pallet::storage]
	#[pallet::getter(fn validation_decisions)]
	pub type ValidationDecisions<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EraIndex, Twox64Concat, T::AccountId, ValidationDecision>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<T::Hash>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode, {}

	#[pallet::error]
	pub enum Error<T> {}

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

			0
		}

		/// Offchain worker entry point.
		///
		/// 1. Listen to a signal,
		/// 2. Run a process at the same time,
		/// 3. Read data from DAC.
		fn offchain_worker(block_number: T::BlockNumber) {
			let data_provider_url = Self::get_data_provider_url();

			info!("data_provider_url: {:?}", data_provider_url.unwrap_or(String::from("no url")));

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
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: AsRef<[u8]> + UncheckedFrom<T::Hash>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
	{
		fn get_data_provider_url() -> Option<String> {
			let url_ref = sp_io::offchain::local_storage_get(
				sp_core::offchain::StorageKind::PERSISTENT,
				DATA_PROVIDER_URL_KEY,
			);

			match url_ref {
				None => {
					let url_key = String::from_utf8(DATA_PROVIDER_URL_KEY.to_vec()).unwrap();
					let msg = format!("[DAC Validator] Data provider URL is not configured. Please configure it using offchain_localStorageSet with key {:?}", url_key);
					warn!("{}", msg);
					None
				},
				Some(url) => Some(String::from_utf8(url).unwrap()),
			}
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
			let data_provider_url = Self::get_data_provider_url();

			match data_provider_url {
				Some(url) => {
					return format!("{}FT.AGGREGATE/ddc:dac:searchCommonIndex/@era:[{}%20{}]/GROUPBY/2/@nodePublicKey/@era/REDUCE/SUM/1/@bytesSent/AS/bytesSentSum", url, era, era);
				},
				None => {
					return format!("{}FT.AGGREGATE/ddc:dac:searchCommonIndex/@era:[{}%20{}]/GROUPBY/2/@nodePublicKey/@era/REDUCE/SUM/1/@bytesSent/AS/bytesSentSum", DEFAULT_DATA_PROVIDER_URL, era, era);
				},
			}
		}

		fn get_bytes_received_query_url(era: EraIndex) -> String {
			let data_provider_url = Self::get_data_provider_url();

			match data_provider_url {
				Some(url) => {
					return format!("{}FT.AGGREGATE/ddc:dac:searchCommonIndex/@era:[{}%20{}]/GROUPBY/2/@nodePublicKey/@era/REDUCE/SUM/1/@bytesReceived/AS/bytesReceivedSum", url, era, era);
				},
				None => {
					return format!("{}FT.AGGREGATE/ddc:dac:searchCommonIndex/@era:[{}%20{}]/GROUPBY/2/@nodePublicKey/@era/REDUCE/SUM/1/@bytesReceived/AS/bytesReceivedSum", DEFAULT_DATA_PROVIDER_URL, era, era);
				},
			}
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
	}
}
