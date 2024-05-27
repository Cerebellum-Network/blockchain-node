//! # DDC Verification Pallet
//!
//! The DDC Verification pallet is used to validate zk-SNARK Proof and Signature
//!
//! - [`Call`]
//! - [`Pallet`]

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use ddc_primitives::{BatchIndex, ClusterId, DdcEra, MmrRootHash};
use frame_support::{pallet_prelude::*, traits::OneSessionHandler};
use frame_system::{
	offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer},
	pallet_prelude::*,
};
pub use pallet::*;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::crypto::KeyTypeId;
use sp_runtime::Percent;
use sp_std::prelude::*;

pub mod weights;
use crate::weights::WeightInfo;

const KEY_TYPE: KeyTypeId = KeyTypeId(*b"cer!");
pub mod sr25519 {
	mod app_sr25519 {
		use sp_application_crypto::{app_crypto, sr25519};

		use crate::KEY_TYPE;
		app_crypto!(sr25519, KEY_TYPE);
	}

	sp_application_crypto::with_pair! {
		pub type AuthorityPair = app_sr25519::Pair;
	}
	pub type AuthoritySignature = app_sr25519::Signature;
	pub type AuthorityId = app_sr25519::Public;
}
pub mod crypto {
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};

	use super::KEY_TYPE;
	app_crypto!(sr25519, KEY_TYPE);

	pub struct OffchainIdentifierId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for OffchainIdentifierId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for OffchainIdentifierId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::PalletId;

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
		frame_support::traits::StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type MaxVerificationKeyLimit: Get<u32>;
		type WeightInfo: WeightInfo;
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;
		type AuthorityIdParameter: Parameter
			+ From<sp_core::sr25519::Public>
			+ Into<Self::AuthorityId>
			+ MaxEncodedLen;
		type OffchainIdentifierId: AppCrypto<Self::Public, Self::Signature>;
		const MAJORITY: u8;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		BillingReportCreated { cluster_id: ClusterId, era: DdcEra },
		VerificationKeyStored { verification_key: Vec<u8> },
		PayoutBatchCreated { cluster_id: ClusterId, era: DdcEra },
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		BillingReportAlreadyExist,
		BadVerificationKey,
		BadRequest,
		NotAValidator,
		AlreadySigned,
	}

	#[pallet::storage]
	#[pallet::getter(fn active_billing_reports)]
	pub type ActiveBillingReports<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ClusterId, Blake2_128Concat, DdcEra, ReceiptParams>;

	#[pallet::storage]
	#[pallet::getter(fn payout_batch)]
	pub type PayoutBatch<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		DdcEra,
		PayoutData<T::Hash>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn payout_validators)]
	pub type PayoutValidators<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ClusterId,
		Blake2_128Concat,
		DdcEra,
		Vec<T::AuthorityId>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn verification_key)]
	pub type VerificationKey<T: Config> =
		StorageValue<_, BoundedVec<u8, T::MaxVerificationKeyLimit>>;

	#[pallet::storage]
	#[pallet::getter(fn validator_set)]
	pub type ValidatorSet<T: Config> = StorageValue<_, Vec<T::AuthorityId>>;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	pub struct ReceiptParams {
		pub merkle_root_hash: MmrRootHash,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(Hash))]
	pub struct PayoutData<Hash> {
		pub batch_index: BatchIndex,
		pub hash: Hash,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(_block_number: BlockNumberFor<T>) {
			log::info!("Hello from pallet-ocw.");

			let signer = Signer::<T, T::OffchainIdentifierId>::all_accounts();
			if !signer.can_sign() {
				log::error!("No local accounts available");
				return
			}

			let results =
				signer.send_signed_transaction(|_account| Call::set_validate_payout_batch {
					cluster_id: Default::default(),
					era: DdcEra::default(),
					payout_data: PayoutData {
						batch_index: BatchIndex::default(),
						hash: T::Hash::default(),
					},
				});

			for (acc, res) in &results {
				match res {
					Ok(()) => log::info!("[{:?}] Submitted response", acc.id),
					Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
				}
			}
		}
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_billing_reports())]
		pub fn create_billing_reports(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			merkle_root_hash: MmrRootHash,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			ensure!(
				ActiveBillingReports::<T>::get(cluster_id, era).is_none(),
				Error::<T>::BillingReportAlreadyExist
			);

			let receipt_params = ReceiptParams { merkle_root_hash };

			ActiveBillingReports::<T>::insert(cluster_id, era, receipt_params);

			Self::deposit_event(Event::<T>::BillingReportCreated { cluster_id, era });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::create_billing_reports())]
		pub fn set_verification_key(
			origin: OriginFor<T>,
			verification_key: Vec<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let bounded_verification_key: BoundedVec<u8, T::MaxVerificationKeyLimit> =
				verification_key
					.clone()
					.try_into()
					.map_err(|_| Error::<T>::BadVerificationKey)?;

			VerificationKey::<T>::put(bounded_verification_key);
			Self::deposit_event(Event::<T>::VerificationKeyStored { verification_key });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::create_billing_reports())]
		pub fn set_validate_payout_batch(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			era: DdcEra,
			payout_data: PayoutData<T::Hash>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let account_bytes: [u8; 32] = Self::account_to_bytes(&who)?;
			let who: T::AuthorityIdParameter =
				sp_application_crypto::sr25519::Public::from_raw(account_bytes).into();
			let who: T::AuthorityId = who.into();
			let authorities = <ValidatorSet<T>>::get().unwrap();

			ensure!(authorities.contains(&who.clone()), Error::<T>::NotAValidator);

			ensure!(
				!<PayoutValidators<T>>::get(cluster_id, era).contains(&who.clone()),
				Error::<T>::AlreadySigned
			);

			<PayoutValidators<T>>::try_mutate(cluster_id, era, |validators| -> DispatchResult {
				validators.push(who);
				Ok(())
			})?;

			let p = Percent::from_percent(T::MAJORITY);
			let threshold = p * authorities.len();

			let signed_validators = <PayoutValidators<T>>::get(cluster_id, era);

			if threshold < signed_validators.len() {
				PayoutBatch::<T>::insert(cluster_id, era, payout_data);
				Self::deposit_event(Event::<T>::PayoutBatchCreated { cluster_id, era });
			}

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	// This function converts a 32 byte AccountId to its byte-array equivalent form.
	fn account_to_bytes<AccountId>(account: &AccountId) -> Result<[u8; 32], DispatchError>
	where
		AccountId: Encode,
	{
		let account_vec = account.encode();
		ensure!(account_vec.len() == 32, "AccountId must be 32 bytes.");
		let mut bytes = [0u8; 32];
		bytes.copy_from_slice(&account_vec);
		Ok(bytes)
	}
}
impl<T: Config> sp_application_crypto::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::AuthorityId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::AuthorityId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
	{
		let validators = validators.map(|(_, k)| k).collect::<Vec<_>>();

		ValidatorSet::<T>::put(validators);
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_authorities: I)
	where
		I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
	{
		let validators = validators.map(|(_, k)| k).collect::<Vec<_>>();
		ValidatorSet::<T>::put(validators);
	}

	fn on_disabled(_i: u32) {}
}
