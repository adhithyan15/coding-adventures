#!/usr/bin/env bash
set -euo pipefail

# Build and test cas-pretty-printer
# Dependencies: symbolic-ir (already in workspace)

cd "$(dirname "$0")"

echo "==> Building cas-pretty-printer"
cargo build -p cas-pretty-printer

echo "==> Testing cas-pretty-printer"
cargo test -p cas-pretty-printer

echo "==> cas-pretty-printer OK"
