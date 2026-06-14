#!/bin/sh
# run-tests.sh — build conduit-capi (Rust cdylib), then run dotnet test.
#
# Why build Rust here instead of relying on BUILD deps= ordering?
# The CodeQL CI workflow runs `build-tool -validate-build-files -language csharp`,
# which processes packages in plan order.  C# packages may be visited before the
# Rust conduit-capi package, so the .so/.dylib may not yet exist.  This script
# makes the C# package self-sufficient regardless of visitation order.
#
# In the normal workflow, `deps=rust/conduit-capi` causes the build-tool to build
# conduit-capi first, making the `cargo build` below a fast no-op.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

CAPI_DIR=$(cd ../../rust/conduit-capi && pwd)

echo "==> Building conduit-capi (release cdylib)"
(cd "$CAPI_DIR" && cargo build --release -q)

# Point the native library resolver at the built artifact.
# Native.Resolve() checks CONDUIT_CAPI_PATH before falling back to OS search.
OS=$(uname -s)
case "$OS" in
  Darwin) LIB_FILE="$CAPI_DIR/../target/release/libconduit_capi.dylib" ;;
  *)      LIB_FILE="$CAPI_DIR/../target/release/libconduit_capi.so"    ;;
esac

echo "==> Running dotnet test (CONDUIT_CAPI_PATH=$LIB_FILE)"
CONDUIT_CAPI_PATH="$LIB_FILE" \
  dotnet test \
    tests/CodingAdventures.Conduit.Tests/CodingAdventures.Conduit.Tests.csproj \
    --disable-build-servers \
    /p:CollectCoverage=true \
    /p:Threshold=80 \
    /p:ThresholdType=line
