#!/bin/sh
# run-tests.sh — build the activation-functions-native Rust cdylib, then run dart test.
set -e
SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"
echo "==> Building activation-functions-native (release cdylib)"
cargo build --release -q
OS=$(uname -s)
case "$OS" in
  Darwin) LIB_FILE="$SELF_DIR/target/release/libactivation_functions_native.dylib" ;;
  *)      LIB_FILE="$SELF_DIR/target/release/libactivation_functions_native.so" ;;
esac
if [ ! -f "$LIB_FILE" ]; then
  echo "ERROR: expected shared library not found at $LIB_FILE" >&2
  exit 1
fi
echo "==> Fetching Dart dependencies"
dart pub get
echo "==> Running dart test (ACTIVATION_FUNCTIONS_NATIVE_PATH=$LIB_FILE)"
ACTIVATION_FUNCTIONS_NATIVE_PATH="$LIB_FILE" dart test --reporter=expanded
