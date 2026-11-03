#!/bin/sh
# run-tests.sh — build conduit-capi (Rust cdylib) then run cabal test.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

REPO_ROOT=$(git rev-parse --show-toplevel)
CAPI_DIR="$REPO_ROOT/code/packages/rust/conduit-capi"
INCLUDE_DIR="$CAPI_DIR/include"

echo "==> Building conduit-capi (release cdylib)"
(cd "$CAPI_DIR/.." && cargo build -p conduit-capi --release -q)

LIB_DIR="$CAPI_DIR/../target/release"

# --extra-lib-dirs only affects LINK time.  At RUN time the test binary loads
# libconduit_capi.{so,dylib} via the OS dynamic loader, so we must also put the
# directory on LD_LIBRARY_PATH (Linux) and DYLD_LIBRARY_PATH (macOS).  Include
# the deps/ subdir too, since cargo sometimes resolves the dylib from there.
export LD_LIBRARY_PATH="$LIB_DIR:$LIB_DIR/deps:${LD_LIBRARY_PATH:-}"
export DYLD_LIBRARY_PATH="$LIB_DIR:$LIB_DIR/deps:${DYLD_LIBRARY_PATH:-}"

echo "==> Running cabal test"
cabal test \
  --extra-lib-dirs="$LIB_DIR" \
  --extra-include-dirs="$INCLUDE_DIR" \
  --test-show-details=always
