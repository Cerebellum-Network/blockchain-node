#!/usr/bin/env bash

set -e

# Initializes third_party submodules, checking out only the parts this
# workspace actually builds.
#
# The orml repository holds ~24 pallets; we build three of them -- oracle,
# plus traits and utilities, which oracle path-depends on. Sparse checkout
# keeps the other 21 off disk (1.4M -> 228K) so third_party/ shows only what
# this chain uses.
#
# Sparse-checkout settings live in .git/ and are not committed, so this runs
# per clone. It is invoked from scripts/init.sh; run it directly after a
# `git submodule` command that resets the working tree.
#
# CI does a plain full submodule checkout instead (submodules: true on the
# actions/checkout steps) -- one extra megabyte is not worth another moving
# part in the pipeline.

ORML_PATH="third_party/open-runtime-module-library"

echo "*** Initializing submodules"
git submodule update --init --recursive

# sparse-checkout needs git >= 2.25. Trimming is a convenience, never a
# correctness requirement, so fall back to the full checkout if unsupported.
if git -C "${ORML_PATH}" sparse-checkout init --cone 2>/dev/null; then
  echo "*** Trimming ${ORML_PATH} to the crates this workspace builds"
  git -C "${ORML_PATH}" sparse-checkout set oracle traits utilities
  git -C "${ORML_PATH}" sparse-checkout list
else
  echo "*** git sparse-checkout unavailable (needs git >= 2.25)"
  echo "*** keeping the full ${ORML_PATH} checkout -- builds are unaffected"
fi
