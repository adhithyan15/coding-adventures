#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-cli -- --nocapture
cargo clippy --locked -p chief-of-staff-cli --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-cli --no-deps
