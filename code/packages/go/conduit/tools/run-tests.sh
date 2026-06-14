#!/bin/sh
# run-tests.sh — build the Conduit C ABI, then run the Go test suite.
#
# The Go package links libconduit_capi.a via #cgo LDFLAGS, so the Rust
# staticlib must exist before `go test` runs. In the normal build workflow
# the build-tool's `deps=rust/conduit-capi` directive handles ordering, but
# the CodeQL workflow runs all 300 Go packages from a pre-generated plan and
# may reach go/conduit before the Rust dep is built. Building it here makes
# this BUILD entry self-sufficient in both contexts.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

CAPI_DIR=$(cd ../../rust/conduit-capi && pwd)

echo "==> Building conduit-capi (release static lib)"
(cd "$CAPI_DIR" && cargo build --release -q)

echo "==> Running Go tests"
CGO_ENABLED=1 go test ./... -v -cover
