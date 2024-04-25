use std::{any::Any, cell::RefCell, rc::Rc, str, sync::Arc};

use codec::{Decode, Encode};
use sc_executor::error::{Error, Result};
use sc_executor_common::{
	error::{MessageWithBacktrace, WasmError},
	runtime_blob::{DataSegmentsSnapshot, RuntimeBlob},
	wasm_runtime::{AllocationStats, InvokeMethod, WasmInstance, WasmModule},
};
use sp_runtime_interface::unpack_ptr_and_len;
use sp_wasm_interface::{Function, FunctionContext, Pointer, Result as WResult, Value, WordSize};
use wasmi::{
	memory_units::Pages, Error as WasmiError, FuncInstance, ImportsBuilder, MemoryInstance,
	MemoryRef, Module, ModuleInstance, ModuleRef, RuntimeValue, TableRef,
};

use crate::{
	freeing_bump::FreeingBumpHeapAllocator,
	sandbox_instance::{
		GuestEnvironment, InstantiationError, MemoryId, SandboxBackend, SandboxContext, Store,
		SupervisorFuncIndex, ERR_EXECUTION, ERR_MODULE, ERR_OK, ERR_OUT_OF_BOUNDS,
	},
	sandbox_wasmi_backend::trap,
	util::MemoryTransfer,
};

const LOG_TARGET: &str = "wasmi_function_executor";

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
unsafe impl Send for FunctionExecutor {}

impl FunctionExecutor {
	pub fn new(
		m: MemoryRef,
		heap_base: u32,
		t: Option<TableRef>,
		host_functions: Arc<Vec<&'static dyn Function>>,
		allow_missing_func_imports: bool,
		missing_functions: Arc<Vec<String>>,
	) -> Result<Self> {
		Ok(FunctionExecutor {
			sandbox_store: Rc::new(RefCell::new(Store::new(SandboxBackend::Wasmi))),
			heap: RefCell::new(FreeingBumpHeapAllocator::new(heap_base)),
			memory: m,
			table: t,
			host_functions,
			allow_missing_func_imports,
			missing_functions,
			panic_message: None,
		})
	}
}

impl FunctionContext for FunctionExecutor {
	fn read_memory_into(&self, address: Pointer<u8>, dest: &mut [u8]) -> WResult<()> {
		self.memory.get_into(address.into(), dest).map_err(|e| e.to_string())
	}

	fn write_memory(&mut self, address: Pointer<u8>, data: &[u8]) -> WResult<()> {
		self.memory.set(address.into(), data).map_err(|e| e.to_string())
	}

	fn allocate_memory(&mut self, size: WordSize) -> WResult<Pointer<u8>> {
		let heap = &mut self.heap.borrow_mut();
		self.memory
			.with_direct_access_mut(|mem| heap.allocate(mem, size).map_err(|e| e.to_string()))
	}

	fn deallocate_memory(&mut self, ptr: Pointer<u8>) -> WResult<()> {
		let heap = &mut self.heap.borrow_mut();
		self.memory
			.with_direct_access_mut(|mem| heap.deallocate(mem, ptr).map_err(|e| e.to_string()))
	}

	fn register_panic_error_message(&mut self, message: &str) {
		self.panic_message = Some(message.to_owned());
	}
}

impl wasmi::Externals for FunctionExecutor {
	fn invoke_index(
		&mut self,
		index: usize,
		args: wasmi::RuntimeArgs,
	) -> std::result::Result<Option<wasmi::RuntimeValue>, wasmi::Trap> {
		let mut args = args.as_ref().iter().copied().map(|value| match value {
			wasmi::RuntimeValue::I32(val) => Value::I32(val),
			wasmi::RuntimeValue::I64(val) => Value::I64(val),
			wasmi::RuntimeValue::F32(val) => Value::F32(val.into()),
			wasmi::RuntimeValue::F64(val) => Value::F64(val.into()),
		});

		if let Some(function) = self.host_functions.clone().get(index) {
			function
				.execute(self, &mut args)
				// .map_err(|msg| Error::FunctionExecution(function.name().to_string(), msg))
				.map_err(|_msg| trap("Function call failed"))
				.map_err(wasmi::Trap::from)
				.map(|v| {
					v.map(|value| match value {
						Value::I32(val) => RuntimeValue::I32(val),
						Value::I64(val) => RuntimeValue::I64(val),
						Value::F32(val) => RuntimeValue::F32(val.into()),
						Value::F64(val) => RuntimeValue::F64(val.into()),
					})
				})
		} else if self.allow_missing_func_imports &&
			index >= self.host_functions.len() &&
			index < self.host_functions.len() + self.missing_functions.len()
		{
			Err(trap("Function is only a stub. Calling a stub is not allowed."))
		} else {
			Err(trap("Could not find host function with index"))
		}
	}
}

struct SandboxContextImpl<'a> {
	executor: &'a mut FunctionExecutor,
	dispatch_thunk: wasmi::FuncRef,
}

impl<'a> SandboxContext for SandboxContextImpl<'a> {
	fn invoke(
		&mut self,
		invoke_args_ptr: Pointer<u8>,
		invoke_args_len: WordSize,
		state: u32,
		func_idx: SupervisorFuncIndex,
	) -> Result<i64> {
		let result = wasmi::FuncInstance::invoke(
			&self.dispatch_thunk,
			&[
				RuntimeValue::I32(u32::from(invoke_args_ptr) as i32),
				RuntimeValue::I32(invoke_args_len as i32),
				RuntimeValue::I32(state as i32),
				RuntimeValue::I32(usize::from(func_idx) as i32),
			],
			self.executor,
		);

		match result {
			Ok(Some(RuntimeValue::I64(val))) => Ok(val),
			Ok(_) => Err("Supervisor function returned unexpected result!".into()),
			Err(err) => Err(Error::Other(err.to_string())),
		}
	}

	fn supervisor_context(&mut self) -> &mut dyn FunctionContext {
		self.executor
	}
}

// impl Sandbox for FunctionExecutor {
impl FunctionExecutor {
	pub fn memory_get(
		&mut self,
		memory_id: MemoryId,
		offset: WordSize,
		buf_ptr: Pointer<u8>,
		buf_len: WordSize,
	) -> WResult<u32> {
		let sandboxed_memory =
			self.sandbox_store.borrow().memory(memory_id).map_err(|e| e.to_string())?;

		let len = buf_len as usize;

		let buffer = match sandboxed_memory.read(Pointer::new(offset as u32), len) {
			Err(_) => return Ok(ERR_OUT_OF_BOUNDS),
			Ok(buffer) => buffer,
		};

		if self.memory.set(buf_ptr.into(), &buffer).is_err() {
			return Ok(ERR_OUT_OF_BOUNDS)
		}

		Ok(ERR_OK)
	}

	pub fn memory_set(
		&mut self,
		memory_id: MemoryId,
		offset: WordSize,
		val_ptr: Pointer<u8>,
		val_len: WordSize,
	) -> WResult<u32> {
		let sandboxed_memory =
			self.sandbox_store.borrow().memory(memory_id).map_err(|e| e.to_string())?;

		let len = val_len as usize;

		#[allow(deprecated)]
		let buffer = match self.memory.get(val_ptr.into(), len) {
			Err(_) => return Ok(ERR_OUT_OF_BOUNDS),
			Ok(buffer) => buffer,
		};

		if sandboxed_memory.write_from(Pointer::new(offset as u32), &buffer).is_err() {
			return Ok(ERR_OUT_OF_BOUNDS)
		}

		Ok(ERR_OK)
	}

	pub fn memory_teardown(&mut self, memory_id: MemoryId) -> WResult<()> {
		self.sandbox_store
			.borrow_mut()
			.memory_teardown(memory_id)
			.map_err(|e| e.to_string())
	}

	pub fn memory_new(&mut self, initial: u32, maximum: u32) -> WResult<MemoryId> {
		self.sandbox_store
			.borrow_mut()
			.new_memory(initial, maximum)
			.map_err(|e| e.to_string())
	}

	pub fn invoke(
		&mut self,
		instance_id: u32,
		export_name: &str,
		mut args: &[u8],
		return_val: Pointer<u8>,
		return_val_len: WordSize,
		state: u32,
	) -> WResult<u32> {
		// trace!(target: "sp-sandbox", "invoke, instance_idx={}", instance_id);

		// Deserialize arguments and convert them into wasmi types.
		let args = Vec::<sp_wasm_interface::Value>::decode(&mut args)
			.map_err(|_| "Can't decode serialized arguments for the invocation")?
			.into_iter()
			.collect::<Vec<_>>();

		let instance =
			self.sandbox_store.borrow().instance(instance_id).map_err(|e| e.to_string())?;

		let dispatch_thunk = self
			.sandbox_store
			.borrow()
			.dispatch_thunk(instance_id)
			.map_err(|e| e.to_string())?;

		match instance.invoke(
			export_name,
			&args,
			state,
			&mut SandboxContextImpl { dispatch_thunk, executor: self },
		) {
			Ok(None) => Ok(ERR_OK),
			Ok(Some(val)) => {
				// Serialize return value and write it back into the memory.
				sp_wasm_interface::ReturnValue::Value(val).using_encoded(|val| {
					if val.len() > return_val_len as usize {
						return Err("Return value buffer is too small".into())
					}
					self.write_memory(return_val, val).map_err(|_| "Return value buffer is OOB")?;
					Ok(ERR_OK)
				})
			},
			Err(_) => Ok(ERR_EXECUTION),
		}
	}

	pub fn instance_teardown(&mut self, instance_id: u32) -> WResult<()> {
		self.sandbox_store
			.borrow_mut()
			.instance_teardown(instance_id)
			.map_err(|e| e.to_string())
	}

	pub fn instance_new(
		&mut self,
		dispatch_thunk_id: u32,
		wasm: &[u8],
		raw_env_def: &[u8],
		state: u32,
	) -> WResult<u32> {
		log::warn!(target: LOG_TARGET, "dispatch_thunk_id =====>>>>> {:?}", dispatch_thunk_id);

		// Extract a dispatch thunk from instance's table by the specified index.
		let dispatch_thunk = {
			let table = self
				.table
				.as_ref()
				.ok_or("Runtime doesn't have a table; sandbox is unavailable")?;
			table
				.get(dispatch_thunk_id)
				.map_err(|_| "dispatch_thunk_idx is out of the table bounds")?
				.ok_or("dispatch_thunk_idx points on an empty table entry")?
		};

		let guest_env = match GuestEnvironment::decode(&*self.sandbox_store.borrow(), raw_env_def) {
			Ok(guest_env) => guest_env,
			Err(_) => return Ok(ERR_MODULE as u32),
		};

		let store = self.sandbox_store.clone();
		let result = store.borrow_mut().instantiate(
			wasm,
			guest_env,
			state,
			&mut SandboxContextImpl { executor: self, dispatch_thunk: dispatch_thunk.clone() },
		);

		let instance_idx_or_err_code =
			match result.map(|i| i.register(&mut store.borrow_mut(), dispatch_thunk)) {
				Ok(instance_idx) => instance_idx,
				Err(InstantiationError::StartTrapped) => ERR_EXECUTION,
				Err(_) => ERR_MODULE,
			};

		Ok(instance_idx_or_err_code)
	}

	pub fn get_global_val(
		&self,
		instance_idx: u32,
		name: &str,
	) -> WResult<Option<sp_wasm_interface::Value>> {
		self.sandbox_store
			.borrow()
			.instance(instance_idx)
			.map(|i| i.get_global_val(name))
			.map_err(|e| e.to_string())
	}
}

pub struct WasmiRuntime {
	/// A wasm module.
	module: Module,
	/// The host functions registered for this instance.
	host_functions: Arc<Vec<&'static dyn Function>>,
	/// Enable stub generation for functions that are not available in `host_functions`.
	/// These stubs will error when the wasm blob tries to call them.
	allow_missing_func_imports: bool,
	/// Numer of heap pages this runtime uses.
	heap_pages: u64,

	global_vals_snapshot: GlobalValsSnapshot,
	data_segments_snapshot: DataSegmentsSnapshot,
}

impl WasmModule for WasmiRuntime {
	fn new_instance(&self) -> std::result::Result<Box<dyn WasmInstance>, Error> {
		// Instantiate this module.
		let (instance, missing_functions, memory) = instantiate_module(
			self.heap_pages as usize,
			&self.module,
			&self.host_functions,
			self.allow_missing_func_imports,
		)
		.map_err(|e| WasmError::Instantiation(e.to_string()))?;

		Ok(Box::new(WasmiInstance {
			instance,
			memory,
			global_vals_snapshot: self.global_vals_snapshot.clone(),
			data_segments_snapshot: self.data_segments_snapshot.clone(),
			host_functions: self.host_functions.clone(),
			allow_missing_func_imports: self.allow_missing_func_imports,
			missing_functions: Arc::new(missing_functions),
		}))
	}
}

/// A state snapshot of an instance taken just after instantiation.
///
/// It is used for restoring the state of the module after execution.
#[derive(Clone)]
struct GlobalValsSnapshot {
	/// The list of all global mutable variables of the module in their sequential order.
	global_mut_values: Vec<RuntimeValue>,
}

impl GlobalValsSnapshot {
	// Returns `None` if instance is not valid.
	fn take(module_instance: &ModuleRef) -> Self {
		// Collect all values of mutable globals.
		let global_mut_values = module_instance
			.globals()
			.iter()
			.filter(|g| g.is_mutable())
			.map(|g| g.get())
			.collect();
		Self { global_mut_values }
	}

	/// Reset the runtime instance to the initial version by restoring
	/// the preserved memory and globals.
	///
	/// Returns `Err` if applying the snapshot is failed.
	fn apply(&self, instance: &ModuleRef) -> std::result::Result<(), WasmError> {
		for (global_ref, global_val) in instance
			.globals()
			.iter()
			.filter(|g| g.is_mutable())
			.zip(self.global_mut_values.iter())
		{
			// the instance should be the same as used for preserving and
			// we iterate the same way it as we do it for preserving values that means that the
			// types should be the same and all the values are mutable. So no error is expected/
			global_ref.set(*global_val).map_err(|_| WasmError::ApplySnapshotFailed)?;
		}
		Ok(())
	}
}

/// Wasmi instance wrapper along with the state snapshot.
pub struct WasmiInstance {
	/// A wasm module instance.
	instance: ModuleRef,
	/// The memory instance of used by the wasm module.
	memory: MemoryRef,
	/// The snapshot of global variable values just after instantiation.
	global_vals_snapshot: GlobalValsSnapshot,
	/// The snapshot of data segments.
	data_segments_snapshot: DataSegmentsSnapshot,
	/// The host functions registered for this instance.
	host_functions: Arc<Vec<&'static dyn Function>>,
	/// Enable stub generation for functions that are not available in `host_functions`.
	/// These stubs will error when the wasm blob trie to call them.
	allow_missing_func_imports: bool,
	/// List of missing functions detected during function resolution
	missing_functions: Arc<Vec<String>>,
}

// This is safe because `WasmiInstance` does not leak any references to `self.memory` and
// `self.instance`
unsafe impl Send for WasmiInstance {}

impl WasmInstance for WasmiInstance {
	fn call(&mut self, method: InvokeMethod, data: &[u8]) -> std::result::Result<Vec<u8>, Error> {
		// We reuse a single wasm instance for multiple calls and a previous call (if any)
		// altered the state. Therefore, we need to restore the instance to original state.

		// First, zero initialize the linear memory.
		self.memory.erase().map_err(|e| {
			// Snapshot restoration failed. This is pretty unexpected since this can happen
			// if some invariant is broken or if the system is under extreme memory pressure
			// (so erasing fails).
			log::error!(target: "wasm-executor", "snapshot restoration failed: {}", e);
			WasmError::ErasingFailed(e.to_string())
		})?;

		// Second, reapply data segments into the linear memory.
		self.data_segments_snapshot
			.apply(|offset, contents| self.memory.set(offset, contents))
			.expect("data_segments_snapshot.apply to be ok"); // todo: fix error type

		// Third, restore the global variables to their initial values.
		self.global_vals_snapshot.apply(&self.instance)?;

		call_in_wasm_module(
			&self.instance,
			&self.memory,
			method,
			data,
			self.host_functions.clone(),
			self.allow_missing_func_imports,
			self.missing_functions.clone(),
		)
	}

	fn get_global_const(
		&mut self,
		name: &str,
	) -> std::result::Result<Option<sp_wasm_interface::Value>, Error> {
		match self.instance.export_by_name(name) {
			Some(global) => Ok(Some(from_wasmi_runtime_value(
				global.as_global().ok_or_else(|| format!("`{}` is not a global", name))?.get(),
			))),
			None => Ok(None),
		}
	}

	fn call_with_allocation_stats(
		&mut self,
		method: InvokeMethod,
		data: &[u8],
	) -> (std::result::Result<Vec<u8>, Error>, Option<AllocationStats>) {
		unimplemented!()
	}

	fn call_export(&mut self, method: &str, data: &[u8]) -> std::result::Result<Vec<u8>, Error> {
		self.call(method.into(), data)
	}

	fn linear_memory_base_ptr(&self) -> Option<*const u8> {
		None
	}
}

fn get_mem_instance(module: &ModuleRef) -> std::result::Result<MemoryRef, Error> {
	Ok(module
		.export_by_name("memory")
		.ok_or(Error::InvalidMemoryReference)?
		.as_memory()
		.ok_or(Error::InvalidMemoryReference)?
		.clone())
}

/// Find the global named `__heap_base` in the given wasm module instance and
/// tries to get its value.
fn get_heap_base(module: &ModuleRef) -> std::result::Result<u32, Error> {
	let heap_base_val = module
		.export_by_name("__heap_base")
		.ok_or(Error::HeapBaseNotFoundOrInvalid)?
		.as_global()
		.ok_or(Error::HeapBaseNotFoundOrInvalid)?
		.get();

	match heap_base_val {
		wasmi::RuntimeValue::I32(v) => Ok(v as u32),
		_ => Err(Error::HeapBaseNotFoundOrInvalid),
	}
}

/// Call a given method in the given wasm-module runtime.
fn call_in_wasm_module(
	module_instance: &ModuleRef,
	memory: &MemoryRef,
	method: InvokeMethod,
	data: &[u8],
	host_functions: Arc<Vec<&'static dyn Function>>,
	allow_missing_func_imports: bool,
	missing_functions: Arc<Vec<String>>,
) -> std::result::Result<Vec<u8>, Error> {
	// Initialize FunctionExecutor.
	let table: Option<TableRef> = module_instance
		.export_by_name("__indirect_function_table")
		.and_then(|e| e.as_table().cloned());
	let heap_base = get_heap_base(module_instance)?;

	let mut function_executor = FunctionExecutor::new(
		memory.clone(),
		heap_base,
		table.clone(),
		host_functions,
		allow_missing_func_imports,
		missing_functions,
	)?;

	// Write the call data
	let offset = function_executor.allocate_memory(data.len() as u32)?;
	function_executor.write_memory(offset, data)?;

	fn convert_trap(executor: &mut FunctionExecutor, trap: wasmi::Trap) -> Error {
		if let Some(message) = executor.panic_message.take() {
			Error::AbortedDueToPanic(MessageWithBacktrace { message, backtrace: None })
		} else {
			Error::AbortedDueToTrap(MessageWithBacktrace {
				message: trap.to_string(),
				backtrace: None,
			})
		}
	}

	let result = match method {
		InvokeMethod::Export(method) => module_instance
			.invoke_export(
				method,
				&[
					wasmi::RuntimeValue::I32(u32::from(offset) as i32),
					wasmi::RuntimeValue::I32(data.len() as i32),
				],
				&mut function_executor,
			)
			.map_err(|error| {
				if let wasmi::Error::Trap(trap) = error {
					convert_trap(&mut function_executor, trap)
				} else {
					// error.into()
					Error::Other(String::from("I dunno"))
				}
			}),
		InvokeMethod::Table(func_ref) => {
			let func = table
				.ok_or(Error::NoTable)?
				.get(func_ref)
				.expect("InvokeMethod::Table to be ok") // todo: fix error
				.ok_or(Error::NoTableEntryWithIndex(func_ref))?;
			FuncInstance::invoke(
				&func,
				&[
					wasmi::RuntimeValue::I32(u32::from(offset) as i32),
					wasmi::RuntimeValue::I32(data.len() as i32),
				],
				&mut function_executor,
			)
			.map_err(|trap| convert_trap(&mut function_executor, trap))
		},
		InvokeMethod::TableWithWrapper { dispatcher_ref, func } => {
			let dispatcher = table
				.ok_or(Error::NoTable)?
				.get(dispatcher_ref)
				.expect("InvokeMethod::TableWithWrapper to be ok") // todo: fix error
				.ok_or(Error::NoTableEntryWithIndex(dispatcher_ref))?;

			FuncInstance::invoke(
				&dispatcher,
				&[
					wasmi::RuntimeValue::I32(func as _),
					wasmi::RuntimeValue::I32(u32::from(offset) as i32),
					wasmi::RuntimeValue::I32(data.len() as i32),
				],
				&mut function_executor,
			)
			.map_err(|trap| convert_trap(&mut function_executor, trap))
		},
	};

	match result {
		Ok(Some(wasmi::RuntimeValue::I64(r))) => {
			let (ptr, length) = unpack_ptr_and_len(r as u64);
			memory
				.get(ptr, length as usize)
				.map_err(|_| Error::RuntimePanicked(String::from("Whatever")))
		},
		Err(e) => {
			log::trace!(
				target: "wasm-executor",
				"Failed to execute code with {} pages",
				memory.current_size().0,
			);
			Err(e)
		},
		_ => Err(Error::InvalidReturn),
	}
}

/// Prepare module instance
fn instantiate_module(
	heap_pages: usize,
	module: &Module,
	host_functions: &[&'static dyn Function],
	allow_missing_func_imports: bool,
) -> std::result::Result<(ModuleRef, Vec<String>, MemoryRef), Error> {
	let resolver = Resolver::new(host_functions, allow_missing_func_imports, heap_pages);
	// start module instantiation. Don't run 'start' function yet.
	let intermediate_instance =
		ModuleInstance::new(module, &ImportsBuilder::new().with_resolver("env", &resolver))
			.expect("ModuleInstance::new to be ok"); // todo: fix error type

	// Verify that the module has the heap base global variable.
	let _ = get_heap_base(intermediate_instance.not_started_instance())?;

	// Get the memory reference. Runtimes should import memory, but to be backwards
	// compatible we also support exported memory.
	let memory = match resolver.import_memory.into_inner() {
		Some(memory) => memory,
		None => {
			log::debug!(
				target: "wasm-executor",
				"WASM blob does not imports memory, falling back to exported memory",
			);

			let memory = get_mem_instance(intermediate_instance.not_started_instance())?;
			memory
				.grow(Pages(heap_pages))
				.map_err(|_| Error::RuntimePanicked(String::from("whatever")))?;

			memory
		},
	};

	if intermediate_instance.has_start() {
		// Runtime is not allowed to have the `start` function.
		Err(Error::RuntimeHasStartFn)
	} else {
		Ok((
			intermediate_instance.assert_no_start(),
			resolver.missing_functions.into_inner(),
			memory,
		))
	}
}

/// Will be used on initialization of a module to resolve function and memory imports.
struct Resolver<'a> {
	/// All the hot functions that we export for the WASM blob.
	host_functions: &'a [&'static dyn Function],
	/// Should we allow missing function imports?
	///
	/// If `true`, we return a stub that will return an error when being called.
	allow_missing_func_imports: bool,
	/// All the names of functions for that we did not provide a host function.
	missing_functions: RefCell<Vec<String>>,
	/// Will be used as initial and maximum size of the imported memory.
	heap_pages: usize,
	/// By default, runtimes should import memory and this is `Some(_)` after
	/// resolving. However, to be backwards compatible, we also support memory
	/// exported by the WASM blob (this will be `None` after resolving).
	import_memory: RefCell<Option<MemoryRef>>,
}

impl<'a> Resolver<'a> {
	fn new(
		host_functions: &'a [&'static dyn Function],
		allow_missing_func_imports: bool,
		heap_pages: usize,
	) -> Resolver<'a> {
		Resolver {
			host_functions,
			allow_missing_func_imports,
			missing_functions: RefCell::new(Vec::new()),
			heap_pages,
			import_memory: Default::default(),
		}
	}
}

pub fn from_sp_wasm_interface_value(value: sp_wasm_interface::Value) -> wasmi::RuntimeValue {
	match value {
		sp_wasm_interface::Value::I32(val) => wasmi::RuntimeValue::I32(val),
		sp_wasm_interface::Value::I64(val) => wasmi::RuntimeValue::I64(val),
		sp_wasm_interface::Value::F32(val) => wasmi::RuntimeValue::F32(val.into()),
		sp_wasm_interface::Value::F64(val) => wasmi::RuntimeValue::F64(val.into()),
	}
}

pub fn from_wasmi_runtime_value(value: wasmi::RuntimeValue) -> sp_wasm_interface::Value {
	match value {
		wasmi::RuntimeValue::I32(val) => sp_wasm_interface::Value::I32(val),
		wasmi::RuntimeValue::I64(val) => sp_wasm_interface::Value::I64(val),
		wasmi::RuntimeValue::F32(val) => sp_wasm_interface::Value::F32(val.into()),
		wasmi::RuntimeValue::F64(val) => sp_wasm_interface::Value::F64(val.into()),
	}
}

pub fn from_sp_wasm_interface_value_type(value: sp_wasm_interface::ValueType) -> wasmi::ValueType {
	match value {
		sp_wasm_interface::ValueType::I32 => wasmi::ValueType::I32,
		sp_wasm_interface::ValueType::I64 => wasmi::ValueType::I64,
		sp_wasm_interface::ValueType::F32 => wasmi::ValueType::F32,
		sp_wasm_interface::ValueType::F64 => wasmi::ValueType::F64,
	}
}

pub fn from_wasmi_value_type(value: wasmi::ValueType) -> sp_wasm_interface::ValueType {
	match value {
		wasmi::ValueType::I32 => sp_wasm_interface::ValueType::I32,
		wasmi::ValueType::I64 => sp_wasm_interface::ValueType::I64,
		wasmi::ValueType::F32 => sp_wasm_interface::ValueType::F32,
		wasmi::ValueType::F64 => sp_wasm_interface::ValueType::F64,
	}
}

fn from_sp_wasm_interface_signature(sig: sp_wasm_interface::Signature) -> wasmi::Signature {
	let args = sig
		.args
		.iter()
		.map(|a| from_sp_wasm_interface_value_type(*a))
		.collect::<Vec<_>>();

	wasmi::Signature::new(
		args,
		match sig.return_value {
			Some(v) => Some(from_sp_wasm_interface_value_type(v)),
			None => None,
		},
	)
}

fn from_wasmi_signature(sig: &wasmi::Signature) -> sp_wasm_interface::Signature {
	sp_wasm_interface::Signature::new(
		sig.params()
			.iter()
			.copied()
			.map(|v| from_wasmi_value_type(v))
			.collect::<Vec<_>>(),
		match sig.return_type() {
			Some(v) => Some(from_wasmi_value_type(v)),
			None => None,
		},
	)
}

impl<'a> wasmi::ModuleImportResolver for Resolver<'a> {
	fn resolve_func(
		&self,
		name: &str,
		signature: &wasmi::Signature,
	) -> std::result::Result<wasmi::FuncRef, wasmi::Error> {
		let signature = from_wasmi_signature(signature);
		for (function_index, function) in self.host_functions.iter().enumerate() {
			if name == function.name() {
				if signature == function.signature() {
					return Ok(wasmi::FuncInstance::alloc_host(
						from_sp_wasm_interface_signature(signature),
						function_index,
					))
				} else {
					return Err(wasmi::Error::Instantiation(format!(
						"Invalid signature for function `{}` expected `{:?}`, got `{:?}`",
						function.name(),
						signature,
						function.signature(),
					)))
				}
			}
		}

		if self.allow_missing_func_imports {
			log::trace!(target: "wasm-executor", "Could not find function `{}`, a stub will be provided instead.", name);
			let id = self.missing_functions.borrow().len() + self.host_functions.len();
			self.missing_functions.borrow_mut().push(name.to_string());

			Ok(wasmi::FuncInstance::alloc_host(from_sp_wasm_interface_signature(signature), id))
		} else {
			Err(wasmi::Error::Instantiation(format!("Export {} not found", name)))
		}
	}

	fn resolve_memory(
		&self,
		field_name: &str,
		memory_type: &wasmi::MemoryDescriptor,
	) -> std::result::Result<MemoryRef, wasmi::Error> {
		if field_name == "memory" {
			match &mut *self.import_memory.borrow_mut() {
				Some(_) =>
					Err(wasmi::Error::Instantiation("Memory can not be imported twice!".into())),
				memory_ref @ None => {
					if memory_type
						.maximum()
						.map(|m| m.saturating_sub(memory_type.initial()))
						.map(|m| self.heap_pages > m as usize)
						.unwrap_or(false)
					{
						Err(wasmi::Error::Instantiation(format!(
							"Heap pages ({}) is greater than imported memory maximum ({}).",
							self.heap_pages,
							memory_type
								.maximum()
								.map(|m| m.saturating_sub(memory_type.initial()))
								.expect("Maximum is set, checked above; qed"),
						)))
					} else {
						let memory = MemoryInstance::alloc(
							Pages(memory_type.initial() as usize + self.heap_pages),
							Some(Pages(memory_type.initial() as usize + self.heap_pages)),
						)?;
						*memory_ref = Some(memory.clone());
						Ok(memory)
					}
				},
			}
		} else {
			Err(wasmi::Error::Instantiation(format!(
				"Unknown memory reference with name: {}",
				field_name
			)))
		}
	}
}

/// Create a new `WasmiRuntime` given the code. This function loads the module and
/// stores it in the instance.
pub fn create_runtime(
	blob: RuntimeBlob,
	heap_pages: u64,
	host_functions: Vec<&'static dyn Function>,
	allow_missing_func_imports: bool,
) -> std::result::Result<WasmiRuntime, WasmError> {
	let data_segments_snapshot =
		DataSegmentsSnapshot::take(&blob).map_err(|e| WasmError::Other(e.to_string()))?;

	let module =
		Module::from_parity_wasm_module(blob.into_inner()).map_err(|_| WasmError::InvalidModule)?;

	let global_vals_snapshot = {
		let (instance, _, _) = instantiate_module(
			heap_pages as usize,
			&module,
			&host_functions,
			allow_missing_func_imports,
		)
		.map_err(|e| WasmError::Instantiation(e.to_string()))?;
		GlobalValsSnapshot::take(&instance)
	};

	Ok(WasmiRuntime {
		module,
		data_segments_snapshot,
		global_vals_snapshot,
		host_functions: Arc::new(host_functions),
		allow_missing_func_imports,
		heap_pages,
	})
}

// THIS impl block was added to experiment with the WasmiInstance, it is not a part of original
// substrate code
impl WasmiRuntime {
	pub fn new_instance_wasmi_instance(&self) -> std::result::Result<Box<WasmiInstance>, Error> {
		let (instance, missing_functions, memory) = instantiate_module(
			self.heap_pages as usize,
			&self.module,
			&self.host_functions,
			self.allow_missing_func_imports,
		)
		.map_err(|e| WasmError::Instantiation(e.to_string()))?;

		Ok(Box::new(WasmiInstance {
			instance,
			memory,
			global_vals_snapshot: self.global_vals_snapshot.clone(),
			data_segments_snapshot: self.data_segments_snapshot.clone(),
			host_functions: self.host_functions.clone(),
			allow_missing_func_imports: self.allow_missing_func_imports,
			missing_functions: Arc::new(missing_functions),
		}))
	}
}

// THIS impl block was added to experiment with the WasmiInstance, it is not a part of original
// substrate code
impl WasmiInstance {
	pub fn create_function_executor(&self) -> FunctionExecutor {
		self.memory
			.erase()
			.map_err(|e| {
				// Snapshot restoration failed. This is pretty unexpected since this can happen
				// if some invariant is broken or if the system is under extreme memory pressure
				// (so erasing fails).
				log::error!(target: "wasm-executor", "snapshot restoration failed: {}", e);
				WasmError::ErasingFailed(e.to_string())
			})
			.expect("memory.erase to be ok");

		// Second, reapply data segments into the linear memory.
		self.data_segments_snapshot
			.apply(|offset, contents| self.memory.set(offset, contents))
			.expect("data_segments_snapshot.apply to be ok"); // todo: fix error type

		self.global_vals_snapshot
			.apply(&self.instance)
			.expect("global_vals_snapshot.apply to be ok");

		let table = &self
			.instance
			.export_by_name("__indirect_function_table")
			.and_then(|e| e.as_table().cloned());

		let heap_base = get_heap_base(&self.instance).expect("get_heap_base to be ok");

		let function_executor = FunctionExecutor::new(
			self.memory.clone(),
			heap_base,
			table.clone(),
			self.host_functions.clone(),
			self.allow_missing_func_imports,
			self.missing_functions.clone(),
		)
		.expect("function_executor to be created");

		function_executor
	}
}
