#!/bin/sh
set -eu

cargo test --locked -p ibm-system360-simulator -- --nocapture
cargo clippy --locked -p ibm-system360-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p ibm-system360-simulator --no-deps
