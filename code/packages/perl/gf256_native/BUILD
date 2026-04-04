#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release 2>&1
echo "Built: target/release/libGF256Native.so (or .dylib)"
