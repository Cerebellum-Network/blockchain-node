use sp_runtime_interface::runtime_interface;
use sp_wasm_interface::{Pointer, Result as SandboxResult, Value, WordSize};

pub type MemoryId = u32;
/// Something that provides access to the sandbox.
#[runtime_interface]
pub trait Sandbox {
	/// Get sandbox memory from the `memory_id` instance at `offset` into the given buffer.
	fn memory_get(
		&mut self,
		memory_id: MemoryId,
		offset: WordSize,
		buf_ptr: Pointer<u8>,
		buf_len: WordSize,
	) -> SandboxResult<u32> {
		return Ok(0);
	}
	/// Set sandbox memory from the given value.
	fn memory_set(
		&mut self,
		memory_id: MemoryId,
		offset: WordSize,
		val_ptr: Pointer<u8>,
		val_len: WordSize,
	) -> SandboxResult<u32> {
		return Ok(0);
	}
	/// Delete a memory instance.
	fn memory_teardown(&mut self, memory_id: MemoryId) -> SandboxResult<()> {
		return Ok(());
	}
	/// Create a new memory instance with the given `initial` size and the `maximum` size.
	/// The size is given in wasm pages.
	fn memory_new(&mut self, initial: u32, maximum: u32) -> SandboxResult<MemoryId> {
		return Ok(0);
	}
	/// Invoke an exported function by a name.
	fn invoke(
		&mut self,
		instance_id: u32,
		export_name: &str,
		args: &[u8],
		return_val: Pointer<u8>,
		return_val_len: WordSize,
		state: u32,
	) -> SandboxResult<u32> {
		return Ok(0);
	}
	/// Delete a sandbox instance.
	fn instance_teardown(&mut self, instance_id: u32) -> SandboxResult<()> {
		return Ok(());
	}
	/// Create a new sandbox instance.
	fn instance_new(
		&mut self,
		dispatch_thunk_id: u32,
		wasm: &[u8],
		raw_env_def: &[u8],
		state: u32,
	) -> SandboxResult<u32> {
		return Ok(0);
	}
	/// Get the value from a global with the given `name`. The sandbox is determined by the
	/// given `instance_idx` instance.
	///
	/// Returns `Some(_)` when the requested global variable could be found.
	fn get_global_val(&self, instance_idx: u32, name: &str) -> SandboxResult<Option<Value>> {
		return Ok(Some(Value::I32(0)));
	}

	/// Instantiate a guest module and return it's index in the store.
	fn instantiate(
		&mut self,
		wasm: &[u8],
		guest_env: &[u8],
		state: u32,
		sandbox_context: &[u8],
	) -> SandboxResult<()> {
		return Ok(());
	}
}
