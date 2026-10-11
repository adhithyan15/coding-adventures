#!/bin/sh
set -eu

cargo test -p chief-of-staff-linux-sandbox -- --nocapture
cargo clippy -p chief-of-staff-linux-sandbox --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-linux-sandbox --no-deps
