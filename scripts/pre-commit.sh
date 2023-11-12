#!/bin/sh

# Prevent committing badly formatted code
cargo fmt -- --check
if [ $? -ne 0 ]; then
	echo "Run \`cargo fmt\` to fix formatting issues before committing."
	exit 1
fi
