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
pub struct RemoveSocietyPallet<Runtime>(PhantomData<Runtime>);

impl<Runtime> OnRuntimeUpgrade for RemoveSocietyPallet<Runtime>
where
	Runtime: frame_system::Config,
{
	fn on_runtime_upgrade() -> Weight {
		clear_prefix(&twox_128(b"Society"), None);
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
