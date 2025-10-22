# DAC Registry Pallet

The **DAC Registry Pallet** provides an on-chain system to register and manage versions of DAC (Data Aggregation Component) WASM modules used by DDC clusters.

## Overview

This pallet enables governance to:
- Add new DAC versions without runtime upgrades
- Manage version lifecycles (activation, updates, deprecations)
- Maintain transparent, auditable history of all DAC versions

Instead of embedding DAC logic directly into the runtime, the pallet stores DAC modules (WASM binaries) on-chain, making DAC logic modular, governance-controlled, and easily upgradeable.

## Key Features

- **Governance Controlled:** Only governance can register, update, or deprecate DAC versions
- **Single-Step Upload:** Full WASM file uploaded in one atomic transaction
- **Versioned Metadata:** Tracks API version, semantic version, and activation block
- **Dynamic Retrieval:** DDC and Inspector components fetch versions by hash
- **Auditable Lifecycle:** Every change and event stored on-chain
- **No Maintenance Needed:** No chunking, CLI, or cleanup process required

## Storage

| Storage Item | Type | Description |
| --- | --- | --- |
| `CodeByHash` | `Map<CodeHash, CodeMeta>` | Metadata for each DAC version |
| `WasmCode` | `Map<CodeHash, Bytes>` | Full DAC WASM binary |
| `Deregistered` | `Map<CodeHash, bool>` | Marks deprecated or disabled DAC versions |

## Metadata Structure

Each DAC version includes:
- **code_hash:** Unique blake2_256 hash of the WASM
- **api_version:** Major/minor compatibility identifier
- **semver:** Semantic version (e.g., `1.5.0`)
- **allowed_from:** Block number when the version becomes active
- **length:** Size of the WASM binary (bytes)

## Extrinsics

### `register_code`

Registers a new DAC version on-chain.

**Parameters:**
- `code`: The WASM binary code
- `api_version`: API version (major.minor)
- `semver`: Semantic version (major.minor.patch)
- `allowed_from`: Block number when this version becomes active

**Origin:** Governance only

### `update_meta`

Updates metadata for an existing DAC version.

**Parameters:**
- `code_hash`: Hash of the code to update
- `api_version`: New API version (major.minor)
- `semver`: New semantic version (major.minor.patch)
- `allowed_from`: New activation block

**Origin:** Governance only

### `deregister_code`

Marks an existing DAC version as inactive or deprecated.

**Parameters:**
- `code_hash`: Hash of the code to deregister

**Origin:** Governance only

## Events

| Event | Description |
| --- | --- |
| `CodeRegistered` | A new DAC version was successfully registered |
| `CodeMetaUpdated` | Metadata of a DAC version was updated |
| `CodeDeregistered` | DAC version marked as inactive or deprecated |

## Public Interface

The pallet provides several public functions for querying the registry:

- `get_code(code_hash)`: Get the WASM code for a given code hash
- `get_metadata(code_hash)`: Get the metadata for a given code hash
- `is_code_active(code_hash)`: Check if a code hash is active (exists and not deregistered)
- `is_code_ready(code_hash)`: Check if a code is ready for use (active and past activation block)

## Configuration

The pallet requires the following configuration:

- `GovernanceOrigin`: Origin that can perform registry operations
- `MaxCodeSize`: Maximum size of a WASM code in bytes
- `WeightInfo`: Weight functions for benchmarking

## Usage Example

```rust
// Register a new DAC version
DacRegistry::register_code(
    RuntimeOrigin::root(),
    wasm_code,
    (1, 0), // API version
    (1, 0, 0), // Semantic version
    100, // Activation block
)?;

// Check if code is ready
let code_hash = compute_code_hash(&wasm_code);
if DacRegistry::is_code_ready(code_hash) {
    let code = DacRegistry::get_code(code_hash).unwrap();
    // Execute the DAC code
}
```

## Testing

The pallet includes comprehensive tests covering:
- Successful registration and retrieval
- Input validation
- Error conditions
- Event emission
- Public interface functions
- Governance origin enforcement

Run tests with:
```bash
cargo test -p pallet-dac-registry
```

## Benchmarking

The pallet includes weight functions for benchmarking. Run benchmarks with:
```bash
cargo run --release --features runtime-benchmarks -- benchmark pallet --pallet=pallet_dac_registry --extrinsic=* --execution=wasm --wasm-execution=compiled --heap-pages=4096
```
