#!/bin/sh
set -eu

cargo test -p chief-of-staff-broker-roots -- --nocapture
cargo clippy -p chief-of-staff-broker-roots --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-broker-roots --no-deps
