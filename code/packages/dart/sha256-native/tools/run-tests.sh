#!/bin/sh
# run-tests.sh — build the sha256-native Rust cdylib, then run dart test.
# Mirrors dart/caesar-cipher-native: cargo-build the release cdylib, locate it
# for this OS, point the FFI loader at it via SHA256_NATIVE_PATH, run tests.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

echo "==> Building sha256-native (release cdylib)"
cargo build --release -q

OS=$(uname -s)
case "$OS" in
  Darwin) LIB_FILE="$SELF_DIR/target/release/libsha256_native.dylib" ;;
  *)      LIB_FILE="$SELF_DIR/target/release/libsha256_native.so" ;;
esac

if [ ! -f "$LIB_FILE" ]; then
  echo "ERROR: expected shared library not found at $LIB_FILE" >&2
  exit 1
fi

echo "==> Fetching Dart dependencies"
dart pub get

echo "==> Running dart test (SHA256_NATIVE_PATH=$LIB_FILE)"
SHA256_NATIVE_PATH="$LIB_FILE" dart test --reporter=expanded
