#!/bin/sh
set -eu

cargo test -p chief-of-staff-broker-callbacks -- --nocapture
cargo clippy -p chief-of-staff-broker-callbacks --all-targets -- -D warnings
