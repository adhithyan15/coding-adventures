#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-broker-e2e -- --nocapture
cargo clippy --locked -p chief-of-staff-broker-e2e --all-targets -- -D warnings
