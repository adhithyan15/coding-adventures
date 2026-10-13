#!/bin/sh
set -eu

cargo test -p chief-of-staff-macos-sandbox -- --nocapture
cargo clippy -p chief-of-staff-macos-sandbox --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-macos-sandbox --no-deps
