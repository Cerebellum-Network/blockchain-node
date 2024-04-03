//! Storage migrations for the Staking pallet.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact, MaxEncodedLen};
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
// use frame_election_provider_support::SortedListProvider;
use frame_support::{
	dispatch::GetStorageVersion, pallet_prelude::ValueQuery, storage_alias,
	traits::OnRuntimeUpgrade,
};
use frame_support::{
	pallet_prelude::*,
	traits::{ConstU32, Currency, Defensive, Get},
	weights::Weight,
	BoundedVec, CloneNoBound, EqNoBound, PartialEqNoBound, RuntimeDebugNoBound,
};
pub use pallet_staking::weights::WeightInfo;
use pallet_staking::{Config, Pallet};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
use sp_runtime::{
	curve::PiecewiseLinear,
	scale_info::TypeInfo,
	traits::{AtLeast32BitUnsigned, Convert, Saturating, StaticLookup, Zero},
	Perbill, Perquintill, Rounding, RuntimeDebug,
};
pub use sp_staking::StakerStatus;
use sp_staking::{
	offence::{Offence, OffenceError, ReportOffence},
	EraIndex, OnStakingUpdate, SessionIndex,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

// pub use pallet::{pallet::*, UseNominatorsAndValidatorsMap, UseValidatorsMap};
use super::*;

const LOG_TARGET: &str = "staking";

/// Used for release versioning upto v12.
///
/// Obsolete from v13. Keeping around to make encoding/decoding of old migration code easier.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
enum ObsoleteReleases {
	V1_0_0Ancient,
	V2_0_0,
	V3_0_0,
	V4_0_0,
	V5_0_0,  // blockable validators.
	V6_0_0,  // removal of all storage associated with offchain phragmen.
	V7_0_0,  // keep track of number of nominators / validators in map
	V8_0_0,  // populate `VoterList`.
	V9_0_0,  // inject validators into `VoterList` as well.
	V10_0_0, // remove `EarliestUnappliedSlash`.
	V11_0_0, // Move pallet storage prefix, e.g. BagsList -> VoterBagsList
	V12_0_0, // remove `HistoryDepth`.
}

impl Default for ObsoleteReleases {
	fn default() -> Self {
		ObsoleteReleases::V12_0_0
	}
}

/// Alias to the old storage item used for release versioning. Obsolete since v13.
#[storage_alias]
type StorageVersion<T: Config> = StorageValue<Pallet<T>, ObsoleteReleases, ValueQuery>;

// pub mod v13 {
//     use super::*;
//
//     pub struct MigrateToV13<T>(sp_std::marker::PhantomData<T>);
//     impl<T: Config> OnRuntimeUpgrade for MigrateToV13<T> {
//         #[cfg(feature = "try-runtime")]
//         fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
//             frame_support::ensure!(
// 				StorageVersion::<T>::get() == ObsoleteReleases::V12_0_0,
// 				"Required v12 before upgrading to v13"
// 			);
//
//             Ok(Default::default())
//         }
//
//         fn on_runtime_upgrade() -> Weight {
//             let current = Pallet::<T>::current_storage_version();
//             let onchain = StorageVersion::<T>::get();
//
//             if current == 13 && onchain == ObsoleteReleases::V12_0_0 {
//                 StorageVersion::<T>::kill();
//                 current.put::<Pallet<T>>();
//
//                 log!(info, "v13 applied successfully");
//                 T::DbWeight::get().reads_writes(1, 2)
//             } else {
//                 log!(warn, "Skipping v13, should be removed");
//                 T::DbWeight::get().reads(1)
//             }
//         }
//
//         #[cfg(feature = "try-runtime")]
//         fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
//             frame_support::ensure!(
// 				Pallet::<T>::on_chain_storage_version() == 13,
// 				"v13 not applied"
// 			);
//
//             frame_support::ensure!(
// 				!StorageVersion::<T>::exists(),
// 				"Storage version not migrated correctly"
// 			);
//
//             Ok(())
//         }
//     }
// }
//
// pub mod v12 {
//     use super::*;
//     use frame_support::{pallet_prelude::ValueQuery, storage_alias};
//
//     #[storage_alias]
//     type HistoryDepth<T: Config> = StorageValue<Pallet<T>, u32, ValueQuery>;
//
//     /// Clean up `HistoryDepth` from storage.
//     ///
//     /// We will be depending on the configurable value of `HistoryDepth` post
//     /// this release.
//     pub struct MigrateToV12<T>(sp_std::marker::PhantomData<T>);
//     impl<T: Config> OnRuntimeUpgrade for MigrateToV12<T> {
//         #[cfg(feature = "try-runtime")]
//         fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
//             frame_support::ensure!(
// 				StorageVersion::<T>::get() == ObsoleteReleases::V11_0_0,
// 				"Expected v11 before upgrading to v12"
// 			);
//
//             if HistoryDepth::<T>::exists() {
//                 frame_support::ensure!(
// 					T::HistoryDepth::get() == HistoryDepth::<T>::get(),
// 					"Provided value of HistoryDepth should be same as the existing storage value"
// 				);
//             } else {
//                 log::info!("No HistoryDepth in storage; nothing to remove");
//             }
//
//             Ok(Default::default())
//         }
//
//         fn on_runtime_upgrade() -> frame_support::weights::Weight {
//             if StorageVersion::<T>::get() == ObsoleteReleases::V11_0_0 {
//                 HistoryDepth::<T>::kill();
//                 StorageVersion::<T>::put(ObsoleteReleases::V12_0_0);
//
//                 log!(info, "v12 applied successfully");
//                 T::DbWeight::get().reads_writes(1, 2)
//             } else {
//                 log!(warn, "Skipping v12, should be removed");
//                 T::DbWeight::get().reads(1)
//             }
//         }
//
//         #[cfg(feature = "try-runtime")]
//         fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
//             frame_support::ensure!(
// 				StorageVersion::<T>::get() == ObsoleteReleases::V12_0_0,
// 				"v12 not applied"
// 			);
//             Ok(())
//         }
//     }
// }
//
// pub mod v11 {
//     use super::*;
//     use frame_support::{
//         storage::migration::move_pallet,
//         traits::{GetStorageVersion, PalletInfoAccess},
//     };
//     #[cfg(feature = "try-runtime")]
//     use sp_io::hashing::twox_128;
//
//     pub struct MigrateToV11<T, P, N>(sp_std::marker::PhantomData<(T, P, N)>);
//     impl<T: Config, P: GetStorageVersion + PalletInfoAccess, N: Get<&'static str>>
// OnRuntimeUpgrade     for MigrateToV11<T, P, N>
//     {
//         #[cfg(feature = "try-runtime")]
//         fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
//             frame_support::ensure!(
// 				StorageVersion::<T>::get() == ObsoleteReleases::V10_0_0,
// 				"must upgrade linearly"
// 			);
//             let old_pallet_prefix = twox_128(N::get().as_bytes());
//
//             frame_support::ensure!(
// 				sp_io::storage::next_key(&old_pallet_prefix).is_some(),
// 				"no data for the old pallet name has been detected"
// 			);
//
//             Ok(Default::default())
//         }
//
//         /// Migrate the entire storage of this pallet to a new prefix.
//         ///
//         /// This new prefix must be the same as the one set in construct_runtime. For safety, use
//         /// `PalletInfo` to get it, as:
//         /// `<Runtime as frame_system::Config>::PalletInfo::name::<VoterBagsList>`.
//         ///
//         /// The migration will look into the storage version in order to avoid triggering a
//         /// migration on an up to date storage.
//         fn on_runtime_upgrade() -> Weight {
//             let old_pallet_name = N::get();
//             let new_pallet_name = <P as PalletInfoAccess>::name();
//
//             if StorageVersion::<T>::get() == ObsoleteReleases::V10_0_0 {
//                 // bump version anyway, even if we don't need to move the prefix
//                 StorageVersion::<T>::put(ObsoleteReleases::V11_0_0);
//                 if new_pallet_name == old_pallet_name {
//                     log!(
// 						warn,
// 						"new bags-list name is equal to the old one, only bumping the version"
// 					);
//                     return
// T::DbWeight::get().reads(1).saturating_add(T::DbWeight::get().writes(1))                 }
//
//                 move_pallet(old_pallet_name.as_bytes(), new_pallet_name.as_bytes());
//                 <T as frame_system::Config>::BlockWeights::get().max_block
//             } else {
//                 log!(warn, "v11::migrate should be removed.");
//                 T::DbWeight::get().reads(1)
//             }
//         }
//
//         #[cfg(feature = "try-runtime")]
//         fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
//             frame_support::ensure!(
// 				StorageVersion::<T>::get() == ObsoleteReleases::V11_0_0,
// 				"wrong version after the upgrade"
// 			);
//
//             let old_pallet_name = N::get();
//             let new_pallet_name = <P as PalletInfoAccess>::name();
//
//             // skip storage prefix checks for the same pallet names
//             if new_pallet_name == old_pallet_name {
//                 return Ok(())
//             }
//
//             let old_pallet_prefix = twox_128(N::get().as_bytes());
//             frame_support::ensure!(
// 				sp_io::storage::next_key(&old_pallet_prefix).is_none(),
// 				"old pallet data hasn't been removed"
// 			);
//
//             let new_pallet_name = <P as PalletInfoAccess>::name();
//             let new_pallet_prefix = twox_128(new_pallet_name.as_bytes());
//             frame_support::ensure!(
// 				sp_io::storage::next_key(&new_pallet_prefix).is_some(),
// 				"new pallet data hasn't been created"
// 			);
//
//             Ok(())
//         }
//     }
// }
//
// pub mod v10 {
//     use super::*;
//     use frame_support::storage_alias;
//
//     #[storage_alias]
//     type EarliestUnappliedSlash<T: Config> = StorageValue<Pallet<T>, EraIndex>;
//
//     /// Apply any pending slashes that where queued.
//     ///
//     /// That means we might slash someone a bit too early, but we will definitely
//     /// won't forget to slash them. The cap of 512 is somewhat randomly taken to
//     /// prevent us from iterating over an arbitrary large number of keys `on_runtime_upgrade`.
//     pub struct MigrateToV10<T>(sp_std::marker::PhantomData<T>);
//     impl<T: Config> OnRuntimeUpgrade for MigrateToV10<T> {
//         fn on_runtime_upgrade() -> frame_support::weights::Weight {
//             if StorageVersion::<T>::get() == ObsoleteReleases::V9_0_0 {
//                 let pending_slashes = UnappliedSlashes::<T>::iter().take(512);
//                 for (era, slashes) in pending_slashes {
//                     for slash in slashes {
//                         // in the old slashing scheme, the slash era was the key at which we read
//                         // from `UnappliedSlashes`.
//                         log!(warn, "prematurely applying a slash ({:?}) for era {:?}", slash,
// era);                         slashing::apply_slash::<T>(slash, era);
//                     }
//                 }
//
//                 EarliestUnappliedSlash::<T>::kill();
//                 StorageVersion::<T>::put(ObsoleteReleases::V10_0_0);
//
//                 log!(info, "MigrateToV10 executed successfully");
//                 T::DbWeight::get().reads_writes(1, 1)
//             } else {
//                 log!(warn, "MigrateToV10 should be removed.");
//                 T::DbWeight::get().reads(1)
//             }
//         }
//     }
// }
//
// pub mod v9 {
//     use super::*;
//     #[cfg(feature = "try-runtime")]
//     use frame_support::codec::{Decode, Encode};
//     #[cfg(feature = "try-runtime")]
//     use sp_std::vec::Vec;
//
//     /// Migration implementation that injects all validators into sorted list.
//     ///
//     /// This is only useful for chains that started their `VoterList` just based on nominators.
//     pub struct InjectValidatorsIntoVoterList<T>(sp_std::marker::PhantomData<T>);
//     impl<T: Config> OnRuntimeUpgrade for InjectValidatorsIntoVoterList<T> {
//         fn on_runtime_upgrade() -> Weight {
//             if StorageVersion::<T>::get() == ObsoleteReleases::V8_0_0 {
//                 let prev_count = T::VoterList::count();
//                 let weight_of_cached = Pallet::<T>::weight_of_fn();
//                 for (v, _) in Validators::<T>::iter() {
//                     let weight = weight_of_cached(&v);
//                     let _ = T::VoterList::on_insert(v.clone(), weight).map_err(|err| {
//                         log!(warn, "failed to insert {:?} into VoterList: {:?}", v, err)
//                     });
//                 }
//
//                 log!(
// 					info,
// 					"injected a total of {} new voters, prev count: {} next count: {}, updating to version 9",
// 					Validators::<T>::count(),
// 					prev_count,
// 					T::VoterList::count(),
// 				);
//
//                 StorageVersion::<T>::put(ObsoleteReleases::V9_0_0);
//                 T::BlockWeights::get().max_block
//             } else {
//                 log!(
// 					warn,
// 					"InjectValidatorsIntoVoterList being executed on the wrong storage \
// 				version, expected ObsoleteReleases::V8_0_0"
// 				);
//                 T::DbWeight::get().reads(1)
//             }
//         }
//
//         #[cfg(feature = "try-runtime")]
//         fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
//             frame_support::ensure!(
// 				StorageVersion::<T>::get() == ObsoleteReleases::V8_0_0,
// 				"must upgrade linearly"
// 			);
//
//             let prev_count = T::VoterList::count();
//             Ok(prev_count.encode())
//         }
//
//         #[cfg(feature = "try-runtime")]
//         fn post_upgrade(prev_count: Vec<u8>) -> Result<(), TryRuntimeError> {
//             let prev_count: u32 = Decode::decode(&mut prev_count.as_slice()).expect(
//                 "the state parameter should be something that was generated by pre_upgrade",
//             );
//             let post_count = T::VoterList::count();
//             let validators = Validators::<T>::count();
//             ensure!(
// 				post_count == prev_count + validators,
// 				"`VoterList` count after the migration must equal to the sum of \
// 				previous count and the current number of validators"
// 			);
//
//             frame_support::ensure!(
// 				StorageVersion::<T>::get() == ObsoleteReleases::V9_0_0,
// 				"must upgrade"
// 			);
//             Ok(())
//         }
//     }
// }
//
// pub mod v8 {
//     use super::*;
//     use crate::{Config, Nominators, Pallet, Weight};
//     use frame_election_provider_support::SortedListProvider;
//     use frame_support::traits::Get;
//
//     #[cfg(feature = "try-runtime")]
//     pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
//         frame_support::ensure!(
// 			StorageVersion::<T>::get() == ObsoleteReleases::V7_0_0,
// 			"must upgrade linearly"
// 		);
//
//         crate::log!(info, "👜 staking bags-list migration passes PRE migrate checks ✅",);
//         Ok(())
//     }
//
//     /// Migration to sorted `VoterList`.
//     pub fn migrate<T: Config>() -> Weight {
//         if StorageVersion::<T>::get() == ObsoleteReleases::V7_0_0 {
//             crate::log!(info, "migrating staking to ObsoleteReleases::V8_0_0");
//
//             let migrated = T::VoterList::unsafe_regenerate(
//                 Nominators::<T>::iter().map(|(id, _)| id),
//                 Pallet::<T>::weight_of_fn(),
//             );
//
//             StorageVersion::<T>::put(ObsoleteReleases::V8_0_0);
//             crate::log!(
// 				info,
// 				"👜 completed staking migration to ObsoleteReleases::V8_0_0 with {} voters migrated",
// 				migrated,
// 			);
//
//             T::BlockWeights::get().max_block
//         } else {
//             T::DbWeight::get().reads(1)
//         }
//     }
//
//     #[cfg(feature = "try-runtime")]
//     pub fn post_migrate<T: Config>() -> Result<(), &'static str> {
//         T::VoterList::try_state().map_err(|_| "VoterList is not in a sane state.")?;
//         crate::log!(info, "👜 staking bags-list migration passes POST migrate checks ✅",);
//         Ok(())
//     }
// }
//
// pub mod v7 {
//     use super::*;
//     use frame_support::storage_alias;
//
//     #[storage_alias]
//     type CounterForValidators<T: Config> = StorageValue<Pallet<T>, u32>;
//     #[storage_alias]
//     type CounterForNominators<T: Config> = StorageValue<Pallet<T>, u32>;
//
//     pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
//         assert!(
//             CounterForValidators::<T>::get().unwrap().is_zero(),
//             "CounterForValidators already set."
//         );
//         assert!(
//             CounterForNominators::<T>::get().unwrap().is_zero(),
//             "CounterForNominators already set."
//         );
//         assert!(Validators::<T>::count().is_zero(), "Validators already set.");
//         assert!(Nominators::<T>::count().is_zero(), "Nominators already set.");
//         assert!(StorageVersion::<T>::get() == ObsoleteReleases::V6_0_0);
//         Ok(())
//     }
//
//     pub fn migrate<T: Config>() -> Weight {
//         log!(info, "Migrating staking to ObsoleteReleases::V7_0_0");
//         let validator_count = Validators::<T>::iter().count() as u32;
//         let nominator_count = Nominators::<T>::iter().count() as u32;
//
//         CounterForValidators::<T>::put(validator_count);
//         CounterForNominators::<T>::put(nominator_count);
//
//         StorageVersion::<T>::put(ObsoleteReleases::V7_0_0);
//         log!(info, "Completed staking migration to ObsoleteReleases::V7_0_0");
//
//         T::DbWeight::get().reads_writes(validator_count.saturating_add(nominator_count).into(),
// 2)     }
// }
//
// pub mod v6 {
//     use super::*;
//     use frame_support::{storage_alias, traits::Get, weights::Weight};
//
//     // NOTE: value type doesn't matter, we just set it to () here.
//     #[storage_alias]
//     type SnapshotValidators<T: Config> = StorageValue<Pallet<T>, ()>;
//     #[storage_alias]
//     type SnapshotNominators<T: Config> = StorageValue<Pallet<T>, ()>;
//     #[storage_alias]
//     type QueuedElected<T: Config> = StorageValue<Pallet<T>, ()>;
//     #[storage_alias]
//     type QueuedScore<T: Config> = StorageValue<Pallet<T>, ()>;
//     #[storage_alias]
//     type EraElectionStatus<T: Config> = StorageValue<Pallet<T>, ()>;
//     #[storage_alias]
//     type IsCurrentSessionFinal<T: Config> = StorageValue<Pallet<T>, ()>;
//
//     /// check to execute prior to migration.
//     pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
//         // these may or may not exist.
//         log!(info, "SnapshotValidators.exits()? {:?}", SnapshotValidators::<T>::exists());
//         log!(info, "SnapshotNominators.exits()? {:?}", SnapshotNominators::<T>::exists());
//         log!(info, "QueuedElected.exits()? {:?}", QueuedElected::<T>::exists());
//         log!(info, "QueuedScore.exits()? {:?}", QueuedScore::<T>::exists());
//         // these must exist.
//         assert!(
//             IsCurrentSessionFinal::<T>::exists(),
//             "IsCurrentSessionFinal storage item not found!"
//         );
//         assert!(EraElectionStatus::<T>::exists(), "EraElectionStatus storage item not found!");
//         Ok(())
//     }
//
//     /// Migrate storage to v6.
//     pub fn migrate<T: Config>() -> Weight {
//         log!(info, "Migrating staking to ObsoleteReleases::V6_0_0");
//
//         SnapshotValidators::<T>::kill();
//         SnapshotNominators::<T>::kill();
//         QueuedElected::<T>::kill();
//         QueuedScore::<T>::kill();
//         EraElectionStatus::<T>::kill();
//         IsCurrentSessionFinal::<T>::kill();
//
//         StorageVersion::<T>::put(ObsoleteReleases::V6_0_0);
//
//         log!(info, "Done.");
//         T::DbWeight::get().writes(6 + 1)
//     }
// }

pub mod v1 {
	use frame_support::{pallet_prelude::ValueQuery, storage_alias};

	use super::*;
	use crate::staking;

	/// A record of the nominations made by a specific account.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct Nominations<AccountId> {
		/// The targets of nomination.
		pub targets: Vec<AccountId>,
		/// The era the nominations were submitted.
		pub submitted_in: EraIndex,
		/// Whether the nominations have been suppressed.
		pub suppressed: bool,
	}

	#[storage_alias]
	pub type Nominators<T: Config> = CountedStorageMap<
		Pallet<T>,
		Twox64Concat,
		<T as frame_system::Config>::AccountId,
		Nominations<<T as frame_system::Config>::AccountId>,
	>;

	#[storage_alias]
	pub type StorageVersion<T: Config> = StorageValue<Pallet<T>, u32>;

	pub struct MigrateToV1<T>(PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			// Todo: double check unwrap
			if StorageVersion::<T>::get().unwrap() == 0u32 {
				let count = Nominators::<T>::count();

				log::info!(
					target: LOG_TARGET,
					" >>> Updating Nominators storage. Migrating {} Nominations...", count
				);

				// Todo: use existing implementation
				let now = <Pallet<T>>::current_era().unwrap_or(Zero::zero());
				Nominators::<T>::translate::<Vec<T::AccountId>, _>(|key, targets| {
					Some(Nominations { targets, submitted_in: now, suppressed: false })
				});

				StorageVersion::<T>::put(1u32);

				let count = Nominators::<T>::count();
				log::info!(
					target: LOG_TARGET,
					" <<< Nominators storage updated! Migrated {} Nominations ✅", count
				);

				T::DbWeight::get().reads_writes((&count + 2) as u64, (count + 1) as u64)
			} else {
				log::info!(target: LOG_TARGET, " >>> Unused migration!");
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
			ensure!(StorageVersion::<T>::get() == 0, "Expected v0 before upgrading to v1");

			if Nominators::<T>::exists() {
				ensure!(
					T::Nominators::get() == Nominators::<T>::get(),
					"Provided value of Nominators should be same as the existing storage value"
				);
			} else {
				log::info!("No Nominators in storage; nothing to migrate");
			}

			let prev_count = Nominators::<T>::get().count();

			Ok(prev_count.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(prev_state: Vec<u8>) -> Result<(), TryRuntimeError> {
			// frame_support::ensure!(
			// 	StorageVersion::<T>::get() == ObsoleteReleases::V12_0_0,
			// 	"v12 not applied"
			// );
			// Ok(())

			let (prev_count): (u64) = Decode::decode(&mut &prev_state[..])
				.expect("pre_upgrade provides a valid state; qed");

			let post_count = Nominators::<T>::get().count();

			ensure!(
				prev_count == post_count,
				"Nominations count before and after the migration must be the same"
			);

			let current_version = Pallet::<T>::current_storage_version();
			let on_chain_version = Pallet::<T>::on_chain_storage_version();

			ensure!(current_version == ObsoleteReleases::V1_0_0Ancient, "must upgrade");
			ensure!(
				current_version == on_chain_version,
				"after migration, the current_version and on_chain_version should be the same"
			);

			Nominators::<T>::iter().try_for_each(|(_id, bucket)| -> Result<(), &'static str> {
				ensure!(
					!bucket.is_removed,
					"At this point all the bucket should have is_removed set to false"
				);
				Ok(())
			})?;

			Ok(())
		}
	}
}
