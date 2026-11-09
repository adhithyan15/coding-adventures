#!/bin/sh
set -eu

cargo test -p capability-runtime-conformance -- --nocapture
cargo clippy -p capability-runtime-conformance --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p capability-runtime-conformance --no-deps
