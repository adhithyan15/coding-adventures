#!/bin/sh
set -eu

cargo test --locked -p ibm-system370-simulator --release -- --nocapture
cargo clippy --locked -p ibm-system370-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p ibm-system370-simulator --no-deps
