//! # DDC Verification Pallet
//!
//! The DDC Verification pallet is used to validate zk-SNARK Proof and Signature
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
#![allow(clippy::missing_docs_in_private_items)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]
#![allow(clippy::manual_inspect)]
use core::str;

use ddc_api::json;
#[cfg(feature = "runtime-benchmarks")]
use ddc_primitives::traits::{ClusterCreator, CustomerDepositor};
use ddc_primitives::{
	ocw_mutex::OcwMutex,
	traits::{
		BucketManager, ClusterManager, ClusterProtocol, ClusterValidator, CustomerVisitor,
		InspReceiptsInterceptor, NodeManager, PayoutProcessor, StorageUsageProvider,
		ValidatorVisitor,
	},
	AggregateKey, BucketStorageUsage, ClusterId, ClusterStatus, EHDId, NodeStorageUsage,
	PayoutFingerprintParams, PayoutReceiptParams, PayoutState, StorageNodePubKey,
};
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, Get, OneSessionHandler, StorageVersion},
};
use frame_system::{
	offchain::{Account, AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer},
	pallet_prelude::*,
};
use itertools::Itertools;
pub use pallet::*;
use scale_info::prelude::{format, string::String};
use serde::{Deserialize, Serialize};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::{crypto::UncheckedFrom, H256};
pub use sp_io::{
	crypto::sr25519_public_keys,
	offchain::{
		local_storage_clear, local_storage_compare_and_set, local_storage_get, local_storage_set,
	},
};
use sp_runtime::{
	offchain::StorageKind,
	traits::{Hash, IdentifyAccount},
	Percent,
};
use sp_staking::StakingInterface;
use sp_std::{collections::btree_set::BTreeSet, fmt::Debug, prelude::*};
pub mod weights;

use crate::weights::WeightInfo;

// #[cfg(feature = "runtime-benchmarks")]
// pub mod benchmarking;

// #[cfg(test)]
// pub(crate) mod mock;
// #[cfg(test)]
// mod tests;

pub mod demo;
pub mod migrations;
use ddc_api::api::{get_g_collectors_nodes, submit_inspection_report};

pub mod aggregate_tree;
pub mod insp_task_manager;
use insp_task_manager::{
	HashedInspPathReceipt, InspEraReport, InspPathsReceipts, InspTaskManager, InspectionError,
};

pub(crate) type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {

	use ddc_primitives::{AggregatorInfo, BucketId, DeltaUsageHash, DAC_VERIFICATION_KEY_TYPE};
	use frame_support::PalletId;
	use sp_core::crypto::AccountId32;
	use sp_runtime::SaturatedConversion;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	const _SUCCESS_CODE: u16 = 200;
	const _BUF_SIZE: usize = 128;
	pub const OCW_MUTEX_ID: &[u8] = b"inspection_lock";

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The accounts's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Weight info type.
		type WeightInfo: WeightInfo;
		/// DDC clusters nodes manager.
		type ClusterValidator: ClusterValidator;
		type ClusterManager: ClusterManager<Self::AccountId, BlockNumberFor<Self>>;
		type PayoutProcessor: PayoutProcessor<Self>;
		/// DDC nodes read-only registry.
		type NodeManager: NodeManager<Self::AccountId>;
		/// The hashing system (algorithm)
		type Hasher: Hash<Output = DeltaUsageHash>;
		/// The identifier type for an authority.
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ Into<sp_core::sr25519::Public>
			+ From<sp_core::sr25519::Public>;
		/// The identifier type for an offchain worker.
		type OffchainIdentifierId: AppCrypto<Self::Public, Self::Signature>;
		/// Block to start from.
		const BLOCK_TO_START: u16;
		const DAC_REDUNDANCY_FACTOR: u16;

		#[pallet::constant]
		type AggregatorsQuorum: Get<Percent>;
		#[pallet::constant]
		type ValidatorsQuorum: Get<Percent>;

		const MAX_PAYOUT_BATCH_COUNT: u16;
		const MAX_PAYOUT_BATCH_SIZE: u16;
		const MAX_MERKLE_NODE_IDENTIFIER: u16;
		/// The access to staking functionality.
		type ValidatorStaking: StakingInterface<
			AccountId = Self::AccountId,
			Balance = BalanceOf<Self>,
		>;
		type AccountIdConverter: From<Self::AccountId> + Into<AccountId32>;
		type CustomerVisitor: CustomerVisitor<Self>;
		type BucketsStorageUsageProvider: StorageUsageProvider<
			BucketId,
			BucketStorageUsage<Self::AccountId>,
		>;
		type NodesStorageUsageProvider: StorageUsageProvider<
			StorageNodePubKey,
			NodeStorageUsage<Self::AccountId>,
		>;
		type Currency: Currency<Self::AccountId>;
		const DISABLE_PAYOUTS_CUTOFF: bool;
		const DEBUG_MODE: bool;
		type ClusterProtocol: ClusterProtocol<
			Self::AccountId,
			BlockNumberFor<Self>,
			BalanceOf<Self>,
		>;
		#[cfg(feature = "runtime-benchmarks")]
		type CustomerDepositor: CustomerDepositor<Self>;
		#[cfg(feature = "runtime-benchmarks")]
		type ClusterCreator: ClusterCreator<Self::AccountId, BlockNumberFor<Self>, BalanceOf<Self>>;
		type BucketManager: BucketManager<Self>;
		type InspReceiptsInterceptor: InspReceiptsInterceptor<Receipt = HashedInspPathReceipt>;
	}

	/// The event type.
	#[pallet::event]
	/// The `generate_deposit` macro generates a function on `Pallet` called `deposit_event` which
	/// will properly convert the error type of your pallet into `RuntimeEvent` (recall `type
	/// RuntimeEvent: From<Event<Self>>`, so it can be converted) and deposit it via
	/// `frame_system::Pallet::deposit_event`.
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		FailedToCollectVerificationKey { validator: T::AccountId },
		FailedToFetchVerificationKey { validator: T::AccountId },
		ValidatorKeySet { validator: T::AccountId },
		InspError { validator: T::AccountId, cluster_id: ClusterId, err: InspectionError },
		FailedToSignPathsInspection { validator: T::AccountId, cluster_id: ClusterId },
		Unexpected { validator: T::AccountId },
		ApiError { validator: T::AccountId },
	}

	/// Debugging errors
	#[derive(Debug, Encode, Decode, Clone, TypeInfo, PartialEq)]
	pub enum InspOCWError {
		/// Traverse Response Retrieval Error.
		FailedToCollectVerificationKey,
		FailedToFetchVerificationKey,
		FailedToSignPathsInspection {
			cluster_id: ClusterId,
		},
		InspError {
			cluster_id: ClusterId,
			err: InspectionError,
		},
		Unexpected,
		ApiError,
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		/// Bad requests.
		BadRequest,
		/// Not a validator.
		Unauthorized,
		/// Not a controller.
		NotController,
		/// Not a validator stash.
		NotValidatorStash,
		/// Given era is already validated and paid.
		EraAlreadyPaid,
		VerificationKeyAlreadySet,
		VerificationKeyAlreadyPaired,
	}

	/// List of validators.
	#[pallet::storage]
	pub type ValidatorSet<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	/// Validator stash key mapping
	#[pallet::storage]
	pub type ValidatorToStashKey<T: Config> = StorageMap<_, Identity, T::AccountId, T::AccountId>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set validator key.
		///
		/// The origin must be a validator.
		///
		/// Parameters:
		/// - `ddc_validator_pub`: validator Key
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_validator_key())]
		pub fn set_validator_key(
			origin: OriginFor<T>,
			ddc_validator_pub: T::AccountId,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			let stash = T::ValidatorStaking::stash_by_ctrl(&controller)
				.map_err(|_| Error::<T>::NotController)?;

			ensure!(
				<ValidatorSet<T>>::get().contains(&ddc_validator_pub),
				Error::<T>::NotValidatorStash
			);

			ValidatorToStashKey::<T>::insert(&ddc_validator_pub, &stash);
			Self::deposit_event(Event::<T>::ValidatorKeySet { validator: ddc_validator_pub });
			Ok(())
		}

		/// Emit consensus errors.
		///
		/// The origin must be a validator.
		///
		/// Parameters:
		/// - errors`: List of consensus errors
		///
		/// Emits `NotEnoughNodesForConsensus`  OR `ActivityNotInConsensus` event depend of error
		/// type, when successful.
		#[pallet::call_index(11)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::emit_consensus_errors(errors.len() as u32))]
		pub fn emit_consensus_errors(
			origin: OriginFor<T>,
			errors: Vec<InspOCWError>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Self::is_ocw_validator(caller.clone()), Error::<T>::Unauthorized);

			for error in errors {
				match error {
					InspOCWError::FailedToCollectVerificationKey => {
						Self::deposit_event(Event::FailedToCollectVerificationKey {
							validator: caller.clone(),
						});
					},
					InspOCWError::FailedToFetchVerificationKey => {
						Self::deposit_event(Event::FailedToFetchVerificationKey {
							validator: caller.clone(),
						});
					},
					InspOCWError::FailedToSignPathsInspection { cluster_id } => {
						Self::deposit_event(Event::FailedToSignPathsInspection {
							validator: caller.clone(),
							cluster_id,
						});
					},
					InspOCWError::Unexpected => {
						Self::deposit_event(Event::Unexpected { validator: caller.clone() });
					},
					InspOCWError::ApiError => {
						Self::deposit_event(Event::ApiError { validator: caller.clone() });
					},
					InspOCWError::InspError { cluster_id, err } => {
						Self::deposit_event(Event::InspError {
							validator: caller.clone(),
							cluster_id,
							err,
						});
					},
				}
			}

			Ok(())
		}

		/// Continue DAC inspection from an era after a given one. It updates `last_paid_era` of a
		/// given cluster, creates an empty payout receipt with a finalized state, and sets an empty
		/// validation result on validators (in case it does not exist yet).
		#[pallet::call_index(12)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::force_skip_inspection())]
		pub fn force_skip_inspection(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			ehd_id: String,
		) -> DispatchResult {
			let era_id = EHDId::try_from(ehd_id.clone()).map_err(|_| Error::<T>::BadRequest)?.2;

			ensure_root(origin)?;
			ensure!(
				era_id > T::ClusterValidator::get_last_paid_era(&cluster_id)?,
				Error::<T>::EraAlreadyPaid
			);

			let fingerprint =
				T::PayoutProcessor::create_payout_fingerprint(PayoutFingerprintParams {
					cluster_id,
					ehd_id: ehd_id.clone(),
					payers_merkle_root: Default::default(),
					payees_merkle_root: Default::default(),
					validators: BTreeSet::new(),
				});

			let payout_receipt_params = PayoutReceiptParams {
				cluster_id,
				era: era_id,
				state: PayoutState::Finalized,
				fingerprint,
				..Default::default()
			};
			let finalized_at = <frame_system::Pallet<T>>::block_number();
			T::PayoutProcessor::create_payout_receipt(
				T::AccountId::decode(&mut [0u8; 32].as_slice()).unwrap(),
				payout_receipt_params,
				Some(finalized_at),
			);

			T::ClusterValidator::set_last_paid_era(&cluster_id, era_id)?;

			Ok(())
		}

		/// For Devnet usage only
		#[pallet::call_index(13)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::force_set_validator_key())]
		pub fn force_set_validator_key(
			origin: OriginFor<T>,
			controller: T::AccountId,
			ddc_validator_pub: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;

			let stash = T::ValidatorStaking::stash_by_ctrl(&controller)
				.map_err(|_| Error::<T>::NotController)?;

			ensure!(
				!<ValidatorSet<T>>::get().contains(&ddc_validator_pub),
				Error::<T>::VerificationKeyAlreadySet
			);
			ensure!(
				!<ValidatorToStashKey<T>>::contains_key(&ddc_validator_pub),
				Error::<T>::VerificationKeyAlreadyPaired
			);

			<ValidatorSet<T>>::append(ddc_validator_pub.clone());
			ValidatorToStashKey::<T>::insert(&ddc_validator_pub, &stash);

			Self::deposit_event(Event::<T>::ValidatorKeySet { validator: ddc_validator_pub });
			Ok(())
		}
	}

	/// Unwrap or send an error log
	macro_rules! unwrap_or_log_error {
		($result:expr, $error_msg:expr) => {
			match $result {
				Ok(val) => val,
				Err(err) => {
					log::error!("{}: {:?}", $error_msg, err);
					return;
				},
			}
		};
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: BlockNumberFor<T>) {
			if block_number.saturated_into::<u32>() % T::BLOCK_TO_START as u32 != 0 {
				return;
			}

			if !sp_io::offchain::is_validator() {
				return;
			}

			// Allow only one instance of the off-chain worker to run at the same time.
			let mut ocw_mutex = OcwMutex::new(OCW_MUTEX_ID.to_vec());
			if !ocw_mutex.try_lock() {
				log::debug!("Another inspection OCW is already running, terminating...",);
				return;
			}
			log::debug!("OCW mutex 0x{} locked.", hex::encode(ocw_mutex.local_storage_key()));

			let verification_account = unwrap_or_log_error!(
				Self::collect_verification_pub_key(),
				"❌ Error collecting validator verification key"
			);

			let signer = Signer::<T, T::OffchainIdentifierId>::any_account()
				.with_filter(vec![verification_account.public.clone()]);

			if !signer.can_sign() {
				log::error!("🚨 OCW signer is not available");
				return;
			}

			Self::store_verification_account_id(verification_account.public.clone().into_account());

			let clusters_ids = unwrap_or_log_error!(
				T::ClusterManager::get_clusters(ClusterStatus::Activated),
				"❌ Error retrieving clusters to validate"
			);
			log::info!("🎡 {:?} of 'Activated' clusters found", clusters_ids.len());

			for cluster_id in clusters_ids {
				let mut errors: Vec<InspOCWError> = Vec::new();

				let inspection_result = Self::start_inspection_phase(
					&cluster_id,
					&verification_account,
					&signer,
					&block_number,
				);

				if let Err(errs) = inspection_result {
					errors.extend(vec![errs]);
				}

				if T::DEBUG_MODE {
					Self::submit_errors(&errors, &verification_account, &signer);
				}
			}
		}
	}

	impl<T: Config> Pallet<T> {
		#[allow(clippy::collapsible_else_if)]
		pub(crate) fn start_inspection_phase(
			cluster_id: &ClusterId,
			verification_account: &Account<T>,
			_signer: &Signer<T, T::OffchainIdentifierId>,
			block_number: &BlockNumberFor<T>,
		) -> Result<(), InspOCWError> {
			let g_collectors = get_g_collectors_nodes::<
				T::AccountId,
				BlockNumberFor<T>,
				T::ClusterManager,
				T::NodeManager,
			>(cluster_id)
			.map_err(|_| InspOCWError::ApiError)?;
			// todo(yahortsaryk): infer G-Collector node deterministically
			let Some(g_collector) = g_collectors.first() else {
				log::warn!("⚠️ No Grouping Collector found in cluster {:?}", cluster_id);
				return Ok(());
			};

			let mut insp_task_manager = InspTaskManager::<T>::new(verification_account.clone());
			insp_task_manager
				.assign_cluster(cluster_id, block_number)
				.map_err(|e| InspOCWError::InspError { cluster_id: *cluster_id, err: e })?;

			for era_receipts in insp_task_manager
				.inspect_cluster(cluster_id, block_number)
				.map_err(|e| InspOCWError::InspError { cluster_id: *cluster_id, err: e })?
			{
				// todo(yahortsaryk): retrieve ID of canonical EHD from inspection result
				let ehd_id = EHDId(*cluster_id, g_collector.clone().0, era_receipts.era);
				let signed_report = Self::build_signed_inspection_report(
					cluster_id,
					era_receipts,
					verification_account,
				)?;

				// todo(yahortsaryk): add .proto definition for `InspEraReport` type
				let signed_report_json_str =
					serde_json::to_string(&signed_report).map_err(|_| InspOCWError::Unexpected)?;

				let _ = submit_inspection_report::<
					T::AccountId,
					BlockNumberFor<T>,
					T::ClusterManager,
					T::NodeManager,
				>(cluster_id, signed_report_json_str)
				.map_err(|e| {
					log::error!("Could not submit inspection report {:?}", e);
					InspOCWError::Unexpected
				})?;

				store_last_inspected_ehd(cluster_id, ehd_id);
			}

			Ok(())
		}

		#[allow(clippy::map_entry)]
		fn build_signed_inspection_report(
			cluster_id: &ClusterId,
			era_receipts: InspPathsReceipts,
			verification_account: &Account<T>,
		) -> Result<InspEraReport, InspOCWError> {
			let hashed_receipts: Vec<HashedInspPathReceipt> =
				era_receipts.receipts.into_iter().fold(vec![], |mut receipts, receipt| {
					let hash = receipt.hash::<T>();
					let hash_hex = hex::encode(hash);
					let hashed_receipt =
						HashedInspPathReceipt::new(receipt, format!("0x{}", hash_hex));
					receipts.push(hashed_receipt);
					receipts
				});

			let hashed_receipts = T::InspReceiptsInterceptor::intercept(hashed_receipts);

			let sig_payload = hashed_receipts
				.iter()
				.map(|hashed_receipt| hashed_receipt.hash.clone())
				.collect::<Vec<_>>()
				.encode();

			let signature = <T::OffchainIdentifierId as AppCrypto<T::Public, T::Signature>>::sign(
				&sig_payload,
				verification_account.public.clone(),
			)
			.ok_or(InspOCWError::FailedToSignPathsInspection { cluster_id: *cluster_id })?;

			let inspector_pub_bytes: Vec<u8> = verification_account.public.encode();
			let inspector_pub = format!("0x{}", hex::encode(&inspector_pub_bytes[1..])); // skip byte of SCALE encoding
			let signature_bytes: Vec<u8> = signature.encode();
			let signature = format!("0x{}", hex::encode(&signature_bytes[1..])); // skip byte of SCALE encoding

			let report = InspEraReport {
				era: era_receipts.era,
				inspector: inspector_pub,
				signature,
				path_receipts: hashed_receipts,
			};

			Ok(report)
		}

		pub(crate) fn submit_errors(
			errors: &Vec<InspOCWError>,
			verification_account: &Account<T>,
			signer: &Signer<T, T::OffchainIdentifierId>,
		) {
			if !errors.is_empty() {
				let call = Call::emit_consensus_errors { errors: errors.to_owned() };
				let result = signer.send_single_signed_transaction(verification_account, call);

				if let Some(Ok(_)) = result {
					log::info!("✔️ Successfully sent 'emit_consensus_errors' call");
				} else {
					log::error!("❌ Failed to send 'emit_consensus_errors' call");
				};
			}
		}

		pub(crate) fn collect_verification_pub_key() -> Result<Account<T>, InspOCWError> {
			let session_verification_keys = <T::OffchainIdentifierId as AppCrypto<
				T::Public,
				T::Signature,
			>>::RuntimeAppPublic::all()
			.into_iter()
			.enumerate()
			.filter_map(|(i, key)| {
				let generic_public = <T::OffchainIdentifierId as AppCrypto<
					T::Public,
					T::Signature,
				>>::GenericPublic::from(key);
				let public_key: T::Public = generic_public.into();
				let account_id = public_key.clone().into_account();

				if <ValidatorSet<T>>::get().contains(&account_id) {
					let account = Account::new(i, account_id, public_key);
					Option::Some(account)
				} else {
					Option::None
				}
			})
			.collect::<Vec<_>>();

			if session_verification_keys.len() != 1 {
				log::error!(
					"🚨 Unexpected number of session verification keys is found. Expected: 1, Actual: {:?}",
					session_verification_keys.len()
				);
				return Err(InspOCWError::FailedToCollectVerificationKey);
			}

			session_verification_keys
				.into_iter()
				.next() // first
				.ok_or(InspOCWError::FailedToCollectVerificationKey)
		}

		pub(crate) fn store_verification_account_id(account_id: T::AccountId) {
			let validator: Vec<u8> = account_id.encode();
			let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();
			local_storage_set(StorageKind::PERSISTENT, &key, &validator);
		}

		pub(crate) fn _fetch_verification_account_id() -> Result<T::AccountId, InspOCWError> {
			let key = format!("offchain::validator::{:?}", DAC_VERIFICATION_KEY_TYPE).into_bytes();

			match local_storage_get(StorageKind::PERSISTENT, &key) {
				Some(data) => {
					let account_id = T::AccountId::decode(&mut &data[..])
						.map_err(|_| InspOCWError::FailedToFetchVerificationKey)?;
					Ok(account_id)
				},
				None => Err(InspOCWError::FailedToFetchVerificationKey),
			}
		}
	}

	impl<T: Config> ValidatorVisitor<T> for Pallet<T> {
		fn is_ocw_validator(caller: T::AccountId) -> bool {
			if ValidatorToStashKey::<T>::contains_key(caller.clone()) {
				<ValidatorSet<T>>::get().contains(&caller)
			} else {
				false
			}
		}

		fn is_quorum_reached(quorum: Percent, members_count: usize) -> bool {
			let threshold = quorum * <ValidatorSet<T>>::get().len();
			threshold <= members_count
		}
	}

	pub(crate) trait Hashable {
		/// Hash of the entity
		fn hash<T: Config>(&self) -> H256;
	}

	/// The 'Aggregate' trait defines a set of members common to activity aggregates, which reflect
	/// the usage of a node or bucket within an Era..
	#[allow(unused)]
	pub(crate) trait Aggregate:
		Hashable + Clone + Ord + PartialEq + Eq + Serialize + for<'de> Deserialize<'de> + Debug
	{
		/// Aggregation key of this aggregate, i.e. bucket composite key or node key
		fn get_key(&self) -> AggregateKey;
		/// Number of activity records this aggregated by this aggregate
		fn get_number_of_leaves(&self) -> u64;
		/// Aggregator provided this aggregate
		fn get_aggregator(&self) -> AggregatorInfo;
	}

	impl Hashable for json::BucketSubAggregate {
		fn hash<T: Config>(&self) -> DeltaUsageHash {
			let mut data = self.bucket_id.encode();
			data.extend_from_slice(&self.node_id.encode());
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			data.extend_from_slice(&self.number_of_puts.encode());
			data.extend_from_slice(&self.number_of_gets.encode());
			T::Hasher::hash(&data)
		}
	}

	impl Hashable for json::BucketAggregateResponse {
		fn hash<T: Config>(&self) -> DeltaUsageHash {
			let mut data = self.bucket_id.encode();
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			data.extend_from_slice(&self.number_of_puts.encode());
			data.extend_from_slice(&self.number_of_gets.encode());
			T::Hasher::hash(&data)
		}
	}

	impl Aggregate for json::BucketAggregateResponse {
		fn get_key(&self) -> AggregateKey {
			AggregateKey::BucketAggregateKey(self.bucket_id)
		}

		fn get_number_of_leaves(&self) -> u64 {
			self.number_of_gets.saturating_add(self.number_of_puts)
		}

		fn get_aggregator(&self) -> AggregatorInfo {
			unimplemented!()
		}
	}

	impl Aggregate for json::BucketSubAggregate {
		fn get_key(&self) -> AggregateKey {
			AggregateKey::BucketSubAggregateKey(self.bucket_id, self.node_id.clone())
		}

		fn get_number_of_leaves(&self) -> u64 {
			self.number_of_gets.saturating_add(self.number_of_puts)
		}

		fn get_aggregator(&self) -> AggregatorInfo {
			self.aggregator.clone()
		}
	}

	impl Hashable for json::NodeAggregate {
		fn hash<T: Config>(&self) -> DeltaUsageHash {
			let mut data = self.node_id.encode();
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			data.extend_from_slice(&self.number_of_puts.encode());
			data.extend_from_slice(&self.number_of_gets.encode());
			T::Hasher::hash(&data)
		}
	}

	impl Aggregate for json::NodeAggregate {
		fn get_key(&self) -> AggregateKey {
			AggregateKey::NodeAggregateKey(self.node_id.clone())
		}

		fn get_aggregator(&self) -> AggregatorInfo {
			self.aggregator.clone()
		}

		fn get_number_of_leaves(&self) -> u64 {
			self.number_of_gets.saturating_add(self.number_of_puts)
		}
	}
	pub trait NodeAggregateLeaf:
		Clone + Ord + PartialEq + Eq + Serialize + for<'de> Deserialize<'de>
	{
		fn leaf_hash<T: Config>(&self) -> DeltaUsageHash;
	}

	pub trait BucketSubAggregateLeaf:
		Clone + Ord + PartialEq + Eq + Serialize + for<'de> Deserialize<'de>
	{
		fn leaf_hash<T: Config>(&self) -> DeltaUsageHash;
	}

	impl NodeAggregateLeaf for json::Leaf {
		fn leaf_hash<T: Config>(&self) -> DeltaUsageHash {
			let mut data = self.record.id.encode();
			data.extend_from_slice(&self.record.upstream.request.requestType.encode());
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			T::Hasher::hash(&data)
		}
	}

	impl BucketSubAggregateLeaf for json::Leaf {
		fn leaf_hash<T: Config>(&self) -> DeltaUsageHash {
			let mut data = self.record.upstream.request.bucketId.encode();
			data.extend_from_slice(&self.record.encode());
			data.extend_from_slice(&self.record.upstream.request.requestType.encode());
			data.extend_from_slice(&self.stored_bytes.encode());
			data.extend_from_slice(&self.transferred_bytes.encode());
			T::Hasher::hash(&data)
		}
	}

	/* ######## Off-chain storage functions ######## */
	pub(crate) fn derive_last_inspected_ehd_key(cluster_id: &ClusterId) -> Vec<u8> {
		format!("offchain::inspected_ehds::v1::{:?}", cluster_id).into_bytes()
	}

	pub(crate) fn store_last_inspected_ehd(cluster_id: &ClusterId, ehd_id: EHDId) {
		let key = derive_last_inspected_ehd_key(cluster_id);

		if let Some(mut inspected_ehds) = fetch_last_inspected_ehds(cluster_id) {
			let last_era = inspected_ehds
				.iter()
				.max_by_key(|ehd| ehd.2)
				.map(|ehd| ehd.2)
				.unwrap_or(Default::default());
			if last_era >= ehd_id.2 {
				return;
			}

			inspected_ehds.push(ehd_id);

			let encoded_ehds_ids =
				inspected_ehds.into_iter().sorted().collect::<Vec<EHDId>>().encode();
			local_storage_set(StorageKind::PERSISTENT, &key, &encoded_ehds_ids);
		} else {
			log::warn!(
				"🗄️  Failed to retrieve last inspected ehds from offchain storage for cluster_id: {:?}",
				cluster_id,
			);
		}
	}

	pub(crate) fn fetch_last_inspected_ehds(cluster_id: &ClusterId) -> Option<Vec<EHDId>> {
		log::info!("🗄️  Trying to fetch last inspected ehds for cluster_id: {:?}", cluster_id,);

		let key = derive_last_inspected_ehd_key(cluster_id);

		let encoded_last_inspected_ehd: Vec<u8> =
			match local_storage_get(StorageKind::PERSISTENT, &key) {
				Some(encoded_data) => encoded_data,
				None => return Some(vec![]),
			};

		match Decode::decode(&mut &encoded_last_inspected_ehd[..]) {
			Ok(last_inspected_ehd) => Some(last_inspected_ehd),
			Err(err) => {
				log::error!(
					"🗄️  Error occured while decoding last inspected ehds in cluster_id: {:?} {:?}",
					cluster_id,
					err
				);
				None
			},
		}
	}

	impl<T: Config> sp_application_crypto::BoundToRuntimeAppPublic for Pallet<T> {
		type Public = T::AuthorityId;
	}

	impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
		type Key = T::AuthorityId;

		#[allow(clippy::multiple_bound_locations)]
		fn on_genesis_session<'a, I: 'a>(validators: I)
		where
			I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
		{
			log::info!("🙌Adding Validator from genesis session.");
			let validators = validators
				.map(|(_, k)| T::AccountId::decode(&mut &k.into().encode()[..]).unwrap())
				.collect::<Vec<_>>();

			// only active validators in session - this is NOT all the validators
			ValidatorSet::<T>::put(validators);
		}

		#[allow(clippy::multiple_bound_locations)]
		fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_authorities: I)
		where
			I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
		{
			log::info!("🙌Adding Validator from new session.");
			let validators = validators
				.map(|(_, k)| T::AccountId::decode(&mut &k.into().encode()[..]).unwrap())
				.collect::<Vec<_>>();
			log::info!("🙌Total validator from new session. {:?}", validators.len());
			ValidatorSet::<T>::put(validators);
		}

		fn on_disabled(_i: u32) {}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		// the first key is 'verification_key' that is used to sign inspection receipts
		// the second key is 'stash_key' that should be used in staking by this validator
		pub validators: Vec<(T::AccountId, T::AccountId)>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { validators: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T>
	where
		T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
	{
		fn build(&self) {
			for (verification_key, stash_key) in &self.validators {
				// <ValidatorSet<T>> should be updated in 'on_genesis_session' handler
				if <ValidatorSet<T>>::get().contains(verification_key) {
					ValidatorToStashKey::<T>::insert(verification_key, stash_key);
				}
			}
		}
	}
}

pub struct NoReceiptsInterceptor;
impl InspReceiptsInterceptor for NoReceiptsInterceptor {
	type Receipt = HashedInspPathReceipt;

	fn intercept(receipts: Vec<HashedInspPathReceipt>) -> Vec<HashedInspPathReceipt> {
		receipts
	}
}
