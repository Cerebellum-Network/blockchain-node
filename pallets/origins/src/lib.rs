//! Custom origins for governance interventions.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::manual_inspect)]
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use cere_runtime_common::constants::currency::{Balance, DOLLARS, GRAND};
	use frame_support::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[derive(PartialEq, Eq, Clone, MaxEncodedLen, Encode, Decode, TypeInfo, RuntimeDebug)]
	#[pallet::origin]
	pub enum Origin {
		/// Origin able to cancel slashes and manage minimum commission.
		StakingAdmin,
		/// Origin for spending up to $10,000,000 DOT from the treasury as well as generally
		/// administering it.
		Treasurer,
		/// Origin for managing the registrar.
		GeneralAdmin,
		/// Origin able to cancel referenda.
		ReferendumCanceller,
		/// Origin able to kill referenda.
		ReferendumKiller,
		/// Origin able to spend around $250 from the treasury at once.
		SmallTipper,
		/// Origin able to spend around $1,000 from the treasury at once.
		BigTipper,
		/// Origin able to spend around $10,000 from the treasury at once.
		SmallSpender,
		/// Origin able to spend around $100,000 from the treasury at once.
		MediumSpender,
		/// Origin able to spend up to $1,000,000 DOT from the treasury at once.
		BigSpender,
		/// Origin able to dispatch a whitelisted call.
		WhitelistedCaller,
		/// Origin for activating clusters protocols.
		ClusterProtocolActivator,
		/// Origin for updating clusters protocols.
		ClusterProtocolUpdater,
	}

	macro_rules! decl_unit_ensures {
		( $name:ident: $success_type:ty = $success:expr ) => {
			pub struct $name;
			impl<O: Into<Result<Origin, O>> + From<Origin>>
				EnsureOrigin<O> for $name
			{
				type Success = $success_type;
				fn try_origin(o: O) -> Result<Self::Success, O> {
					o.into().and_then(|o| match o {
						Origin::$name => Ok($success),
						r => Err(O::from(r)),
					})
				}
				#[cfg(feature = "runtime-benchmarks")]
				fn try_successful_origin() -> Result<O, ()> {
					Ok(O::from(Origin::$name))
				}
			}
		};
		( $name:ident ) => { decl_unit_ensures! { $name : () = () } };
		( $name:ident: $success_type:ty = $success:expr, $( $rest:tt )* ) => {
			decl_unit_ensures! { $name: $success_type = $success }
			decl_unit_ensures! { $( $rest )* }
		};
		( $name:ident, $( $rest:tt )* ) => {
			decl_unit_ensures! { $name }
			decl_unit_ensures! { $( $rest )* }
		};
		() => {}
	}
	decl_unit_ensures!(
		StakingAdmin,
		Treasurer,
		GeneralAdmin,
		ReferendumCanceller,
		ReferendumKiller,
		WhitelistedCaller,
		ClusterProtocolActivator,
		ClusterProtocolUpdater,
	);

	macro_rules! decl_ensure {
		(
			$vis:vis type $name:ident: EnsureOrigin<Success = $success_type:ty> {
				$( $item:ident = $success:expr, )*
			}
		) => {
			$vis struct $name;
			impl<O: Into<Result<Origin, O>> + From<Origin>>
				EnsureOrigin<O> for $name
			{
				type Success = $success_type;
				fn try_origin(o: O) -> Result<Self::Success, O> {
					o.into().and_then(|o| match o {
						$(
							Origin::$item => Ok($success),
						)*
						r => Err(O::from(r)),
					})
				}
				#[cfg(feature = "runtime-benchmarks")]
				fn try_successful_origin() -> Result<O, ()> {
					// By convention the more privileged origins go later, so for greatest chance
					// of success, we want the last one.
					let _result: Result<O, ()> = Err(());
					$(
						let _result: Result<O, ()> = Ok(O::from(Origin::$item));
					)*
					_result
				}
			}
		}
	}

	decl_ensure! {
		pub type Spender: EnsureOrigin<Success = Balance> {
			SmallTipper = 250 * DOLLARS,
			BigTipper = GRAND,
			SmallSpender = 10 * GRAND,
			MediumSpender = 100 * GRAND,
			BigSpender = 1_000 * GRAND,
			Treasurer = 10_000 * GRAND,
		}
	}
}
