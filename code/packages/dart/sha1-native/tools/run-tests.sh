#!/bin/sh
# run-tests.sh — build the sha1-native Rust cdylib, then run dart test.
set -e
SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"
echo "==> Building sha1-native (release cdylib)"
cargo build --release -q
OS=$(uname -s)
case "$OS" in
  Darwin) LIB_FILE="$SELF_DIR/target/release/libsha1_native.dylib" ;;
  *)      LIB_FILE="$SELF_DIR/target/release/libsha1_native.so" ;;
esac
if [ ! -f "$LIB_FILE" ]; then
  echo "ERROR: expected shared library not found at $LIB_FILE" >&2
  exit 1
fi
echo "==> Fetching Dart dependencies"
dart pub get
echo "==> Running dart test (SHA1_NATIVE_PATH=$LIB_FILE)"
SHA1_NATIVE_PATH="$LIB_FILE" dart test --reporter=expanded
