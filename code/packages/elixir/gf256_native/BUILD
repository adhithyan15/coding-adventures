#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

echo "==> Building Rust NIF..."
cd native/gf256_native
cargo build --release 2>&1
cd ../..

mkdir -p priv
UNAME=$(uname -s)
if [ "$UNAME" = "Darwin" ]; then
  cp native/gf256_native/target/release/libgf256_native.dylib priv/gf256_native.so
else
  cp native/gf256_native/target/release/libgf256_native.so priv/gf256_native.so
fi

echo "==> Running Mix tests..."
mix deps.get --quiet
mix compile
mix test
