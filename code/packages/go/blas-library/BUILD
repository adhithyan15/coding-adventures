#!/usr/bin/env bash
# Build script for the Go BLAS library.
#
# Runs tests with verbose output and coverage reporting.
# Exit code is non-zero if any test fails.
set -euo pipefail

echo "=== BLAS Library: Go ==="
echo "Running tests..."
go test ./... -v -cover
echo ""
echo "Running go vet..."
go vet ./...
echo ""
echo "Done."
