#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-broker-callbacks -- --nocapture
cargo clippy --locked -p chief-of-staff-broker-callbacks --all-targets -- -D warnings
