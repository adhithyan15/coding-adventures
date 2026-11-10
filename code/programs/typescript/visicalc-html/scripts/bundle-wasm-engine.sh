#!/usr/bin/env bash
#
# bundle-wasm-engine.sh — produce the browser bundle that loads the Rust
# spreadsheet engine compiled to WebAssembly.
#
# The bundle is GENERATED and ignored by version control (BUILD-001): build it
# after a clone, and again after the engine or the loader changes.
# `code/scripts/build-demo-wasm.sh` does that for every demo engine.
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
# Source of the .wasm: the spreadsheet-wasm crate's built artifact
# (pkg/spreadsheet_engine.wasm, also generated and ignored). If it is missing
# we build it.
#
# Usage:
#   cd code/programs/typescript/visicalc-html && bash scripts/bundle-wasm-engine.sh
# Tooling comes from mise; run via `mise exec -- bash scripts/...` if needed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEMO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
# The repo root is wherever `code/` lives. Ask git for it so this survives the
# demo being relocated (it moved from demo/ into code/programs/typescript/ — a
# hard-coded `../..` here silently resolved to code/programs and broke the
# build). Fall back to walking up to the `code/` ancestor if git is unavailable.
REPO_ROOT="$(cd "$SCRIPT_DIR" && git rev-parse --show-toplevel 2>/dev/null || (d="$DEMO_DIR"; while [ "$d" != "/" ] && [ ! -d "$d/code/packages" ]; do d="$(dirname "$d")"; done; echo "$d"))"

WASM="$REPO_ROOT/code/packages/rust/spreadsheet-wasm/pkg/spreadsheet_engine.wasm"
LOADER="$SCRIPT_DIR/wasm-loader.js"
OUT_DIR="$DEMO_DIR/vendor"
OUT="$OUT_DIR/spreadsheet-engine-wasm.js"

if [ ! -f "$WASM" ]; then
  echo "pkg/spreadsheet_engine.wasm is missing; building it with spreadsheet-wasm/build-wasm.sh"
  bash "$REPO_ROOT/code/packages/rust/spreadsheet-wasm/build-wasm.sh"
fi

# The bundle is a pure function of the .wasm and the loader, so it is
# regenerated rather than edited. Neither it nor the .wasm is committed, so
# there is nothing to check for staleness.
render_bundle() {
  printf '// AUTO-GENERATED — DO NOT EDIT. Regenerate with scripts/bundle-wasm-engine.sh\n'
  printf 'var __SPREADSHEET_WASM_B64__ = "%s";\n' "$(base64 < "$WASM" | tr -d '\n')"
  cat "$LOADER"
}

mkdir -p "$OUT_DIR"
render_bundle > "$OUT"
echo "Wrote $OUT ($(wc -c < "$OUT") bytes; wasm $(wc -c < "$WASM") bytes)"
