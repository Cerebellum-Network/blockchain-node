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

use std::{cell::RefCell, str, sync::Arc};

use cere_dev_runtime as cere_dev;
use cere_runtime as cere;
use cere_runtime::wasm_binary_unwrap;
use lazy_static::lazy_static;
use parking_lot::ReentrantMutex;
use sc_executor_common::wasm_runtime::WasmModule;
use sp_runtime_interface::runtime_interface;
use sp_wasm_interface::{HostFunctions, Pointer};
use wasmi::{memory_units::Pages, MemoryInstance, TableInstance};

mod freeing_bump;

mod util;

mod sandbox_instance;
mod sandbox_wasmi_backend;

mod wasmi_executor;
use wasmi_executor::{create_runtime, FunctionExecutor, WasmiInstance};

pub fn create_function_executor() -> FunctionExecutor {
	let runtime = wasm_binary_unwrap();
	log::info!(target: "wasm_binary_unwrap", "===> LENGHT OF WASM BINARY {}", runtime.len());
	let blob = sc_executor_common::runtime_blob::RuntimeBlob::uncompress_if_needed(runtime)
		.expect("Runtime Blob to be ok");
	let heap_pages = 2048;
	let allow_missing_func_imports = true;

	let mut host_functions = sp_io::SubstrateHostFunctions::host_functions();
	let benchmarking_host_functions =
		frame_benchmarking::benchmarking::HostFunctions::host_functions();
	let sandbox_host_functions = crate::sandbox::HostFunctions::host_functions();

	host_functions.extend(benchmarking_host_functions);
	host_functions.extend(sandbox_host_functions);

	let runtime = create_runtime(blob, heap_pages, host_functions, allow_missing_func_imports)
		// .map(|runtime| -> Arc<dyn WasmModule> { Arc::new(runtime) });
		.expect("Runtime to be created");

	let runtime_wasmi_instance =
		runtime.new_instance_wasmi_instance().expect("Runtime instance to be created");
	let function_executor = runtime_wasmi_instance.create_function_executor();
	function_executor
}

lazy_static! {
	static ref SANDBOX: ReentrantMutex<RefCell<FunctionExecutor>> =
		ReentrantMutex::new(RefCell::new(create_function_executor()));
}

// lazy_static! {
// 	static ref SANDBOX: ReentrantMutex<RefCell<FunctionExecutor>> =
// 		ReentrantMutex::new(RefCell::new(
// 			FunctionExecutor::new(
// 				MemoryInstance::alloc(Pages(65536 as usize), None).unwrap(),
// 				10000,
// 				Some(TableInstance::alloc(u32::MAX, None).unwrap()),
// 				Arc::new(Vec::new()),
// 				true,
// 				Arc::new(Vec::new())
// 			)
// 			.unwrap()
// 		));
// }

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
