#!/bin/sh
# run-tests.sh — build the md5-native Rust cdylib, then run dart test.
# Mirrors dart/sha256-native.
set -e
SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"
echo "==> Building md5-native (release cdylib)"
cargo build --release -q
OS=$(uname -s)
case "$OS" in
  Darwin) LIB_FILE="$SELF_DIR/target/release/libmd5_native.dylib" ;;
  *)      LIB_FILE="$SELF_DIR/target/release/libmd5_native.so" ;;
esac
if [ ! -f "$LIB_FILE" ]; then
  echo "ERROR: expected shared library not found at $LIB_FILE" >&2
  exit 1
fi
echo "==> Fetching Dart dependencies"
dart pub get
echo "==> Running dart test (MD5_NATIVE_PATH=$LIB_FILE)"
MD5_NATIVE_PATH="$LIB_FILE" dart test --reporter=expanded
