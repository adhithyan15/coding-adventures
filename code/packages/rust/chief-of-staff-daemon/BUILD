#!/bin/sh
set -eu

cargo test -p chief-of-staff-daemon -- --nocapture
cargo clippy -p chief-of-staff-daemon --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon --no-deps
