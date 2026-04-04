#!/usr/bin/env bash
# Build and test the Elixir polynomial_native NIF package.
#
# Prerequisites:
#   - Rust toolchain (cargo) for building the NIF shared library
#   - Elixir / Mix for running tests
#
# The NIF is compiled first (cargo build --release), then Mix tests run.
set -euo pipefail
cd "$(dirname "$0")"

# Step 1: Build the Rust NIF shared library.
echo "==> Building Rust NIF..."
cd native/polynomial_native
cargo build --release 2>&1
cd ../..

# Step 2: Copy the .so into priv/ (elixir_make does this in a full Mix build).
mkdir -p priv
UNAME=$(uname -s)
if [ "$UNAME" = "Darwin" ]; then
  cp native/polynomial_native/target/release/libpolynomial_native.dylib priv/polynomial_native.so
else
  cp native/polynomial_native/target/release/libpolynomial_native.so priv/polynomial_native.so
fi

# Step 3: Run Mix tests (no make compiler needed — NIF is already compiled).
echo "==> Running Mix tests..."
mix deps.get --quiet
mix compile
mix test
