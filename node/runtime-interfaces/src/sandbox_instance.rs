use std::{collections::HashMap, rc::Rc, str};

use codec::{Decode, Encode};
use sc_executor::error::{Error, Result};
use sp_core::RuntimeDebug;
use sp_wasm_interface::{FunctionContext, Pointer, WordSize};

use crate::{
	sandbox_wasmi_backend::{
		wasmi_get_global, wasmi_instantiate, wasmi_invoke, wasmi_new_memory, WasmiMemoryWrapper,
	},
	util::MemoryTransfer,
};

/// Sandboxed instance of a wasm module.
///
/// It's primary purpose is to [`invoke`] exported functions on it.
///
/// All imports of this instance are specified at the creation time and
/// imports are implemented by the supervisor.
///
/// Hence, in order to invoke an exported function on a sandboxed module instance,
/// it's required to provide supervisor externals: it will be used to execute
/// code in the supervisor context.
///
/// This is generic over a supervisor function reference type.
///
/// [`invoke`]: #method.invoke
pub struct SandboxInstance {
	pub backend_instance: BackendInstance,
	pub guest_to_supervisor_mapping: GuestToSupervisorFunctionMapping,
}

/// Index of a function within guest index space.
///
/// This index is supposed to be used as index for `Externals`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GuestFuncIndex(pub usize);

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

/// This struct holds a mapping from guest index space to supervisor.
pub struct GuestToSupervisorFunctionMapping {
	/// Position of elements in this vector are interpreted
	/// as indices of guest functions and are mapped to
	/// corresponding supervisor function indices.
	pub funcs: Vec<SupervisorFuncIndex>,
}

impl GuestToSupervisorFunctionMapping {
	/// Create an empty function mapping
	pub fn new() -> GuestToSupervisorFunctionMapping {
		GuestToSupervisorFunctionMapping { funcs: Vec::new() }
	}

	/// Add a new supervisor function to the mapping.
	/// Returns a newly assigned guest function index.
	pub fn define(&mut self, supervisor_func: SupervisorFuncIndex) -> GuestFuncIndex {
		let idx = self.funcs.len();
		self.funcs.push(supervisor_func);
		GuestFuncIndex(idx)
	}

	/// Find supervisor function index by its corresponding guest function index
	pub fn func_by_guest_index(
		&self,
		guest_func_idx: GuestFuncIndex,
	) -> Option<SupervisorFuncIndex> {
		self.funcs.get(guest_func_idx.0).cloned()
	}
}

/// Implementation of [`Externals`] that allows execution of guest module with
/// [externals][`Externals`] that might refer functions defined by supervisor.
///
/// [`Externals`]: ../wasmi/trait.Externals.html
pub struct GuestExternals<'a> {
	/// Instance of sandboxed module to be dispatched
	pub sandbox_instance: &'a SandboxInstance,

	/// External state passed to guest environment, see the `instantiate` function
	pub state: u32,
}

/// Holds sandbox function and memory imports and performs name resolution
pub struct Imports {
	/// Maps qualified function name to its guest function index
	pub func_map: HashMap<(Vec<u8>, Vec<u8>), GuestFuncIndex>,

	/// Maps qualified field name to its memory reference
	pub memories_map: HashMap<(Vec<u8>, Vec<u8>), Memory>,
}

impl Imports {
	pub fn func_by_name(&self, module_name: &str, func_name: &str) -> Option<GuestFuncIndex> {
		self.func_map
			.get(&(module_name.as_bytes().to_owned(), func_name.as_bytes().to_owned()))
			.cloned()
	}

	pub fn memory_by_name(&self, module_name: &str, memory_name: &str) -> Option<Memory> {
		self.memories_map
			.get(&(module_name.as_bytes().to_owned(), memory_name.as_bytes().to_owned()))
			.cloned()
	}
}

/// Module instance in terms of selected backend
pub enum BackendInstance {
	/// Wasmi module instance
	Wasmi(wasmi::ModuleRef),
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

/// Index of a function inside the supervisor.
///
/// This is a typically an index in the default table of the supervisor, however
/// the exact meaning of this index is depends on the implementation of dispatch function.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SupervisorFuncIndex(usize);

impl From<SupervisorFuncIndex> for usize {
	fn from(index: SupervisorFuncIndex) -> Self {
		index.0
	}
}

/// This struct keeps track of all sandboxed components.
///
/// This is generic over a supervisor function reference type.
pub struct Store<DT> {
	/// Stores the instance and the dispatch thunk associated to per instance.
	///
	/// Instances are `Some` until torn down.
	pub instances: Vec<Option<(Rc<SandboxInstance>, DT)>>,
	/// Memories are `Some` until torn down.
	pub memories: Vec<Option<Memory>>,
	backend_context: BackendContext,
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

/// An environment in which the guest module is instantiated.
pub struct GuestEnvironment {
	/// Function and memory imports of the guest module
	pub imports: Imports,

	/// Supervisor functinons mapped to guest index space
	pub guest_to_supervisor_mapping: GuestToSupervisorFunctionMapping,
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

/// Error error that can be returned from host function.
#[derive(Encode, Decode, RuntimeDebug)]
pub struct HostError;

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

/// An unregistered sandboxed instance.
///
/// To finish off the instantiation the user must call `register`.
#[must_use]
pub struct UnregisteredInstance {
	pub sandbox_instance: Rc<SandboxInstance>,
}

impl UnregisteredInstance {
	/// Finalizes instantiation of this module.
	pub fn register<DT>(self, store: &mut Store<DT>, dispatch_thunk: DT) -> u32 {
		// At last, register the instance.
		store.register_sandbox_instance(self.sandbox_instance, dispatch_thunk)
	}
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
