#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-broker-roots -- --nocapture
cargo clippy --locked -p chief-of-staff-broker-roots --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-broker-roots --no-deps
