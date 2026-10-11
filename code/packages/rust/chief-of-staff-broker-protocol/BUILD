#!/bin/sh
set -eu

cargo test -p chief-of-staff-broker-protocol -- --nocapture
cargo clippy -p chief-of-staff-broker-protocol --all-targets -- -D warnings
