#!/bin/sh
set -eu

cargo test -p motorola6800-gatelevel -- --nocapture
cargo clippy -p motorola6800-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p motorola6800-gatelevel --no-deps
