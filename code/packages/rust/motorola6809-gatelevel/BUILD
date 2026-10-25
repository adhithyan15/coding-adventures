#!/bin/sh
set -eu

cargo test -p motorola6809-gatelevel -- --nocapture
cargo clippy -p motorola6809-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p motorola6809-gatelevel --no-deps
