use sp_runtime_interface::runtime_interface;
use sp_wasm_interface::{Pointer, Value, WordSize};

pub type MemoryId = u32;
/// Something that provides access to the sandbox.
#[runtime_interface]
pub trait Sandbox {
	/// Get sandbox memory from the `memory_id` instance at `offset` into the given buffer.
	fn memory_get(
		&mut self,
		_memory_id: MemoryId,
		_offset: WordSize,
		_buf_ptr: Pointer<u8>,
		_buf_len: WordSize,
	) -> u32 {
		return 0;
	}
	/// Set sandbox memory from the given value.
	fn memory_set(
		&mut self,
		_memory_id: MemoryId,
		_offset: WordSize,
		_val_ptr: Pointer<u8>,
		_val_len: WordSize,
	) -> u32 {
		return 0;
	}
	/// Delete a memory instance.
	fn memory_teardown(&mut self, _memory_id: MemoryId) {
		return;
	}
	/// Create a new memory instance with the given `initial` size and the `maximum` size.
	/// The size is given in wasm pages.
	fn memory_new(&mut self, _initial: u32, _maximum: u32) -> u32 {
		return 0;
	}
	/// Invoke an exported function by a name.
	fn invoke(
		&mut self,
		_instance_id: u32,
		_export_name: &str,
		_args: &[u8],
		_return_val: Pointer<u8>,
		_return_val_len: WordSize,
		_state: u32,
	) -> u32 {
		return 0;
	}
	/// Delete a sandbox instance.
	fn instance_teardown(&mut self, _instance_id: u32) {
		return;
	}

	/// Get the value from a global with the given `name`. The sandbox is determined by the
	/// given `instance_idx` instance.
	///
	/// Returns `Some(_)` when the requested global variable could be found.
	fn get_global_val(&self, _instance_idx: u32, _name: &str) -> Option<Value> {
		return Some(Value::I32(0));
	}

	/// Instantiate a new sandbox instance with the given `wasm_code`.
	fn instantiate(
		&mut self,
		_dispatch_thunk: u32,
		_wasm_code: &[u8],
		_env_def: &[u8],
		_state_ptr: Pointer<u8>,
	) -> u32 {
		return 0;
	}
}
