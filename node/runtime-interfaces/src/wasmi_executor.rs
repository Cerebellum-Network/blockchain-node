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

pub struct FunctionExecutor<'a> {
	sandbox_store: Rc<RefCell<Store<wasmi::FuncRef>>>,
	heap: RefCell<FreeingBumpHeapAllocator>,
	memory: MemoryRef,
	table: Option<TableRef>,
	host_functions: Arc<Vec<&'static dyn Function>>,
	allow_missing_func_imports: bool,
	missing_functions: Arc<Vec<String>>,
	panic_message: Option<String>,
	runtime_memory: Option<Box<&'a mut dyn FunctionContext>>,
	is_runtime_context: bool,
	debug_memory: bool,
}
unsafe impl Send for FunctionExecutor<'_> {}

impl<'a> FunctionExecutor<'a> {
	pub fn new(
		m: MemoryRef,
		heap_base: u32,
		t: Option<TableRef>,
		host_functions: Arc<Vec<&'static dyn Function>>,
		allow_missing_func_imports: bool,
		missing_functions: Arc<Vec<String>>,
		sandbox_store: Option<Rc<RefCell<Store<wasmi::FuncRef>>>>,
	) -> Result<Self> {
		Ok(FunctionExecutor {
			sandbox_store: match sandbox_store {
				Some(store) => store,
				None => Rc::new(RefCell::new(Store::new(SandboxBackend::Wasmi))),
			},
			heap: RefCell::new(FreeingBumpHeapAllocator::new(heap_base)),
			memory: m,
			table: t,
			host_functions,
			allow_missing_func_imports,
			missing_functions,
			panic_message: None,
			runtime_memory: None,
			is_runtime_context: true,
			debug_memory: false,
		})
	}

	pub fn set_runtime_memory(&mut self, runtime_memory: Box<&'a mut dyn FunctionContext>) {
		self.runtime_memory = Some(runtime_memory);
	}
}

fn display_runtime_memory(method: &'static str, ctx: &dyn FunctionContext) {
	let memory_size: WordSize = 135593984; // based on 4.8.4
	let offset: WordSize = 0;

	let buffer = ctx
		.read_memory(Pointer::new(offset), memory_size)
		.expect("Could not read memory of size 135593984");

	let buffer_slice = buffer.as_slice();
	let buffer_hash = sp_core::blake2_256(buffer_slice);
	let buffer_hash_hex_string: String =
		buffer_hash.iter().map(|byte| format!("{:02x}", byte)).collect();

	log::info!("MemoryRef {} ===> buffer_hash={:?}", method, buffer_hash_hex_string,);
}

fn display_fn_executor_memory(method: &'static str, memory: &MemoryRef) {
	let limits = memory.0.limits.clone();
	let initial = memory.0.initial;
	let maximum = memory.0.maximum;
	let current_size = memory.0.current_size.clone();
	let buffer = memory.0.buffer.borrow();
	let buffer_slice = buffer.as_slice();
	let buffer_hash = sp_core::blake2_256(buffer_slice);
	let buffer_hash_hex_string: String =
		buffer_hash.iter().map(|byte| format!("{:02x}", byte)).collect();

	log::info!(
		"MemoryRef {} ===> buffer_hash={:?} limits={:?}, initial={:?}, maximum={:?}, current_size={:?}, buffer={:?}",
		method,
		buffer_hash_hex_string,
		limits,
		initial,
		maximum,
		current_size,
		buffer.len()
	);
}

// The closest output to 4.8.4 invocation - 2967592, 2000280
impl FunctionContext for FunctionExecutor<'_> {
	fn read_memory_into(&self, address: Pointer<u8>, dest: &mut [u8]) -> WResult<()> {
		if self.is_runtime_context {
			if self.debug_memory {
				let runtime_memory = self.runtime_memory.as_ref().unwrap();
				display_runtime_memory("PRE 🚩 read_memory_into", **runtime_memory);
			}

			let res = self
				.runtime_memory
				.as_ref()
				.expect("Runtime memory to be set")
				.read_memory_into(address, dest);

			if self.debug_memory {
				let runtime_memory = self.runtime_memory.as_ref().unwrap();
				display_runtime_memory("POST 🏁 read_memory_into", **runtime_memory);
			}

			res
		} else {
			if self.debug_memory {
				display_fn_executor_memory("PRE 🚩 read_memory_into", &self.memory);
			}
			let res = self.memory.get_into(address.into(), dest).map_err(|e| e.to_string());
			if self.debug_memory {
				display_fn_executor_memory("POST 🏁 read_memory_into", &self.memory);
			}

			res
		}
	}

	fn write_memory(&mut self, address: Pointer<u8>, data: &[u8]) -> WResult<()> {
		if self.is_runtime_context {
			if self.debug_memory {
				let runtime_memory = self.runtime_memory.as_ref().unwrap();
				display_runtime_memory("PRE 🚩 write_memory", **runtime_memory);
			}
			// self.memory.set(address.clone().into(), data.clone()).map_err(|e| e.to_string());
			let res = self
				.runtime_memory
				.as_mut()
				.expect("Runtime memory to be set")
				.write_memory(address, data);

			if self.debug_memory {
				let runtime_memory: &Box<&mut dyn FunctionContext> =
					self.runtime_memory.as_ref().unwrap();
				display_runtime_memory("POST 🏁 write_memory", **runtime_memory);
			}

			res
		} else {
			if self.debug_memory {
				display_fn_executor_memory("PRE 🚩 write_memory", &self.memory);
			}
			let res =
				self.memory.set(address.clone().into(), data.clone()).map_err(|e| e.to_string());
			if self.debug_memory {
				display_fn_executor_memory("POST 🏁 write_memory", &self.memory);
			}

			res
		}
	}

	fn allocate_memory(&mut self, size: WordSize) -> WResult<Pointer<u8>> {
		if self.is_runtime_context {
			if self.debug_memory {
				let runtime_memory = self.runtime_memory.as_ref().unwrap();
				display_runtime_memory("PRE 🚩 allocate_memory", **runtime_memory);
			}
			// let heap = &mut self.heap.borrow_mut();
			// self.memory
			// 	.with_direct_access_mut(|mem| heap.allocate(mem, size).map_err(|e| e.to_string()));
			let res = self
				.runtime_memory
				.as_mut()
				.expect("Runtime memory to be set")
				.allocate_memory(size);
			if self.debug_memory {
				let runtime_memory = self.runtime_memory.as_ref().unwrap();
				display_runtime_memory("POST 🏁 allocate_memory", **runtime_memory);
			}

			res
		} else {
			if self.debug_memory {
				display_fn_executor_memory("PRE 🚩 allocate_memory", &self.memory);
			}
			let heap = &mut self.heap.borrow_mut();
			let res = self
				.memory
				.with_direct_access_mut(|mem| heap.allocate(mem, size).map_err(|e| e.to_string()));

			if self.debug_memory {
				display_fn_executor_memory("POST 🏁 allocate_memory", &self.memory);
			}

			res
		}
	}

	fn deallocate_memory(&mut self, ptr: Pointer<u8>) -> WResult<()> {
		if self.is_runtime_context {
			if self.debug_memory {
				let runtime_memory = self.runtime_memory.as_ref().unwrap();
				display_runtime_memory("PRE 🚩 deallocate_memory", **runtime_memory);
			}
			// let heap = &mut self.heap.borrow_mut();
			// self.memory.with_direct_access_mut(|mem| {
			// 	heap.deallocate(mem, ptr.clone()).map_err(|e| e.to_string())
			// });
			let res = self
				.runtime_memory
				.as_mut()
				.expect("Runtime memory to be set")
				.deallocate_memory(ptr);

			if self.debug_memory {
				let runtime_memory = self.runtime_memory.as_ref().unwrap();
				display_runtime_memory("POST 🏁 deallocate_memory", **runtime_memory);
			}

			res
		} else {
			if self.debug_memory {
				display_fn_executor_memory("PRE 🚩 deallocate_memory", &self.memory);
			}

			let heap = &mut self.heap.borrow_mut();
			let res = self.memory.with_direct_access_mut(|mem| {
				heap.deallocate(mem, ptr.clone()).map_err(|e| e.to_string())
			});

			if self.debug_memory {
				display_fn_executor_memory("POST 🏁 deallocate_memory", &self.memory);
			}

			res
		}
	}

	fn register_panic_error_message(&mut self, message: &str) {
		if self.is_runtime_context {
			if self.debug_memory {
				let runtime_memory = self.runtime_memory.as_ref().unwrap();
				display_runtime_memory("register_panic_error_message", **runtime_memory);
			}

			// self.panic_message = Some(message.clone().to_owned());
			self.runtime_memory
				.as_mut()
				.expect("Runtime memory to be set")
				.register_panic_error_message(message);
		} else {
			if self.debug_memory {
				display_fn_executor_memory("register_panic_error_message", &self.memory);
			}
			self.panic_message = Some(message.clone().to_owned());
		}
	}
}

// impl FunctionContext for FunctionExecutor<'_> {
// 	fn read_memory_into(&self, address: Pointer<u8>, dest: &mut [u8]) -> WResult<()> {
// 		self.memory.get_into(address.into(), dest).map_err(|e| e.to_string())
// 	}

// 	fn write_memory(&mut self, address: Pointer<u8>, data: &[u8]) -> WResult<()> {
// 		self.memory.set(address.into(), data).map_err(|e| e.to_string())
// 	}

// 	fn allocate_memory(&mut self, size: WordSize) -> WResult<Pointer<u8>> {
// 		let heap = &mut self.heap.borrow_mut();
// 		self.memory
// 			.with_direct_access_mut(|mem| heap.allocate(mem, size).map_err(|e| e.to_string()))
// 	}

// 	fn deallocate_memory(&mut self, ptr: Pointer<u8>) -> WResult<()> {
// 		let heap = &mut self.heap.borrow_mut();
// 		self.memory
// 			.with_direct_access_mut(|mem| heap.deallocate(mem, ptr).map_err(|e| e.to_string()))
// 	}

// 	fn register_panic_error_message(&mut self, message: &str) {
// 		self.panic_message = Some(message.to_owned());
// 	}
// }

// impl FunctionContext for FunctionExecutor<'_> {
// 	fn read_memory_into(&self, address: Pointer<u8>, dest: &mut [u8]) -> WResult<()> {
// 		self.runtime_memory
// 			.as_ref()
// 			.expect("Runtime memory to be set")
// 			.read_memory_into(address, dest)
// 	}

// 	fn write_memory(&mut self, address: Pointer<u8>, data: &[u8]) -> WResult<()> {
// 		self.runtime_memory
// 			.as_mut()
// 			.expect("Runtime memory to be set")
// 			.write_memory(address, data)
// 	}

// 	fn allocate_memory(&mut self, size: WordSize) -> WResult<Pointer<u8>> {
// 		self.runtime_memory
// 			.as_mut()
// 			.expect("Runtime memory to be set")
// 			.allocate_memory(size)
// 	}

// 	fn deallocate_memory(&mut self, ptr: Pointer<u8>) -> WResult<()> {
// 		self.runtime_memory
// 			.as_mut()
// 			.expect("Runtime memory to be set")
// 			.deallocate_memory(ptr)
// 	}

// 	fn register_panic_error_message(&mut self, message: &str) {
// 		self.runtime_memory
// 			.as_mut()
// 			.expect("Runtime memory to be set")
// 			.register_panic_error_message(message)
// 	}
// }

impl wasmi::Externals for FunctionExecutor<'_> {
	fn invoke_index(
		&mut self,
		index: usize,
		args: wasmi::RuntimeArgs,
	) -> std::result::Result<Option<wasmi::RuntimeValue>, wasmi::Trap> {
		// self.is_runtime_context = false;

		let mut args = args.as_ref().iter().copied().map(|value| match value {
			wasmi::RuntimeValue::I32(val) => Value::I32(val),
			wasmi::RuntimeValue::I64(val) => Value::I64(val),
			wasmi::RuntimeValue::F32(val) => Value::F32(val.into()),
			wasmi::RuntimeValue::F64(val) => Value::F64(val.into()),
		});

		let host_functions_names: Vec<&str> =
			self.host_functions.iter().map(|f| f.name()).collect();
		log::info!(target: LOG_TARGET, "---> FunctionExecutor.invoke_index {:?}, name: {:?}", index, host_functions_names.get(index));

		// log::info!(target: LOG_TARGET, "---> index={:?}, host_functions.len()={:?}", index,
		// host_functions_names.len());
		// log::info!(target: LOG_TARGET, "---> host_functions={:?}",
		// host_functions_names);

		let res = if let Some(function) = self.host_functions.clone().get(index) {
			let res = function
				.execute(self, &mut args)
				// .execute(
				// 	**self.runtime_memory.as_mut().expect("Runtime memory to be set"),
				// 	&mut args,
				// )
				.map_err(|msg| Error::FunctionExecution(function.name().to_string(), msg))
				.map_err(|msg| {
					log::info!(target: LOG_TARGET, "---> FunctionExecutor.invoke_index ---> error 1 {:?}", msg);
					trap("Function call failed")
				})
				.map_err(|err| {
					log::info!(target: LOG_TARGET, "---> FunctionExecutor.invoke_index ---> error 2 {:?}", err);
					wasmi::Trap::from(err)
				})
				.map(|v| {
					let reslt = v.map(|value| match value {
						Value::I32(val) => RuntimeValue::I32(val),
						Value::I64(val) => RuntimeValue::I64(val),
						Value::F32(val) => RuntimeValue::F32(val.into()),
						Value::F64(val) => RuntimeValue::F64(val.into()),
					});
					log::info!(target: LOG_TARGET, "---> FunctionExecutor.invoke_index ===> reslt {:?}", reslt);
					reslt
				});
			res
		} else if self.allow_missing_func_imports &&
			index >= self.host_functions.len() &&
			index < self.host_functions.len() + self.missing_functions.len()
		{
			log::info!(target: LOG_TARGET, "---> FunctionExecutor.invoke_index ---> error 3 Function is only a stub. Calling a stub is not allowed - index {:?}", index);
			Err(trap(
				format!(
					"Function is only a stub. Calling a stub is not allowed - index {:?}",
					index
				)
				.as_str(),
			))
		} else {
			log::info!(target: LOG_TARGET, "---> FunctionExecutor.invoke_index ---> error 4 Could not find host function with index");
			Err(trap("Could not find host function with index"))
		};

		// self.is_runtime_context = true;

		res
	}
}

struct SandboxContextImpl<'a, 'b> {
	executor: &'a mut FunctionExecutor<'b>,
	dispatch_thunk: wasmi::FuncRef,
}

impl<'a, 'b> SandboxContext for SandboxContextImpl<'a, 'b> {
	fn invoke(
		&mut self,
		invoke_args_ptr: Pointer<u8>,
		invoke_args_len: WordSize,
		state: u32,
		func_idx: SupervisorFuncIndex,
	) -> Result<i64> {
		// self.executor.is_runtime_context = false;
		log::info!(target: LOG_TARGET, "SandboxContext.invoke START: invoke_args_ptr={:?}, invoke_args_len={:?}, state={:?}, func_idx={:?}, dispatch_thunk={:?}", invoke_args_ptr, invoke_args_len, state, func_idx, &self.dispatch_thunk);

		if self.executor.debug_memory {
			display_fn_executor_memory(
				"PRE 🚩 wasmi::FuncInstance::invoke fn_executor_memory",
				&self.executor.memory,
			);

			let runtime_memory = self.executor.runtime_memory.as_ref().unwrap();
			display_runtime_memory(
				"PRE 🚩 wasmi::FuncInstance::invoke runtime_memory",
				**runtime_memory,
			);
		}

		// here the host_functions table is rendered
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
		log::info!(target: LOG_TARGET, "SandboxContext.invoke WIP: invoke_args_ptr={:?}, invoke_args_len={:?}, state={:?}, func_idx={:?}, dispatch_thunk={:?}", invoke_args_ptr, invoke_args_len, state, func_idx, &self.dispatch_thunk);

		let res = match result {
			Ok(Some(RuntimeValue::I64(val))) => {
				log::info!(target: LOG_TARGET, "SandboxContext.invoke WIP.A: val={:?}", val);
				Ok(val)
			},
			Ok(res) => {
				log::info!(target: LOG_TARGET, "SandboxContext.invoke WIP.B: res={:?}", res);
				Err("Supervisor function returned unexpected result!".into())
			},
			Err(err) => {
				log::info!(target: LOG_TARGET, "SandboxContext.invoke WIP.C: err={:?}", err);
				Err(Error::Other(err.to_string()))
			},
		};

		if self.executor.debug_memory {
			display_fn_executor_memory(
				"POST 🏁 wasmi::FuncInstance::invoke fn_executor_memory",
				&self.executor.memory,
			);

			let runtime_memory = self.executor.runtime_memory.as_ref().unwrap();
			display_runtime_memory(
				"POST 🏁 wasmi::FuncInstance::invoke runtime_memory",
				**runtime_memory,
			);
		}

		// self.executor.is_runtime_context = true;

		res
	}

	fn supervisor_context(&mut self) -> &mut dyn FunctionContext {
		self.executor
	}

	fn current_block_runtime_context(&mut self) -> &mut dyn FunctionContext {
		**self.executor.runtime_memory.as_mut().expect("Runtime memory to be set")
	}

	fn use_runtime_fn_context(&mut self, is_runtime_context: bool) {
		self.executor.is_runtime_context = is_runtime_context;
	}

	fn is_runtime_fn_context(&mut self) -> bool {
		self.executor.is_runtime_context
	}
}

// impl Sandbox for FunctionExecutor {
impl FunctionExecutor<'_> {
	// pub fn memory_get(
	// 	&mut self,
	// 	memory_id: MemoryId,
	// 	offset: WordSize,
	// 	data: &mut [u8],
	// 	// buf_ptr: Pointer<u8>,
	// 	// buf_len: WordSize,
	// ) -> WResult<u32> {
	// 	log::info!(target: LOG_TARGET, "memory_get START: memory_id={:?}, offset={:?}", memory_id,
	// offset);

	// 	let sandboxed_memory =
	// 		self.sandbox_store.borrow().memory(memory_id).map_err(|e| e.to_string())?;

	// 	// let len = buf_len as usize;

	// 	// let buffer = match sandboxed_memory.read(Pointer::new(offset as u32), len) {
	// 	// 	Err(_) => return Ok(ERR_OUT_OF_BOUNDS),
	// 	// 	Ok(buffer) => buffer,
	// 	// };

	// 	// if self.memory.set(buf_ptr.into(), &buffer).is_err() {
	// 	// 	return Ok(ERR_OUT_OF_BOUNDS)
	// 	// }

	// 	if sandboxed_memory.read_into(Pointer::new(offset as u32), data).is_err() {
	// 		return Ok(ERR_OUT_OF_BOUNDS)
	// 	}

	// 	log::info!(target: LOG_TARGET, "memory_get END: memory_id={:?}, offset={:?}", memory_id,
	// offset);

	// 	Ok(ERR_OK)
	// }

	// pub fn memory_set(
	// 	&mut self,
	// 	memory_id: MemoryId,
	// 	offset: WordSize,
	// 	data: &[u8],
	// ) -> WResult<u32> {
	// 	log::info!(target: LOG_TARGET, "memory_set START: memory_id={:?}, offset={:?}", memory_id,
	// offset);

	// 	let sandboxed_memory =
	// 		self.sandbox_store.borrow().memory(memory_id).map_err(|e| e.to_string())?;

	// 	// let len = val_len as usize;

	// 	// #[allow(deprecated)]
	// 	// let buffer = match self.memory.get(val_ptr.into(), len) {
	// 	// 	Err(_) => return Ok(ERR_OUT_OF_BOUNDS),
	// 	// 	Ok(buffer) => buffer,
	// 	// };

	// 	if sandboxed_memory.write_from(Pointer::new(offset as u32), &data).is_err() {
	// 		return Ok(ERR_OUT_OF_BOUNDS)
	// 	}

	// 	log::info!(target: LOG_TARGET, "memory_set END: memory_id={:?}, offset={:?}", memory_id,
	// offset);

	// 	Ok(ERR_OK)
	// }

	pub fn memory_get(
		&mut self,
		memory_id: MemoryId,
		offset: WordSize,
		buf_ptr: Pointer<u8>,
		buf_len: WordSize,
	) -> WResult<u32> {
		log::info!(target: LOG_TARGET, "memory_get START: memory_id={:?}, offset={:?}, buf_ptr={:?}, buf_len={:?}", memory_id, offset, buf_ptr, buf_len);

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

		log::info!(target: LOG_TARGET, "memory_get END: memory_id={:?}, offset={:?}, buf_ptr={:?}, buf_len={:?}", memory_id, offset, buf_ptr, buf_len);

		Ok(ERR_OK)
	}

	pub fn memory_set(
		&mut self,
		memory_id: MemoryId,
		offset: WordSize,
		val_ptr: Pointer<u8>,
		val_len: WordSize,
	) -> WResult<u32> {
		log::info!(target: LOG_TARGET, "memory_set START: memory_id={:?}, offset={:?}, val_ptr={:?}, val_len={:?}", memory_id, offset, val_ptr, val_len);

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

		log::info!(target: LOG_TARGET, "memory_set END: memory_id={:?}, offset={:?}, val_ptr={:?}, val_len={:?}", memory_id, offset, val_ptr, val_len);

		Ok(ERR_OK)
	}

	pub fn memory_teardown(&mut self, memory_id: MemoryId) -> WResult<()> {
		log::info!(target: LOG_TARGET, "memory_teardown START: memory_id={:?}", memory_id);

		let res = self
			.sandbox_store
			.borrow_mut()
			.memory_teardown(memory_id)
			.map_err(|e| e.to_string());

		log::info!(target: LOG_TARGET, "memory_teardown END: memory_id={:?}", memory_id);
		res
	}

	pub fn memory_new(&mut self, initial: u32, maximum: u32) -> WResult<MemoryId> {
		log::info!(target: LOG_TARGET, "memory_new START: initial={:?}, maximum={:?}", initial, maximum);

		let res = self
			.sandbox_store
			.borrow_mut()
			.new_memory(initial, maximum)
			.map_err(|e| e.to_string());

		log::info!(target: LOG_TARGET, "memory_new END: initial={:?}, maximum={:?}, new_memory={:?}", initial, maximum, res);

		res
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
		if instance_id == 0u32 {
			self.debug_memory = true;
		}

		log::info!(target: LOG_TARGET, "invoke START: instance_id={:?}, export_name={:?}, args={:?}, return_val={:?}, return_val_len={:?}, state={:?}", instance_id, export_name, args, return_val, return_val_len, state);

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
		log::info!(target: LOG_TARGET, "invoke dispatch_thunk={:?}", dispatch_thunk);

		log::info!(target: LOG_TARGET, "invoke WIP 1: ---> {:?}", instance_id);
		log::info!(target: LOG_TARGET, "executor.memory={:?}, executor.sandbox_store.instances={:?}, sandbox_store.memories={:?}", self.memory, self.sandbox_store.borrow().instances.len(), self.sandbox_store.borrow().memories);

		if self.debug_memory {
			display_fn_executor_memory("PRE 🚩 instance.invoke() fn_executor_memory", &self.memory);

			let runtime_memory = self.runtime_memory.as_ref().unwrap();
			display_runtime_memory("PRE 🚩 instance.invoke(...) runtime_memory", **runtime_memory);
		}

		let res = match instance.invoke(
			export_name,
			&args,
			state,
			&mut SandboxContextImpl { dispatch_thunk, executor: self },
		) {
			// Ok(None) => Ok(ERR_OK),
			Ok(None) => {
				log::info!(target: LOG_TARGET, "invoke WIP 1.A: ---> {:?}", instance_id);
				Ok(ERR_OK)
			},
			Ok(Some(val)) => {
				log::info!(target: LOG_TARGET, "invoke WIP 1.B: ---> {:?}", instance_id);
				// Serialize return value and write it back into the memory.
				sp_wasm_interface::ReturnValue::Value(val).using_encoded(|val| {
					if val.len() > return_val_len as usize {
						return Err("Return value buffer is too small".into())
					}
					self.write_memory(return_val, val).map_err(|_| "Return value buffer is OOB")?;
					Ok(ERR_OK)
				})
			},
			Err(err) => {
				log::info!(target: LOG_TARGET, "invoke WIP 1.C: ---> {:?}, error={:?}", instance_id, err);
				Ok(ERR_EXECUTION)
			},
		};

		if self.debug_memory {
			display_fn_executor_memory(
				"POST 🏁 instance.invoke() fn_executor_memory",
				&self.memory,
			);

			let runtime_memory = self.runtime_memory.as_ref().unwrap();
			display_runtime_memory("POST 🏁 instance.invoke() runtime_memory", **runtime_memory);
		}

		log::info!(target: LOG_TARGET, "invoke END: instance_id={:?}, export_name={:?}, args={:?}, return_val={:?}, return_val_len={:?}, state={:?}", instance_id, export_name, args, return_val, return_val_len, state);

		if instance_id == 0u32 {
			self.debug_memory = false;
		}

		res
	}

	pub fn instance_teardown(&mut self, instance_id: u32) -> WResult<()> {
		log::info!(target: LOG_TARGET, "instance_teardown START: instance_id={:?}", instance_id);

		let res = self
			.sandbox_store
			.borrow_mut()
			.instance_teardown(instance_id)
			.map_err(|e| e.to_string());

		log::info!(target: LOG_TARGET, "instance_teardown END: instance_id={:?}", instance_id);

		res
	}

	pub fn instance_new(
		&mut self,
		dispatch_thunk_id: u32,
		wasm: &[u8],
		raw_env_def: &[u8],
		state: u32,
	) -> WResult<u32> {
		self.debug_memory = true;

		log::info!(target: LOG_TARGET, "instance_new START: dispatch_thunk_id={:?}, raw_env_def={:?}, state={:?}", dispatch_thunk_id, raw_env_def, state);

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
		log::info!(target: LOG_TARGET, "instance_new dispatch_thunk={:?}", dispatch_thunk);

		let guest_env = match GuestEnvironment::decode(&*self.sandbox_store.borrow(), raw_env_def) {
			Ok(guest_env) => guest_env,
			Err(_) => return Ok(ERR_MODULE as u32),
		};

		let store = self.sandbox_store.clone();

		if self.debug_memory {
			display_fn_executor_memory(
				"PRE 🚩 store.instantiate(...) fn_executor_memory",
				&self.memory,
			);

			let runtime_memory = self.runtime_memory.as_ref().unwrap();
			display_runtime_memory(
				"PRE 🚩 store.instantiate(...) runtime_memory",
				**runtime_memory,
			);
		}

		let result = store.borrow_mut().instantiate(
			wasm,
			guest_env,
			state,
			&mut SandboxContextImpl { executor: self, dispatch_thunk: dispatch_thunk.clone() },
		);

		if self.debug_memory {
			display_fn_executor_memory(
				"POST 🏁 store.instantiate(...) fn_executor_memory",
				&self.memory,
			);

			let runtime_memory = self.runtime_memory.as_ref().unwrap();
			display_runtime_memory(
				"POST 🏁 store.instantiate(...) runtime_memory",
				**runtime_memory,
			);
		}

		let instance_idx_or_err_code =
			match result.map(|i| i.register(&mut store.borrow_mut(), dispatch_thunk)) {
				Ok(instance_idx) => instance_idx,
				Err(InstantiationError::StartTrapped) => ERR_EXECUTION,
				Err(_) => ERR_MODULE,
			};

		log::info!(target: LOG_TARGET, "instance_new END: dispatch_thunk_id={:?}, raw_env_def={:?}, state={:?}, instance_idx_or_err_code={:?}", dispatch_thunk_id, raw_env_def, state, instance_idx_or_err_code);

		self.debug_memory = false;

		Ok(instance_idx_or_err_code)
	}

	pub fn get_global_val(
		&self,
		instance_idx: u32,
		name: &str,
	) -> WResult<Option<sp_wasm_interface::Value>> {
		log::info!(target: LOG_TARGET, "get_global_val START: instance_idx={:?}, name={:?}", instance_idx, name);

		let res = self
			.sandbox_store
			.borrow()
			.instance(instance_idx)
			.map(|i| i.get_global_val(name))
			.map_err(|e| e.to_string());

		log::info!(target: LOG_TARGET, "get_global_val END: instance_idx={:?}, name={:?}", instance_idx, name);

		res
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

	// custom field
	is_inited: bool,
	sandbox_store: Option<Rc<RefCell<Store<wasmi::FuncRef>>>>,
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
			self.sandbox_store.clone(),
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
	sandbox_store: Option<Rc<RefCell<Store<wasmi::FuncRef>>>>,
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
		sandbox_store,
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
		InvokeMethod::Export(method) => {
			log::info!(
				target: LOG_TARGET,
				"call_in_wasm_module ---> method={}",
				method,
			);
			module_instance
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
				})
		},
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

	let module: parity_wasm::elements::Module = blob.into_inner();

	// let rules = wasm_instrument::gas_metering::ConstantCostRules::default();
	// let backend = wasm_instrument::gas_metering::host_function::Injector::new(
	// 	"env",
	// 	"ext_custom_host_functions_gas_version_1",
	// );
	// let module = wasm_instrument::gas_metering::inject(module, backend, &rules).unwrap();

	let module = Module::from_parity_wasm_module(module).map_err(|_| WasmError::InvalidModule)?;

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
			is_inited: false,
			sandbox_store: None,
		}))
	}
}

// THIS impl block was added to experiment with the WasmiInstance, it is not a part of original
// substrate code
impl WasmiRuntime {
	pub fn new_wasmi_instance(&self) -> std::result::Result<Box<WasmiInstance>, Error> {
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
			is_inited: false,
			sandbox_store: None,
		}))
	}
}

// THIS impl block was added to experiment with the WasmiInstance, it is not a part of original
// substrate code
impl WasmiInstance {
	// pub fn create_function_executor(&self) -> FunctionExecutor {
	// 	self.memory
	// 		.erase()
	// 		.map_err(|e| {
	// 			// Snapshot restoration failed. This is pretty unexpected since this can happen
	// 			// if some invariant is broken or if the system is under extreme memory pressure
	// 			// (so erasing fails).
	// 			log::error!(target: "wasm-executor", "snapshot restoration failed: {}", e);
	// 			WasmError::ErasingFailed(e.to_string())
	// 		})
	// 		.expect("memory.erase to be ok");

	// 	// Second, reapply data segments into the linear memory.
	// 	self.data_segments_snapshot
	// 		.apply(|offset, contents| self.memory.set(offset, contents))
	// 		.expect("data_segments_snapshot.apply to be ok"); // todo: fix error type

	// 	self.global_vals_snapshot
	// 		.apply(&self.instance)
	// 		.expect("global_vals_snapshot.apply to be ok");

	// 	let table = &self
	// 		.instance
	// 		.export_by_name("__indirect_function_table")
	// 		.and_then(|e| e.as_table().cloned());

	// 	log::info!(
	// 		target: LOG_TARGET,
	// 		"Looking at __indirect_function_table: table={:?}",
	// 		table.clone().expect("Table to be inited"),
	// 	);

	// 	let heap_base = get_heap_base(&self.instance).expect("get_heap_base to be ok");

	// 	let function_executor = FunctionExecutor::new(
	// 		self.memory.clone(),
	// 		heap_base,
	// 		table.clone(),
	// 		self.host_functions.clone(),
	// 		self.allow_missing_func_imports,
	// 		self.missing_functions.clone(),
	// 		None,
	// 	)
	// 	.expect("function_executor to be created");

	// 	function_executor
	// }

	pub fn create_function_executor_2(&mut self) -> FunctionExecutor {
		log::info!(target: "wasm-executor", "is_inited ---> {}", self.is_inited);

		if !self.is_inited {
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

			log::info!(
				target: LOG_TARGET,
				"Looking at __indirect_function_table: table={:?}",
				table.clone().expect("Table to be inited"),
			);

			let heap_base = get_heap_base(&self.instance).expect("get_heap_base to be ok");

			self.sandbox_store = Some(Rc::new(RefCell::new(Store::new(SandboxBackend::Wasmi))));

			let function_executor = FunctionExecutor::new(
				self.memory.clone(),
				heap_base,
				table.clone(),
				self.host_functions.clone(),
				self.allow_missing_func_imports,
				self.missing_functions.clone(),
				self.sandbox_store.clone(),
			)
			.expect("function_executor to be created");

			self.is_inited = true;
			function_executor
		} else {
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
				self.sandbox_store.clone(),
			)
			.expect("function_executor to be created");

			function_executor
		}
	}
}
