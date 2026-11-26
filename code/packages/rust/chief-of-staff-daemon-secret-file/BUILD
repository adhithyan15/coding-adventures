#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-daemon-secret-file -- --nocapture
cargo clippy --locked -p chief-of-staff-daemon-secret-file --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-daemon-secret-file --no-deps
