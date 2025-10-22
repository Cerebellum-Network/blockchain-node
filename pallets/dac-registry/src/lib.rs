//! # DAC Registry Pallet
//!
//! A pallet for registering and managing versions of DAC (Data Aggregation Component) WASM modules.
//! This pallet enables governance to add new DAC versions without runtime upgrades, manage version
//! lifecycles, and maintain transparent, auditable history of all DAC versions.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use crate::weights::WeightInfo;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_core::H256;
	use sp_runtime::SaturatedConversion;
	use sp_std::vec::Vec;

	/// Type alias for code hash
	pub type CodeHash = H256;

	/// Type alias for API version (major.minor)
	pub type ApiVersion = (u32, u32);

	/// Type alias for semantic version (major.minor.patch)
	pub type SemVer = (u32, u32, u32);

	/// Type alias for WASM code bytes
	pub type WasmCodeBytes = Vec<u8>;

	/// Metadata for a DAC version
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct CodeMeta {
		/// API version (major.minor)
		pub api_version: ApiVersion,
		/// Semantic version (major.minor.patch)
		pub semver: SemVer,
		/// Block number when this version becomes active
		pub allowed_from: u64,
		/// Size of the WASM binary in bytes
		pub length: u32,
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Governance origin for DAC registry operations
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Maximum size of a WASM code in bytes
		#[pallet::constant]
		type MaxCodeSize: Get<u32>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	/// Storage for DAC code metadata by hash
	#[pallet::storage]
	#[pallet::getter(fn code_by_hash)]
	pub type CodeByHash<T: Config> =
		StorageMap<_, Blake2_128Concat, CodeHash, CodeMeta, OptionQuery>;

	/// Storage for WASM code by hash
	#[pallet::storage]
	#[pallet::getter(fn wasm_code)]
	pub type WasmCode<T: Config> =
		StorageMap<_, Blake2_128Concat, CodeHash, BoundedVec<u8, T::MaxCodeSize>, OptionQuery>;

	/// Storage for deregistered code hashes
	#[pallet::storage]
	#[pallet::getter(fn deregistered)]
	pub type Deregistered<T: Config> = StorageMap<_, Blake2_128Concat, CodeHash, bool, ValueQuery>;

	/// Pallets use events to inform users when important changes are made.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new DAC version was successfully registered.
		/// [code_hash, api_version, semver, length]
		CodeRegistered { code_hash: CodeHash, api_version: ApiVersion, semver: SemVer, length: u32 },
		/// Metadata of a DAC version was updated.
		/// [code_hash, api_version, semver, allowed_from]
		CodeMetaUpdated {
			code_hash: CodeHash,
			api_version: ApiVersion,
			semver: SemVer,
			allowed_from: u64,
		},
		/// DAC version marked as inactive or deprecated.
		/// [code_hash]
		CodeDeregistered { code_hash: CodeHash },
	}

	/// Errors that can occur in this pallet.
	#[pallet::error]
	pub enum Error<T> {
		/// Code already exists with this hash
		CodeAlreadyExists,
		/// Code does not exist
		CodeNotFound,
		/// Code is too large
		CodeTooLarge,
		/// Code is deregistered and cannot be used
		CodeDeregistered,
		/// Invalid API version format
		InvalidApiVersion,
		/// Invalid semantic version format
		InvalidSemVer,
		/// Activation block is in the past
		InvalidActivationBlock,
		/// Code is empty
		EmptyCode,
	}

	/// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register a new DAC version on-chain.
		///
		/// This function allows governance to register a new DAC WASM module with its metadata.
		/// The code hash is computed from the WASM binary and used as the storage key.
		///
		/// # Parameters
		/// - `origin`: Must be governance origin
		/// - `code`: The WASM binary code
		/// - `api_version`: API version (major.minor)
		/// - `semver`: Semantic version (major.minor.patch)
		/// - `allowed_from`: Block number when this version becomes active
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::register_code(code.len() as u32))]
		pub fn register_code(
			origin: OriginFor<T>,
			code: Vec<u8>,
			api_version: ApiVersion,
			semver: SemVer,
			allowed_from: BlockNumberFor<T>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Validate code
			ensure!(!code.is_empty(), Error::<T>::EmptyCode);
			ensure!(code.len() as u32 <= T::MaxCodeSize::get(), Error::<T>::CodeTooLarge);

			// Validate API version
			ensure!(api_version.0 > 0 || api_version.1 > 0, Error::<T>::InvalidApiVersion);

			// Validate semantic version
			ensure!(semver.0 > 0 || semver.1 > 0 || semver.2 > 0, Error::<T>::InvalidSemVer);

			// Validate activation block
			ensure!(
				allowed_from > frame_system::Pallet::<T>::block_number(),
				Error::<T>::InvalidActivationBlock
			);

			// Compute code hash
			let code_hash = sp_io::hashing::blake2_256(&code);
			let code_hash = CodeHash::from_slice(&code_hash);

			// Check if code already exists
			ensure!(!CodeByHash::<T>::contains_key(code_hash), Error::<T>::CodeAlreadyExists);

			// Create metadata
			let meta = CodeMeta {
				api_version,
				semver,
				allowed_from: allowed_from.saturated_into(),
				length: code.len() as u32,
			};

			// Convert Vec<u8> to BoundedVec
			let bounded_code = BoundedVec::<u8, T::MaxCodeSize>::try_from(code)
				.map_err(|_| Error::<T>::CodeTooLarge)?;

			// Store code and metadata
			WasmCode::<T>::insert(code_hash, bounded_code);
			CodeByHash::<T>::insert(code_hash, meta.clone());

			// Emit event
			Self::deposit_event(Event::CodeRegistered {
				code_hash,
				api_version: meta.api_version,
				semver: meta.semver,
				length: meta.length,
			});

			Ok(())
		}

		/// Update metadata for an existing DAC version.
		///
		/// This function allows governance to update the metadata of an existing DAC version
		/// without changing the stored WASM code.
		///
		/// # Parameters
		/// - `origin`: Must be governance origin
		/// - `code_hash`: Hash of the code to update
		/// - `api_version`: New API version (major.minor)
		/// - `semver`: New semantic version (major.minor.patch)
		/// - `allowed_from`: New activation block
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::update_meta())]
		pub fn update_meta(
			origin: OriginFor<T>,
			code_hash: CodeHash,
			api_version: ApiVersion,
			semver: SemVer,
			allowed_from: BlockNumberFor<T>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Check if code exists
			let old_meta = CodeByHash::<T>::get(code_hash).ok_or(Error::<T>::CodeNotFound)?;

			// Check if code is deregistered
			ensure!(!Deregistered::<T>::get(code_hash), Error::<T>::CodeDeregistered);

			// Validate API version
			ensure!(api_version.0 > 0 || api_version.1 > 0, Error::<T>::InvalidApiVersion);

			// Validate semantic version
			ensure!(semver.0 > 0 || semver.1 > 0 || semver.2 > 0, Error::<T>::InvalidSemVer);

			// Validate activation block
			ensure!(
				allowed_from > frame_system::Pallet::<T>::block_number(),
				Error::<T>::InvalidActivationBlock
			);

			// Create new metadata
			let new_meta = CodeMeta {
				api_version,
				semver,
				allowed_from: allowed_from.saturated_into(),
				length: old_meta.length, // Keep original length
			};

			// Update metadata
			CodeByHash::<T>::insert(code_hash, new_meta.clone());

			// Emit event
			Self::deposit_event(Event::CodeMetaUpdated {
				code_hash,
				api_version: new_meta.api_version,
				semver: new_meta.semver,
				allowed_from: new_meta.allowed_from,
			});

			Ok(())
		}

		/// Mark an existing DAC version as inactive or deprecated.
		///
		/// This function prevents further use of a DAC version by DDC clusters while
		/// retaining metadata for auditability.
		///
		/// # Parameters
		/// - `origin`: Must be governance origin
		/// - `code_hash`: Hash of the code to deregister
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::deregister_code())]
		pub fn deregister_code(origin: OriginFor<T>, code_hash: CodeHash) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Check if code exists
			ensure!(CodeByHash::<T>::contains_key(code_hash), Error::<T>::CodeNotFound);

			// Check if already deregistered
			ensure!(!Deregistered::<T>::get(code_hash), Error::<T>::CodeDeregistered);

			// Mark as deregistered
			Deregistered::<T>::insert(code_hash, true);

			// Emit event
			Self::deposit_event(Event::CodeDeregistered { code_hash });

			Ok(())
		}
	}

	/// Public interface for querying DAC registry
	impl<T: Config> Pallet<T> {
		/// Get the WASM code for a given code hash
		pub fn get_code(code_hash: CodeHash) -> Option<BoundedVec<u8, T::MaxCodeSize>> {
			// Check if code is deregistered
			if Deregistered::<T>::get(code_hash) {
				return None;
			}
			WasmCode::<T>::get(code_hash)
		}

		/// Get the metadata for a given code hash
		pub fn get_metadata(code_hash: CodeHash) -> Option<CodeMeta> {
			// Check if code is deregistered
			if Deregistered::<T>::get(code_hash) {
				return None;
			}
			CodeByHash::<T>::get(code_hash)
		}

		/// Check if a code hash is active (exists and not deregistered)
		pub fn is_code_active(code_hash: CodeHash) -> bool {
			CodeByHash::<T>::contains_key(code_hash) && !Deregistered::<T>::get(code_hash)
		}

		/// Check if a code is ready for use (active and past activation block)
		pub fn is_code_ready(code_hash: CodeHash) -> bool {
			if !Self::is_code_active(code_hash) {
				return false;
			}

			if let Some(meta) = CodeByHash::<T>::get(code_hash) {
				frame_system::Pallet::<T>::block_number() >= meta.allowed_from.saturated_into()
			} else {
				false
			}
		}
	}
}
