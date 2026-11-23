#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-broker-protocol -- --nocapture
cargo clippy --locked -p chief-of-staff-broker-protocol --all-targets -- -D warnings
