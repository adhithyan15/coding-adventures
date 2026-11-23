#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-agent-manifest -- --nocapture
cargo clippy --locked -p chief-of-staff-agent-manifest --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-agent-manifest --no-deps
