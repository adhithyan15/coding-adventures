#!/bin/sh
set -eu

cargo test --locked -p chief-of-staff-skill-package -- --nocapture
cargo clippy --locked -p chief-of-staff-skill-package --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p chief-of-staff-skill-package --no-deps
