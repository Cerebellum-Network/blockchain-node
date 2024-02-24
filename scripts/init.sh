#!/usr/bin/env bash

set -e

echo "*** Initializing WASM build environment"

<<<<<<< HEAD
<<<<<<< HEAD
rustup install 1.79.0

rustup target add wasm32-unknown-unknown --toolchain 1.79.0

rustup component add rust-src
=======
rustup install nightly-2024-03-12

rustup target add wasm32-unknown-unknown --toolchain nightly-2024-03-12
>>>>>>> 656e5410 (Fix/staking migrations (#300))
=======
rustup install 1.77.0

rustup target add wasm32-unknown-unknown --toolchain 1.77.0
>>>>>>> 7ad744e7 (Enable Clippy (#324))

ln -sf $PWD/scripts/pre-commit.sh $PWD/.git/hooks/pre-commit || true
ln -sf $PWD/scripts/pre-push.sh $PWD/.git/hooks/pre-push || true
