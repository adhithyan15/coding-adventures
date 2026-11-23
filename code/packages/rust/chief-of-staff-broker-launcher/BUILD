#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-broker-launcher -- --nocapture
cargo clippy --locked -p chief-of-staff-broker-launcher --all-targets -- -D warnings
