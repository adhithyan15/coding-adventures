#!/bin/sh
# run-tests.sh — build conduit-capi (Rust cdylib), then run smoke tests.
#
# Self-sufficient: builds the Rust native library before dotnet test so this
# script works correctly regardless of CodeQL's package visitation order.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

CAPI_DIR=$(cd ../../../packages/rust/conduit-capi && pwd)

echo "==> Building conduit-capi (release cdylib)"
(cd "$CAPI_DIR" && cargo build --release -q)

OS=$(uname -s)
case "$OS" in
  Darwin) LIB_FILE="$CAPI_DIR/../target/release/libconduit_capi.dylib" ;;
  *)      LIB_FILE="$CAPI_DIR/../target/release/libconduit_capi.so"    ;;
esac

echo "==> Running smoke tests (CONDUIT_CAPI_PATH=$LIB_FILE)"
CONDUIT_CAPI_PATH="$LIB_FILE" \
  dotnet test \
    tests/ConduitHello.Smoke/ConduitHello.Smoke.csproj \
    --disable-build-servers \
    -v minimal
