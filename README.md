# Cere Network Pallet

# Check by command
SKIP_WASM_BUILD=1 cargo check

# Test by command
SKIP_WASM_BUILD=1 cargo test

# Add to Node
[dependencies.pallet-cere-ddc]
default_features = false
git = 'https://github.com/Cerebellum-Network/ddc-pallet'
branch = 'master'