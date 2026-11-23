#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-windows-sandbox -- --nocapture
cargo clippy --locked -p chief-of-staff-windows-sandbox --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-windows-sandbox --no-deps
