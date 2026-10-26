#!/bin/sh
set -eu

cargo test -p ibm-system360-gatelevel -- --nocapture
cargo clippy -p ibm-system360-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ibm-system360-gatelevel --no-deps
