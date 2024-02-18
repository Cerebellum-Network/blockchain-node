#![allow(clippy::all)]
// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::{ValueQuery, *},
	traits::Get,
};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
pub use pallet::*;
use sp_core::U256;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

mod mock;
mod tests;

type TokenId = U256;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub struct Erc721Token {
	pub id: TokenId,
	pub metadata: Vec<u8>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Some identifier for this token type, possibly the originating ethereum address.
		/// This is not explicitly used for anything, but may reflect the bridge's notion of
		/// resource ID.
		type Identifier: Get<[u8; 32]>;
	}

	/// Maps tokenId to Erc721 object
	#[pallet::storage]
	#[pallet::getter(fn tokens)]
	pub type Tokens<T: Config> = StorageMap<_, Blake2_128Concat, TokenId, Erc721Token>;

	/// Maps tokenId to owner
	#[pallet::storage]
	#[pallet::getter(fn owner_of)]
	pub type TokenOwner<T: Config> = StorageMap<_, Blake2_128Concat, TokenId, T::AccountId>;

	#[pallet::type_value]
	pub fn DefaultTokenCount<T: Config>() -> U256 {
		U256::zero()
	}

	#[pallet::storage]
	#[pallet::getter(fn token_count)]
	pub type TokenCount<T: Config> =
		StorageValue<Value = U256, QueryKind = ValueQuery, OnEmpty = DefaultTokenCount<T>>;

	#[pallet::error]
	pub enum Error<T> {
		/// ID not recognized
		TokenIdDoesNotExist,
		/// Already exists with an owner
		TokenAlreadyExists,
		/// Origin is not owner
		NotOwner,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New token created
		Minted(T::AccountId, TokenId),
		/// Token transfer between two parties
		Transferred(T::AccountId, T::AccountId, TokenId),
		/// Token removed from the system
		Burned(TokenId),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a new token with the given token ID and metadata, and gives ownership to owner
		#[pallet::call_index(0)]
		#[pallet::weight(195_000_000)]
		pub fn mint(
			origin: OriginFor<T>,
			owner: T::AccountId,
			id: TokenId,
			metadata: Vec<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::mint_token(owner, id, metadata)?;

			Ok(())
		}

		/// Changes ownership of a token sender owns
		#[pallet::call_index(1)]
		#[pallet::weight(195_000_000)]
		pub fn transfer(origin: OriginFor<T>, to: T::AccountId, id: TokenId) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::transfer_from(sender, to, id)?;

			Ok(())
		}

		/// Remove token from the system
		#[pallet::call_index(2)]
		#[pallet::weight(195_000_000)]
		pub fn burn(origin: OriginFor<T>, id: TokenId) -> DispatchResult {
			ensure_root(origin)?;

			let owner = Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;

			Self::burn_token(owner, id)?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Creates a new token in the system.
		pub fn mint_token(owner: T::AccountId, id: TokenId, metadata: Vec<u8>) -> DispatchResult {
			ensure!(!Tokens::<T>::contains_key(id), Error::<T>::TokenAlreadyExists);

			let new_token = Erc721Token { id, metadata };

			<Tokens<T>>::insert(id, new_token);
			<TokenOwner<T>>::insert(id, owner.clone());
			let new_total = <TokenCount<T>>::get().saturating_add(U256::one());
			<TokenCount<T>>::put(new_total);

			Self::deposit_event(Event::Minted(owner, id));

			Ok(())
		}

		/// Modifies ownership of a token
		pub fn transfer_from(from: T::AccountId, to: T::AccountId, id: TokenId) -> DispatchResult {
			// Check from is owner and token exists
			let owner = Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;
			ensure!(owner == from, Error::<T>::NotOwner);
			// Update owner
			<TokenOwner<T>>::insert(id, to.clone());

			Self::deposit_event(Event::Transferred(from, to, id));

			Ok(())
		}

		/// Deletes a token from the system.
		pub fn burn_token(from: T::AccountId, id: TokenId) -> DispatchResult {
			let owner = Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;
			ensure!(owner == from, Error::<T>::NotOwner);

			<Tokens<T>>::remove(id);
			<TokenOwner<T>>::remove(id);
			let new_total = <TokenCount<T>>::get().saturating_sub(U256::one());
			<TokenCount<T>>::put(new_total);

			Self::deposit_event(Event::Burned(id));

			Ok(())
		}
	}
}
