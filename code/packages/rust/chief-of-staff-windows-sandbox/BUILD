#!/bin/sh
set -eu

cargo test -p chief-of-staff-windows-sandbox -- --nocapture
cargo clippy -p chief-of-staff-windows-sandbox --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-windows-sandbox --no-deps
