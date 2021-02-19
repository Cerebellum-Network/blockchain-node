# CereDDCModule

A module for sending encrypted string data to another account.

## Overview

DDC - Decentralized Data Cloud by Cere Network. The module provides functionality for sending data to another account, including:

* Send Data
To use it in your runtime, you need to implement the module follow below instructions.

### Terminology

### Goals

A account can send encrypted string data to another account.

* String data can be updated.

## Interface

### Dispatchable Functions

* `send_data` - Transfers an `data` of encrypted string from the function caller's account (`origin`) to a `send_to` account.

### Public Functions

* `stringDataOf` - Get the string data of `AccountId`.
* `send_data` - Send encrypted string data to another account.

## Usage

* Go to Developer -> Extrinsic submission sub menu, submit the cereDdcModule's send_data method.
* Go to Developer -> Chain state sub menu, query chain state of the cereDdcModule's stringDataOf.

### Prerequisites

Import the CereDDCModule and derive your runtime's configuration traits from the CereDDCModule trait.

### Import Instruction

* Pull ddc-pallet. From project root folder:
```bash
git submodule update --remote
```

* (Optional) Specify the branch of ddc-pallet. From project root folder:
```bash
cd ./frame/ddc-pallet
git checkout feat/ddc-pallet-integration
```

* Import to frame structure node:
In ./Cargo.toml add
```rust
[workspace]
members = [
	"bin/node-template/node",
	...
	"frame/vesting",
	"frame/ddc-pallet",
	"primitives/allocator",
	...
	"utils/wasm-builder",
]
```

### Code Snippet

In ./bin/node/runtime/Cargo.toml add
```rust
frame-executive = { version = "2.0.0", default-features = false, path = "../../../frame/executive" }
...
pallet-cere-ddc = { version = "2.0.0", default-features = false, path = "../../../frame/ddc-pallet" }
```

In .bin/node/runtime/src/lib.rs find "construct_runtime!" then add bellow source:
```rust
pub use pallet_cere_ddc;
...
parameter_types! {
	// Minimum bounds on storage are important to secure your chain.
	pub const MinDataLength: usize = 1;
	// Maximum bounds on storage are important to secure your chain.
	pub const MaxDataLength: usize = usize::MAX;
}

/// Configure the send data pallet
impl pallet_cere_ddc::Trait for Runtime {
	type MinLength = MinDataLength;
	type MaxLength = MaxDataLength;
	// The ubiquitous event type.
	type Event = Event;
}
  
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = node_primitives::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
        ...
        Multisig: pallet_multisig::{Module, Call, Storage, Event<T>},
        CereDDCModule: pallet_cere_ddc::{Module, Call, Storage, Event<T>},
	}
);
```

### Command List
* Check before compiling node by command:
```bash
cd ./frame/ddc-pallet
SKIP_WASM_BUILD=1 cargo check
```

* Run unit test command:
```bash
cd ./frame/ddc-pallet
SKIP_WASM_BUILD=1 cargo test
```

* Build and run node. From project root folder:
```bash
cargo build --release
./target/release/cere --dev --ws-external
```

## Assumptions

Below are assumptions that must be held when using this module.  If any of
them are violated, the behavior of this module is undefined.

* The length of string data should be grater than
  `1`.
* The length of string data should be less than
  `usize::MAX`.

## Related Modules

* [`System`](https://docs.rs/frame-system/latest/frame_system/)
* [`Support`](https://docs.rs/frame-support/latest/frame_support/)

## License: Apache-2.0

* [`LICENSE-APACHE2`](https://github.com/paritytech/substrate/blob/master/LICENSE-APACHE2)
