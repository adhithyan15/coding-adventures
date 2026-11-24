#!/bin/sh
set -eu

cargo test --locked -p coding-adventures-ge225-simulator -- --nocapture
cargo clippy --locked -p coding-adventures-ge225-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p coding-adventures-ge225-simulator --no-deps
