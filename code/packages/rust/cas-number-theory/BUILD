#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

echo "==> Building cas-number-theory"
cargo build -p cas-number-theory

echo "==> Testing cas-number-theory"
cargo test -p cas-number-theory

echo "==> cas-number-theory OK"
