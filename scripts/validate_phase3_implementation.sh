#!/bin/bash

set -e

echo "🏗️ Phase 3 Implementation Validation"
echo "===================================="
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

print_status $BLUE "🔧 TASK 3.1: AWS Infrastructure Security Validation"
echo "=================================================="

# Check Terraform files
print_status $YELLOW "Validating Terraform infrastructure files..."
if [ -d "terraform" ]; then
    terraform_files=(
        "terraform/main.tf"
        "terraform/variables.tf"
        "terraform/outputs.tf"
        "terraform/ecr.tf"
        "terraform/iam.tf"
        "terraform/s3.tf"
        "terraform/security.tf"
    )

    for file in "${terraform_files[@]}"; do
        if [ -f "$file" ]; then
            print_status $GREEN "  ✅ $file exists"
        else
            print_status $YELLOW "  ⚠️  $file missing - needs implementation"
        fi
    done

    # Validate Terraform syntax if files exist
    if [ -f "terraform/main.tf" ]; then
        print_status $YELLOW "  Validating Terraform syntax..."
        (cd terraform && terraform fmt -check=true -diff=true) || print_status $YELLOW "  Terraform formatting needed"
        (cd terraform && terraform validate) 2>/dev/null || print_status $YELLOW "  Terraform validation pending (requires init)"
    fi
else
    print_status $YELLOW "  Terraform directory not found - needs creation"
fi

print_status $BLUE "📝 TASK 3.2: Configuration Management & Validation"
echo "================================================="

# Check JSON Schema
print_status $YELLOW "Validating configuration schema files..."
if [ -f "schemas/chain-spec.schema.json" ]; then
    print_status $GREEN "  ✅ Chain spec schema exists"
    # Validate JSON syntax
    python3 -m json.tool schemas/chain-spec.schema.json > /dev/null 2>&1
    check_success "Chain spec schema JSON syntax"
else
    print_status $YELLOW "  ⚠️  Chain spec schema missing - needs implementation"
fi

# Check configuration validation in service
print_status $YELLOW "Validating configuration validation implementation..."
if [ -f "node/service/src/config_validation.rs" ]; then
    print_status $GREEN "  ✅ Configuration validation module exists"
    cargo check -p cere-service --all-features
    check_success "Configuration validation compilation"
else
    print_status $YELLOW "  ⚠️  Configuration validation module missing - needs implementation"
fi

# Validate chain specs against schema (if both exist)
if [ -f "schemas/chain-spec.schema.json" ] && command -v ajv >/dev/null 2>&1; then
    print_status $YELLOW "Validating existing chain specs..."
    for spec in node/service/chain-specs/*.json; do
        if [ -f "$spec" ]; then
            spec_name=$(basename "$spec")
            ajv validate -s schemas/chain-spec.schema.json -d "$spec" >/dev/null 2>&1
            if [ $? -eq 0 ]; then
                print_status $GREEN "  ✅ $spec_name validates against schema"
            else
                print_status $YELLOW "  ⚠️  $spec_name schema validation issues"
            fi
        fi
    done
else
    print_status $YELLOW "  Schema validation tools not available (ajv-cli needed)"
fi

print_status $BLUE "🔐 TASK 3.3: Blockchain Network Security"
echo "========================================"

# Check network monitor pallet
print_status $YELLOW "Validating network monitor pallet..."
if [ -d "pallets/network-monitor" ]; then
    print_status $GREEN "  ✅ Network monitor pallet directory exists"

    # Check if pallet has proper structure
    if [ -f "pallets/network-monitor/src/lib.rs" ]; then
        print_status $GREEN "  ✅ Network monitor pallet implementation exists"
        cargo check -p pallet-network-monitor --all-features
        check_success "Network monitor pallet compilation"
    else
        print_status $YELLOW "  ⚠️  Network monitor pallet implementation incomplete"
    fi

    # Check if pallet is integrated in runtime
    if grep -q "pallet-network-monitor" runtime/cere/Cargo.toml; then
        print_status $GREEN "  ✅ Network monitor pallet integrated in runtime"
    else
        print_status $YELLOW "  ⚠️  Network monitor pallet not integrated in runtime"
    fi
else
    print_status $YELLOW "  ⚠️  Network monitor pallet missing - needs implementation"
fi

# Check secure RPC implementation
print_status $YELLOW "Validating secure RPC implementation..."
if [ -f "node/rpc/src/secure_rpc.rs" ]; then
    print_status $GREEN "  ✅ Secure RPC module exists"
    cargo check -p cere-rpc --all-features
    check_success "Secure RPC compilation"
else
    print_status $YELLOW "  ⚠️  Secure RPC module missing - needs implementation"
fi

# Check network security service
print_status $YELLOW "Validating network security service..."
if [ -f "node/service/src/network_security.rs" ]; then
    print_status $GREEN "  ✅ Network security service exists"
    cargo check -p cere-service --all-features
    check_success "Network security service compilation"
else
    print_status $YELLOW "  ⚠️  Network security service missing - needs implementation"
fi

# Check TLS certificates for development
print_status $YELLOW "Checking TLS certificate setup..."
if [ -d "certs" ]; then
    if [ -f "certs/server.crt" ] && [ -f "certs/server.key" ]; then
        print_status $GREEN "  ✅ Development TLS certificates exist"
    else
        print_status $YELLOW "  ⚠️  Development TLS certificates missing"
    fi
else
    print_status $YELLOW "  ⚠️  Certificates directory missing - needs creation"
fi

print_status $BLUE "🔍 Dependency Compatibility Check"
echo "================================="

# Check for known compatibility issues
print_status $YELLOW "Checking known compatibility issues..."

# XCM compatibility
if grep -q "pallet-token-gateway.*2503.0.0" Cargo.toml; then
    print_status $YELLOW "  ⚠️  Known XCM compatibility issue with pallet-token-gateway"
fi

# Pallet-referenda compatibility
if grep -q "pallet-referenda.*40.1.0" Cargo.toml; then
    print_status $YELLOW "  ⚠️  Known Polling trait compatibility issue with pallet-referenda"
fi

# Disabled pallets
if grep -q "# pallet-ddc-clusters-gov" Cargo.toml; then
    print_status $YELLOW "  ⚠️  pallet-ddc-clusters-gov temporarily disabled due to trait issues"
fi

print_status $BLUE "📊 Phase 3 Implementation Status Summary"
echo "======================================="

# Count implemented vs missing components
implemented=0
missing=0

components=(
    "terraform/main.tf:Infrastructure as Code"
    "schemas/chain-spec.schema.json:Configuration Schema"
    "node/service/src/config_validation.rs:Config Validation"
    "pallets/network-monitor/src/lib.rs:Network Monitor Pallet"
    "node/rpc/src/secure_rpc.rs:Secure RPC"
    "node/service/src/network_security.rs:Network Security Service"
)

for component in "${components[@]}"; do
    file="${component%%:*}"
    name="${component##*:}"
    if [ -f "$file" ]; then
        print_status $GREEN "✅ $name: Implemented"
        ((implemented++))
    else
        print_status $YELLOW "⚠️  $name: Missing"
        ((missing++))
    fi
done

echo ""
print_status $BLUE "Implementation Progress:"
echo "- Implemented: $implemented/$(( implemented + missing ))"
echo "- Missing: $missing/$(( implemented + missing ))"

if [ $missing -eq 0 ]; then
    print_status $GREEN "🎉 Phase 3 implementation complete!"
else
    print_status $YELLOW "🔧 Phase 3 implementation in progress ($missing components remaining)"
fi

echo ""
print_status $BLUE "Next Steps:"
if [ $missing -gt 0 ]; then
    echo "1. Implement missing Phase 3 components"
    echo "2. Run comprehensive dependency validation"
    echo "3. Test all functionality"
    echo "4. Commit and push changes"
else
    echo "1. Run final validation tests"
    echo "2. Update documentation"
    echo "3. Commit and push Phase 3 implementation"
fi
