#!/bin/sh
set -eu

cargo test -p chief-of-staff-skill-parser -- --nocapture
cargo clippy -p chief-of-staff-skill-parser --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-skill-parser --no-deps
