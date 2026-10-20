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
  echo "error: $WASM is missing; build it with code/packages/rust/spreadsheet-wasm/build-wasm.sh" >&2
  exit 1
fi

# The bundle is a pure function of the committed .wasm and the loader, so it
# is regenerated rather than edited, and `--check` (CI) fails when the
# committed bundle is not what those two produce. The .wasm itself is checked
# against its sources by spreadsheet-wasm's `build-wasm.sh --check`.
render_bundle() {
  printf '// AUTO-GENERATED — DO NOT EDIT. Regenerate with scripts/bundle-wasm-engine.sh\n'
  printf 'var __SPREADSHEET_WASM_B64__ = "%s";\n' "$(base64 < "$WASM" | tr -d '\n')"
  cat "$LOADER"
}

if [ "${1:-}" = "--check" ]; then
  if ! render_bundle | cmp -s - "$OUT"; then
    echo "error: $OUT is stale: it does not embed the committed .wasm with the current loader." >&2
    echo "       Regenerate with: bash $SCRIPT_DIR/bundle-wasm-engine.sh" >&2
    exit 1
  fi
  echo "vendor/spreadsheet-engine-wasm.js is current"
  exit 0
fi

mkdir -p "$OUT_DIR"
render_bundle > "$OUT"
echo "Wrote $OUT ($(wc -c < "$OUT") bytes; wasm $(wc -c < "$WASM") bytes)"
