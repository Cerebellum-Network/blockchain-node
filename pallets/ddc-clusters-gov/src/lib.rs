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

use ddc_primitives::{
	traits::{
		cluster::{ClusterCreator, ClusterEconomics, ClusterManager, ClusterQuery},
		cluster_gov::{DefaultVote, MemberCount},
		node::{NodeCreator, NodeVisitor},
		pallet::GetDdcOrigin,
		staking::StakerCreator,
	},
	ClusterGovParams, ClusterId, ClusterNodeStatus, ClusterStatus, NodePubKey,
};
use frame_support::{
	codec::{Decode, Encode},
	dispatch::{DispatchError, DispatchResult, Dispatchable, GetDispatchInfo, Pays, Weight},
	pallet_prelude::*,
	traits::{
		schedule::DispatchTime, Currency, ExistenceRequirement, LockableCurrency, OriginTrait,
		StorePreimage, UnfilteredDispatchable,
	},
};
use frame_system::pallet_prelude::*;
pub use frame_system::Config as SysConfig;
pub use pallet::*;
use pallet_referenda::ReferendumIndex;
use scale_info::TypeInfo;
use sp_runtime::{traits::AccountIdConversion, RuntimeDebug, SaturatedConversion};
use sp_std::prelude::*;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::WeightInfo;

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type ReferendaCall<T> = pallet_referenda::Call<T>;

/// Info for keeping track of a proposal being voted on.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Votes<AccountId, BlockNumber> {
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
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum ClusterMember {
	ClusterManager,
	NodeProvider(NodePubKey),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Proposal<AccountId, Call> {
	author: AccountId,
	kind: ProposalKind,
	call: Call,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum ProposalKind {
	ActivateCluster,
	UpdateClusterEconomics,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SubmissionDeposit<AccountId> {
	depositor: AccountId,
	amount: u128,
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
		type DefaultVote: DefaultVote;
		type MinValidatedNodesCount: Get<u16>;
		type ReferendumEnactmentDuration: Get<Self::BlockNumber>;
		#[cfg(feature = "runtime-benchmarks")]
		type NodeCreator: NodeCreator<Self>;
		#[cfg(feature = "runtime-benchmarks")]
		type StakerCreator: StakerCreator<Self, BalanceOf<Self>>;
	}

	#[pallet::storage]
	#[pallet::getter(fn proposal_of)]
	pub type ClusterProposal<T: Config> = StorageMap<
		_,
		Identity,
		ClusterId,
		Proposal<T::AccountId, T::ClusterProposalCall>,
		OptionQuery,
	>;

	/// Votes on a given cluster proposal, if it is ongoing.
	#[pallet::storage]
	#[pallet::getter(fn voting)]
	pub type ClusterProposalVoting<T: Config> =
		StorageMap<_, Identity, ClusterId, Votes<T::AccountId, T::BlockNumber>, OptionQuery>;

	/// Public referendums initiated by clusters
	#[pallet::storage]
	#[pallet::getter(fn submission_depositor)]
	pub type SubmissionDeposits<T: Config> =
		StorageMap<_, Identity, ReferendumIndex, SubmissionDeposit<T::AccountId>, OptionQuery>;

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
		ReferendumSubmitted { cluster_id: ClusterId },
		/// A proposal was closed because its threshold was reached or after its duration was up.
		Closed { cluster_id: ClusterId, yes: MemberCount, no: MemberCount },
		/// A proposal was not removed by its author.
		Removed { cluster_id: ClusterId },
		/// The submission deposit has been refunded.
		SubmissionDepositRetained {
			/// Index of the referendum.
			referenda_index: ReferendumIndex,
			/// The account who placed the deposit.
			depositor: T::AccountId,
			/// The amount placed by the account.
			amount: BalanceOf<T>,
		},
		/// The submission deposit has been refunded.
		SubmissionDepositRefunded {
			/// Index of the referendum.
			referenda_index: ReferendumIndex,
			/// The account who placed the deposit.
			depositor: T::AccountId,
			/// The amount placed by the account.
			amount: BalanceOf<T>,
		},
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
		/// Cluster does not contain this node
		NoClusterNode,
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
		NoSubmissionDeposit,
		NotNodeProvider,
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
			ensure!(cluster_status == ClusterStatus::Bonded, Error::<T>::UnexpectedState);

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
				Votes { threshold, ayes: vec![], nays: vec![], start, end }
			};
			let call: <T as Config>::ClusterProposalCall =
				T::ClusterProposalCall::from(Call::<T>::activate_cluster {
					cluster_id,
					cluster_gov_params,
				});
			let proposal =
				Proposal { call, author: caller_id.clone(), kind: ProposalKind::ActivateCluster };

			<ClusterProposal<T>>::insert(cluster_id, proposal);
			<ClusterProposalVoting<T>>::insert(cluster_id, votes);
			Self::deposit_event(Event::Proposed { account: caller_id, cluster_id, threshold });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10_000)]
		pub fn propose_update_cluster_economics(
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
			ensure!(cluster_status == ClusterStatus::Activated, Error::<T>::UnexpectedState);

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
				Votes { threshold, ayes: vec![], nays: vec![], start, end }
			};
			let call: <T as Config>::ClusterProposalCall =
				T::ClusterProposalCall::from(Call::<T>::update_cluster_economics {
					cluster_id,
					cluster_gov_params,
				});
			let proposal = Proposal {
				call,
				author: caller_id.clone(),
				kind: ProposalKind::UpdateClusterEconomics,
			};

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
			Self::do_close(cluster_id, caller_id)
		}

		#[pallet::call_index(4)]
		#[pallet::weight(10_000)]
		pub fn retract_proposal(origin: OriginFor<T>, cluster_id: ClusterId) -> DispatchResult {
			let caller_id = ensure_signed(origin)?;
			let proposal =
				ClusterProposal::<T>::get(cluster_id).ok_or(Error::<T>::ProposalMissing)?;
			if proposal.author != caller_id {
				Err(Error::<T>::NotProposalAuthor.into())
			} else {
				Self::do_remove_proposal(cluster_id);
				Ok(())
			}
		}

		#[pallet::call_index(5)]
		#[pallet::weight(10_000)]
		pub fn refund_submission_deposit(
			origin: OriginFor<T>,
			referenda_index: ReferendumIndex,
		) -> DispatchResult {
			ensure_signed_or_root(origin)?;
			let submission_deposit = SubmissionDeposits::<T>::get(referenda_index)
				.ok_or(Error::<T>::NoSubmissionDeposit)?;

			let refund_call =
				pallet_referenda::Call::<T>::refund_submission_deposit { index: referenda_index };
			let result = refund_call
				.dispatch_bypass_filter(frame_system::RawOrigin::Signed(Self::account_id()).into());

			match result {
				Ok(_) => (),
				// Check the error type as the 'refund_submission_deposit' extrinsic might have been
				// called in the original 'pallet_referenda' before the current extrinsic calleed,
				// so the funds are already unlocked for the pallet's balance and need to be
				// refunded to the original depositor.
				Err(ref e) if e.error == pallet_referenda::Error::<T>::NoDeposit.into() => (),
				Err(e) => return Err(e.error),
			}

			Self::do_refund_submission_deposit(
				referenda_index,
				submission_deposit.depositor,
				submission_deposit.amount.saturated_into::<BalanceOf<T>>(),
			)?;

			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(10_000)]
		pub fn activate_cluster(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			T::OpenGovActivatorOrigin::ensure_origin(origin)?;
			T::ClusterCreator::activate_cluster(&cluster_id)?;
			T::ClusterEconomics::update_cluster_economics(&cluster_id, cluster_gov_params)
		}

		#[pallet::call_index(7)]
		#[pallet::weight(10_000)]
		pub fn update_cluster_economics(
			origin: OriginFor<T>,
			cluster_id: ClusterId,
			cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			T::OpenGovUpdaterOrigin::ensure_origin(origin)?;
			T::ClusterEconomics::update_cluster_economics(&cluster_id, cluster_gov_params)
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
							Err(Error::<T>::NotNodeProvider.into())
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
						.map_err(|_| Error::<T>::NoClusterNode)?;
					if node_state.status != ClusterNodeStatus::ValidationSucceeded {
						Err(Error::<T>::NotValidatedNode.into())
					} else {
						let node_provider = T::NodeVisitor::get_node_provider_id(&node_pub_key)?;
						if origin == node_provider {
							let voting = ClusterProposalVoting::<T>::get(cluster_id)
								.ok_or(Error::<T>::ProposalMissing)?;
							if node_state.added_at < voting.start {
								Ok(())
							} else {
								Err(Error::<T>::VoteProhibited.into())
							}
						} else {
							Err(Error::<T>::NotNodeProvider.into())
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
		fn do_close(cluster_id: ClusterId, caller_id: T::AccountId) -> DispatchResultWithPostInfo {
			let voting = Self::voting(&cluster_id).ok_or(Error::<T>::ProposalMissing)?;

			let mut no_votes = voting.nays.len() as MemberCount;
			let mut yes_votes = voting.ayes.len() as MemberCount;
			let seats = voting.threshold as MemberCount;
			let approved = yes_votes >= voting.threshold;
			let disapproved = seats.saturating_sub(no_votes) < voting.threshold;
			// Allow (dis-)approving the proposal as soon as there are enough votes.
			if approved {
				let (proposal, len) = Self::validate_and_get_public_proposal(&cluster_id)?;
				Self::deposit_event(Event::Closed { cluster_id, yes: yes_votes, no: no_votes });
				let proposal_weight = Self::do_approve_proposal(cluster_id, proposal, caller_id)?;
				let proposal_count = 1;

				return Ok((
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
					.into())
			} else if disapproved {
				Self::deposit_event(Event::Closed { cluster_id, yes: yes_votes, no: no_votes });
				Self::do_disapprove_proposal(cluster_id);
				let proposal_count = 1;

				return Ok((
					Some(<T as pallet::Config>::WeightInfo::close_early_disapproved(
						seats,
						proposal_count,
					)),
					Pays::No,
				)
					.into())
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
				let (proposal, len) = Self::validate_and_get_public_proposal(&cluster_id)?;
				Self::deposit_event(Event::Closed { cluster_id, yes: yes_votes, no: no_votes });
				let proposal_weight = Self::do_approve_proposal(cluster_id, proposal, caller_id)?;
				let proposal_count = 1;

				Ok((
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
					.into())
			} else {
				Self::deposit_event(Event::Closed { cluster_id, yes: yes_votes, no: no_votes });
				Self::do_disapprove_proposal(cluster_id);
				let proposal_count = 1;

				Ok((
					Some(<T as pallet::Config>::WeightInfo::close_disapproved(
						seats,
						proposal_count,
					)),
					Pays::No,
				)
					.into())
			}
		}

		fn do_approve_proposal(
			cluster_id: ClusterId,
			proposal: ReferendaCall<T>,
			depositor: T::AccountId,
		) -> Result<Weight, DispatchError> {
			Self::deposit_event(Event::Approved { cluster_id });

			let dispatch_weight = proposal.get_dispatch_info().weight;
			let submission_deposit = Self::do_submission_deposit(depositor.clone())?;

			let post_info = proposal
				.dispatch_bypass_filter(frame_system::RawOrigin::Signed(Self::account_id()).into())
				.map_err(|e| e.error)?;
			Self::deposit_event(Event::ReferendumSubmitted { cluster_id });

			let referenda_index = pallet_referenda::ReferendumCount::<T>::get() - 1;
			Self::do_retain_submission_deposit(referenda_index, depositor, submission_deposit);
			let proposal_weight = post_info.actual_weight.unwrap_or(dispatch_weight);

			Self::do_remove_proposal(cluster_id);
			Ok(proposal_weight)
		}

		/// Removes a proposal from the pallet, and deposit the `Disapproved` event.
		fn do_disapprove_proposal(cluster_id: ClusterId) {
			Self::deposit_event(Event::Disapproved { cluster_id });
			Self::do_remove_proposal(cluster_id)
		}

		/// Removes a proposal from the pallet, cleaning up votes and the vector of proposals.
		fn do_remove_proposal(cluster_id: ClusterId) {
			ClusterProposal::<T>::remove(&cluster_id);
			ClusterProposalVoting::<T>::remove(&cluster_id);
			Self::deposit_event(Event::Removed { cluster_id });
		}

		fn validate_and_get_public_proposal(
			cluster_id: &ClusterId,
		) -> Result<(ReferendaCall<T>, usize), DispatchError> {
			let proposal =
				ClusterProposal::<T>::get(cluster_id).ok_or(Error::<T>::ProposalMissing)?;

			let call: <T as pallet_referenda::Config>::RuntimeCall = proposal.call.into();
			let bounded_call =
				T::Preimages::bound(call).map_err(|_| Error::<T>::ProposalMissing)?;

			let proposal_origin = match proposal.kind {
				ProposalKind::ActivateCluster => T::OpenGovActivatorTrackOrigin::get(),
				ProposalKind::UpdateClusterEconomics => T::OpenGovUpdaterTrackOrigin::get(),
			};

			let pallets_origin: <T::RuntimeOrigin as OriginTrait>::PalletsOrigin =
				proposal_origin.caller().clone();
			let referenda_call = pallet_referenda::Call::<T>::submit {
				proposal_origin: Box::new(pallets_origin),
				proposal: bounded_call,
				enactment_moment: DispatchTime::After(T::ReferendumEnactmentDuration::get()),
			};

			let referenda_call_len = referenda_call.encode().len();

			Ok((referenda_call, referenda_call_len))
		}

		fn do_submission_deposit(depositor: T::AccountId) -> Result<BalanceOf<T>, DispatchError> {
			let submission_deposit =
				<T as pallet_referenda::Config>::SubmissionDeposit::get().saturated_into::<u128>();
			let amount = submission_deposit.saturated_into::<BalanceOf<T>>();
			<T as pallet::Config>::Currency::transfer(
				&depositor,
				&Self::account_id(),
				amount,
				ExistenceRequirement::KeepAlive,
			)?;
			Ok(amount)
		}

		fn do_retain_submission_deposit(
			referenda_index: ReferendumIndex,
			depositor: T::AccountId,
			amount: BalanceOf<T>,
		) {
			let deposit = SubmissionDeposit {
				depositor: depositor.clone(),
				amount: amount.saturated_into::<u128>(),
			};
			SubmissionDeposits::<T>::insert(referenda_index, deposit);
			Self::deposit_event(Event::SubmissionDepositRetained {
				depositor,
				referenda_index,
				amount,
			});
		}

		fn do_refund_submission_deposit(
			referenda_index: ReferendumIndex,
			depositor: T::AccountId,
			amount: BalanceOf<T>,
		) -> Result<(), DispatchError> {
			<T as pallet::Config>::Currency::transfer(
				&Self::account_id(),
				&depositor,
				amount,
				ExistenceRequirement::AllowDeath,
			)?;
			SubmissionDeposits::<T>::remove(referenda_index);
			Self::deposit_event(Event::SubmissionDepositRefunded {
				referenda_index,
				depositor,
				amount,
			});
			Ok(())
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
