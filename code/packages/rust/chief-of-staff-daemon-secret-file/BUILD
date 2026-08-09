#!/bin/sh
set -eu

cargo test -p chief-of-staff-daemon-secret-file -- --nocapture
cargo clippy -p chief-of-staff-daemon-secret-file --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-secret-file --no-deps
