#!/bin/sh
set -eu

cargo test -p chief-of-staff-agent-broker -- --nocapture
cargo clippy -p chief-of-staff-agent-broker --all-targets -- -D warnings
