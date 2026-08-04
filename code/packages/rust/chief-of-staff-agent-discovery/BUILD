#!/bin/sh
set -eu
cargo test -p chief-of-staff-agent-discovery -- --nocapture
cargo clippy -p chief-of-staff-agent-discovery --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-agent-discovery --no-deps
