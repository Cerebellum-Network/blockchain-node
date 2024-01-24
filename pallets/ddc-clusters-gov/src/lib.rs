//! # DDC Nodes Pallet
//!
//! The DDC Clusters pallet is used to manage DDC Clusters
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## GenesisConfig
//!
//! The DDC Clusters pallet depends on the [`GenesisConfig`]. The
//! `GenesisConfig` is optional and allow to set some initial nodes in DDC.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]
#![feature(is_some_and)] // ToDo: delete at rustc > 1.70

pub mod weights;
use ddc_primitives::{ClusterGovParams, ClusterId};
use ddc_traits::{
	cluster::ClusterVisitor,
	pallet::{GetDdcOrigin, PalletsOriginOf},
};
use frame_support::{
	codec::{Decode, Encode, MaxEncodedLen},
	dispatch::{
		DispatchError, DispatchResultWithPostInfo, Dispatchable, GetDispatchInfo, Pays,
		PostDispatchInfo,
	},
	pallet_prelude::*,
	traits::{
		Currency, EnsureOriginWithArg, LockableCurrency, OriginTrait, UnfilteredDispatchable,
	},
};
use frame_system::pallet_prelude::*;
pub use frame_system::Config as SysConfig;
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{traits::AccountIdConversion, RuntimeDebug};
use sp_std::prelude::*;

pub type ProposalIndex = u32;
pub type MemberCount = u32;

use crate::weights::WeightInfo;

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Info for keeping track of a motion being voted on.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Votes<AccountId, BlockNumber> {
	/// The number of approval votes that are needed to pass the motion.
	threshold: MemberCount,
	/// The current set of voters that approved it.
	ayes: Vec<AccountId>,
	/// The current set of voters that rejected it.
	nays: Vec<AccountId>,
	/// The hard end time of this vote.
	end: BlockNumber,
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::PalletId;

	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type PalletId: Get<PalletId>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		type WeightInfo: WeightInfo;
		// todo: move to pallet_referenda
		type SubmitOrigin: EnsureOriginWithArg<
			Self::RuntimeOrigin,
			PalletsOriginOf<Self>,
			Success = Self::AccountId,
		>;

		type ClusterProposalCall: Parameter
			+ From<Call<Self>>
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>;
		type ClusterVisitor: ClusterVisitor<Self>;
		type ClusterGovCreatorOrigin: GetDdcOrigin<Self>;
		type ClusterProposalDuration: Get<Self::BlockNumber>;
		type ClusterMaxProposals: Get<ProposalIndex>;
	}

	#[pallet::storage]
	#[pallet::getter(fn proposal_of)]
	pub type ClusterProposal<T: Config> =
		StorageMap<_, Identity, ClusterId, T::ClusterProposalCall, OptionQuery>;

	/// Votes on a given proposal, if it is ongoing.
	#[pallet::storage]
	#[pallet::getter(fn voting)]
	pub type ClusterProposalVoting<T: Config> =
		StorageMap<_, Identity, ClusterId, Votes<T::AccountId, T::BlockNumber>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		GenericEvent,
		/// A motion (given hash) has been proposed (by given account) with a threshold (given
		/// `MemberCount`).
		Proposed {
			account: T::AccountId,
			cluster_id: ClusterId,
			threshold: MemberCount,
		},
		/// A motion (given hash) has been voted on by given account, leaving
		/// a tally (yes votes and no votes given respectively as `MemberCount`).
		Voted {
			account: T::AccountId,
			proposal_hash: T::Hash,
			voted: bool,
			yes: MemberCount,
			no: MemberCount,
		},
		/// A motion was approved by the required threshold.
		Approved {
			proposal_hash: T::Hash,
		},
		/// A motion was not approved by the required threshold.
		Disapproved {
			proposal_hash: T::Hash,
		},
		/// A motion was executed; result will be `Ok` if it returned without error.
		Executed {
			proposal_hash: T::Hash,
			result: DispatchResult,
		},
		/// A proposal was closed because its threshold was reached or after its duration was up.
		Closed {
			proposal_hash: T::Hash,
			yes: MemberCount,
			no: MemberCount,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		GenericError,
		/// Account is not a member
		NotMember,
		/// Account is not a cluster manager
		NotClusterManager,
		/// Cluster does not exist
		NoCluster,
		/// Proposal must exist
		ProposalMissing,
		/// Duplicate vote ignored
		DuplicateVote,
		/// The close call was made too early, before the end of the voting.
		TooEarly,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(10_000)]
		pub fn submit_public(
			origin: OriginFor<T>,
			proposal_origin: Box<PalletsOriginOf<T>>,
		) -> DispatchResult {
			let _caller_id = T::SubmitOrigin::ensure_origin(origin, &proposal_origin)?;
			Self::deposit_event(Event::<T>::GenericEvent);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10_000)]
		pub fn submit_via_internal(
			origin: OriginFor<T>,
			proposal_origin: Box<PalletsOriginOf<T>>,
		) -> DispatchResult {
			let _caller_id = ensure_signed(origin)?;
			let call = Call::<T>::submit_public { proposal_origin };
			call.dispatch_bypass_filter(frame_system::RawOrigin::Signed(Self::account_id()).into())
				.map(|_| ())
				.map_err(|e| e.error)?;

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(10_000)]
		pub fn submit_internal(origin: OriginFor<T>) -> DispatchResult {
			let _caller_id = ensure_signed(origin)?;

			// let origin: T::RuntimeOrigin = frame_system::RawOrigin::Signed(_caller_id).into();
			// let pallets_origin: <T::RuntimeOrigin as
			// frame_support::traits::OriginTrait>::PalletsOrigin = origin.caller().clone();
			// let call = Call::<T>::submit_public { proposal_origin: Box::new(pallets_origin) };
			// call.dispatch_bypass_filter(frame_system::RawOrigin::Signed(Self::account_id()).
			// into()) 	.map(|_| ())
			// 	.map_err(|e| e.error)?;

			let origin2 = T::ClusterGovCreatorOrigin::get();
			let pallets_origin2: <T::RuntimeOrigin as
			frame_support::traits::OriginTrait>::PalletsOrigin = origin2.caller().clone();
			let call2 = Call::<T>::submit_public { proposal_origin: Box::new(pallets_origin2) };
			call2
				.dispatch_bypass_filter(frame_system::RawOrigin::Signed(Self::account_id()).into())
				// .dispatch_bypass_filter(
				// 	frame_system::RawOrigin::Signed(Self::sub_account_id(5u32)).into(),
				// )
				.map(|_| ())
				.map_err(|e| e.error)?;

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(10_000)]
		pub fn propose_activate_cluster(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			_cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			let cluster_manager_id = T::ClusterVisitor::get_manager_account_id(&cluster_id)
				.map_err(|_| Error::<T>::NoCluster)?;

			ensure!(cluster_manager_id == caller_id, Error::<T>::NotClusterManager);

			let threshold = 64u32;
			let votes = {
				let end =
					frame_system::Pallet::<T>::block_number() + T::ClusterProposalDuration::get();
				Votes { threshold, ayes: vec![], nays: vec![], end }
			};

			let creator_origin = T::ClusterGovCreatorOrigin::get();
			let pallets_origin: <T::RuntimeOrigin as OriginTrait>::PalletsOrigin =
				creator_origin.caller().clone();
			let call = T::ClusterProposalCall::from(Call::<T>::submit_public {
				proposal_origin: Box::new(pallets_origin),
			});

			<ClusterProposal<T>>::insert(cluster_id, call);
			<ClusterProposalVoting<T>>::insert(cluster_id, votes);
			Self::deposit_event(Event::Proposed { account: caller_id, cluster_id, threshold });

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(10_000)]
		pub fn execute_proposal(origin: OriginFor<T>, cluster_id: ClusterId) -> DispatchResult {
			let _caller_id = ensure_signed(origin)?;

			let call = <ClusterProposal<T>>::try_get(cluster_id)
				.map_err(|_| Error::<T>::ProposalMissing)?;

			call.dispatch(frame_system::RawOrigin::Signed(Self::account_id()).into())
				.map(|_| ())
				.map_err(|e| e.error)?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}
}
