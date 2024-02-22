#!/usr/bin/env bash

set -e

echo "*** Initializing WASM build environment"

rustup install nightly-2023-05-23

rustup target add wasm32-unknown-unknown --toolchain nightly-2023-05-23

ln -sf $PWD/scripts/pre-commit.sh $PWD/.git/hooks/pre-commit || true
ln -sf $PWD/scripts/pre-push.sh $PWD/.git/hooks/pre-push || true
