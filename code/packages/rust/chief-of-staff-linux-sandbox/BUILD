#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-linux-sandbox -- --nocapture
cargo clippy --locked -p chief-of-staff-linux-sandbox --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-linux-sandbox --no-deps
