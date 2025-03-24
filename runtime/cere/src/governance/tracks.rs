//! Track configurations for governance.

use cere_runtime_common::constants::tracks::*;

use super::*;
use crate::{Balance, BlockNumber};

const TRACKS_DATA: [(u16, pallet_referenda::TrackInfo<Balance, BlockNumber>); 14] = [
	(
		ROOT_TRACK_ID,
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
		WHITELISTED_CALLER_TRACK_ID,
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
		STAKING_ADMIN_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "staking_admin",
			max_deciding: 10,
			decision_deposit: 10 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_STAKING_ADMIN,
			min_support: SUP_STAKING_ADMIN,
		},
	),
	(
		TREASURER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "treasurer",
			max_deciding: 10,
			decision_deposit: 10 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_TREASURER,
			min_support: SUP_TREASURER,
		},
	),
	(
		GENERAL_ADMIN_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "general_admin",
			max_deciding: 10,
			decision_deposit: 10 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_GENERAL_ADMIN,
			min_support: SUP_GENERAL_ADMIN,
		},
	),
	(
		REFERENDUM_CANCELER_TRACK_ID,
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
		REFERENDUM_KILLER_TRACK_ID,
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
		SMALL_TIPPER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "small_tipper",
			max_deciding: 200,
			decision_deposit: 10 * GRAND,
			prepare_period: MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 10 * MINUTES,
			min_enactment_period: MINUTES,
			min_approval: APP_SMALL_TIPPER,
			min_support: SUP_SMALL_TIPPER,
		},
	),
	(
		BIG_TIPPER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "big_tipper",
			max_deciding: 100,
			decision_deposit: 10 * GRAND,
			prepare_period: 10 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_BIG_TIPPER,
			min_support: SUP_BIG_TIPPER,
		},
	),
	(
		SMALL_SPENDER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "small_spender",
			max_deciding: 50,
			decision_deposit: 10 * GRAND,
			prepare_period: 4 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 12 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_SMALL_SPENDER,
			min_support: SUP_SMALL_SPENDER,
		},
	),
	(
		MEDIUM_SPENDER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "medium_spender",
			max_deciding: 50,
			decision_deposit: 10 * GRAND,
			prepare_period: 4 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 24 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_MEDIUM_SPENDER,
			min_support: SUP_MEDIUM_SPENDER,
		},
	),
	(
		BIG_SPENDER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "big_spender",
			max_deciding: 50,
			decision_deposit: 10 * GRAND,
			prepare_period: 4 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 48 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_BIG_SPENDER,
			min_support: SUP_BIG_SPENDER,
		},
	),
	(
		CLUSTER_PROTOCOL_ACTIVATOR_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "cluster_protocol_activator",
			max_deciding: 50,
			decision_deposit: 10 * GRAND,
			prepare_period: 30 * MINUTES,
			decision_period: 28 * DAYS,
			confirm_period: 10 * MINUTES,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_CLUSTER_PROTOCOL_ACTIVATOR,
			min_support: SUP_CLUSTER_PROTOCOL_ACTIVATOR,
		},
	),
	(
		CLUSTER_PROTOCOL_UPDATER_TRACK_ID,
		pallet_referenda::TrackInfo {
			name: "cluster_protocol_updater",
			max_deciding: 50,
			decision_deposit: 10 * GRAND,
			prepare_period: 30 * MINUTES,
			decision_period: 28 * DAYS,
			confirm_period: 10 * MINUTES,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_CLUSTER_PROTOCOL_UPDATER,
			min_support: SUP_CLUSTER_PROTOCOL_UPDATER,
		},
	),
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
			}
		} else {
			Err(())
		}
	}
}

pallet_referenda::impl_tracksinfo_get!(TracksInfo, Balance, BlockNumber);
