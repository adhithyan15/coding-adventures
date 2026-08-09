#!/bin/sh
set -eu

cargo test -p chief-of-staff-host -- --nocapture
cargo clippy -p chief-of-staff-host --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-host --no-deps
