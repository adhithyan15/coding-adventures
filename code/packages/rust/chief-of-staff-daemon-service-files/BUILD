#!/bin/sh
set -eu

cargo test -p chief-of-staff-daemon-service-files -- --nocapture
cargo clippy -p chief-of-staff-daemon-service-files --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-service-files --no-deps
