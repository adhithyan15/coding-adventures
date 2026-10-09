#!/bin/sh
set -eu

cargo test -p chief-of-staff-spawn-isolation -- --nocapture
cargo clippy -p chief-of-staff-spawn-isolation --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-spawn-isolation --no-deps
