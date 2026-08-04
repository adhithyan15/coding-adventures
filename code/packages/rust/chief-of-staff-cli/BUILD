#!/bin/sh
set -eu

cargo test -p chief-of-staff-cli -- --nocapture
cargo clippy -p chief-of-staff-cli --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-cli --no-deps
