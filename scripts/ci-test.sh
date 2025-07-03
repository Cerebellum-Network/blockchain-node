#!/bin/bash

# Script to test CI optimizations locally
# This simulates the CI environment and runs the optimized build process

set -e

echo "========== Testing CI Build Optimizations =========="

# Set CI environment variables
export CI=true
export CARGO_TERM_COLOR=always
export CARGO_INCREMENTAL=1
export RUST_BACKTRACE=1

# Print system information (cross-platform)
echo "System Information:"
if command -v nproc >/dev/null 2>&1; then
    echo "Available cores: $(nproc)"
elif command -v sysctl >/dev/null 2>&1; then
    echo "Available cores: $(sysctl -n hw.ncpu)"
else
    echo "Available cores: Unknown"
fi

if command -v free >/dev/null 2>&1; then
    echo "Memory: $(free -h)"
elif command -v vm_stat >/dev/null 2>&1; then
    echo "Memory: $(vm_stat | head -4)"
else
    echo "Memory: Unknown"
fi

echo "Disk space: $(df -h . | tail -1)"

# Check if required tools are installed
echo "Checking required tools..."
command -v clang >/dev/null 2>&1 || { echo "Error: clang not found. Please install clang."; exit 1; }
command -v protoc >/dev/null 2>&1 || { echo "Error: protoc not found. Please install protobuf-compiler."; exit 1; }

# LLD check (optional on macOS)
if command -v lld >/dev/null 2>&1; then
    echo "LLD linker: Available"
else
    echo "LLD linker: Not available (using default linker)"
fi

# Run a simple test first - just check if the fast-ci profile works
echo "========== Testing Fast-CI Profile =========="
echo "1. Testing fast-ci profile with a simple package..."
time cargo check --package cere-runtime-common --profile fast-ci

echo "========== Fast-CI Profile Test Complete =========="
echo "The fast-ci profile is working correctly!"

 
