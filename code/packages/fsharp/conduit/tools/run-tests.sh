#!/bin/sh
# run-tests.sh — build conduit-capi (Rust cdylib), then run dotnet test.
#
# In the normal build-tool flow, `deps=rust/conduit-capi` ensures conduit-capi
# is built first, making the `cargo build` below a fast no-op.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

CAPI_DIR=$(cd ../../rust/conduit-capi && pwd)

echo "==> Building conduit-capi (release cdylib)"
(cd "$CAPI_DIR" && cargo build --release -q)

OS=$(uname -s)
case "$OS" in
  Darwin) LIB_FILE="$CAPI_DIR/../target/release/libconduit_capi.dylib" ;;
  *)      LIB_FILE="$CAPI_DIR/../target/release/libconduit_capi.so"    ;;
esac

echo "==> Running dotnet test (CONDUIT_CAPI_PATH=$LIB_FILE)"
CONDUIT_CAPI_PATH="$LIB_FILE" \
  dotnet test \
    tests/CodingAdventures.Conduit.Tests/CodingAdventures.Conduit.Tests.fsproj \
    --disable-build-servers \
    /p:CollectCoverage=true \
    /p:Threshold=80 \
    /p:ThresholdType=line \
    /p:Include="[CodingAdventures.Conduit]*"
