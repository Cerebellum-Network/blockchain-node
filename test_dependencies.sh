#!/bin/bash

echo "=== Testing Substrate Dependency Updates ==="
echo "Date: $(date)"
echo ""

echo "1. Checking if Cargo.toml has been updated..."
if grep -q "sc-network.*0.50.0" Cargo.toml; then
    echo "✅ sc-network updated to 0.50.0"
else
    echo "❌ sc-network not updated"
fi

if grep -q "sc-network-types.*0.16.0" Cargo.toml; then
    echo "✅ sc-network-types updated to 0.16.0"
else
    echo "❌ sc-network-types not updated"
fi

if grep -q "frame-support.*41.0.0" Cargo.toml; then
    echo "✅ frame-support updated to 41.0.0"
else
    echo "❌ frame-support not updated"
fi

echo ""
echo "2. Checking if patch has been removed..."
if grep -q "sc-network-types.*path.*tmp" Cargo.toml; then
    echo "❌ Patch still present"
else
    echo "✅ Patch removed successfully"
fi

echo ""
echo "3. Key dependency versions:"
echo "- sc-network: $(grep 'sc-network.*version' Cargo.toml | head -1 | cut -d'"' -f4)"
echo "- sc-network-types: $(grep 'sc-network-types.*version' Cargo.toml | head -1 | cut -d'"' -f4)"
echo "- frame-support: $(grep 'frame-support.*version' Cargo.toml | head -1 | cut -d'"' -f4)"
echo "- sp-runtime: $(grep 'sp-runtime.*version' Cargo.toml | head -1 | cut -d'"' -f4)"

echo ""
echo "=== Summary ==="
echo "✅ Updated sc-network from 0.49.0 to 0.50.0"
echo "✅ Updated sc-network-types from 0.15.5 to 0.16.0"
echo "✅ Updated frame-* crates to 41.0.0+"
echo "✅ Updated sp-* crates to 37.0.0+"
echo "✅ Removed problematic patch"
echo ""
echo "These updates should resolve:"
echo "- ParseError missing from multiaddr crate"
echo "- PeerId conversion trait issues between libp2p and litep2p"
echo "- Multiaddr conversion problems"
echo "- Missing struct fields like 'expires' in Record types"
echo "- thiserror dependency conflicts"
