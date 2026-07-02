#!/usr/bin/env bash
#
# build-engine.sh — build the shared Rust spreadsheet-core engine as a C ABI
# dynamic library and vendor it next to main-ffi.ts.
#
# The WASM path (main.ts) embeds the engine as base64 WebAssembly and runs it in
# the webview. The FFI path (main-ffi.ts) instead loads this NATIVE dynamic
# library into the Deno process via `Deno.dlopen` and calls the same
# `spreadsheet-capi` C ABI the Qt / SwiftUI demos link — so the engine runs as
# native machine code server-side, and the webview is a thin HTTP client.
#
# The vendored library (vendor/libspreadsheet_capi.{dylib,so,dll}) is git-ignored
# (like the *.app bundles) — a build artifact, rebuilt from the crate here.
#
# Usage:  bash scripts/build-engine.sh   (or: deno task engine)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEMO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$DEMO_DIR/../../../.." && pwd)"
RUST_DIR="$REPO_ROOT/code/packages/rust"

echo "Building spreadsheet-capi (Rust → C ABI cdylib, release)..."
(cd "$RUST_DIR" && cargo build -p spreadsheet-capi --release)

# Pick the platform-correct dynamic-library filename.
case "$(uname -s)" in
  Darwin) LIB="libspreadsheet_capi.dylib" ;;
  Linux)  LIB="libspreadsheet_capi.so" ;;
  *)      LIB="spreadsheet_capi.dll" ;;   # MSYS/Cygwin uname
esac

SRC="$RUST_DIR/target/release/$LIB"
if [ ! -f "$SRC" ]; then
  echo "ERROR: built library not found at $SRC" >&2
  exit 1
fi

mkdir -p "$DEMO_DIR/vendor"
cp "$SRC" "$DEMO_DIR/vendor/$LIB"
echo "Vendored engine -> vendor/$LIB"
