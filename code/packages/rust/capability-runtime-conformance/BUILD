#!/bin/sh
set -eu

cargo test --locked -p capability-runtime-conformance -- --nocapture
cargo clippy --locked -p capability-runtime-conformance --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p capability-runtime-conformance --no-deps
