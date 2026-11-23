#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-daemon-credential -- --nocapture
cargo clippy --locked -p chief-of-staff-daemon-credential --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-daemon-credential --no-deps
