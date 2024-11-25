//! Types and traits for interfacing between the host and the wasm runtime.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
use alloc::{borrow::Cow, vec, vec::Vec};
use core::{iter::Iterator, marker::PhantomData, mem, result};
use sp_wasm_interface::{Function,  Result as WResult, Pointer};


// #[cfg(not(all(feature = "std", feature = "wasmtime")))]
// #[macro_export]
// macro_rules! if_wasmtime_is_enabled {
// 	($($token:tt)*) => {};
// }

/// Sandbox memory identifier.
pub type MemoryId = u32;

/// Result type used by traits in this crate.
#[cfg(feature = "std")]
pub type Result<T> = result::Result<T, String>;
#[cfg(not(feature = "std"))]
pub type Result<T> = result::Result<T, &'static str>;

/// Value types supported by Substrate on the boundary between host/Wasm.
#[derive(Copy, Clone, PartialEq, Debug, Eq)]
pub enum ValueType {
	/// An `i32` value type.
	I32,
	/// An `i64` value type.
	I64,
	/// An `f32` value type.
	F32,
	/// An `f64` value type.
	F64,
}

impl From<ValueType> for u8 {
	fn from(val: ValueType) -> u8 {
		match val {
			ValueType::I32 => 0,
			ValueType::I64 => 1,
			ValueType::F32 => 2,
			ValueType::F64 => 3,
		}
	}
}

impl TryFrom<u8> for ValueType {
	type Error = ();

	fn try_from(val: u8) -> core::result::Result<ValueType, ()> {
		match val {
			0 => Ok(Self::I32),
			1 => Ok(Self::I64),
			2 => Ok(Self::F32),
			3 => Ok(Self::F64),
			_ => Err(()),
		}
	}
}

/// Values supported by Substrate on the boundary between host/Wasm.
#[derive(PartialEq, Debug, Clone, Copy, codec::Encode, codec::Decode)]
pub enum Value {
	/// A 32-bit integer.
	I32(i32),
	/// A 64-bit integer.
	I64(i64),
	/// A 32-bit floating-point number stored as raw bit pattern.
	///
	/// You can materialize this value using `f32::from_bits`.
	F32(u32),
	/// A 64-bit floating-point number stored as raw bit pattern.
	///
	/// You can materialize this value using `f64::from_bits`.
	F64(u64),
}

impl Value {
	/// Returns the type of this value.
	pub fn value_type(&self) -> ValueType {
		match self {
			Value::I32(_) => ValueType::I32,
			Value::I64(_) => ValueType::I64,
			Value::F32(_) => ValueType::F32,
			Value::F64(_) => ValueType::F64,
		}
	}

	/// Return `Self` as `i32`.
	pub fn as_i32(&self) -> Option<i32> {
		match self {
			Self::I32(val) => Some(*val),
			_ => None,
		}
	}
}

/// Provides `Sealed` trait to prevent implementing trait `PointerType` and `WasmTy` outside of this
/// crate.
mod private {
	pub trait Sealed {}

	impl Sealed for u8 {}
	impl Sealed for u16 {}
	impl Sealed for u32 {}
	impl Sealed for u64 {}

	impl Sealed for i32 {}
	impl Sealed for i64 {}
}

/// The word size used in wasm. Normally known as `usize` in Rust.
pub type WordSize = u32;

/// Something that can be converted into a wasm compatible `Value`.
pub trait IntoValue {
	/// The type of the value in wasm.
	const VALUE_TYPE: ValueType;

	/// Convert `self` into a wasm `Value`.
	fn into_value(self) -> Value;
}

/// Something that can may be created from a wasm `Value`.
pub trait TryFromValue: Sized {
	/// Try to convert the given `Value` into `Self`.
	fn try_from_value(val: Value) -> Option<Self>;
}


pub trait FunctionContext {
	/// Read memory from `address` into a vector.
	fn read_memory(&self, address: Pointer<u8>, size: WordSize) -> Result<Vec<u8>> {
		let mut vec = vec![0; size as usize];
		self.read_memory_into(address, &mut vec)?;
		Ok(vec)
	}
	/// Read memory into the given `dest` buffer from `address`.
	fn read_memory_into(&self, address: Pointer<u8>, dest: &mut [u8]) -> Result<()>;
	/// Write the given data at `address` into the memory.
	fn write_memory(&mut self, address: Pointer<u8>, data: &[u8]) -> Result<()>;
	/// Allocate a memory instance of `size` bytes.
	fn allocate_memory(&mut self, size: WordSize) -> Result<Pointer<u8>>;
	/// Deallocate a given memory instance.
	fn deallocate_memory(&mut self, ptr: Pointer<u8>) -> Result<()>;
	/// Registers a panic error message within the executor.
	///
	/// This is meant to be used in situations where the runtime
	/// encounters an unrecoverable error and intends to panic.
	///
	/// Panicking in WASM is done through the [`unreachable`](https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-control)
	/// instruction which causes an unconditional trap and immediately aborts
	/// the execution. It does not however allow for any diagnostics to be
	/// passed through to the host, so while we do know that *something* went
	/// wrong we don't have any direct indication of what *exactly* went wrong.
	///
	/// As a workaround we use this method right before the execution is
	/// actually aborted to pass an error message to the host so that it
	/// can associate it with the next trap, and return that to the caller.
	///
	/// A WASM trap should be triggered immediately after calling this method;
	/// otherwise the error message might be associated with a completely
	/// unrelated trap.
	///
	/// It should only be called once, however calling it more than once
	/// is harmless and will overwrite the previously set error message.
	fn register_panic_error_message(&mut self, message: &str);
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
	// fn instance_new(
	// 	&mut self,
	// 	dispatch_thunk_id: u32,
	// 	wasm: &[u8],
	// 	raw_env_def: &[u8],
	// 	state: u32,
	// ) -> Result<u32>;

	/// Get the value from a global with the given `name`. The sandbox is determined by the
	/// given `instance_idx` instance.
	///
	/// Returns `Some(_)` when the requested global variable could be found.
	fn get_global_val(&self, instance_idx: u32, name: &str) -> WResult<Option<Value>>;

	/// Instantiate a new sandbox instance with the given `wasm_code`.
	fn instantiate(
		&mut self,
		dispatch_thunk: u32,
		wasm_code: &[u8],
		env_def: &[u8],
		state_ptr: Pointer<u8>,
	) -> u32;
}
