//! Track configurations for governance.

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
use cere_runtime_common::constants::tracks::*;

use super::*;
use crate::{Balance, BlockNumber};

const TRACKS_DATA: [(u16, pallet_referenda::TrackInfo<Balance, BlockNumber>); 14] = [
	(
		ROOT_TRACK_ID,
=======
=======
use cere_runtime_common::constants::{currency::*, time::*, tracks::*};
=======
use cere_runtime_common::constants::tracks::*;
>>>>>>> 8df744f4 (Backporting Referendum Support Curves to `dev` branch (#365))

>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
use super::*;
use crate::{Balance, BlockNumber};

const TRACKS_DATA: [(u16, pallet_referenda::TrackInfo<Balance, BlockNumber>); 14] = [
	(
<<<<<<< HEAD
		0,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
		ROOT_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "root",
			max_deciding: 1,
			decision_deposit: 100 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 24 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_ROOT,
			min_support: SUP_ROOT,
		},
	),
	(
<<<<<<< HEAD
<<<<<<< HEAD
		WHITELISTED_CALLER_TRACK_ID,
=======
		1,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
		WHITELISTED_CALLER_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "whitelisted_caller",
			max_deciding: 100,
			decision_deposit: 10 * GRAND,
			prepare_period: 30 * MINUTES,
			decision_period: 28 * DAYS,
			confirm_period: 10 * MINUTES,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_WHITELISTED_CALLER,
			min_support: SUP_WHITELISTED_CALLER,
		},
	),
	(
<<<<<<< HEAD
<<<<<<< HEAD
		STAKING_ADMIN_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "staking_admin",
			max_deciding: 10,
			decision_deposit: 10 * GRAND,
<<<<<<< HEAD
=======
		10,
=======
		STAKING_ADMIN_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "staking_admin",
			max_deciding: 10,
			decision_deposit: 5 * GRAND,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
>>>>>>> 8df744f4 (Backporting Referendum Support Curves to `dev` branch (#365))
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_STAKING_ADMIN,
			min_support: SUP_STAKING_ADMIN,
		},
	),
	(
<<<<<<< HEAD
<<<<<<< HEAD
		TREASURER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "treasurer",
			max_deciding: 10,
			decision_deposit: 10 * GRAND,
<<<<<<< HEAD
=======
		11,
=======
		TREASURER_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "treasurer",
			max_deciding: 10,
			decision_deposit: GRAND,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
>>>>>>> 8df744f4 (Backporting Referendum Support Curves to `dev` branch (#365))
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_TREASURER,
			min_support: SUP_TREASURER,
		},
	),
	(
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		GENERAL_ADMIN_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "general_admin",
			max_deciding: 10,
			decision_deposit: 10 * GRAND,
=======
		13,
=======
		FELLOWSHIP_ADMIN_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "fellowship_admin",
			max_deciding: 10,
			decision_deposit: 5 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_FELLOWSHIP_ADMIN,
			min_support: SUP_FELLOWSHIP_ADMIN,
		},
	),
	(
=======
>>>>>>> 8df744f4 (Backporting Referendum Support Curves to `dev` branch (#365))
		GENERAL_ADMIN_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "general_admin",
			max_deciding: 10,
<<<<<<< HEAD
			decision_deposit: 5 * GRAND,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
			decision_deposit: 10 * GRAND,
>>>>>>> 8df744f4 (Backporting Referendum Support Curves to `dev` branch (#365))
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_GENERAL_ADMIN,
			min_support: SUP_GENERAL_ADMIN,
		},
	),
	(
<<<<<<< HEAD
<<<<<<< HEAD
		REFERENDUM_CANCELER_TRACK_ID,
=======
		20,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
		REFERENDUM_CANCELER_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "referendum_canceller",
			max_deciding: 1_000,
			decision_deposit: 10 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 7 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_REFERENDUM_CANCELLER,
			min_support: SUP_REFERENDUM_CANCELLER,
		},
	),
	(
<<<<<<< HEAD
<<<<<<< HEAD
		REFERENDUM_KILLER_TRACK_ID,
=======
		21,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
		REFERENDUM_KILLER_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "referendum_killer",
			max_deciding: 1_000,
			decision_deposit: 50 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_REFERENDUM_KILLER,
			min_support: SUP_REFERENDUM_KILLER,
		},
	),
	(
<<<<<<< HEAD
<<<<<<< HEAD
		SMALL_TIPPER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "small_tipper",
			max_deciding: 200,
			decision_deposit: 10 * GRAND,
<<<<<<< HEAD
=======
		30,
=======
		SMALL_TIPPER_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "small_tipper",
			max_deciding: 200,
			decision_deposit: DOLLARS,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
>>>>>>> 8df744f4 (Backporting Referendum Support Curves to `dev` branch (#365))
			prepare_period: MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 10 * MINUTES,
			min_enactment_period: MINUTES,
			min_approval: APP_SMALL_TIPPER,
			min_support: SUP_SMALL_TIPPER,
		},
	),
	(
<<<<<<< HEAD
<<<<<<< HEAD
		BIG_TIPPER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "big_tipper",
			max_deciding: 100,
			decision_deposit: 10 * GRAND,
<<<<<<< HEAD
=======
		31,
=======
		BIG_TIPPER_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "big_tipper",
			max_deciding: 100,
			decision_deposit: 10 * DOLLARS,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
>>>>>>> 8df744f4 (Backporting Referendum Support Curves to `dev` branch (#365))
			prepare_period: 10 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_BIG_TIPPER,
			min_support: SUP_BIG_TIPPER,
		},
	),
	(
<<<<<<< HEAD
<<<<<<< HEAD
		SMALL_SPENDER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "small_spender",
			max_deciding: 50,
			decision_deposit: 10 * GRAND,
<<<<<<< HEAD
=======
		32,
=======
		SMALL_SPENDER_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "small_spender",
			max_deciding: 50,
			decision_deposit: 100 * DOLLARS,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
>>>>>>> 8df744f4 (Backporting Referendum Support Curves to `dev` branch (#365))
			prepare_period: 4 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 12 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_SMALL_SPENDER,
			min_support: SUP_SMALL_SPENDER,
		},
	),
	(
<<<<<<< HEAD
<<<<<<< HEAD
		MEDIUM_SPENDER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "medium_spender",
			max_deciding: 50,
			decision_deposit: 10 * GRAND,
<<<<<<< HEAD
=======
		33,
=======
		MEDIUM_SPENDER_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "medium_spender",
			max_deciding: 50,
			decision_deposit: 200 * DOLLARS,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
>>>>>>> 8df744f4 (Backporting Referendum Support Curves to `dev` branch (#365))
			prepare_period: 4 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 24 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_MEDIUM_SPENDER,
			min_support: SUP_MEDIUM_SPENDER,
		},
	),
	(
<<<<<<< HEAD
<<<<<<< HEAD
		BIG_SPENDER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "big_spender",
			max_deciding: 50,
			decision_deposit: 10 * GRAND,
<<<<<<< HEAD
=======
		34,
=======
		BIG_SPENDER_TRACK_ID,
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
		pallet_referenda::TrackInfo {
			name: "big_spender",
			max_deciding: 50,
			decision_deposit: 400 * DOLLARS,
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
>>>>>>> 8df744f4 (Backporting Referendum Support Curves to `dev` branch (#365))
			prepare_period: 4 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 48 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_BIG_SPENDER,
			min_support: SUP_BIG_SPENDER,
		},
	),
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
	(
		CLUSTER_PROTOCOL_ACTIVATOR_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "cluster_protocol_activator",
			max_deciding: 50,
<<<<<<< HEAD
<<<<<<< HEAD
			decision_deposit: DOLLARS,
			prepare_period: 0,
			decision_period: 2 * MINUTES,
			confirm_period: MINUTES,
			min_enactment_period: 0,
=======
			decision_deposit: DOLLARS, // todo: define value for Devnet
			prepare_period: 0,         // todo: define value for Devnet
			decision_period: 2 * MINUTES,
			confirm_period: MINUTES,
			min_enactment_period: 0, // todo: define value for Devnet
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
=======
			decision_deposit: DOLLARS,
			prepare_period: 0,
			decision_period: 2 * MINUTES,
			confirm_period: MINUTES,
			min_enactment_period: 0,
>>>>>>> af308160 (ClusterGov params on-chain params (#419))
			min_approval: APP_CLUSTER_PROTOCOL_ACTIVATOR,
			min_support: SUP_CLUSTER_PROTOCOL_ACTIVATOR,
		},
	),
	(
		CLUSTER_PROTOCOL_UPDATER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "cluster_protocol_updater",
			max_deciding: 50,
<<<<<<< HEAD
<<<<<<< HEAD
			decision_deposit: DOLLARS,
			prepare_period: 0,
			decision_period: 2 * MINUTES,
			confirm_period: MINUTES,
			min_enactment_period: 0,
=======
			decision_deposit: DOLLARS, // todo: define value for Devnet
			prepare_period: 0,         // todo: define value for Devnet
			decision_period: 2 * MINUTES,
			confirm_period: MINUTES,
			min_enactment_period: 0, // todo: define value for Devnet
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
=======
			decision_deposit: DOLLARS,
			prepare_period: 0,
			decision_period: 2 * MINUTES,
			confirm_period: MINUTES,
			min_enactment_period: 0,
>>>>>>> af308160 (ClusterGov params on-chain params (#419))
			min_approval: APP_CLUSTER_PROTOCOL_UPDATER,
			min_support: SUP_CLUSTER_PROTOCOL_UPDATER,
		},
	),
<<<<<<< HEAD
=======
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
];

pub struct TracksInfo;
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = u16;
	type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;
	fn tracks() -> &'static [(Self::Id, pallet_referenda::TrackInfo<Balance, BlockNumber>)] {
		&TRACKS_DATA[..]
	}
	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		if let Ok(system_origin) = frame_system::RawOrigin::try_from(id.clone()) {
			match system_origin {
<<<<<<< HEAD
<<<<<<< HEAD
				frame_system::RawOrigin::Root => Ok(ROOT_TRACK_ID),
				_ => Err(()),
			}
		} else if let Ok(custom_origin) = pallet_origins::pallet::Origin::try_from(id.clone()) {
			match custom_origin {
				pallet_origins::pallet::Origin::WhitelistedCaller =>
					Ok(WHITELISTED_CALLER_TRACK_ID),
				// General admin
				pallet_origins::pallet::Origin::StakingAdmin => Ok(STAKING_ADMIN_TRACK_ID),
				pallet_origins::pallet::Origin::Treasurer => Ok(TREASURER_TRACK_ID),
				pallet_origins::pallet::Origin::GeneralAdmin => Ok(GENERAL_ADMIN_TRACK_ID),
				// Referendum admins
				pallet_origins::pallet::Origin::ReferendumCanceller =>
					Ok(REFERENDUM_CANCELER_TRACK_ID),
				pallet_origins::pallet::Origin::ReferendumKiller => Ok(REFERENDUM_KILLER_TRACK_ID),
				// Limited treasury spenders
				pallet_origins::pallet::Origin::SmallTipper => Ok(SMALL_TIPPER_TRACK_ID),
				pallet_origins::pallet::Origin::BigTipper => Ok(BIG_TIPPER_TRACK_ID),
				pallet_origins::pallet::Origin::SmallSpender => Ok(SMALL_SPENDER_TRACK_ID),
				pallet_origins::pallet::Origin::MediumSpender => Ok(MEDIUM_SPENDER_TRACK_ID),
				pallet_origins::pallet::Origin::BigSpender => Ok(BIG_SPENDER_TRACK_ID),
				// DDC admins
				pallet_origins::pallet::Origin::ClusterProtocolActivator =>
					Ok(CLUSTER_PROTOCOL_ACTIVATOR_TRACK_ID),
				pallet_origins::pallet::Origin::ClusterProtocolUpdater =>
					Ok(CLUSTER_PROTOCOL_UPDATER_TRACK_ID),
=======
				frame_system::RawOrigin::Root => Ok(0),
=======
				frame_system::RawOrigin::Root => Ok(ROOT_TRACK_ID),
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
				_ => Err(()),
			}
		} else if let Ok(custom_origin) = pallet_origins::pallet::Origin::try_from(id.clone()) {
			match custom_origin {
				pallet_origins::pallet::Origin::WhitelistedCaller =>
					Ok(WHITELISTED_CALLER_TRACK_ID),
				// General admin
				pallet_origins::pallet::Origin::StakingAdmin => Ok(STAKING_ADMIN_TRACK_ID),
				pallet_origins::pallet::Origin::Treasurer => Ok(TREASURER_TRACK_ID),
				pallet_origins::pallet::Origin::FellowshipAdmin => Ok(FELLOWSHIP_ADMIN_TRACK_ID),
				pallet_origins::pallet::Origin::GeneralAdmin => Ok(GENERAL_ADMIN_TRACK_ID),
				// Referendum admins
				pallet_origins::pallet::Origin::ReferendumCanceller =>
					Ok(REFERENDUM_CANCELER_TRACK_ID),
				pallet_origins::pallet::Origin::ReferendumKiller => Ok(REFERENDUM_KILLER_TRACK_ID),
				// Limited treasury spenders
<<<<<<< HEAD
				origins::Origin::SmallTipper => Ok(30),
				origins::Origin::BigTipper => Ok(31),
				origins::Origin::SmallSpender => Ok(32),
				origins::Origin::MediumSpender => Ok(33),
				origins::Origin::BigSpender => Ok(34),
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======
				pallet_origins::pallet::Origin::SmallTipper => Ok(SMALL_TIPPER_TRACK_ID),
				pallet_origins::pallet::Origin::BigTipper => Ok(BIG_TIPPER_TRACK_ID),
				pallet_origins::pallet::Origin::SmallSpender => Ok(SMALL_SPENDER_TRACK_ID),
				pallet_origins::pallet::Origin::MediumSpender => Ok(MEDIUM_SPENDER_TRACK_ID),
				pallet_origins::pallet::Origin::BigSpender => Ok(BIG_SPENDER_TRACK_ID),
				// DDC admins
				pallet_origins::pallet::Origin::ClusterProtocolActivator =>
					Ok(CLUSTER_PROTOCOL_ACTIVATOR_TRACK_ID),
				pallet_origins::pallet::Origin::ClusterProtocolUpdater =>
					Ok(CLUSTER_PROTOCOL_UPDATER_TRACK_ID),
>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
			}
		} else {
			Err(())
		}
	}
}
<<<<<<< HEAD
<<<<<<< HEAD

=======
>>>>>>> c8ff9efa (Introduce OpenGov into Cere and CereDev (#238))
=======

>>>>>>> 1c1576b4 (Cluster Governance Pallet (#249))
pallet_referenda::impl_tracksinfo_get!(TracksInfo, Balance, BlockNumber);
