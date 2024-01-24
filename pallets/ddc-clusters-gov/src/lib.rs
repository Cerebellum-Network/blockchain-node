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
		schedule::DispatchTime, Bounded, Currency, EnsureOriginWithArg, LockableCurrency,
		OriginTrait, StorePreimage, UnfilteredDispatchable,
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

	/// The current storage version.
	const STORAGE_VERSION: frame_support::traits::StorageVersion =
		frame_support::traits::StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_referenda::Config<I> {
		type PalletId: Get<PalletId>;
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		type WeightInfo: WeightInfo;

		type ClusterProposalCall: Parameter
			+ From<Call<Self, I>>
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ IsType<<Self as pallet_referenda::Config<I>>::RuntimeCall>;

		type ClusterVisitor: ClusterVisitor<Self>;
		type ClusterGovOrigin: GetDdcOrigin<Self>;
		type ClusterProposalDuration: Get<Self::BlockNumber>;
		type ClusterMaxProposals: Get<ProposalIndex>;
		type ClusterActivatorOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		type ClusterAdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	#[pallet::storage]
	#[pallet::getter(fn proposal_of)]
	pub type ClusterProposal<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Identity, ClusterId, T::ClusterProposalCall, OptionQuery>;

	/// Votes on a given cluster proposal, if it is ongoing.
	#[pallet::storage]
	#[pallet::getter(fn voting)]
	pub type ClusterProposalVoting<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Identity, ClusterId, Votes<T::AccountId, T::BlockNumber>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
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
		ClusterActivated {
			cluster_id: ClusterId,
		},
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
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
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::call_index(0)]
		#[pallet::weight(10_000)]
		pub fn propose_activate_cluster(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			// _cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			let cluster_manager_id = T::ClusterVisitor::get_manager_account_id(&cluster_id)
				.map_err(|_| Error::<T, I>::NoCluster)?;

			ensure!(cluster_manager_id == caller_id, Error::<T, I>::NotClusterManager);

			// todo: calculate the threshold based on number of validated nodes
			let threshold = 64u32;
			let votes = {
				let end =
					frame_system::Pallet::<T>::block_number() + T::ClusterProposalDuration::get();
				Votes { threshold, ayes: vec![], nays: vec![], end }
			};
			let proposal: <T as Config<I>>::ClusterProposalCall =
				T::ClusterProposalCall::from(Call::<T, I>::activate_cluster { cluster_id });

			<ClusterProposal<T, I>>::insert(cluster_id, proposal);
			<ClusterProposalVoting<T, I>>::insert(cluster_id, votes);
			Self::deposit_event(Event::Proposed { account: caller_id, cluster_id, threshold });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10_000)]
		pub fn execute_proposal(origin: OriginFor<T>, cluster_id: ClusterId) -> DispatchResult {
			let _caller_id = ensure_signed(origin)?;

			// todo: check the local consensus on proposal
			Self::propose_public(cluster_id)?;

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(10_000)]
		pub fn activate_cluster(origin: OriginFor<T>, cluster_id: ClusterId) -> DispatchResult {
			T::ClusterActivatorOrigin::ensure_origin(origin)?;

			// todo: activate cluster and update its economic
			Self::deposit_event(Event::ClusterActivated { cluster_id });

			Ok(())
		}
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		pub fn propose_public(cluster_id: ClusterId) -> DispatchResult {
			let proposal = <ClusterProposal<T, I>>::try_get(cluster_id)
				.map_err(|_| Error::<T, I>::ProposalMissing)?;

			let call: <T as pallet_referenda::Config<I>>::RuntimeCall = proposal.into();
			let bounded_call =
				T::Preimages::bound(call).map_err(|_| Error::<T, I>::ProposalMissing)?;

			let cluster_gov_origin = T::ClusterGovOrigin::get();
			let pallets_origin: <T::RuntimeOrigin as OriginTrait>::PalletsOrigin =
				cluster_gov_origin.caller().clone();
			let referenda_call = pallet_referenda::Call::<T, I>::submit {
				proposal_origin: Box::new(pallets_origin),
				proposal: bounded_call,
				enactment_moment: DispatchTime::After(T::BlockNumber::from(1u32)),
			};

			referenda_call
				.dispatch_bypass_filter(frame_system::RawOrigin::Signed(Self::account_id()).into())
				.map(|_| ())
				.map_err(|e| e.error)?;

			Ok(())
		}
	}
}
