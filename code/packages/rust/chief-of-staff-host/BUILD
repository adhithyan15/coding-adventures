#!/bin/sh
set -eu

cargo test -p chief-of-staff-host --features test-broker -- --nocapture
cargo clippy -p chief-of-staff-host --all-targets --features test-broker -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-host --no-deps
