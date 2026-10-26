#!/bin/sh
set -eu

cargo test -p ibm-system370-simulator --release -- --nocapture
cargo clippy -p ibm-system370-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ibm-system370-simulator --no-deps
