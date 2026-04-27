#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

echo "==> Building cas-trig"
cargo build -p cas-trig

echo "==> Testing cas-trig"
cargo test -p cas-trig

echo "==> cas-trig OK"
