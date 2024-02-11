//! storage migrations for pallet
#![allow(clippy::type_complexity)]
use frame_support::{
	log, pallet_prelude::*, traits::OnRuntimeUpgrade, weights::Weight, DefaultNoBound,
};
use pallet_contracts::migration::{IsFinished, MigrationStep};
use sp_std::marker::PhantomData;
#[cfg(feature = "try-runtime")]
use sp_std::{vec, vec::Vec};

#[derive(Encode, Decode, MaxEncodedLen, DefaultNoBound)]
pub struct ContractsMigration<Runtime>(PhantomData<Runtime>);

impl<Runtime> OnRuntimeUpgrade for ContractsMigration<Runtime>
where
	Runtime: frame_system::Config + pallet_contracts::Config,
{
	fn on_runtime_upgrade() -> Weight {
		let version = StorageVersion::get::<pallet_contracts::Pallet<Runtime>>();
		log::info!("Checking migration for contracts at {:?}", version);
		StorageVersion::new(10).put::<pallet_contracts::Pallet<Runtime>>();
		Weight::zero()
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		Ok(vec![])
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		Ok(())
	}
}

impl<Runtime> MigrationStep for ContractsMigration<Runtime>
where
	Runtime: frame_system::Config + pallet_contracts::Config,
{
	const VERSION: u16 = 10;

	fn max_step_weight() -> Weight {
		Weight::zero()
	}

	fn step(&mut self) -> (IsFinished, Weight) {
		(IsFinished::Yes, Weight::zero())
	}
}
