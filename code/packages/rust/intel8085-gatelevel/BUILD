#!/bin/sh
set -eu

cargo test --locked --release -p intel8085-gatelevel -- --nocapture
cargo clippy --locked -p intel8085-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p intel8085-gatelevel --no-deps
