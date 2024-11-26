use sp_runtime_interface::runtime_interface;
use sp_wasm_interface::{Pointer, Result as SandboxResult, Value, WordSize,  };
pub type MemoryId = u32;
use std::{cell::RefCell, rc::Rc, str, sync::Arc};
mod sandbox_util;
mod freeing_bump;
mod env;
mod util;
mod sandbox_interface;
mod wasmi_backend;
use crate::sandbox_util::Store;
use sp_wasm_interface::Function;
use wasmi::TableRef;
use wasmi::MemoryRef;
use crate::freeing_bump::FreeingBumpHeapAllocator;
use crate::sandbox_interface::Sandbox as SandboxI;

#[cfg(feature = "std")]
use sp_externalities::{Externalities, ExternalitiesExt};

pub use sp_externalities::MultiRemovalResults;

sp_externalities::decl_extension! {
    pub struct SandboxExt(FunctionExecutor);
}

/// Something that provides access to the sandbox.
#[runtime_interface]
pub trait Sandbox {
	/// Get sandbox memory from the `memory_id` instance at `offset` into the given buffer.
	fn memory_get(
		&mut self,
		memory_idx: u32,
		offset: u32,
		buf_ptr: Pointer<u8>,
		buf_len: u32,
	) -> u32 {
		log::info!("Going through the memory_get function");
		let sandbox = self.extension::<SandboxExt>().expect("memory_get: Sandbox should have been initialized before; qed");

		sandbox
			.memory_get(memory_idx, offset, buf_ptr, buf_len)
			.expect("Failed to get memory with sandbox")
		// return 0;
	}
	/// Set sandbox memory from the given value.
	fn memory_set(
		&mut self,
		memory_idx: u32,
		offset: u32,
		val_ptr: Pointer<u8>,
		val_len: u32,
	) -> u32 {
		log::info!("Going through the memory_set function");
		let sandbox = self.extension::<SandboxExt>().expect("memory_set: Sandbox should have been initialized before; qed");

		sandbox
			.memory_set(memory_idx, offset, val_ptr, val_len)
			.expect("Failed to set memory with sandbox")
		// return 0;
	}
	/// Delete a memory instance.
	fn memory_teardown(&mut self, memory_idx: u32) {
		log::info!("Going through the memory_teardown function");
		let sandbox = self.extension::<SandboxExt>().expect("memory_teardown: Sandbox should have been initialized before; qed");

		sandbox
			.memory_teardown(memory_idx)
			.expect("Failed to teardown memory with sandbox")
	}
	/// Create a new memory instance with the given `initial` size and the `maximum` size.
	/// The size is given in wasm pages.
	fn memory_new(&mut self, initial: u32, maximum: u32) -> u32 {
		log::info!("Going through the memory_new function");
		let sandbox = self.extension::<SandboxExt>().expect("memory_new: Sandbox should have been initialized before; qed");


		sandbox
			.memory_new(initial, maximum)
			.expect("Failed to create new memory with sandbox")
	}
	/// Invoke an exported function by a name.
	fn invoke(
		&mut self,
		instance_idx: u32,
		function: &str,
		args: &[u8],
		return_val_ptr: Pointer<u8>,
		return_val_len: u32,
		state_ptr: Pointer<u8>,
	) -> u32 {
		log::info!("Going through the invoke function");
		let sandbox = self.extension::<SandboxExt>().expect("invoke: Sandbox should have been initialized before; qed");

		sandbox
			.invoke(instance_idx, function, args, return_val_ptr, return_val_len, state_ptr.into())
			.expect("Failed to invoke function with sandbox")
		// return 0;
	}
	/// Delete a sandbox instance.
	fn instance_teardown(&mut self, instance_idx: u32) {
		log::info!("Going through the instance_teardown function");
		let sandbox = self.extension::<SandboxExt>().expect("instance_teardown: Sandbox should have been initialized before; qed");

		sandbox
			.instance_teardown(instance_idx)
			.expect("Failed to teardown sandbox instance")
	}

	/// Get the value from a global with the given `name`. The sandbox is determined by the
	/// given `instance_idx` instance.
	///
	/// Returns `Some(_)` when the requested global variable could be found.
	fn get_global_val(
		&mut self,
		instance_idx: u32,
		name: &str,
	) -> Option<sp_wasm_interface::Value> {
		log::info!("Going through the get_global_val function");
		let sandbox = self.extension::<SandboxExt>().expect("get_global_val: Sandbox should have been initialized before; qed");

		sandbox
			.get_global_val(instance_idx, name)
			.expect("Failed to get global from sandbox")
		// return Some(Value::I32(0));
	}

	/// Instantiate a new sandbox instance with the given `wasm_code`.
	fn instantiate(
		&mut self,
		dispatch_thunk: u32,
		wasm_code: &[u8],
		env_def: &[u8],
		state_ptr: Pointer<u8>,
	) -> u32 {
		log::info!("Going through the instantiate function");
		let sandbox = self.extension::<SandboxExt>().expect("instantiate: Sandbox should have been initialized before; qed");

		// let sandbox = instance_new(dispatch_thunk, wasm_code, env_def, state_ptr);

		sandbox
			.instance_new(dispatch_thunk, wasm_code, env_def, state_ptr.into())
			.expect("Failed to instantiate a new sandbox")

		// return 0;
	}
}

pub struct FunctionExecutor {
	sandbox_store: Rc<RefCell<Store<wasmi::FuncRef>>>,
	heap: RefCell<FreeingBumpHeapAllocator>,
	memory: MemoryRef,
	table: Option<TableRef>,
	host_functions: Arc<Vec<&'static dyn Function>>,
	allow_missing_func_imports: bool,
	missing_functions: Arc<Vec<String>>,
	panic_message: Option<String>,
}

unsafe impl Send for FunctionExecutor{}

// pub struct SandboxTest {}
//
// /// Something that provides access to the sandbox.
// pub trait SandboxT {
// 	/// Get sandbox memory from the `memory_id` instance at `offset` into the given buffer.
// 	fn memory_get(
// 		&mut self,
// 		memory_id: MemoryId,
// 		offset: WordSize,
// 		buf_ptr: Pointer<u8>,
// 		buf_len: WordSize,
// 	) -> SandboxResult<u32>;
// 	/// Set sandbox memory from the given value.
// 	fn memory_set(
// 		&mut self,
// 		memory_id: MemoryId,
// 		offset: WordSize,
// 		val_ptr: Pointer<u8>,
// 		val_len: WordSize,
// 	) -> SandboxResult<u32>;
// 	/// Delete a memory instance.
// 	fn memory_teardown(&mut self, memory_id: MemoryId) -> SandboxResult<()>;
// 	/// Create a new memory instance with the given `initial` size and the `maximum` size.
// 	/// The size is given in wasm pages.
// 	fn memory_new(&mut self, initial: u32, maximum: u32) -> SandboxResult<MemoryId>;
// 	/// Invoke an exported function by a name.
// 	fn invoke(
// 		&mut self,
// 		instance_id: u32,
// 		export_name: &str,
// 		args: &[u8],
// 		return_val: Pointer<u8>,
// 		return_val_len: WordSize,
// 		state: u32,
// 	) -> SandboxResult<u32>;
// 	/// Delete a sandbox instance.
// 	fn instance_teardown(&mut self, instance_id: u32) -> SandboxResult<()>;
// 	/// Create a new sandbox instance.
// 	// fn instance_new(
// 	// 	&mut self,
// 	// 	dispatch_thunk_id: u32,
// 	// 	wasm: &[u8],
// 	// 	raw_env_def: &[u8],
// 	// 	state: u32,
// 	// ) -> Result<u32>;
//
// 	/// Get the value from a global with the given `name`. The sandbox is determined by the
// 	/// given `instance_idx` instance.
// 	///
// 	/// Returns `Some(_)` when the requested global variable could be found.
// 	fn get_global_val(&self, instance_idx: u32, name: &str) -> SandboxResult<Option<Value>>;
//
// 	/// Instantiate a new sandbox instance with the given `wasm_code`.
// 	fn instantiate(
// 		&mut self,
// 		dispatch_thunk: u32,
// 		wasm_code: &[u8],
// 		env_def: &[u8],
// 		state_ptr: Pointer<u8>,
// 	) -> u32;
// }
//
// impl SandboxT for SandboxTest {
// 	fn memory_get(&mut self, memory_id: MemoryId, offset: WordSize, buf_ptr: Pointer<u8>, buf_len: WordSize) -> SandboxResult<u32> {
// 		todo!()
// 	}
//
// 	fn memory_set(&mut self, memory_id: MemoryId, offset: WordSize, val_ptr: Pointer<u8>, val_len: WordSize) -> SandboxResult<u32> {
// 		todo!()
// 	}
//
// 	fn memory_teardown(&mut self, memory_id: MemoryId) -> SandboxResult<()> {
// 		todo!()
// 	}
//
// 	fn memory_new(&mut self, initial: u32, maximum: u32) -> SandboxResult<MemoryId> {
// 		todo!()
// 	}
//
// 	fn invoke(&mut self, instance_id: u32, export_name: &str, args: &[u8], return_val: Pointer<u8>, return_val_len: WordSize, state: u32) -> SandboxResult<u32> {
// 		todo!()
// 	}
//
// 	fn instance_teardown(&mut self, instance_id: u32) -> SandboxResult<()> {
// 		todo!()
// 	}
//
// 	fn get_global_val(&self, instance_idx: u32, name: &str) -> SandboxResult<Option<Value>> {
// 		todo!()
// 	}
//
// 	fn instantiate(&mut self, dispatch_thunk: u32, wasm_code: &[u8], env_def: &[u8], state_ptr: Pointer<u8>) -> u32 {
// 		todo!()
// 	}
// }
