#!/usr/bin/env bash

set -e

# Initializes third_party submodules, checking out only the parts this
# workspace actually builds.
#
# The orml repository holds ~24 pallets but we only use three of them
# (oracle, plus traits and utilities which oracle path-depends on). Sparse
# checkout keeps the rest off disk: 1.4M -> 228K.
#
# Sparse-checkout settings live in .git/ and are not committed, so this has
# to be run once per clone. CI does a plain full submodule checkout instead —
# the extra megabyte is not worth another moving part in the pipeline.

ORML_PATH="third_party/open-runtime-module-library"

echo "*** Initializing submodules"
git submodule update --init --recursive

echo "*** Trimming ${ORML_PATH} to the crates this workspace builds"
git -C "${ORML_PATH}" sparse-checkout init --cone
git -C "${ORML_PATH}" sparse-checkout set oracle traits utilities

echo "*** Done"
git -C "${ORML_PATH}" sparse-checkout list
