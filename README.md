### Cere Network DDC Pallet

### Dispatchable Functions
* `send_data` - Send string data to another account.

### Command to test
# Clone from git to /frame folder
cd ./frame
git clone https://github.com/Cerebellum-Network/ddc-pallet
cd ./ddc-pallet
git checkout feat/ddc-pallet-integration

### Import to node instruction
## Frame structure node:
In ./Cargo.toml add
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

# Check by command
SKIP_WASM_BUILD=1 cargo check

# Test by command
SKIP_WASM_BUILD=1 cargo test

In ./bin/node/runtime/Cargo.toml add
# frame dependencies
frame-executive = { version = "2.0.0", default-features = false, path = "../../../frame/executive" }
...
pallet-cere-ddc = { version = "2.0.0", default-features = false, path = "../../../frame/ddc-pallet" }

In .bin/node/runtime/src/lib.rs find "construct_runtime!" then add bellow source:
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
# Change to root project folder then build source by command
cargo build --release

## Github: Avaiable soon
# Add dependency for ddc-pallet
In ./bin/node/runtime/Cargo.toml add
# frame dependencies
frame-executive = { version = "2.0.0", default-features = false, path = "../../../frame/executive" }
...
pallet-cere-ddc = { version = "2.0.0", default-features = false }

# Modify source in runtime
In .bin/node/runtime/src/lib.rs find "construct_runtime!" then add source like above:

# Change to root project folder then build source by command
cargo build --release