#!/bin/sh
set -eu

cargo test -p chief-of-staff-skill-runtime -- --nocapture
cargo clippy -p chief-of-staff-skill-runtime --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-skill-runtime --no-deps
