// This file is part of Substrate.

// Copyright (C) 2019-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Types and traits for interfacing between the host and the wasm runtime.

#![cfg_attr(not(feature = "std"), no_std)]

use std::{cell::RefCell, collections::HashMap, fmt, ops::Range, rc::Rc, str, sync::Arc};

use codec::{Decode, Encode};
use lazy_static::lazy_static;
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use sc_executor::error::{Error, Result};
use sp_core::RuntimeDebug;
use sp_runtime_interface::runtime_interface;
use sp_std::vec::Vec;
use sp_wasm_interface::{
	Function, FunctionContext, Pointer, Result as WResult, ReturnValue, Value, WordSize,
};
use wasmi::{
	memory_units::Pages, ImportResolver, MemoryInstance, MemoryRef, ModuleInstance, RuntimeArgs,
	RuntimeValue, TableInstance, TableRef, Trap,
};

environmental::environmental!(SandboxContextStore: trait SandboxContext);

mod freeing_bump;
use freeing_bump::FreeingBumpHeapAllocator;

const LOG_TARGET: &str = "cere-sanbox";

/// Error error that can be returned from host function.
#[derive(Encode, Decode, RuntimeDebug)]
pub struct HostError;

pub struct SandboxInstance {
	backend_instance: BackendInstance,
	guest_to_supervisor_mapping: GuestToSupervisorFunctionMapping,
}

impl SandboxInstance {
	/// Invoke an exported function by a name.
	///
	/// `supervisor_externals` is required to execute the implementations
	/// of the syscalls that published to a sandboxed module instance.
	///
	/// The `state` parameter can be used to provide custom data for
	/// these syscall implementations.
	pub fn invoke(
		&self,
		export_name: &str,
		args: &[sp_wasm_interface::Value],
		state: u32,
		sandbox_context: &mut dyn SandboxContext,
	) -> std::result::Result<Option<sp_wasm_interface::Value>, Error> {
		match &self.backend_instance {
			BackendInstance::Wasmi(wasmi_instance) =>
				wasmi_invoke(self, wasmi_instance, export_name, args, state, sandbox_context),
		}
	}

	/// Get the value from a global with the given `name`.
	///
	/// Returns `Some(_)` if the global could be found.
	pub fn get_global_val(&self, name: &str) -> Option<sp_wasm_interface::Value> {
		match &self.backend_instance {
			BackendInstance::Wasmi(wasmi_instance) => wasmi_get_global(wasmi_instance, name),
		}
	}
}

enum BackendInstance {
	/// Wasmi module instance
	Wasmi(wasmi::ModuleRef),
}

struct GuestToSupervisorFunctionMapping {
	/// Position of elements in this vector are interpreted
	/// as indices of guest functions and are mapped to
	/// corresponding supervisor function indices.
	funcs: Vec<SupervisorFuncIndex>,
}

impl GuestToSupervisorFunctionMapping {
	/// Create an empty function mapping
	fn new() -> GuestToSupervisorFunctionMapping {
		GuestToSupervisorFunctionMapping { funcs: Vec::new() }
	}

	/// Add a new supervisor function to the mapping.
	/// Returns a newly assigned guest function index.
	fn define(&mut self, supervisor_func: SupervisorFuncIndex) -> GuestFuncIndex {
		let idx = self.funcs.len();
		self.funcs.push(supervisor_func);
		GuestFuncIndex(idx)
	}

	/// Find supervisor function index by its corresponding guest function index
	fn func_by_guest_index(&self, guest_func_idx: GuestFuncIndex) -> Option<SupervisorFuncIndex> {
		self.funcs.get(guest_func_idx.0).cloned()
	}
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SupervisorFuncIndex(usize);

impl From<SupervisorFuncIndex> for usize {
	fn from(index: SupervisorFuncIndex) -> Self {
		index.0
	}
}

/// Memory reference in terms of a selected backend
#[derive(Clone, Debug)]
pub enum Memory {
	/// Wasmi memory reference
	Wasmi(WasmiMemoryWrapper),
}

impl Memory {
	/// View as wasmi memory
	pub fn as_wasmi(&self) -> Option<WasmiMemoryWrapper> {
		match self {
			Memory::Wasmi(memory) => Some(memory.clone()),
		}
	}
}

impl MemoryTransfer for Memory {
	fn read(&self, source_addr: Pointer<u8>, size: usize) -> Result<Vec<u8>> {
		match self {
			Memory::Wasmi(sandboxed_memory) => sandboxed_memory.read(source_addr, size),
		}
	}

	fn read_into(&self, source_addr: Pointer<u8>, destination: &mut [u8]) -> Result<()> {
		match self {
			Memory::Wasmi(sandboxed_memory) => sandboxed_memory.read_into(source_addr, destination),
		}
	}

	fn write_from(&self, dest_addr: Pointer<u8>, source: &[u8]) -> Result<()> {
		match self {
			Memory::Wasmi(sandboxed_memory) => sandboxed_memory.write_from(dest_addr, source),
		}
	}
}

/// Wasmi provides direct access to its memory using slices.
///
/// This wrapper limits the scope where the slice can be taken to
#[derive(Debug, Clone)]
pub struct WasmiMemoryWrapper(wasmi::MemoryRef);

impl WasmiMemoryWrapper {
	/// Take ownership of the memory region and return a wrapper object
	fn new(memory: wasmi::MemoryRef) -> Self {
		Self(memory)
	}
}

/// Construct a range from an offset to a data length after the offset.
/// Returns None if the end of the range would exceed some maximum offset.
pub fn checked_range(offset: usize, len: usize, max: usize) -> Option<Range<usize>> {
	let end = offset.checked_add(len)?;
	if end <= max {
		Some(offset..end)
	} else {
		None
	}
}

/// Provides safe memory access interface using an external buffer
pub trait MemoryTransfer {
	/// Read data from a slice of memory into a newly allocated buffer.
	///
	/// Returns an error if the read would go out of the memory bounds.
	fn read(&self, source_addr: Pointer<u8>, size: usize) -> Result<Vec<u8>>;

	/// Read data from a slice of memory into a destination buffer.
	///
	/// Returns an error if the read would go out of the memory bounds.
	fn read_into(&self, source_addr: Pointer<u8>, destination: &mut [u8]) -> Result<()>;

	/// Write data to a slice of memory.
	///
	/// Returns an error if the write would go out of the memory bounds.
	fn write_from(&self, dest_addr: Pointer<u8>, source: &[u8]) -> Result<()>;
}

impl MemoryTransfer for WasmiMemoryWrapper {
	fn read(&self, source_addr: Pointer<u8>, size: usize) -> Result<Vec<u8>> {
		self.0.with_direct_access(|source| {
			let range = checked_range(source_addr.into(), size, source.len())
				.ok_or_else(|| Error::Other("memory read is out of bounds".into()))?;

			Ok(Vec::from(&source[range]))
		})
	}

	fn read_into(&self, source_addr: Pointer<u8>, destination: &mut [u8]) -> Result<()> {
		self.0.with_direct_access(|source| {
			let range = checked_range(source_addr.into(), destination.len(), source.len())
				.ok_or_else(|| Error::Other("memory read is out of bounds".into()))?;

			destination.copy_from_slice(&source[range]);
			Ok(())
		})
	}

	fn write_from(&self, dest_addr: Pointer<u8>, source: &[u8]) -> Result<()> {
		self.0.with_direct_access_mut(|destination| {
			let range = checked_range(dest_addr.into(), source.len(), destination.len())
				.ok_or_else(|| Error::Other("memory write is out of bounds".into()))?;

			destination[range].copy_from_slice(source);
			Ok(())
		})
	}
}

#[derive(Debug)]
struct CustomHostError(String);

impl fmt::Display for CustomHostError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "HostError: {}", self.0)
	}
}

impl wasmi::HostError for CustomHostError {}
/// Construct trap error from specified message
fn trap(msg: &'static str) -> Trap {
	Trap::host(CustomHostError(msg.into()))
}

impl<'a> wasmi::Externals for GuestExternals<'a> {
	fn invoke_index(
		&mut self,
		index: usize,
		args: RuntimeArgs,
	) -> std::result::Result<Option<RuntimeValue>, Trap> {
		SandboxContextStore::with(|sandbox_context| {
			// Make `index` typesafe again.
			let index = GuestFuncIndex(index);

			// Convert function index from guest to supervisor space
			let func_idx = self.sandbox_instance
				.guest_to_supervisor_mapping
				.func_by_guest_index(index)
				.expect(
					"`invoke_index` is called with indexes registered via `FuncInstance::alloc_host`;
					`FuncInstance::alloc_host` is called with indexes that were obtained from `guest_to_supervisor_mapping`;
					`func_by_guest_index` called with `index` can't return `None`;
					qed"
				);

            let invoke_args_data: Vec<u8> = args
                .as_ref()
                .iter()
                .cloned()
                .map(|value| match value {
                    wasmi::RuntimeValue::I32(val) => Value::I32(val),
                    wasmi::RuntimeValue::I64(val) => Value::I64(val),
                    wasmi::RuntimeValue::F32(val) => Value::F32(val.into()),
                    wasmi::RuntimeValue::F64(val) => Value::F64(val.into()),
                })
                .collect::<Vec<_>>()
                .encode();

			let state = self.state;

			// Move serialized arguments inside the memory, invoke dispatch thunk and
			// then free allocated memory.
			let invoke_args_len = invoke_args_data.len() as WordSize;
			let invoke_args_ptr = sandbox_context
				.supervisor_context()
				.allocate_memory(invoke_args_len)
				.map_err(|_| trap("Can't allocate memory in supervisor for the arguments"))?;

			let deallocate = |supervisor_context: &mut dyn FunctionContext, ptr, fail_msg| {
				supervisor_context.deallocate_memory(ptr).map_err(|_| trap(fail_msg))
			};

			if sandbox_context
				.supervisor_context()
				.write_memory(invoke_args_ptr, &invoke_args_data)
				.is_err()
			{
				deallocate(
					sandbox_context.supervisor_context(),
					invoke_args_ptr,
					"Failed dealloction after failed write of invoke arguments",
				)?;
				return Err(trap("Can't write invoke args into memory"))
			}

			let result = sandbox_context.invoke(
				invoke_args_ptr,
				invoke_args_len,
				state,
				func_idx,
			);

			deallocate(
				sandbox_context.supervisor_context(),
				invoke_args_ptr,
				"Can't deallocate memory for dispatch thunk's invoke arguments",
			)?;

			// let result = result?;
			let result = result.map_err(|_| trap("Could not invoke sandbox"))?;

			// dispatch_thunk returns pointer to serialized arguments.
			// Unpack pointer and len of the serialized result data.
			let (serialized_result_val_ptr, serialized_result_val_len) = {
				// Cast to u64 to use zero-extension.
				let v = result as u64;
				let ptr = (v as u64 >> 32) as u32;
				let len = (v & 0xFFFFFFFF) as u32;
				(Pointer::new(ptr), len)
			};

			let serialized_result_val = sandbox_context
				.supervisor_context()
				.read_memory(serialized_result_val_ptr, serialized_result_val_len)
				.map_err(|_| trap("Can't read the serialized result from dispatch thunk"));

			deallocate(
				sandbox_context.supervisor_context(),
				serialized_result_val_ptr,
				"Can't deallocate memory for dispatch thunk's result",
			)
			.and(serialized_result_val)
			.and_then(|serialized_result_val| {
				let result_val = std::result::Result::<ReturnValue, HostError>::decode(&mut serialized_result_val.as_slice())
					.map_err(|_| trap("Decoding Result<ReturnValue, HostError> failed!"))?;

				match result_val {
					Ok(return_value) => Ok(match return_value {
						ReturnValue::Unit => None,
						ReturnValue::Value(typed_value) => Some( match typed_value {
                            Value::I32(val) => RuntimeValue::I32(val),
                            Value::I64(val) => RuntimeValue::I64(val),
                            Value::F32(val) => RuntimeValue::F32(val.into()),
                            Value::F64(val) => RuntimeValue::F64(val.into()),
                        }),
					}),
					Err(HostError) => Err(trap("Supervisor function returned sandbox::HostError")),
				}
			})
		}).expect("SandboxContextStore is set when invoking sandboxed functions; qed")
	}
}

/// Sandbox memory identifier.
pub type MemoryId = u32;

/// Constant for specifying no limit when creating a sandboxed
/// memory instance. For FFI purposes.
pub const MEM_UNLIMITED: u32 = -1i32 as u32;

/// No error happened.
///
/// For FFI purposes.
pub const ERR_OK: u32 = 0;

/// Validation or instantiation error occurred when creating new
/// sandboxed module instance.
///
/// For FFI purposes.
pub const ERR_MODULE: u32 = -1i32 as u32;

/// Out-of-bounds access attempted with memory or table.
///
/// For FFI purposes.
pub const ERR_OUT_OF_BOUNDS: u32 = -2i32 as u32;

/// Execution error occurred (typically trap).
///
/// For FFI purposes.
pub const ERR_EXECUTION: u32 = -3i32 as u32;

pub struct Store<DT> {
	/// Stores the instance and the dispatch thunk associated to per instance.
	///
	/// Instances are `Some` until torn down.
	instances: Vec<Option<(Rc<SandboxInstance>, DT)>>,
	/// Memories are `Some` until torn down.
	memories: Vec<Option<Memory>>,
	backend_context: BackendContext,
}

/// Allocate new memory region
pub fn wasmi_new_memory(initial: u32, maximum: Option<u32>) -> Result<Memory> {
	let memory = Memory::Wasmi(WasmiMemoryWrapper::new(
		MemoryInstance::alloc(Pages(initial as usize), maximum.map(|m| Pages(m as usize)))
			.map_err(|error| Error::Other(error.to_string()))?,
	));

	Ok(memory)
}

/// Index of a function within guest index space.
///
/// This index is supposed to be used as index for `Externals`.
#[derive(Copy, Clone, Debug, PartialEq)]
struct GuestFuncIndex(usize);

/// Holds sandbox function and memory imports and performs name resolution
struct Imports {
	/// Maps qualified function name to its guest function index
	func_map: HashMap<(Vec<u8>, Vec<u8>), GuestFuncIndex>,

	/// Maps qualified field name to its memory reference
	memories_map: HashMap<(Vec<u8>, Vec<u8>), Memory>,
}

impl Imports {
	fn func_by_name(&self, module_name: &str, func_name: &str) -> Option<GuestFuncIndex> {
		self.func_map
			.get(&(module_name.as_bytes().to_owned(), func_name.as_bytes().to_owned()))
			.cloned()
	}

	fn memory_by_name(&self, module_name: &str, memory_name: &str) -> Option<Memory> {
		self.memories_map
			.get(&(module_name.as_bytes().to_owned(), memory_name.as_bytes().to_owned()))
			.cloned()
	}
}

impl ImportResolver for Imports {
	fn resolve_func(
		&self,
		module_name: &str,
		field_name: &str,
		signature: &wasmi::Signature,
	) -> std::result::Result<wasmi::FuncRef, wasmi::Error> {
		let idx = self.func_by_name(module_name, field_name).ok_or_else(|| {
			wasmi::Error::Instantiation(format!("Export {}:{} not found", module_name, field_name))
		})?;

		Ok(wasmi::FuncInstance::alloc_host(signature.clone(), idx.0))
	}

	fn resolve_memory(
		&self,
		module_name: &str,
		field_name: &str,
		_memory_type: &wasmi::MemoryDescriptor,
	) -> std::result::Result<wasmi::MemoryRef, wasmi::Error> {
		let mem = self.memory_by_name(module_name, field_name).ok_or_else(|| {
			wasmi::Error::Instantiation(format!("Export {}:{} not found", module_name, field_name))
		})?;

		let wrapper = mem.as_wasmi().ok_or_else(|| {
			wasmi::Error::Instantiation(format!(
				"Unsupported non-wasmi export {}:{}",
				module_name, field_name
			))
		})?;

		// Here we use inner memory reference only to resolve the imports
		// without accessing the memory contents. All subsequent memory accesses
		// should happen through the wrapper, that enforces the memory access protocol.
		let mem = wrapper.0;

		Ok(mem)
	}

	fn resolve_global(
		&self,
		module_name: &str,
		field_name: &str,
		_global_type: &wasmi::GlobalDescriptor,
	) -> std::result::Result<wasmi::GlobalRef, wasmi::Error> {
		Err(wasmi::Error::Instantiation(format!("Export {}:{} not found", module_name, field_name)))
	}

	fn resolve_table(
		&self,
		module_name: &str,
		field_name: &str,
		_table_type: &wasmi::TableDescriptor,
	) -> std::result::Result<wasmi::TableRef, wasmi::Error> {
		Err(wasmi::Error::Instantiation(format!("Export {}:{} not found", module_name, field_name)))
	}
}

/// An environment in which the guest module is instantiated.
pub struct GuestEnvironment {
	/// Function and memory imports of the guest module
	imports: Imports,

	/// Supervisor functinons mapped to guest index space
	guest_to_supervisor_mapping: GuestToSupervisorFunctionMapping,
}

/// Error occurred during instantiation of a sandboxed module.
pub enum InstantiationError {
	/// Something wrong with the environment definition. It either can't
	/// be decoded, have a reference to a non-existent or torn down memory instance.
	EnvironmentDefinitionCorrupted,
	/// Provided module isn't recognized as a valid webassembly binary.
	ModuleDecoding,
	/// Module is a well-formed webassembly binary but could not be instantiated. This could
	/// happen because, e.g. the module imports entries not provided by the environment.
	Instantiation,
	/// Module is well-formed, instantiated and linked, but while executing the start function
	/// a trap was generated.
	StartTrapped,
	/// The code was compiled with a CPU feature not available on the host.
	CpuFeature,
}

/// An entry in a environment definition table.
///
/// Each entry has a two-level name and description of an entity
/// being defined.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct Entry {
	/// Module name of which corresponding entity being defined.
	pub module_name: Vec<u8>,
	/// Field name in which corresponding entity being defined.
	pub field_name: Vec<u8>,
	/// External entity being defined.
	pub entity: ExternEntity,
}

/// Describes an entity to define or import into the environment.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub enum ExternEntity {
	/// Function that is specified by an index in a default table of
	/// a module that creates the sandbox.
	#[codec(index = 1)]
	Function(u32),

	/// Linear memory that is specified by some identifier returned by sandbox
	/// module upon creation new sandboxed memory.
	#[codec(index = 2)]
	Memory(u32),
}

/// Definition of runtime that could be used by sandboxed code.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct EnvironmentDefinition {
	/// Vector of all entries in the environment definition.
	pub entries: Vec<Entry>,
}

fn decode_environment_definition(
	mut raw_env_def: &[u8],
	memories: &[Option<Memory>],
) -> std::result::Result<(Imports, GuestToSupervisorFunctionMapping), InstantiationError> {
	let env_def = EnvironmentDefinition::decode(&mut raw_env_def)
		.map_err(|_| InstantiationError::EnvironmentDefinitionCorrupted)?;

	let mut func_map = HashMap::new();
	let mut memories_map = HashMap::new();
	let mut guest_to_supervisor_mapping = GuestToSupervisorFunctionMapping::new();

	for entry in &env_def.entries {
		let module = entry.module_name.clone();
		let field = entry.field_name.clone();

		match entry.entity {
			ExternEntity::Function(func_idx) => {
				let externals_idx =
					guest_to_supervisor_mapping.define(SupervisorFuncIndex(func_idx as usize));
				func_map.insert((module, field), externals_idx);
			},
			ExternEntity::Memory(memory_idx) => {
				let memory_ref = memories
					.get(memory_idx as usize)
					.cloned()
					.ok_or(InstantiationError::EnvironmentDefinitionCorrupted)?
					.ok_or(InstantiationError::EnvironmentDefinitionCorrupted)?;
				memories_map.insert((module, field), memory_ref);
			},
		}
	}

	Ok((Imports { func_map, memories_map }, guest_to_supervisor_mapping))
}

impl GuestEnvironment {
	/// Decodes an environment definition from the given raw bytes.
	///
	/// Returns `Err` if the definition cannot be decoded.
	pub fn decode<DT>(
		store: &Store<DT>,
		raw_env_def: &[u8],
	) -> std::result::Result<Self, InstantiationError> {
		let (imports, guest_to_supervisor_mapping) =
			decode_environment_definition(raw_env_def, &store.memories)?;
		Ok(Self { imports, guest_to_supervisor_mapping })
	}
}

/// Instantiate a module within a sandbox context
pub fn wasmi_instantiate(
	wasm: &[u8],
	guest_env: GuestEnvironment,
	state: u32,
	sandbox_context: &mut dyn SandboxContext,
) -> std::result::Result<Rc<SandboxInstance>, InstantiationError> {
	let wasmi_module =
		wasmi::Module::from_buffer(wasm).map_err(|_| InstantiationError::ModuleDecoding)?;
	let wasmi_instance = ModuleInstance::new(&wasmi_module, &guest_env.imports)
		.map_err(|_| InstantiationError::Instantiation)?;

	let sandbox_instance = Rc::new(SandboxInstance {
		// In general, it's not a very good idea to use `.not_started_instance()` for
		// anything but for extracting memory and tables. But in this particular case, we
		// are extracting for the purpose of running `start` function which should be ok.
		backend_instance: BackendInstance::Wasmi(wasmi_instance.not_started_instance().clone()),
		guest_to_supervisor_mapping: guest_env.guest_to_supervisor_mapping,
	});

	with_guest_externals(&sandbox_instance, state, |guest_externals| {
		SandboxContextStore::using(sandbox_context, || {
			wasmi_instance
				.run_start(guest_externals)
				.map_err(|_| InstantiationError::StartTrapped)
		})
	})?;

	Ok(sandbox_instance)
}

/// Invoke a function within a sandboxed module
pub fn wasmi_invoke(
	instance: &SandboxInstance,
	module: &wasmi::ModuleRef,
	export_name: &str,
	args: &[Value],
	state: u32,
	sandbox_context: &mut dyn SandboxContext,
) -> std::result::Result<Option<Value>, Error> {
	with_guest_externals(instance, state, |guest_externals| {
		SandboxContextStore::using(sandbox_context, || {
			let args = args
				.iter()
				.cloned()
				.map(|value| match value {
					Value::I32(val) => RuntimeValue::I32(val),
					Value::I64(val) => RuntimeValue::I64(val),
					Value::F32(val) => RuntimeValue::F32(val.into()),
					Value::F64(val) => RuntimeValue::F64(val.into()),
				})
				.collect::<Vec<_>>();

			module
				.invoke_export(export_name, &args, guest_externals)
				.map(|result| {
					result.map(|value| match value {
						wasmi::RuntimeValue::I32(val) => Value::I32(val),
						wasmi::RuntimeValue::I64(val) => Value::I64(val),
						wasmi::RuntimeValue::F32(val) => Value::F32(val.into()),
						wasmi::RuntimeValue::F64(val) => Value::F64(val.into()),
					})
				})
				.map_err(|error| Error::Other(error.to_string()))
		})
	})
}

/// Get global value by name
pub fn wasmi_get_global(instance: &wasmi::ModuleRef, name: &str) -> Option<Value> {
	// Some(instance.export_by_name(name)?.as_global()?.get().into())
	let value = instance.export_by_name(name)?.as_global()?.get();
	let val = match value {
		wasmi::RuntimeValue::I32(val) => Value::I32(val),
		wasmi::RuntimeValue::I64(val) => Value::I64(val),
		wasmi::RuntimeValue::F32(val) => Value::F32(val.into()),
		wasmi::RuntimeValue::F64(val) => Value::F64(val.into()),
	};

	Some(val)
}

/// Implementation of [`Externals`] that allows execution of guest module with
/// [externals][`Externals`] that might refer functions defined by supervisor.
///
/// [`Externals`]: ../wasmi/trait.Externals.html
pub struct GuestExternals<'a> {
	/// Instance of sandboxed module to be dispatched
	sandbox_instance: &'a SandboxInstance,

	/// External state passed to guest environment, see the `instantiate` function
	state: u32,
}

fn with_guest_externals<R, F>(sandbox_instance: &SandboxInstance, state: u32, f: F) -> R
where
	F: FnOnce(&mut GuestExternals) -> R,
{
	f(&mut GuestExternals { sandbox_instance, state })
}

impl<DT: Clone> Store<DT> {
	/// Create a new empty sandbox store.
	pub fn new(backend: SandboxBackend) -> Self {
		Store {
			instances: Vec::new(),
			memories: Vec::new(),
			backend_context: BackendContext::new(backend),
		}
	}

	/// Create a new memory instance and return it's index.
	///
	/// # Errors
	///
	/// Returns `Err` if the memory couldn't be created.
	/// Typically happens if `initial` is more than `maximum`.
	pub fn new_memory(&mut self, initial: u32, maximum: u32) -> Result<u32> {
		let memories = &mut self.memories;
		let backend_context = &self.backend_context;

		let maximum = match maximum {
			MEM_UNLIMITED => None,
			specified_limit => Some(specified_limit),
		};

		let memory = match &backend_context {
			BackendContext::Wasmi => wasmi_new_memory(initial, maximum)?,

			#[cfg(feature = "wasmer-sandbox")]
			BackendContext::Wasmer(context) => wasmer_new_memory(context, initial, maximum)?,
		};

		let mem_idx = memories.len();
		memories.push(Some(memory));

		Ok(mem_idx as u32)
	}

	/// Returns `SandboxInstance` by `instance_idx`.
	///
	/// # Errors
	///
	/// Returns `Err` If `instance_idx` isn't a valid index of an instance or
	/// instance is already torndown.
	pub fn instance(&self, instance_idx: u32) -> Result<Rc<SandboxInstance>> {
		self.instances
			.get(instance_idx as usize)
			.ok_or("Trying to access a non-existent instance")?
			.as_ref()
			.map(|v| v.0.clone())
			.ok_or_else(|| "Trying to access a torndown instance".into())
	}

	/// Returns dispatch thunk by `instance_idx`.
	///
	/// # Errors
	///
	/// Returns `Err` If `instance_idx` isn't a valid index of an instance or
	/// instance is already torndown.
	pub fn dispatch_thunk(&self, instance_idx: u32) -> Result<DT> {
		self.instances
			.get(instance_idx as usize)
			.as_ref()
			.ok_or("Trying to access a non-existent instance")?
			.as_ref()
			.map(|v| v.1.clone())
			.ok_or_else(|| "Trying to access a torndown instance".into())
	}

	/// Returns reference to a memory instance by `memory_idx`.
	///
	/// # Errors
	///
	/// Returns `Err` If `memory_idx` isn't a valid index of an memory or
	/// if memory has been torn down.
	pub fn memory(&self, memory_idx: u32) -> Result<Memory> {
		self.memories
			.get(memory_idx as usize)
			.cloned()
			.ok_or("Trying to access a non-existent sandboxed memory")?
			.ok_or_else(|| "Trying to access a torndown sandboxed memory".into())
	}

	/// Tear down the memory at the specified index.
	///
	/// # Errors
	///
	/// Returns `Err` if `memory_idx` isn't a valid index of an memory or
	/// if it has been torn down.
	pub fn memory_teardown(&mut self, memory_idx: u32) -> Result<()> {
		match self.memories.get_mut(memory_idx as usize) {
			None => Err("Trying to teardown a non-existent sandboxed memory".into()),
			Some(None) => Err("Double teardown of a sandboxed memory".into()),
			Some(memory) => {
				*memory = None;
				Ok(())
			},
		}
	}

	/// Tear down the instance at the specified index.
	///
	/// # Errors
	///
	/// Returns `Err` if `instance_idx` isn't a valid index of an instance or
	/// if it has been torn down.
	pub fn instance_teardown(&mut self, instance_idx: u32) -> Result<()> {
		match self.instances.get_mut(instance_idx as usize) {
			None => Err("Trying to teardown a non-existent instance".into()),
			Some(None) => Err("Double teardown of an instance".into()),
			Some(instance) => {
				*instance = None;
				Ok(())
			},
		}
	}

	/// Instantiate a guest module and return it's index in the store.
	///
	/// The guest module's code is specified in `wasm`. Environment that will be available to
	/// guest module is specified in `guest_env`. A dispatch thunk is used as function that
	/// handle calls from guests. `state` is an opaque pointer to caller's arbitrary context
	/// normally created by `sp_sandbox::Instance` primitive.
	///
	/// Note: Due to borrowing constraints dispatch thunk is now propagated using DTH
	///
	/// Returns uninitialized sandboxed module instance or an instantiation error.
	pub fn instantiate(
		&mut self,
		wasm: &[u8],
		guest_env: GuestEnvironment,
		state: u32,
		sandbox_context: &mut dyn SandboxContext,
	) -> std::result::Result<UnregisteredInstance, InstantiationError> {
		let sandbox_instance = match self.backend_context {
			BackendContext::Wasmi => wasmi_instantiate(wasm, guest_env, state, sandbox_context)?,
		};

		Ok(UnregisteredInstance { sandbox_instance })
	}
}

/// An unregistered sandboxed instance.
///
/// To finish off the instantiation the user must call `register`.
#[must_use]
pub struct UnregisteredInstance {
	sandbox_instance: Rc<SandboxInstance>,
}

impl UnregisteredInstance {
	/// Finalizes instantiation of this module.
	pub fn register<DT>(self, store: &mut Store<DT>, dispatch_thunk: DT) -> u32 {
		// At last, register the instance.
		store.register_sandbox_instance(self.sandbox_instance, dispatch_thunk)
	}
}

/// The sandbox context used to execute sandboxed functions.
pub trait SandboxContext {
	/// Invoke a function in the supervisor environment.
	///
	/// This first invokes the dispatch thunk function, passing in the function index of the
	/// desired function to call and serialized arguments. The thunk calls the desired function
	/// with the deserialized arguments, then serializes the result into memory and returns
	/// reference. The pointer to and length of the result in linear memory is encoded into an
	/// `i64`, with the upper 32 bits representing the pointer and the lower 32 bits representing
	/// the length.
	///
	/// # Errors
	///
	/// Returns `Err` if the dispatch_thunk function has an incorrect signature or traps during
	/// execution.
	fn invoke(
		&mut self,
		invoke_args_ptr: Pointer<u8>,
		invoke_args_len: WordSize,
		state: u32,
		func_idx: SupervisorFuncIndex,
	) -> Result<i64>;

	/// Returns the supervisor context.
	fn supervisor_context(&mut self) -> &mut dyn FunctionContext;
}

/// Sandbox backend to use
pub enum SandboxBackend {
	/// Wasm interpreter
	Wasmi,
}

// Private routines
impl<DT> Store<DT> {
	fn register_sandbox_instance(
		&mut self,
		sandbox_instance: Rc<SandboxInstance>,
		dispatch_thunk: DT,
	) -> u32 {
		let instance_idx = self.instances.len();
		self.instances.push(Some((sandbox_instance, dispatch_thunk)));
		instance_idx as u32
	}
}

pub struct WasmiSandboxExecutor {
	sandbox_store: Rc<RefCell<Store<wasmi::FuncRef>>>,
	heap: RefCell<FreeingBumpHeapAllocator>,
	memory: MemoryRef,
	table: Option<TableRef>,
	host_functions: Arc<Vec<&'static dyn Function>>,
	allow_missing_func_imports: bool,
	missing_functions: Arc<Vec<String>>,
	panic_message: Option<String>,
}
unsafe impl Send for WasmiSandboxExecutor {}

impl WasmiSandboxExecutor {
	fn new(
		m: MemoryRef,
		heap_base: u32,
		t: Option<TableRef>,
		host_functions: Arc<Vec<&'static dyn Function>>,
		allow_missing_func_imports: bool,
		missing_functions: Arc<Vec<String>>,
	) -> Result<Self> {
		Ok(WasmiSandboxExecutor {
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

lazy_static! {
	// // We have to use the ReentrantMutex as every test's thread that needs to perform some configuration on the mock acquires the lock at least 2 times:
	// // the first time when the mock configuration happens, and
	// // the second time when the pallet calls the MockNodeVisitor during execution
	// static ref MOCK_NODE: ReentrantMutex<RefCell<WasmiSandboxExecutor>> =
	// 	ReentrantMutex::new(RefCell::new(WasmiSandboxExecutor::default()));

	static ref SANDBOX: ReentrantMutex<RefCell<WasmiSandboxExecutor>> =
	ReentrantMutex::new(RefCell::new(WasmiSandboxExecutor::new(
		// MemoryInstance::alloc(Pages(1000 as usize), Some(Pages(10000000000 as usize))).unwrap(),
		MemoryInstance::alloc(Pages(65536 as usize), None).unwrap(),
		10000,
		// None,
		Some(TableInstance::alloc(u32::MAX, None).unwrap()),
		Arc::new(Vec::new()),
		true,
		Arc::new(Vec::new())
	).unwrap()));
}

/// Information specific to a particular execution backend
enum BackendContext {
	/// Wasmi specific context
	Wasmi,
}

impl BackendContext {
	pub fn new(backend: SandboxBackend) -> BackendContext {
		match backend {
			SandboxBackend::Wasmi => BackendContext::Wasmi,
		}
	}
}

impl WasmiSandboxExecutor {
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
			Err(_) => return Ok(ERR_OUT_OF_BOUNDS),
			Ok(buffer) => buffer,
		};

		if self.memory.set(buf_ptr.into(), &buffer).is_err() {
			return Ok(ERR_OUT_OF_BOUNDS)
		}

		Ok(ERR_OK)
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
			Err(_) => return Ok(ERR_OUT_OF_BOUNDS),
			Ok(buffer) => buffer,
		};

		if sandboxed_memory.write_from(Pointer::new(offset as u32), &buffer).is_err() {
			return Ok(ERR_OUT_OF_BOUNDS)
		}

		Ok(ERR_OK)
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

struct SandboxContextImpl<'a> {
	executor: &'a mut WasmiSandboxExecutor,
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

impl FunctionContext for WasmiSandboxExecutor {
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

impl wasmi::Externals for WasmiSandboxExecutor {
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
				.map_err(|msg| trap("Function call failed"))
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

/// Something that provides access to the sandbox.
#[runtime_interface(wasm_only)]
pub trait Sandbox {
	/// Instantiate a new sandbox instance with the given `wasm_code`.
	fn instantiate(
		&mut self,
		dispatch_thunk: u32,
		wasm_code: &[u8],
		env_def: &[u8],
		state_ptr: Pointer<u8>,
	) -> u32 {
		let lock = SANDBOX.lock();
		let mut sandbox = lock.borrow_mut();
		sandbox
			.instance_new(dispatch_thunk, wasm_code, env_def, state_ptr.into())
			.expect("Failed to instantiate a new sandbox")
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
		let lock = SANDBOX.lock();
		let mut sandbox = lock.borrow_mut();
		sandbox
			.invoke(instance_idx, function, args, return_val_ptr, return_val_len, state_ptr.into())
			.expect("Failed to invoke function with sandbox")
	}

	/// Create a new memory instance with the given `initial` size and the `maximum` size.
	/// The size is given in wasm pages.
	fn memory_new(&mut self, initial: u32, maximum: u32) -> u32 {
		let lock = SANDBOX.lock();
		let mut sandbox = lock.borrow_mut();
		sandbox
			.memory_new(initial, maximum)
			.expect("Failed to create new memory with sandbox")
	}

	/// Get sandbox memory from the `memory_id` instance at `offset` into the given buffer.
	fn memory_get(
		&mut self,
		memory_idx: u32,
		offset: u32,
		buf_ptr: Pointer<u8>,
		buf_len: u32,
	) -> u32 {
		let lock = SANDBOX.lock();
		let mut sandbox = lock.borrow_mut();
		sandbox
			.memory_get(memory_idx, offset, buf_ptr, buf_len)
			.expect("Failed to get memory with sandbox")
	}

	/// Set sandbox memory from the given value.
	fn memory_set(
		&mut self,
		memory_idx: u32,
		offset: u32,
		val_ptr: Pointer<u8>,
		val_len: u32,
	) -> u32 {
		let lock = SANDBOX.lock();
		let mut sandbox = lock.borrow_mut();
		sandbox
			.memory_set(memory_idx, offset, val_ptr, val_len)
			.expect("Failed to set memory with sandbox")
	}

	/// Delete a memory instance.
	fn memory_teardown(&mut self, memory_idx: u32) {
		let lock = SANDBOX.lock();
		let mut sandbox = lock.borrow_mut();
		sandbox
			.memory_teardown(memory_idx)
			.expect("Failed to teardown memory with sandbox")
	}

	/// Delete a sandbox instance.
	fn instance_teardown(&mut self, instance_idx: u32) {
		let lock = SANDBOX.lock();
		let mut sandbox = lock.borrow_mut();
		sandbox
			.instance_teardown(instance_idx)
			.expect("Failed to teardown sandbox instance")
	}

	/// Get the value from a global with the given `name`. The sandbox is determined by the
	/// given `instance_idx` instance.
	fn get_global_val(
		&mut self,
		instance_idx: u32,
		name: &str,
	) -> Option<sp_wasm_interface::Value> {
		let lock = SANDBOX.lock();
		let sandbox = lock.borrow();
		sandbox
			.get_global_val(instance_idx, name)
			.expect("Failed to get global from sandbox")
	}
}
