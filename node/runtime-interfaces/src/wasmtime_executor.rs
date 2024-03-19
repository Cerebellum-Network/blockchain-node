use sc_executor_wasmtime::WasmtimeRuntime;
use sp_wasm_interface::{Pointer, Result as WResult, Result as SandboxResult, Value, WordSize};
use wasmtime::Func;

use crate::{
	sandbox_instance::{MemoryId, Store},
	util::Sandbox,
};

// The sandbox store is inside of a Option<Box<..>>> so that we can temporarily borrow it.
struct SandboxStore(Option<Box<Store<Func>>>);

// There are a bunch of `Rc`s within the sandbox store, however we only manipulate
// those within one thread so this should be safe.
unsafe impl Send for SandboxStore {}

// impl<'a> Sandbox for HostContext<'a> {
// 	fn memory_get(
// 		&mut self,
// 		memory_id: MemoryId,
// 		offset: WordSize,
// 		buf_ptr: Pointer<u8>,
// 		buf_len: WordSize,
// 	) -> SandboxResult<u32> {
// 		unimplemented!()
// 	}

// 	/// Set sandbox memory from the given value.
// 	fn memory_set(
// 		&mut self,
// 		memory_id: MemoryId,
// 		offset: WordSize,
// 		val_ptr: Pointer<u8>,
// 		val_len: WordSize,
// 	) -> SandboxResult<u32> {
// 		unimplemented!()
// 	}
// 	/// Delete a memory instance.
// 	fn memory_teardown(&mut self, memory_id: MemoryId) -> SandboxResult<()> {
// 		unimplemented!()
// 	}
// 	/// Create a new memory instance with the given `initial` size and the `maximum` size.
// 	/// The size is given in wasm pages.
// 	fn memory_new(&mut self, initial: u32, maximum: u32) -> SandboxResult<MemoryId> {
// 		unimplemented!()
// 	}
// 	/// Invoke an exported function by a name.
// 	fn invoke(
// 		&mut self,
// 		instance_id: u32,
// 		export_name: &str,
// 		args: &[u8],
// 		return_val: Pointer<u8>,
// 		return_val_len: WordSize,
// 		state: u32,
// 	) -> SandboxResult<u32> {
// 		unimplemented!()
// 	}
// 	/// Delete a sandbox instance.
// 	fn instance_teardown(&mut self, instance_id: u32) -> SandboxResult<()> {
// 		unimplemented!()
// 	}
// 	/// Create a new sandbox instance.
// 	fn instance_new(
// 		&mut self,
// 		dispatch_thunk_id: u32,
// 		wasm: &[u8],
// 		raw_env_def: &[u8],
// 		state: u32,
// 	) -> SandboxResult<u32> {
// 		unimplemented!()
// 	}

// 	/// Get the value from a global with the given `name`. The sandbox is determined by the
// 	/// given `instance_idx` instance.
// 	///
// 	/// Returns `Some(_)` when the requested global variable could be found.
// 	fn get_global_val(&self, instance_idx: u32, name: &str) -> SandboxResult<Option<Value>> {
// 		unimplemented!()
// 	}
// }
