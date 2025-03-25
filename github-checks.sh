#!/bin/bash

set -e
set -o pipefail

function print_message() {
  echo "========== $1 =========="
}

# Step 1: Install Rust toolchain and dependencies
print_message "Installing Rust toolchain and dependencies"
rustup install nightly-2024-03-12
rustup override set nightly-2024-03-12
rustup component add rustfmt

# Install macOS dependencies using Homebrew
print_message "Installing macOS dependencies"
brew update
brew install clang llvm openssl protobuf

# Check TOML formatting using dprint
print_message "Checking TOML formatting"
if ! command -v dprint &> /dev/null; then
  print_message "Installing dprint"
  cargo install dprint
fi
dprint check

# Check code formatting using rustfmt
print_message "Checking code formatting"
cargo fmt -- --check

# Build and check Cargo project
print_message "Building Cargo project"
cargo build --release --features try-runtime

# Run Try-Runtime checks
print_message "Running Try-Runtime checks"
try-runtime --runtime ./target/release/wbuild/cere-dev-runtime/cere_dev_runtime.compact.compressed.wasm \
  on-runtime-upgrade --blocktime 6000 --disable-idempotency-checks live --uri wss://archive.devnet.cere.network:443

# Run development chain for a short duration
print_message "Running development chain"
timeout 30 ./target/release/cere --dev || true

# Build for benchmarking
print_message "Building for benchmarking"
pushd node && cargo check --features=runtime-benchmarks --release && popd

# Step 8: Run Clippy for linting
print_message "Running Clippy for linting"
cargo clippy --no-deps --all-targets --features runtime-benchmarks,try-runtime --workspace -- --deny warnings

# Step 9: Run tests with Tarpaulin for coverage
print_message "Running tests with Tarpaulin"
if ! command -v cargo-tarpaulin &> /dev/null; then
  print_message "Installing cargo-tarpaulin"
  cargo install cargo-tarpaulin
fi
cargo tarpaulin --verbose --locked --no-fail-fast --workspace --features runtime-benchmarks --out Xml

# Step 10: Upload coverage report (local simulation)
print_message "Uploading coverage report"
if [ -f ./tarpaulin-report.xml ]; then
  echo "Coverage report generated successfully at ./tarpaulin-report.xml"
else
  echo "Coverage report generation failed."
fi

print_message "All checks completed successfully!"

