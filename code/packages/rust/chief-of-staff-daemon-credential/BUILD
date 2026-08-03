#!/bin/sh
set -eu

cargo test -p chief-of-staff-daemon-credential -- --nocapture
cargo clippy -p chief-of-staff-daemon-credential --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-credential --no-deps
