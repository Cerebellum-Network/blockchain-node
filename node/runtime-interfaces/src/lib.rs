use sp_runtime_interface::runtime_interface;
use sp_wasm_interface::{Pointer, Result as SandboxResult, Value, WordSize};
pub type MemoryId = u32;
use std::{cell::RefCell, rc::Rc, str, sync::Arc};
mod env;
mod freeing_bump;
mod sandbox_interface;
mod sandbox_util;
mod util;
mod wasmi_backend;
use crate::freeing_bump::FreeingBumpHeapAllocator;
use crate::sandbox_interface::Sandbox as SandboxI;
use crate::sandbox_util::{SandboxBackend, Store};
use sp_wasm_interface::Function;
use wasmi::memory_units::Pages;
use wasmi::MemoryRef;
use wasmi::TableInstance;
use wasmi::TableRef;

#[cfg(feature = "std")]
use sp_externalities::ExternalitiesExt;

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
		let sandbox = self
			.extension::<SandboxExt>()
			.expect("memory_get: Sandbox should have been initialized before; qed");

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
		let sandbox = self
			.extension::<SandboxExt>()
			.expect("memory_set: Sandbox should have been initialized before; qed");

		sandbox
			.memory_set(memory_idx, offset, val_ptr, val_len)
			.expect("Failed to set memory with sandbox")
		// return 0;
	}
	/// Delete a memory instance.
	fn memory_teardown(&mut self, memory_idx: u32) {
		log::info!("Going through the memory_teardown function");
		let sandbox = self
			.extension::<SandboxExt>()
			.expect("memory_teardown: Sandbox should have been initialized before; qed");

		sandbox
			.memory_teardown(memory_idx)
			.expect("Failed to teardown memory with sandbox")
	}
	/// Create a new memory instance with the given `initial` size and the `maximum` size.
	/// The size is given in wasm pages.
	fn memory_new(&mut self, initial: u32, maximum: u32) -> u32 {
		log::info!("Going through the memory_new function");

		let function_executor: FunctionExecutor = FunctionExecutor::new(initial, maximum);
		let _ = self.register_extension(SandboxExt::from(function_executor));

		log::info!("Extension should be registered");

		let sandbox = self
			.extension::<SandboxExt>()
			.expect("memory_new: Sandbox should have been initialized before; qed");

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
		let sandbox = self
			.extension::<SandboxExt>()
			.expect("invoke: Sandbox should have been initialized before; qed");

		sandbox
			.invoke(instance_idx, function, args, return_val_ptr, return_val_len, state_ptr.into())
			.expect("Failed to invoke function with sandbox")
		// return 0;
	}
	/// Delete a sandbox instance.
	fn instance_teardown(&mut self, instance_idx: u32) {
		log::info!("Going through the instance_teardown function");
		let sandbox = self
			.extension::<SandboxExt>()
			.expect("instance_teardown: Sandbox should have been initialized before; qed");

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
		let sandbox = self
			.extension::<SandboxExt>()
			.expect("get_global_val: Sandbox should have been initialized before; qed");

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
		let sandbox = self
			.extension::<SandboxExt>()
			.expect("instantiate: Sandbox should have been initialized before; qed");

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
impl FunctionExecutor {
	pub fn new(initial: u32, maximum: u32) -> Self {
		let sandbox_backend = SandboxBackend::Wasmi;
		FunctionExecutor {
			sandbox_store: Rc::new(RefCell::new(Store::new(sandbox_backend))),
			heap: RefCell::new(FreeingBumpHeapAllocator::new(0)), // Replace `0` with appropriate heap base.
			memory: wasmi::MemoryInstance::alloc(
				Pages(initial.try_into().unwrap()),       // Initial size in pages.
				Some(Pages(maximum.try_into().unwrap())), // Maximum size in pages.
			)
			.expect("Failed to allocate memory instance"),
			table: Some(TableInstance::alloc(100, Some(1000)).unwrap()), // Default table size.
			host_functions: Arc::new(Vec::new()),
			allow_missing_func_imports: false,
			missing_functions: Arc::new(Vec::new()),
			panic_message: None,
		}
	}
}

unsafe impl Send for FunctionExecutor {}
