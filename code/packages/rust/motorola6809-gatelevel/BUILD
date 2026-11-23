#!/bin/sh
set -eu

cargo test --locked --release -p motorola6809-gatelevel -- --nocapture
cargo clippy --locked -p motorola6809-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p motorola6809-gatelevel --no-deps
