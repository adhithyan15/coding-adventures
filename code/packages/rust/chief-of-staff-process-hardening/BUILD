#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-process-hardening -- --nocapture
cargo clippy --locked -p chief-of-staff-process-hardening --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-process-hardening --no-deps
