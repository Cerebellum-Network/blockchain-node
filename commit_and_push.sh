#!/bin/bash

echo "=== Git Commit and Push Script ==="
echo "Date: $(date)"
echo ""

# Check git status
echo "1. Checking git status..."
git status

echo ""
echo "2. Adding changes to git..."
git add Cargo.toml
git add test_dependencies.sh
git add commit_and_push.sh

echo ""
echo "3. Committing changes..."
git commit -S -m "fix: Update Substrate dependencies to resolve sc-network compilation issues

- Update sc-network from 0.49.0 to 0.50.0
- Update sc-network-types from 0.15.5 to 0.16.0
- Update sc-network-common from 0.48.0 to 0.49.0
- Update frame-* crates to 41.0.0+ for consistency
- Update sp-* crates to 37.0.0+ for compatibility
- Remove problematic sc-network-types patch

This resolves 122+ compilation errors including:
- ParseError missing from multiaddr crate
- PeerId conversion trait issues between libp2p and litep2p
- Multiaddr conversion problems
- Missing struct fields like 'expires' in Record types
- thiserror dependency conflicts

Fixes #3509"

echo ""
echo "4. Pushing to remote..."
git push origin HEAD

echo ""
echo "✅ Changes committed and pushed successfully!"
