use frame_support::{parameter_types, traits::EnsureOrigin};
use frame_system::{Config, EnsureRootWithSuccess};
use sp_core::crypto::{AccountId32, Ss58Codec};

use super::*;

mod origins;
pub use origins::{
	pallet_custom_origins, GeneralAdmin, ReferendumCanceller, ReferendumKiller, Spender,
	StakingAdmin, Treasurer, WhitelistedCaller,
};

mod tracks;
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

impl origins::pallet_custom_origins::Config for Runtime {}

impl pallet_whitelist::Config for Runtime {
	type WeightInfo = pallet_whitelist::weights::SubstrateWeight<Runtime>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WhitelistOrigin = EitherOf<EnsureRoot<AccountId>, EnsureTechCommittee<Self>>;
	type DispatchWhitelistedOrigin = EitherOf<EnsureRoot<AccountId>, WhitelistedCaller>;
	type Preimages = Preimage;
}

const TECH_COMMITTEE_MULTISIG: &str = "6QX7ZM6oT9zsG1b8i54TAi35UEGqR6XdjHs9vsA3PNv77yJ5"; // Multisig: Tech Comm #1 + Tech Comm #2, threshold = 2
pub struct EnsureTechCommittee<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> EnsureOrigin<T::RuntimeOrigin> for EnsureTechCommittee<T> {
	type Success = ();

	fn try_origin(o: T::RuntimeOrigin) -> Result<Self::Success, T::RuntimeOrigin> {
		let tech_comm_account_32: AccountId32 =
			AccountId32::from_ss58check(TECH_COMMITTEE_MULTISIG).unwrap();
		let mut bytes = AccountId32::as_ref(&tech_comm_account_32);
		let tech_comm_id: T::AccountId = T::AccountId::decode(&mut bytes).unwrap();
		o.into().and_then(|o| match o {
			frame_system::RawOrigin::Signed(ref who) if who == &tech_comm_id => Ok(()),
			r => Err(T::RuntimeOrigin::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<T::RuntimeOrigin, ()> {
		let tech_comm_account_32: AccountId32 =
			AccountId32::from_ss58check(TECH_COMMITTEE_MULTISIG).unwrap();
		let mut bytes = AccountId32::as_ref(&tech_comm_account_32);
		let tech_comm_id: T::AccountId = T::AccountId::decode(&mut bytes).unwrap();
		Ok(T::RuntimeOrigin::from(frame_system::RawOrigin::Signed(tech_comm_id)))
	}
}

impl pallet_referenda::Config for Runtime {
	type WeightInfo = pallet_referenda::weights::SubstrateWeight<Runtime>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	type SubmitOrigin = frame_system::EnsureSigned<AccountId>;
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
