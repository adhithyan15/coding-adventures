#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-daemon -- --nocapture
cargo clippy --locked -p chief-of-staff-daemon --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-daemon --no-deps
