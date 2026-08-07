#!/bin/sh
set -eu

cargo test -p chief-of-staff-agent-manifest -- --nocapture
cargo clippy -p chief-of-staff-agent-manifest --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-agent-manifest --no-deps
