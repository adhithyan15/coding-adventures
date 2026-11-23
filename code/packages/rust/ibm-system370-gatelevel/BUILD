#!/bin/sh
set -eu

cargo test --locked -p ibm-system370-gatelevel --release -- --nocapture
cargo clippy --locked -p ibm-system370-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p ibm-system370-gatelevel --no-deps
