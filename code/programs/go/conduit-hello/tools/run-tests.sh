#!/bin/sh
# run-tests.sh — build the Conduit C ABI, then run the conduit-hello smoke test.
#
# Mirrors code/packages/go/conduit/tools/run-tests.sh: the Rust staticlib must
# exist before `go test` runs, and this script makes the BUILD entry
# self-sufficient for both the normal build workflow and the CodeQL build.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

CAPI_DIR=$(cd ../../../packages/rust/conduit-capi && pwd)

echo "==> Building conduit-capi (release static lib)"
(cd "$CAPI_DIR" && cargo build --release -q)

echo "==> Running smoke test"
CGO_ENABLED=1 go test ./... -v
