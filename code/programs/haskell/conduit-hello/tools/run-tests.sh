#!/bin/sh
# run-tests.sh — build conduit-capi then run smoke tests for conduit-hello.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

REPO_ROOT=$(git rev-parse --show-toplevel)
CAPI_DIR="$REPO_ROOT/code/packages/rust/conduit-capi"
INCLUDE_DIR="$CAPI_DIR/include"

echo "==> Building conduit-capi (release cdylib)"
(cd "$CAPI_DIR/.." && cargo build -p conduit-capi --release -q)

OS=$(uname -s)
LIB_DIR="$CAPI_DIR/../target/release"

echo "==> Running cabal test (smoke tests)"
cabal test \
  --extra-lib-dirs="$LIB_DIR" \
  --extra-include-dirs="$INCLUDE_DIR" \
  --test-show-details=always
