#!/usr/bin/env sh
# Runs every check the project requires before a merge. Exits on the first failure.
set -eu

cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
echo "All checks passed."
