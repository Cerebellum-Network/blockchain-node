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
use ddc_primitives::{
	traits::{
		cluster::{ClusterCreator, ClusterEconomics, ClusterManager, ClusterQuery},
		cluster_gov::{DefaultVote, MemberCount},
		node::NodeVisitor,
		pallet::GetDdcOrigin,
	},
	ClusterGovParams, ClusterId, ClusterNodeStatus, ClusterStatus, NodePubKey,
};
use frame_support::{
	codec::{Decode, Encode},
	dispatch::{DispatchError, DispatchResult, Dispatchable, GetDispatchInfo, Pays, Weight},
	pallet_prelude::*,
	traits::{
		schedule::DispatchTime, Currency, LockableCurrency, OriginTrait, StorePreimage,
		UnfilteredDispatchable,
	},
};
use frame_system::pallet_prelude::*;
pub use frame_system::Config as SysConfig;
pub use pallet::*;
use scale_info::TypeInfo;
use sp_io::storage;
use sp_runtime::{traits::AccountIdConversion, RuntimeDebug};
use sp_std::prelude::*;
pub use weights::WeightInfo;

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Info for keeping track of a proposal being voted on.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Votes<AccountId, BlockNumber> {
	/// Proposal author
	author: AccountId,
	/// The number of approval votes that are needed to pass the proposal.
	threshold: MemberCount,
	/// The current set of voters that approved it.
	ayes: Vec<AccountId>,
	/// The current set of voters that rejected it.
	nays: Vec<AccountId>,
	/// Block at which proposal was inited.
	start: BlockNumber,
	/// The hard end time of this vote.
	end: BlockNumber,

	is_activation: bool, // todo: remove to make proposal generic
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum ClusterMember {
	ClusterManager,
	NodeProvider(NodePubKey),
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
	pub trait Config: frame_system::Config + pallet_referenda::Config {
		type PalletId: Get<PalletId>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		type WeightInfo: WeightInfo;
		type OpenGovActivatorTrackOrigin: GetDdcOrigin<Self>;
		type OpenGovActivatorOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		type OpenGovUpdaterTrackOrigin: GetDdcOrigin<Self>;
		type OpenGovUpdaterOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		type ClusterProposalDuration: Get<Self::BlockNumber>;
		type ClusterProposalCall: Parameter
			+ From<Call<Self>>
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ IsType<<Self as pallet_referenda::Config>::RuntimeCall>
			+ GetDispatchInfo;
		type ClusterCreator: ClusterCreator<Self, BalanceOf<Self>>;
		type ClusterManager: ClusterManager<Self>;
		type ClusterEconomics: ClusterEconomics<Self, BalanceOf<Self>>;
		type NodeVisitor: NodeVisitor<Self>;
		/// Default voting strategy.
		type DefaultVote: DefaultVote;
		type MinValidatedNodesCount: Get<u16>;
	}

	#[pallet::storage]
	#[pallet::getter(fn proposal_of)]
	pub type ClusterProposal<T: Config> =
		StorageMap<_, Identity, ClusterId, T::ClusterProposalCall, OptionQuery>;

	/// Votes on a given cluster proposal, if it is ongoing.
	#[pallet::storage]
	#[pallet::getter(fn voting)]
	pub type ClusterProposalVoting<T: Config> =
		StorageMap<_, Identity, ClusterId, Votes<T::AccountId, T::BlockNumber>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A proposal (given hash) has been proposed (by given account) with a threshold (given
		/// `MemberCount`).
		Proposed { account: T::AccountId, cluster_id: ClusterId, threshold: MemberCount },
		/// A proposal (given hash) has been voted on by given account, leaving
		/// a tally (yes votes and no votes given respectively as `MemberCount`).
		Voted {
			account: T::AccountId,
			cluster_id: ClusterId,
			voted: bool,
			yes: MemberCount,
			no: MemberCount,
		},
		/// A proposal was approved by the required threshold.
		Approved { cluster_id: ClusterId },
		/// A proposal was not approved by the required threshold.
		Disapproved { cluster_id: ClusterId },
		/// A proposal was executed; result will be `Ok` if it returned without error.
		Executed { cluster_id: ClusterId, result: DispatchResult },
		/// A proposal was closed because its threshold was reached or after its duration was up.
		Closed { cluster_id: ClusterId, yes: MemberCount, no: MemberCount },
		/// A proposal was not removed by its author.
		Removed { cluster_id: ClusterId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Account is not a member
		NotClusterMember,
		/// Account is not a cluster manager
		NotClusterManager,
		/// Account is not a member
		NotValidatedNode,
		/// Cluster does not exist
		NoCluster,
		/// Account is not proposal author
		NotProposalAuthor,
		/// Proposal must exist
		ProposalMissing,
		/// Active proposal is ongoing
		ActiveProposal,
		/// Duplicate vote ignored
		DuplicateVote,
		/// The close call was made too early, before the end of the voting.
		TooEarly,
		AwaitsValidation,
		NotEnoughValidatedNodes,
		UnexpectedState,
		VoteProhibited,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(10_000)]
		pub fn propose_activate_cluster(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			Self::ensure_cluster_manager(caller_id.clone(), cluster_id)?;

			ensure!(!<ClusterProposal<T>>::contains_key(cluster_id), Error::<T>::ActiveProposal);

			let cluster_status =
				<T::ClusterEconomics as ClusterQuery<T>>::get_cluster_status(&cluster_id)
					.map_err(|_| Error::<T>::NoCluster)?;
			ensure!(cluster_status == ClusterStatus::Inactive, Error::<T>::UnexpectedState);

			let cluster_nodes_stats = T::ClusterManager::get_nodes_stats(&cluster_id)
				.map_err(|_| Error::<T>::NoCluster)?;
			ensure!(cluster_nodes_stats.await_validation == 0, Error::<T>::AwaitsValidation);
			ensure!(
				cluster_nodes_stats.validation_succeeded >= T::MinValidatedNodesCount::get(),
				Error::<T>::NotEnoughValidatedNodes
			);

			// Collect votes from 100% of Validated Nodes + 1 vote from Cluster Manager
			let threshold = cluster_nodes_stats.validation_succeeded as u32 + 1;
			let votes = {
				let start = frame_system::Pallet::<T>::block_number();
				let end = start + T::ClusterProposalDuration::get();
				Votes {
					threshold,
					ayes: vec![],
					nays: vec![],
					start,
					end,
					author: caller_id.clone(),
					is_activation: true,
				}
			};
			let proposal: <T as Config>::ClusterProposalCall =
				T::ClusterProposalCall::from(Call::<T>::activate_cluster {
					cluster_id,
					cluster_gov_params,
				});

			<ClusterProposal<T>>::insert(cluster_id, proposal);
			<ClusterProposalVoting<T>>::insert(cluster_id, votes);
			Self::deposit_event(Event::Proposed { account: caller_id, cluster_id, threshold });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10_000)]
		pub fn propose_update_cluster(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
			member: ClusterMember,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			Self::ensure_validated_member(caller_id.clone(), cluster_id, member)?;

			ensure!(!<ClusterProposal<T>>::contains_key(cluster_id), Error::<T>::ActiveProposal);

			let cluster_status =
				<T::ClusterEconomics as ClusterQuery<T>>::get_cluster_status(&cluster_id)
					.map_err(|_| Error::<T>::NoCluster)?;
			ensure!(cluster_status == ClusterStatus::Active, Error::<T>::UnexpectedState);

			let cluster_nodes_stats = T::ClusterManager::get_nodes_stats(&cluster_id)
				.map_err(|_| Error::<T>::NoCluster)?;
			ensure!(cluster_nodes_stats.await_validation == 0, Error::<T>::AwaitsValidation);
			ensure!(
				cluster_nodes_stats.validation_succeeded >= T::MinValidatedNodesCount::get(),
				Error::<T>::NotEnoughValidatedNodes
			);

			// Collect votes from 100% of Validated Nodes + 1 vote from Cluster Manager
			let threshold = cluster_nodes_stats.validation_succeeded as u32 + 1;
			let votes = {
				let start = frame_system::Pallet::<T>::block_number();
				let end = start + T::ClusterProposalDuration::get();
				Votes {
					threshold,
					ayes: vec![],
					nays: vec![],
					start,
					end,
					author: caller_id.clone(),
					is_activation: false,
				}
			};
			let proposal: <T as Config>::ClusterProposalCall =
				T::ClusterProposalCall::from(Call::<T>::update_cluster {
					cluster_id,
					cluster_gov_params,
				});

			<ClusterProposal<T>>::insert(cluster_id, proposal);
			<ClusterProposalVoting<T>>::insert(cluster_id, votes);
			Self::deposit_event(Event::Proposed { account: caller_id, cluster_id, threshold });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(10_000)]
		pub fn vote_proposal(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			approve: bool,
			member: ClusterMember,
		) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			Self::ensure_allowed_voter(caller_id.clone(), cluster_id, member)?;
			let _ = Self::do_vote(caller_id, cluster_id, approve)?;
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(10_000)]
		pub fn close_proposal(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			member: ClusterMember,
		) -> DispatchResultWithPostInfo {
			let caller_id = ensure_signed(origin)?;
			Self::ensure_validated_member(caller_id.clone(), cluster_id, member)?;
			Self::do_close(cluster_id)?
		}

		#[pallet::call_index(4)]
		#[pallet::weight(10_000)]
		pub fn retract_proposal(origin: OriginFor<T>, cluster_id: ClusterId) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			let voting =
				ClusterProposalVoting::<T>::get(cluster_id).ok_or(Error::<T>::ProposalMissing)?;
			if voting.author != caller_id {
				Err(Error::<T>::NotProposalAuthor.into())
			} else {
				Self::do_remove_proposal(cluster_id);
				Self::deposit_event(Event::Removed { cluster_id });
				Ok(())
			}
		}

		#[pallet::call_index(5)]
		#[pallet::weight(10_000)]
		pub fn activate_cluster(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			T::OpenGovActivatorOrigin::ensure_origin(origin)?;
			T::ClusterCreator::activate_cluster(cluster_id)?;
			T::ClusterEconomics::update_cluster_economics(cluster_id, cluster_gov_params)
		}

		#[pallet::call_index(6)]
		#[pallet::weight(10_000)]
		pub fn update_cluster(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			T::OpenGovUpdaterOrigin::ensure_origin(origin)?;
			T::ClusterEconomics::update_cluster_economics(cluster_id, cluster_gov_params)
		}
	}

	impl<T: Config> Pallet<T> {
		fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		fn ensure_cluster_manager(
			origin: T::AccountId,
			cluster_id: ClusterId,
		) -> Result<(), DispatchError> {
			let cluster_manager = T::ClusterManager::get_manager_account_id(&cluster_id)
				.map_err(|_| Error::<T>::NoCluster)?;
			ensure!(origin == cluster_manager, Error::<T>::NotClusterManager);
			Ok(())
		}

		fn ensure_validated_member(
			origin: T::AccountId,
			cluster_id: ClusterId,
			member: ClusterMember,
		) -> Result<(), DispatchError> {
			match member {
				ClusterMember::ClusterManager => Self::ensure_cluster_manager(origin, cluster_id),
				ClusterMember::NodeProvider(node_pub_key) => {
					let is_validated_node = T::ClusterManager::contains_node(
						&cluster_id,
						&node_pub_key,
						Some(ClusterNodeStatus::ValidationSucceeded),
					);
					if !is_validated_node {
						Err(Error::<T>::NotValidatedNode.into())
					} else {
						let node_provider = T::NodeVisitor::get_node_provider_id(&node_pub_key)?;
						if origin == node_provider {
							Ok(())
						} else {
							Err(Error::<T>::NotValidatedNode.into())
						}
					}
				},
			}
		}

		fn ensure_allowed_voter(
			origin: T::AccountId,
			cluster_id: ClusterId,
			member: ClusterMember,
		) -> Result<(), DispatchError> {
			match member {
				ClusterMember::ClusterManager => Self::ensure_cluster_manager(origin, cluster_id),
				ClusterMember::NodeProvider(node_pub_key) => {
					let node_state = T::ClusterManager::get_node_state(&cluster_id, &node_pub_key)
						.map_err(|_| Error::<T>::NotValidatedNode)?;
					if node_state.status != ClusterNodeStatus::ValidationSucceeded {
						Err(Error::<T>::NotValidatedNode.into())
					} else {
						let voting = ClusterProposalVoting::<T>::get(cluster_id)
							.ok_or(Error::<T>::ProposalMissing)?;
						if node_state.added_at < voting.start {
							Ok(())
						} else {
							Err(Error::<T>::VoteProhibited.into())
						}
					}
				},
			}
		}

		fn do_vote(
			voter_id: T::AccountId,
			cluster_id: ClusterId,
			approve: bool,
		) -> Result<bool, DispatchError> {
			let mut voting = Self::voting(&cluster_id).ok_or(Error::<T>::ProposalMissing)?;

			let position_yes = voting.ayes.iter().position(|a| a == &voter_id);
			let position_no = voting.nays.iter().position(|a| a == &voter_id);

			// Detects first vote of the member in the motion
			let is_account_voting_first_time = position_yes.is_none() && position_no.is_none();

			if approve {
				if position_yes.is_none() {
					voting.ayes.push(voter_id.clone());
				} else {
					return Err(Error::<T>::DuplicateVote.into())
				}
				if let Some(pos) = position_no {
					voting.nays.swap_remove(pos);
				}
			} else {
				if position_no.is_none() {
					voting.nays.push(voter_id.clone());
				} else {
					return Err(Error::<T>::DuplicateVote.into())
				}
				if let Some(pos) = position_yes {
					voting.ayes.swap_remove(pos);
				}
			}

			let yes_votes = voting.ayes.len() as MemberCount;
			let no_votes = voting.nays.len() as MemberCount;
			Self::deposit_event(Event::Voted {
				account: voter_id,
				cluster_id,
				voted: approve,
				yes: yes_votes,
				no: no_votes,
			});

			ClusterProposalVoting::<T>::insert(&cluster_id, voting);

			Ok(is_account_voting_first_time)
		}

		/// Close a vote that is either approved, disapproved or whose voting period has ended.
		fn do_close(cluster_id: ClusterId) -> Result<DispatchResultWithPostInfo, DispatchError> {
			let voting = Self::voting(&cluster_id).ok_or(Error::<T>::ProposalMissing)?;

			let mut no_votes = voting.nays.len() as MemberCount;
			let mut yes_votes = voting.ayes.len() as MemberCount;
			let seats = voting.threshold as MemberCount;
			let approved = yes_votes >= voting.threshold;
			let disapproved = seats.saturating_sub(no_votes) < voting.threshold;
			// Allow (dis-)approving the proposal as soon as there are enough votes.
			if approved {
				let (proposal, len) = Self::validate_and_get_proposal(&cluster_id)?;
				Self::deposit_event(Event::Closed { cluster_id, yes: yes_votes, no: no_votes });
				let proposal_weight =
					Self::do_approve_proposal(cluster_id, proposal, voting.is_activation)?;
				let proposal_count = 1;

				return Ok(Ok((
					Some(
						<T as pallet::Config>::WeightInfo::close_early_approved(
							len as u32,
							seats,
							proposal_count,
						)
						.saturating_add(proposal_weight),
					),
					Pays::Yes,
				)
					.into()))
			} else if disapproved {
				Self::deposit_event(Event::Closed { cluster_id, yes: yes_votes, no: no_votes });
				Self::do_disapprove_proposal(cluster_id);
				let proposal_count = 1;

				return Ok(Ok((
					Some(<T as pallet::Config>::WeightInfo::close_early_disapproved(
						seats,
						proposal_count,
					)),
					Pays::No,
				)
					.into()))
			}

			// Only allow actual closing of the proposal after the voting period has ended.
			ensure!(frame_system::Pallet::<T>::block_number() >= voting.end, Error::<T>::TooEarly);

			// default voting strategy.
			let default = T::DefaultVote::default_vote(None, yes_votes, no_votes, seats);

			let abstentions = seats - (yes_votes + no_votes);
			match default {
				true => yes_votes += abstentions,
				false => no_votes += abstentions,
			}
			let approved = yes_votes >= voting.threshold;

			if approved {
				let (proposal, len) = Self::validate_and_get_proposal(&cluster_id)?;
				Self::deposit_event(Event::Closed { cluster_id, yes: yes_votes, no: no_votes });
				let proposal_weight =
					Self::do_approve_proposal(cluster_id, proposal, voting.is_activation)?;
				let proposal_count = 1;

				Ok(Ok((
					Some(
						<T as pallet::Config>::WeightInfo::close_approved(
							len as u32,
							seats,
							proposal_count,
						)
						.saturating_add(proposal_weight),
					),
					Pays::Yes,
				)
					.into()))
			} else {
				Self::deposit_event(Event::Closed { cluster_id, yes: yes_votes, no: no_votes });
				Self::do_disapprove_proposal(cluster_id);
				let proposal_count = 1;

				Ok(Ok((
					Some(<T as pallet::Config>::WeightInfo::close_disapproved(
						seats,
						proposal_count,
					)),
					Pays::No,
				)
					.into()))
			}
		}

		fn do_approve_proposal(
			cluster_id: ClusterId,
			proposal: <T as Config>::ClusterProposalCall,
			is_activation: bool,
		) -> Result<Weight, DispatchError> {
			Self::deposit_event(Event::Approved { cluster_id });
			let (result, proposal_weight) = Self::do_propose_public(proposal, is_activation)?;
			Self::deposit_event(Event::Executed {
				cluster_id,
				result: result.map(|_| ()).map_err(|e| e.error),
			});
			Self::do_remove_proposal(cluster_id);
			Ok(proposal_weight)
		}

		/// Removes a proposal from the pallet, and deposit the `Disapproved` event.
		fn do_disapprove_proposal(cluster_id: ClusterId) {
			Self::deposit_event(Event::Disapproved { cluster_id });
			Self::do_remove_proposal(cluster_id)
		}

		fn do_propose_public(
			proposal: <T as Config>::ClusterProposalCall,
			is_activation: bool,
		) -> Result<(DispatchResultWithPostInfo, Weight), DispatchError> {
			let call: <T as pallet_referenda::Config>::RuntimeCall = proposal.into();
			let bounded_call =
				T::Preimages::bound(call).map_err(|_| Error::<T>::ProposalMissing)?;

			let proposal_origin = if is_activation {
				T::OpenGovActivatorTrackOrigin::get()
			} else {
				T::OpenGovUpdaterTrackOrigin::get()
			};

			let pallets_origin: <T::RuntimeOrigin as OriginTrait>::PalletsOrigin =
				proposal_origin.caller().clone();
			let referenda_call = pallet_referenda::Call::<T>::submit {
				proposal_origin: Box::new(pallets_origin),
				proposal: bounded_call,
				enactment_moment: DispatchTime::After(T::BlockNumber::from(1u32)),
			};

			let result = referenda_call
				.dispatch_bypass_filter(frame_system::RawOrigin::Signed(Self::account_id()).into());

			let proposal_weight =
				Self::get_result_weight(result.clone()).unwrap_or(Weight::from_ref_time(10000));

			Ok((result, proposal_weight))
		}

		/// Removes a proposal from the pallet, cleaning up votes and the vector of proposals.
		fn do_remove_proposal(cluster_id: ClusterId) {
			ClusterProposal::<T>::remove(&cluster_id);
			ClusterProposalVoting::<T>::remove(&cluster_id);
		}

		/// Return the weight of a dispatch call result as an `Option`.
		///
		/// Will return the weight regardless of what the state of the result is.
		fn get_result_weight(result: DispatchResultWithPostInfo) -> Option<Weight> {
			match result {
				Ok(post_info) => post_info.actual_weight,
				Err(err) => err.post_info.actual_weight,
			}
		}

		fn validate_and_get_proposal(
			cluster_id: &ClusterId,
		) -> Result<(<T as Config>::ClusterProposalCall, usize), DispatchError> {
			let key = ClusterProposal::<T>::hashed_key_for(cluster_id);
			// read the length of the proposal storage entry directly
			let proposal_len =
				storage::read(&key, &mut [0; 0], 0).ok_or(Error::<T>::ProposalMissing)?;
			let proposal =
				ClusterProposal::<T>::get(cluster_id).ok_or(Error::<T>::ProposalMissing)?;
			Ok((proposal, proposal_len as usize))
		}
	}
}

/// Set the prime member's vote as the default vote.
pub struct NayAsDefaultVote;
impl DefaultVote for NayAsDefaultVote {
	fn default_vote(
		_prime_vote: Option<bool>,
		_yes_votes: MemberCount,
		_no_votes: MemberCount,
		_len: MemberCount,
	) -> bool {
		false
	}
}