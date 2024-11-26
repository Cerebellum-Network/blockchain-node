use std::{cell::RefCell, rc::Rc, str, sync::Arc, result};
use sp_wasm_interface::{Function, FunctionContext, Pointer,  Result as WResult, Value, WordSize};
use crate::FunctionExecutor;
use crate::env as sandbox_env;
use log::{debug, error, trace};
use codec::{Decode, Encode};
use crate::util::MemoryTransfer;
use wasmi::RuntimeValue;
use crate::sandbox_util as sandbox;
use crate::wasmi_backend::trap;
use sc_executor::error::{Error, Result};

/// Sandbox memory identifier.
pub type MemoryId = u32;

// /// Result type used by traits in this crate.
// #[cfg(feature = "std")]
// pub type Result<T> = result::Result<T, String>;
// #[cfg(not(feature = "std"))]
// pub type Result<T> = result::Result<T, &'static str>;


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
			Err(trap("Could not find host function with index"))
		} else {
			Err(trap("Could not find host function with index"))
		}
	}
}

struct SandboxContext<'a> {
	executor: &'a mut FunctionExecutor,
	dispatch_thunk: wasmi::FuncRef,
}

impl<'a> sandbox::SandboxContext for SandboxContext<'a> {
	fn invoke(
		&mut self,
		invoke_args_ptr: Pointer<u8>,
		invoke_args_len: WordSize,
		state: u32,
		func_idx: sandbox::SupervisorFuncIndex,
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

	// fn sandbox(&mut self) -> &mut dyn Sandbox {
	// 	self
	// }

	fn register_panic_error_message(&mut self, message: &str) {
		self.panic_message = Some(message.to_owned());
	}
}

/// Something that provides access to the sandbox.
pub trait Sandbox {
	/// Get sandbox memory from the `memory_id` instance at `offset` into the given buffer.
	fn memory_get(
		&mut self,
		memory_id: MemoryId,
		offset: WordSize,
		buf_ptr: Pointer<u8>,
		buf_len: WordSize,
	) -> WResult<u32>;
	/// Set sandbox memory from the given value.
	fn memory_set(
		&mut self,
		memory_id: MemoryId,
		offset: WordSize,
		val_ptr: Pointer<u8>,
		val_len: WordSize,
	) -> WResult<u32>;
	/// Delete a memory instance.
	fn memory_teardown(&mut self, memory_id: MemoryId) -> WResult<()>;
	/// Create a new memory instance with the given `initial` size and the `maximum` size.
	/// The size is given in wasm pages.
	fn memory_new(&mut self, initial: u32, maximum: u32) -> WResult<MemoryId>;
	/// Invoke an exported function by a name.
	fn invoke(
		&mut self,
		instance_id: u32,
		export_name: &str,
		args: &[u8],
		return_val: Pointer<u8>,
		return_val_len: WordSize,
		state: u32,
	) -> WResult<u32>;
	/// Delete a sandbox instance.
	fn instance_teardown(&mut self, instance_id: u32) -> WResult<()>;
	/// Create a new sandbox instance.
	fn instance_new(
		&mut self,
		dispatch_thunk_id: u32,
		wasm: &[u8],
		raw_env_def: &[u8],
		state: u32,
	) -> WResult<u32>;

	/// Get the value from a global with the given `name`. The sandbox is determined by the
	/// given `instance_idx` instance.
	///
	/// Returns `Some(_)` when the requested global variable could be found.
	fn get_global_val(&self, instance_idx: u32, name: &str) -> WResult<Option<Value>>;
}

impl Sandbox for FunctionExecutor {
	fn memory_get(
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
			Err(_) => return Ok(sandbox_env::ERR_OUT_OF_BOUNDS),
			Ok(buffer) => buffer,
		};

		if self.memory.set(buf_ptr.into(), &buffer).is_err() {
			return Ok(sandbox_env::ERR_OUT_OF_BOUNDS)
		}

		Ok(sandbox_env::ERR_OK)
	}

	fn memory_set(
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
			Err(_) => return Ok(sandbox_env::ERR_OUT_OF_BOUNDS),
			Ok(buffer) => buffer,
		};

		if sandboxed_memory.write_from(Pointer::new(offset as u32), &buffer).is_err() {
			return Ok(sandbox_env::ERR_OUT_OF_BOUNDS)
		}

		Ok(sandbox_env::ERR_OK)
	}

	fn memory_teardown(&mut self, memory_id: MemoryId) -> WResult<()> {
		self.sandbox_store
			.borrow_mut()
			.memory_teardown(memory_id)
			.map_err(|e| e.to_string())
	}

	fn memory_new(&mut self, initial: u32, maximum: u32) -> WResult<MemoryId> {
		self.sandbox_store
			.borrow_mut()
			.new_memory(initial, maximum)
			.map_err(|e| e.to_string())
	}

	fn invoke(
		&mut self,
		instance_id: u32,
		export_name: &str,
		mut args: &[u8],
		return_val: Pointer<u8>,
		return_val_len: WordSize,
		state: u32,
	) -> WResult<u32> {
		trace!(target: "sp-sandbox", "invoke, instance_idx={}", instance_id);

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
			&mut SandboxContext { dispatch_thunk, executor: self },
		) {
			Ok(None) => Ok(sandbox_env::ERR_OK),
			Ok(Some(val)) => {
				// Serialize return value and write it back into the memory.
				sp_wasm_interface::ReturnValue::Value(val).using_encoded(|val| {
					if val.len() > return_val_len as usize {
						return Err("Return value buffer is too small".into())
					}
					self.write_memory(return_val, val).map_err(|_| "Return value buffer is OOB")?;
					Ok(sandbox_env::ERR_OK)
				})
			},
			Err(_) => Ok(sandbox_env::ERR_EXECUTION),
		}
	}

	fn instance_teardown(&mut self, instance_id: u32) -> WResult<()> {
		self.sandbox_store
			.borrow_mut()
			.instance_teardown(instance_id)
			.map_err(|e| e.to_string())
	}

	fn instance_new(
		&mut self,
		dispatch_thunk_id: u32,
		wasm: &[u8],
		raw_env_def: &[u8],
		state: u32,
	) -> WResult<u32> {
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

		let guest_env =
			match sandbox::GuestEnvironment::decode(&*self.sandbox_store.borrow(), raw_env_def) {
				Ok(guest_env) => guest_env,
				Err(_) => return Ok(sandbox_env::ERR_MODULE as u32),
			};

		let store = self.sandbox_store.clone();
		let result = store.borrow_mut().instantiate(
			wasm,
			guest_env,
			state,
			&mut SandboxContext { executor: self, dispatch_thunk: dispatch_thunk.clone() },
		);

		let instance_idx_or_err_code =
			match result.map(|i| i.register(&mut store.borrow_mut(), dispatch_thunk)) {
				Ok(instance_idx) => instance_idx,
				Err(sandbox::InstantiationError::StartTrapped) => sandbox_env::ERR_EXECUTION,
				Err(_) => sandbox_env::ERR_MODULE,
			};

		Ok(instance_idx_or_err_code)
	}

	fn get_global_val(
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
