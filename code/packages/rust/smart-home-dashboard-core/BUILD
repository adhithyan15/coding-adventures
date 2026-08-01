#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
cargo test -p smart-home-dashboard-core
cargo clippy -p smart-home-dashboard-core --all-targets -- -D warnings
