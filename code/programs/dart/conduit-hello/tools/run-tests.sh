#!/bin/sh
# run-tests.sh — build conduit-capi, then run smoke tests for conduit-hello.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

# Use git to find the repo root — works both in the main tree and in isolated
# worktrees (git traverses up past the worktree directory to the real .git).
REPO_ROOT=$(git rev-parse --show-toplevel)
CAPI_DIR="$REPO_ROOT/code/packages/rust/conduit-capi"

echo "==> Building conduit-capi + conduit-dart-bridge (release cdylib)"
(cd "$CAPI_DIR/.." && cargo build -p conduit-capi -p conduit-dart-bridge --release -q)

OS=$(uname -s)
case "$OS" in
  Darwin)
    LIB_FILE="$CAPI_DIR/../target/release/libconduit_capi.dylib"
    BRIDGE_FILE="$CAPI_DIR/../target/release/libconduit_dart_bridge.dylib"
    ;;
  *)
    LIB_FILE="$CAPI_DIR/../target/release/libconduit_capi.so"
    BRIDGE_FILE="$CAPI_DIR/../target/release/libconduit_dart_bridge.so"
    ;;
esac

echo "==> Fetching Dart dependencies"
dart pub get

echo "==> Running smoke tests"
CONDUIT_CAPI_PATH="$LIB_FILE" CONDUIT_DART_BRIDGE_PATH="$BRIDGE_FILE" dart test \
  --reporter=expanded \
  test/smoke_test.dart
