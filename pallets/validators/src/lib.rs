//! # DDC Nodes Pallet
//!
//! The Validators pallet is a temporary pallet used to recover from the incident in Mainnet
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use frame_support::pallet_prelude::*;
pub use pallet::*;
use pallet_session::SessionManager;
use sp_std::prelude::*;
pub mod weights;
use hex_literal::hex;

use crate::weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::dispatch::DispatchResult;
	use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};

	use super::*;

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
		frame_support::traits::StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		ValidatorAdded { validators: Vec<T::AccountId> },
	}

	#[pallet::error]
	pub enum Error<T> {
		NodeAlreadyExists,
	}

	#[pallet::storage]
	#[pallet::getter(fn validators)]
	pub(crate) type Validators<T: Config> =
		StorageValue<_, <T as frame_system::Config>::AccountId, OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::add_validators())]
		pub fn add_validators(
			origin: OriginFor<T>,
			validators: Vec<T::AccountId>,
		) -> DispatchResult {
			// let caller_id = ensure_signed(origin)?;
			// let node = Node::<T>::new(node_pub_key.clone(), caller_id, node_params)
			// 	.map_err(Into::<Error<T>>::into)?;
			// Self::create(node).map_err(Into::<Error<T>>::into)?;
			// Self::deposit_event(Event::<T>::NodeCreated { node_pub_key });
			Ok(())
		}
	}

	impl<T: frame_system::Config> SessionManager<T::AccountId> for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	{
		fn new_session(_new_index: sp_staking::SessionIndex) -> Option<Vec<T::AccountId>> {
			const VALIDATOR1: [u8; 32] =
				hex!("6ca3a3f6a78889ed70a6b46c2d621afcd3da2ea68e20a2eddd6f095e7ded586d");
			const VALIDATOR2: [u8; 32] =
				hex!("fa63378688e615e71b10ddb392482076b4e639c9d31181f370fe0858b8db7006");
			const VALIDATOR3: [u8; 32] =
				hex!("9e0e0270982a25080e436f7de803f06ed881b15209343c0dd16984dcae267406");
			const VALIDATOR4: [u8; 32] =
				hex!("0634cd2127a7bd444f7d004f78fa6ba771faa62991fa3f138c59926fd8cd971c");

			Some(vec![VALIDATOR1.into(), VALIDATOR2.into(), VALIDATOR3.into(), VALIDATOR4.into()])
		}

		fn end_session(_end_index: sp_staking::SessionIndex) {
			// Do nothing
		}

		fn start_session(_start_index: sp_staking::SessionIndex) {
			// Do nothing
		}
	}
}
