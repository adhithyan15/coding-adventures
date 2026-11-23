#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-skill-runtime -- --nocapture
cargo clippy --locked -p chief-of-staff-skill-runtime --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-skill-runtime --no-deps
