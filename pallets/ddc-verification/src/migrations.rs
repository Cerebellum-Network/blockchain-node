use frame_support::{migration, pallet_prelude::*, traits::OnRuntimeUpgrade};
use log;

use super::*;

const LOG_TARGET: &str = "ddc-verification";

pub mod v1 {
	use super::*;

	pub fn migrate_to_v1<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		log::info!(
			target: LOG_TARGET,
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 0 && current_version == 1 {
			log::info!(target: LOG_TARGET, "Running migration to v1.");

			let res = migration::clear_storage_prefix(
				<Pallet<T>>::name().as_bytes(),
				b"ClusterToValidate",
				b"",
				None,
				None,
			);

			log::info!(
				target: LOG_TARGET,
				"Cleared '{}' entries from 'ClusterToValidate' storage prefix.",
				res.unique
			);

			if res.maybe_cursor.is_some() {
				log::error!(
					target: LOG_TARGET,
					"Storage prefix 'ClusterToValidate' is not completely cleared."
				);
			}

			// Update storage version.
			StorageVersion::new(1).put::<Pallet<T>>();
			log::info!(
				target: LOG_TARGET,
				"Storage migrated to version {:?}",
				current_version
			);

			T::DbWeight::get().reads_writes(1, res.unique.into())
		} else {
			log::info!(target: LOG_TARGET, " >>> Unused migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v1::<T>()
		}
	}
}

pub mod v2 {
	use super::*;

	pub fn migrate_to_v2<T: Config>() -> Weight {
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		let current_version = Pallet::<T>::in_code_storage_version();

		log::info!(
			target: LOG_TARGET,
			"Running migration with current storage version {:?} / onchain {:?}",
			current_version,
			on_chain_version
		);

		if on_chain_version == 1 && current_version == 2 {
			log::info!(target: LOG_TARGET, "Running migration to v2.");

			let res = migration::clear_storage_prefix(
				<Pallet<T>>::name().as_bytes(),
				b"EraValidations",
				b"",
				None,
				None,
			);

			log::info!(
				target: LOG_TARGET,
				"Cleared '{}' entries from 'EraValidations' storage prefix.",
				res.unique
			);

			if res.maybe_cursor.is_some() {
				log::error!(
					target: LOG_TARGET,
					"Storage prefix 'EraValidations' is not completely cleared."
				);
			}

			// Update storage version.
			StorageVersion::new(2).put::<Pallet<T>>();
			log::info!(
				target: LOG_TARGET,
				"Storage migrated to version {:?}",
				current_version
			);

			T::DbWeight::get().reads_writes(1, res.unique.into())
		} else {
			log::info!(target: LOG_TARGET, " >>> Unused migration!");
			T::DbWeight::get().reads(1)
		}
	}

	pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
		fn on_runtime_upgrade() -> Weight {
			migrate_to_v2::<T>()
		}
	}
}
