#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-agent-broker -- --nocapture
cargo clippy --locked -p chief-of-staff-agent-broker --all-targets -- -D warnings
