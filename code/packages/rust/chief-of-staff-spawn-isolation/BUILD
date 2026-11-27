#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-spawn-isolation -- --nocapture
cargo clippy --locked -p chief-of-staff-spawn-isolation --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-spawn-isolation --no-deps
