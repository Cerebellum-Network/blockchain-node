#!/bin/sh

# Check code with clippy before publishing
cargo clippy --all --all-targets -- -D warnings
if [ $? -ne 0 ]; then
	echo "Run \`cargo clippy --fix --all --allow-staged --allow-dirty\` to apply clippy's suggestions."
	exit 1
fi
