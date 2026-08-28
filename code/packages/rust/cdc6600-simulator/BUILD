#!/bin/sh
set -eu

cargo test -p cdc6600-simulator -- --nocapture
cargo clippy -p cdc6600-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cdc6600-simulator --no-deps
