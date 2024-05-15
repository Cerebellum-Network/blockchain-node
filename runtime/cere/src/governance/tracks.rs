//! Track configurations for governance.

use cere_runtime_common::constants::tracks::TRACKS_DATA;

use super::*;
use crate::{Balance, BlockNumber};

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
				frame_system::RawOrigin::Root => Ok(0),
				_ => Err(()),
			}
		} else if let Ok(custom_origin) = pallet_origins::pallet::Origin::try_from(id.clone()) {
			match custom_origin {
				pallet_origins::pallet::Origin::WhitelistedCaller => Ok(1),
				// General admin
				pallet_origins::pallet::Origin::StakingAdmin => Ok(10),
				pallet_origins::pallet::Origin::Treasurer => Ok(11),
				pallet_origins::pallet::Origin::FellowshipAdmin => Ok(13),
				pallet_origins::pallet::Origin::GeneralAdmin => Ok(14),
				// Referendum admins
				pallet_origins::pallet::Origin::ReferendumCanceller => Ok(20),
				pallet_origins::pallet::Origin::ReferendumKiller => Ok(21),
				// Limited treasury spenders
				pallet_origins::pallet::Origin::SmallTipper => Ok(30),
				pallet_origins::pallet::Origin::BigTipper => Ok(31),
				pallet_origins::pallet::Origin::SmallSpender => Ok(32),
				pallet_origins::pallet::Origin::MediumSpender => Ok(33),
				pallet_origins::pallet::Origin::BigSpender => Ok(34),
				// DDC admins
				pallet_origins::pallet::Origin::ClusterGovCreator => Ok(CLUSTER_ACTIVATOR_TRACK_ID),
				pallet_origins::pallet::Origin::ClusterGovEditor => {
					Ok(CLUSTER_ECONOMICS_UPDATER_TRACK_ID)
				},
			}
		} else {
			Err(())
		}
	}
}

pallet_referenda::impl_tracksinfo_get!(TracksInfo, Balance, BlockNumber);
