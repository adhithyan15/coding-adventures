#!/bin/sh
set -eu

cargo test --locked -p cdc6600-simulator -- --nocapture
cargo clippy --locked -p cdc6600-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p cdc6600-simulator --no-deps
