use frame_support::{
	pallet_prelude::EnsureOrigin,
	parameter_types,
	traits::{EitherOf, EnsureOriginWithArg, OriginTrait},
};
use frame_system::EnsureRootWithSuccess;
use sp_std::marker::PhantomData;

use super::*;

mod tracks;
use cere_runtime_common::constants::tracks::{
	CLUSTER_PROTOCOL_ACTIVATOR_TRACK_ID, CLUSTER_PROTOCOL_UPDATER_TRACK_ID,
};
use ddc_primitives::traits::pallet::PalletsOriginOf;
pub use pallet_origins::pallet::{
	ClusterProtocolActivator, ClusterProtocolUpdater, GeneralAdmin, ReferendumCanceller,
	ReferendumKiller, Spender, StakingAdmin, Treasurer, WhitelistedCaller,
};
pub use tracks::TracksInfo;

parameter_types! {
	pub const VoteLockingPeriod: BlockNumber = 7 * DAYS;
}

impl pallet_conviction_voting::Config for Runtime {
	type WeightInfo = pallet_conviction_voting::weights::SubstrateWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type VoteLockingPeriod = VoteLockingPeriod;
	type MaxVotes = ConstU32<512>;
	type MaxTurnout =
		frame_support::traits::tokens::currency::ActiveIssuanceOf<Balances, Self::AccountId>;
	type Polls = Referenda;
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = DOLLARS;
	pub const UndecidingTimeout: BlockNumber = 14 * DAYS;
}

parameter_types! {
	pub const MaxBalance: Balance = Balance::max_value();
}
pub type TreasurySpender = EitherOf<EnsureRootWithSuccess<AccountId, MaxBalance>, Spender>;

impl pallet_origins::Config for Runtime {}

impl pallet_whitelist::Config for Runtime {
	type WeightInfo = pallet_whitelist::weights::SubstrateWeight<Runtime>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WhitelistOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureMembers<AccountId, TechCommCollective, 3>,
	>;
	type DispatchWhitelistedOrigin = EitherOf<EnsureRoot<AccountId>, WhitelistedCaller>;
	type Preimages = Preimage;
}

impl pallet_referenda::Config for Runtime {
	type WeightInfo = pallet_referenda::weights::SubstrateWeight<Runtime>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	type SubmitOrigin = EnsureOfPermittedReferendaOrigin<Self>;
	type CancelOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumCanceller>;
	type KillOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumKiller>;
	type Slash = Treasury;
	type Votes = pallet_conviction_voting::VotesOf<Runtime>;
	type Tally = pallet_conviction_voting::TallyOf<Runtime>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
	type Preimages = Preimage;
}

pub struct EnsureOfPermittedReferendaOrigin<T>(PhantomData<T>);
impl<T: frame_system::Config> EnsureOriginWithArg<T::RuntimeOrigin, PalletsOriginOf<T>>
	for EnsureOfPermittedReferendaOrigin<T>
where
	<T as frame_system::Config>::RuntimeOrigin: OriginTrait<PalletsOrigin = OriginCaller>,
{
	type Success = T::AccountId;

	fn try_origin(
		o: T::RuntimeOrigin,
		proposal_origin: &PalletsOriginOf<T>,
	) -> Result<Self::Success, T::RuntimeOrigin> {
		let origin = <frame_system::EnsureSigned<_> as EnsureOrigin<_>>::try_origin(o.clone())?;

		let track_id =
			match <TracksInfo as pallet_referenda::TracksInfo<Balance, BlockNumber>>::track_for(
				proposal_origin,
			) {
				Ok(track_id) => track_id,
				Err(_) => return Err(o),
			};

		if track_id == CLUSTER_PROTOCOL_ACTIVATOR_TRACK_ID
			|| track_id == CLUSTER_PROTOCOL_UPDATER_TRACK_ID
		{
			let clusters_governance = <ClustersGovWrapper as PalletVisitor<T>>::get_account_id();
			if origin == clusters_governance {
				Ok(origin)
			} else {
				Err(o)
			}
		} else {
			Ok(origin)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(
		_proposal_origin: &PalletsOriginOf<T>,
	) -> Result<T::RuntimeOrigin, ()> {
		let origin = frame_benchmarking::account::<T::AccountId>("successful_origin", 0, 0);
		Ok(frame_system::RawOrigin::Signed(origin).into())
	}
}
