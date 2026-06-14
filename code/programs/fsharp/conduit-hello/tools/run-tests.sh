#!/bin/sh
# run-tests.sh — build conduit-capi (Rust cdylib), then run smoke tests.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

# Use git to find the repo root — works both in the main tree and in isolated
# worktrees (git traverses up past the worktree directory to the real .git).
REPO_ROOT=$(git rev-parse --show-toplevel)
CAPI_DIR="$REPO_ROOT/code/packages/rust/conduit-capi"

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
    tests/ConduitHello.Smoke/ConduitHello.Smoke.fsproj \
    --disable-build-servers
