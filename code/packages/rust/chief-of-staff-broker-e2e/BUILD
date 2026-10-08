#!/bin/sh
set -eu

cargo test -p chief-of-staff-broker-e2e -- --nocapture
cargo clippy -p chief-of-staff-broker-e2e --all-targets -- -D warnings
