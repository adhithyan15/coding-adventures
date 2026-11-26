#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-daemon-service-files -- --nocapture
cargo clippy --locked -p chief-of-staff-daemon-service-files --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-daemon-service-files --no-deps
