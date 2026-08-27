#!/bin/sh
set -eu

cargo test -p coding-adventures-ge225-simulator -- --nocapture
cargo clippy -p coding-adventures-ge225-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding-adventures-ge225-simulator --no-deps
