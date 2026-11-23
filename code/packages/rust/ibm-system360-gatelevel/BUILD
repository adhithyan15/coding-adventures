#!/bin/sh
set -eu

cargo test --locked -p ibm-system360-gatelevel -- --nocapture
cargo clippy --locked -p ibm-system360-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p ibm-system360-gatelevel --no-deps
