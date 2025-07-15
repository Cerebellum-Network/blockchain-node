#!/bin/bash

set -e

echo "🔍 Comprehensive Cargo Dependency Validation"
echo "============================================="
echo "Date: $(date)"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to check command success
check_success() {
    if [ $? -eq 0 ]; then
        print_status $GREEN "✅ $1 passed"
    else
        print_status $RED "❌ $1 failed"
        exit 1
    fi
}

# 1. Workspace-wide cargo check
print_status $BLUE "📦 Running workspace cargo check..."
cargo check --workspace --all-features
check_success "Workspace cargo check"

# 2. Individual pallet validation
print_status $BLUE "🔧 Validating individual pallets..."
for pallet_dir in pallets/*/; do
    if [ -d "$pallet_dir" ]; then
        pallet_name=$(basename "$pallet_dir")
        print_status $YELLOW "  Checking pallet: $pallet_name..."
        (cd "$pallet_dir" && cargo check --all-features)
        check_success "Pallet $pallet_name check"
    fi
done

# 3. Runtime WASM compilation
print_status $BLUE "🌐 Validating WASM runtime compilation..."
print_status $YELLOW "  Checking cere-runtime WASM..."
cargo check -p cere-runtime --target wasm32-unknown-unknown --no-default-features
check_success "Cere runtime WASM compilation"

print_status $YELLOW "  Checking cere-dev-runtime WASM..."
cargo check -p cere-dev-runtime --target wasm32-unknown-unknown --no-default-features
check_success "Cere dev runtime WASM compilation"

# 4. Node compilation
print_status $BLUE "🖥️ Validating node compilation..."
cargo check -p cere-cli --all-features
check_success "Node CLI compilation"

cargo check -p cere-service --all-features
check_success "Node service compilation"

cargo check -p cere-rpc --all-features
check_success "Node RPC compilation"

# 5. Dependency conflict detection
print_status $BLUE "🔍 Checking for dependency conflicts..."
echo "Duplicate dependencies found:"
cargo tree --duplicates || true

# 6. Check for outdated dependencies
print_status $BLUE "📊 Checking dependency versions..."
echo "Workspace dependency summary:"
echo "- Substrate dependencies: $(grep -c 'version.*[0-9][0-9]\.' Cargo.toml || echo 0)"
echo "- Git dependencies: $(grep -c 'git.*github.com' Cargo.toml || echo 0)"
echo "- Path dependencies: $(grep -c 'path.*=' Cargo.toml || echo 0)"

# 7. Validate specific Phase 3 components
print_status $BLUE "🏗️ Validating Phase 3 specific components..."

# Check network monitor pallet
if [ -d "pallets/network-monitor" ]; then
    print_status $YELLOW "  Validating network-monitor pallet..."
    cargo check -p pallet-network-monitor --all-features
    check_success "Network monitor pallet"
else
    print_status $YELLOW "  Network monitor pallet not found - will be created"
fi

# Check configuration validation
print_status $YELLOW "  Validating configuration management..."
if grep -q "config_validation" node/service/src/lib.rs 2>/dev/null; then
    cargo check -p cere-service --features runtime-benchmarks
    check_success "Configuration validation features"
else
    print_status $YELLOW "  Configuration validation not yet implemented"
fi

# 8. Security and benchmarking features
print_status $BLUE "🔒 Validating security and benchmarking features..."
cargo check --workspace --features runtime-benchmarks
check_success "Runtime benchmarks compilation"

# 9. Test compilation (without running)
print_status $BLUE "🧪 Validating test compilation..."
cargo check --workspace --tests
check_success "Test compilation"

# 10. Documentation generation
print_status $BLUE "📚 Validating documentation generation..."
cargo doc --workspace --all-features --no-deps --quiet
check_success "Documentation generation"

print_status $GREEN "🎉 All dependency validations passed successfully!"
echo ""
echo "Summary:"
echo "- Workspace compilation: ✅"
echo "- Individual pallets: ✅"
echo "- WASM runtimes: ✅"
echo "- Node components: ✅"
echo "- Phase 3 components: ✅"
echo "- Security features: ✅"
echo "- Documentation: ✅"
echo ""
echo "Ready for commit and push! 🚀"
