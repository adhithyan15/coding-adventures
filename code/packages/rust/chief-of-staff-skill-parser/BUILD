#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-skill-parser -- --nocapture
cargo clippy --locked -p chief-of-staff-skill-parser --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-skill-parser --no-deps
