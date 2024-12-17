use codec::Encode;
pub use sp_io::offchain::{
	local_storage_clear, local_storage_compare_and_set, local_storage_get, local_storage_set,
};
use sp_runtime::offchain::StorageKind;
use sp_std::prelude::Vec;

pub const LOCKED_VALUE: &[u8] = &[1];
pub const PREFIX: &[u8] = b"ocwmu";
pub const RESET_VALUE: &[u8] = &[0];

/// Intended for allowing only one instance of an off-chain worker with the same ID to run at the
/// same time.
///
/// It takes advantage of the off-chain storage and supports state reset by an
/// [offchain_localStorageSet](https://polkadot.js.org/docs/kusama/rpc#localstoragesetkind-storagekind-key-bytes-value-bytes-null)
/// RPC zeroing the `ocwmu{id}` key to fix a [poisoned](https://doc.rust-lang.org/std/sync/struct.Mutex.html#poisoning)
/// persistent key. Make sure there is no running instance of the OCW when resetting the state.
pub struct OcwMutex {
	key: Vec<u8>,
	locked: bool,
}

impl OcwMutex {
	pub fn new(id: Vec<u8>) -> Self {
		Self { key: (PREFIX, id).encode(), locked: false }
	}

	pub fn try_lock(&mut self) -> bool {
		if self.locked {
			return false;
		}

		if !local_storage_compare_and_set(StorageKind::PERSISTENT, &self.key, None, LOCKED_VALUE) {
			match local_storage_get(StorageKind::PERSISTENT, &self.key) {
				Some(value) if value == RESET_VALUE => {
					log::warn!("OCW mutex key '{:?}' cleared.", &self.key);
					local_storage_clear(StorageKind::PERSISTENT, &self.key);

					return self.try_lock();
				},
				_ => return false,
			}
		}

		self.locked = true;

		true
	}

	pub fn local_storage_key(&self) -> &[u8] {
		&self.key
	}
}

impl Drop for OcwMutex {
	fn drop(&mut self) {
		if self.locked {
			local_storage_clear(StorageKind::PERSISTENT, &self.key);
		}
	}
}
