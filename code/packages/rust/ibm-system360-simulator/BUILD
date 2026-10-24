#!/bin/sh
set -eu

cargo test -p ibm-system360-simulator -- --nocapture
cargo clippy -p ibm-system360-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ibm-system360-simulator --no-deps
