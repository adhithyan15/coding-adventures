#!/bin/sh
set -eu

cargo test --release -p intel8085-gatelevel -- --nocapture
cargo clippy -p intel8085-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p intel8085-gatelevel --no-deps
