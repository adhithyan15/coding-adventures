#!/bin/sh
# run-tests.sh — build the caesar-cipher-native Rust cdylib, then run dart test.
#
# Mirrors dart/conduit's harness: cargo-build the shared library in release
# mode, locate the produced file for this OS, then point the Dart FFI loader at
# it via CAESAR_CIPHER_NATIVE_PATH before running the test suite.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

echo "==> Building caesar-cipher-native (release cdylib)"
# This package is its own standalone cargo workspace (see Cargo.toml), so a
# plain `cargo build` here targets ./target, not the monorepo workspace.
cargo build --release -q

OS=$(uname -s)
case "$OS" in
  Darwin) LIB_FILE="$SELF_DIR/target/release/libcaesar_cipher_native.dylib" ;;
  *)      LIB_FILE="$SELF_DIR/target/release/libcaesar_cipher_native.so" ;;
esac

if [ ! -f "$LIB_FILE" ]; then
  echo "ERROR: expected shared library not found at $LIB_FILE" >&2
  exit 1
fi

echo "==> Fetching Dart dependencies"
dart pub get

echo "==> Running dart test (CAESAR_CIPHER_NATIVE_PATH=$LIB_FILE)"
CAESAR_CIPHER_NATIVE_PATH="$LIB_FILE" dart test --reporter=expanded
