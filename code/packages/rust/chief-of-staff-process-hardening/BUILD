#!/bin/sh
set -eu

cargo test -p chief-of-staff-process-hardening -- --nocapture
cargo clippy -p chief-of-staff-process-hardening --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-process-hardening --no-deps
