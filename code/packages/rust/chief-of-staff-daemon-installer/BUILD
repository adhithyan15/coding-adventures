#!/bin/sh
set -eu

cargo test -p chief-of-staff-daemon-installer -- --nocapture
cargo clippy -p chief-of-staff-daemon-installer --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-installer --no-deps
