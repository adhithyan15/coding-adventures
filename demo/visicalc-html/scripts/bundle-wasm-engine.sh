#!/usr/bin/env bash
#
# bundle-wasm-engine.sh — produce the committed browser bundle that loads the
# Rust spreadsheet engine compiled to WebAssembly.
#
# Output: vendor/spreadsheet-engine-wasm.js =
#     var __SPREADSHEET_WASM_B64__ = "<base64 of the .wasm>";
#     <scripts/wasm-loader.js>
#
# The .wasm bytes are embedded as base64 (not fetched) so index.html still
# opens directly from disk via file:// — no server, no CORS. The loader sets
# window.SpreadsheetEngine (same API as the TypeScript engine) and resolves
# window.__spreadsheetEngineReady once the module instantiates.
#
# Source of the .wasm: the spreadsheet-wasm crate's committed artifact. If it
# is missing we build it.
#
# Usage:
#   cd demo/visicalc-html && bash scripts/bundle-wasm-engine.sh
# Tooling comes from mise; run via `mise exec -- bash scripts/...` if needed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEMO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$DEMO_DIR/../.." && pwd)"

WASM="$REPO_ROOT/code/packages/rust/spreadsheet-wasm/pkg/spreadsheet_engine.wasm"
LOADER="$SCRIPT_DIR/wasm-loader.js"
OUT_DIR="$DEMO_DIR/vendor"
OUT="$OUT_DIR/spreadsheet-engine-wasm.js"

if [ ! -f "$WASM" ]; then
  echo "Building the .wasm (missing)…"
  (cd "$REPO_ROOT/code/packages/rust/spreadsheet-wasm" && bash build-wasm.sh)
fi

mkdir -p "$OUT_DIR"

# base64, stripped of newlines (one long string literal).
B64="$(base64 < "$WASM" | tr -d '\n')"

{
  printf '// AUTO-GENERATED — DO NOT EDIT. Regenerate with scripts/bundle-wasm-engine.sh\n'
  printf 'var __SPREADSHEET_WASM_B64__ = "%s";\n' "$B64"
  cat "$LOADER"
} > "$OUT"

echo "Wrote $OUT ($(wc -c < "$OUT") bytes; wasm $(wc -c < "$WASM") bytes)"
