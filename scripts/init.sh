#!/usr/bin/env bash

set -e

echo "*** Initializing WASM build environment"

rustup install 1.81.0

rustup target add wasm32-unknown-unknown --toolchain 1.81.0

rustup component add rust-src


ln -sf $PWD/scripts/pre-commit.sh $PWD/.git/hooks/pre-commit || true
ln -sf $PWD/scripts/pre-push.sh $PWD/.git/hooks/pre-push || true
