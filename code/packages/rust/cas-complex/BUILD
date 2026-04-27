#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

echo "==> Building cas-complex"
cargo build -p cas-complex

echo "==> Testing cas-complex"
cargo test -p cas-complex

echo "==> cas-complex OK"
