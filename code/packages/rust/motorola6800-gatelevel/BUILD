#!/bin/sh
set -eu

cargo test --locked -p motorola6800-gatelevel -- --nocapture
cargo clippy --locked -p motorola6800-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p motorola6800-gatelevel --no-deps
