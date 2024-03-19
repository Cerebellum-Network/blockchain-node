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

use lazy_static::lazy_static;
use parking_lot::ReentrantMutex;
use sp_runtime_interface::runtime_interface;
use sp_wasm_interface::Pointer;
use wasmi::{memory_units::Pages, MemoryInstance, TableInstance};

mod freeing_bump;

mod util;

mod sandbox_instance;

mod wasmi_function_executor;
use wasmi_function_executor::FunctionExecutor;

mod sandbox_wasmi_backend;

lazy_static! {
	static ref SANDBOX: ReentrantMutex<RefCell<FunctionExecutor>> =
	ReentrantMutex::new(RefCell::new(FunctionExecutor::new(
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
