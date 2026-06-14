#!/usr/bin/env bash
#
# build-capi.sh — build the spreadsheet C ABI shared library and run the C
# smoke test against it, proving a real C program can drive the engine.
#
# Usage:  cd code/packages/rust/spreadsheet-capi && bash build-capi.sh
# Tooling comes from mise; run via `mise exec -- bash build-capi.sh` if needed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="$(cd "$SCRIPT_DIR/.." && pwd)" # code/packages/rust

echo "Building spreadsheet-capi (release)…"
(cd "$WORKSPACE" && cargo build -p spreadsheet-capi --release)

# Locate the built dynamic library (platform-specific extension).
LIBDIR="$WORKSPACE/target/release"
case "$(uname -s)" in
  Darwin) LIB="$LIBDIR/libspreadsheet_capi.dylib" ;;
  Linux)  LIB="$LIBDIR/libspreadsheet_capi.so" ;;
  *)      LIB="$LIBDIR/spreadsheet_capi.dll" ;;
esac
echo "Library: $LIB"

BIN="$(mktemp -d)/smoke"
echo "Compiling + linking test/smoke.c…"
cc -I "$SCRIPT_DIR/include" "$SCRIPT_DIR/test/smoke.c" "$LIB" -o "$BIN"

echo "Running C smoke test…"
"$BIN"
