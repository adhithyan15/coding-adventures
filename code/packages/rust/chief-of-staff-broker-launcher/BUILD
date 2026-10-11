#!/bin/sh
set -eu

cargo test -p chief-of-staff-broker-launcher -- --nocapture
cargo clippy -p chief-of-staff-broker-launcher --all-targets -- -D warnings
