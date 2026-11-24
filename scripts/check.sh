#!/usr/bin/env sh
# Runs every check the project requires before a merge. Exits on the first failure.
set -eu

cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
# The lines above build debug only; this catches code that is unused once debug-only code is left out.
cargo check -p classivar --release --locked
echo "All checks passed."
