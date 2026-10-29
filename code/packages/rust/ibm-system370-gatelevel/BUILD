#!/bin/sh
set -eu

cargo test -p ibm-system370-gatelevel --release -- --nocapture
cargo clippy -p ibm-system370-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ibm-system370-gatelevel --no-deps
