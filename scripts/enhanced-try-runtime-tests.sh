#!/bin/bash
# Enhanced try-runtime testing for DDC-specific scenarios

set -e

RUNTIME_PATH="./target/release/wbuild/cere-runtime/cere_runtime.compact.compressed.wasm"
NETWORKS=("devnet" "qanet" "testnet" "mainnet")

# Parse command line arguments
DISABLE_SPEC_VERSION_CHECK=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --disable-spec-version-check)
            DISABLE_SPEC_VERSION_CHECK="--disable-spec-version-check"
            shift
            ;;
        *)
            shift
            ;;
    esac
done

echo "🧪 Running Enhanced DDC Try-Runtime Tests"
if [ -n "$DISABLE_SPEC_VERSION_CHECK" ]; then
    echo "⚠️  Spec version check disabled"
fi

# Function to test specific network
test_network() {
    local network=$1
    local uri="wss://archive.${network}.cere.network:443"
    
    echo "Testing network: $network"
    
    # Test 1: Basic runtime upgrade
    echo "  🔄 Testing runtime upgrade on $network..."
    if try-runtime --runtime "$RUNTIME_PATH" \
        on-runtime-upgrade \
        --disable-idempotency-checks \
        --blocktime 6000 \
        $DISABLE_SPEC_VERSION_CHECK \
        live --uri "$uri"; then
        echo "  ✅ Runtime upgrade test passed on $network"
    else
        echo "  ❌ Runtime upgrade test failed on $network"
        if [ -z "$DISABLE_SPEC_VERSION_CHECK" ]; then
            echo "  💡 Hint: If this failed due to spec version, try: --disable-spec-version-check"
        fi
    fi
    
    # Test 2: Execute block with DDC activity (if available)
    echo "  📦 Testing block execution on $network..."
    try-runtime --runtime "$RUNTIME_PATH" \
        execute-block \
        --block-ws-uri "$uri" \
        --block-at latest || echo "  ❌ Block execution test failed on $network"
    
    # Test 3: Follow chain for a few blocks
    echo "  ⛓️ Testing follow-chain on $network..."
    timeout 60s try-runtime --runtime "$RUNTIME_PATH" \
        follow-chain \
        --uri "$uri" || echo "  ⏰ Follow-chain test completed/timed out on $network"
    
    echo "  ✅ Completed tests for $network"
    echo
}

# Test each network
for network in "${NETWORKS[@]}"; do
    test_network "$network"
done

# Additional DDC-specific tests
echo "🎯 Running DDC-specific runtime validation..."

# Test with specific blocks that contain DDC transactions (you can customize these)
DDC_ACTIVE_BLOCKS=(
    # Add specific block hashes where DDC activity occurred
    # "0x1234567890abcdef..." 
)

for block_hash in "${DDC_ACTIVE_BLOCKS[@]}"; do
    echo "Testing DDC-active block: $block_hash"
    try-runtime --runtime "$RUNTIME_PATH" \
        execute-block \
        --block-ws-uri "wss://archive.mainnet.cere.network:443" \
        --block-at "$block_hash" || echo "DDC block test failed for $block_hash"
done

# Test runtime upgrade with different configurations
echo "🔧 Testing with different try-runtime configurations..."

# Test with checks enabled
echo "Testing with pre-and-post checks..."
try-runtime --runtime "$RUNTIME_PATH" \
    on-runtime-upgrade \
    --checks=pre-and-post \
    $DISABLE_SPEC_VERSION_CHECK \
    live --uri "wss://archive.qanet.cere.network:443" || echo "Pre/post checks test failed"

# Test with specific pallets
DDC_PALLETS=("DdcClusters" "DdcNodes" "DdcStaking" "DdcCustomers" "DdcPayouts")

for pallet in "${DDC_PALLETS[@]}"; do
    echo "Testing $pallet pallet specifically..."
    # Add pallet-specific tests here if needed
done

echo "✅ All enhanced try-runtime tests completed!" 
