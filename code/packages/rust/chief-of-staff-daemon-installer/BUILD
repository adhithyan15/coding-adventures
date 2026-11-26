#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-daemon-installer -- --nocapture
cargo clippy --locked -p chief-of-staff-daemon-installer --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-daemon-installer --no-deps
