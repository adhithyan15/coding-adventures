#!/bin/sh
# run-tests.sh — build the heap-native Rust cdylib, then run dart test.
set -e
SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"
echo "==> Building heap-native (release cdylib)"
cargo build --release -q
OS=$(uname -s)
case "$OS" in
  Darwin) LIB_FILE="$SELF_DIR/target/release/libheap_native.dylib" ;;
  *)      LIB_FILE="$SELF_DIR/target/release/libheap_native.so" ;;
esac
if [ ! -f "$LIB_FILE" ]; then
  echo "ERROR: expected shared library not found at $LIB_FILE" >&2
  exit 1
fi
echo "==> Fetching Dart dependencies"
dart pub get
echo "==> Running dart test (HEAP_NATIVE_PATH=$LIB_FILE)"
HEAP_NATIVE_PATH="$LIB_FILE" dart test --reporter=expanded
