#!/usr/bin/env bash
#
# build.sh — regenerate the Grid and FormulaBar React components from
# their .mil/.mll/.msl source files.
#
# Per UI26 §11: the generated `src/components/*.tsx` files are NOT
# committed to the repo — this script rebuilds them on demand. Run
# manually or via `npm run components`.
#
# Usage:
#   cd code/programs/typescript/visicalc
#   bash scripts/build.sh
#
# Output:
#   src/components/Grid.tsx
#   src/components/GridEvent.ts        (re-export, hand-written shim)
#   src/components/FormulaBar.tsx
#   src/components/FormulaBarEvent.ts  (re-export, hand-written shim)

set -euo pipefail

# Find the repo root by walking up from this script's directory.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEMO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$DEMO_DIR/../../../.." && pwd)"

MOSAIC_COMPILE="$REPO_ROOT/code/packages/rust/target/debug/mosaic-compile"

# Build mosaic-compile if it doesn't exist or is older than the source.
# Always (re)build mosaic-compile so emitter fixes reach the generated code.
# cargo is incremental, so this is ~free when nothing changed. The old
# `if [ ! -x "$MOSAIC_COMPILE" ]` guard skipped the build whenever ANY stale
# binary already existed, silently serving pre-fix output (this is what hid the
# Compose formula-bar textStyle fix — the demo kept using an old mosaic-compile).
echo "Building mosaic-compile..."
(cd "$REPO_ROOT/code/packages/rust" && cargo build -p mosaic-compile)

OUT_DIR="$DEMO_DIR/src/components"
mkdir -p "$OUT_DIR"

echo "Compiling Grid..."
# UI34 PR-4 — Grid.desktop.mll is now a one-liner that references
# `pkg::mosaic-pkg-grid::Grid`.  We pass --package-search-path
# explicitly so the build works regardless of the directory the
# script is invoked from; the resolver locates the package,
# recursively compiles its Grid + Cell triple, and substitutes
# the resolved sub-tree into the demo's layout before the React
# emitter runs.  The generated Grid.tsx is byte-identical to the
# pre-UI34 inlined-kernel-primitives output.
"$MOSAIC_COMPILE" --backend react \
  --interface           "$REPO_ROOT/code/programs/mosaic/visicalc/Grid.mil" \
  --layout              "$REPO_ROOT/code/programs/mosaic/visicalc/Grid.desktop.mll" \
  --style               "$REPO_ROOT/code/programs/mosaic/visicalc/Grid.dark.msl" \
  --package-search-path "$REPO_ROOT/code/packages" \
  -o "$OUT_DIR/Grid.tsx"

echo "Compiling FormulaBar..."
"$MOSAIC_COMPILE" --backend react \
  --interface "$REPO_ROOT/code/programs/mosaic/visicalc/FormulaBar.mil" \
  --layout    "$REPO_ROOT/code/programs/mosaic/visicalc/FormulaBar.desktop.mll" \
  --style     "$REPO_ROOT/code/programs/mosaic/visicalc/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.tsx"

# The pipeline emitter writes `export type {Component}Event = ...` into
# each generated .tsx file, so the host (src/app/state.ts) imports
# `GridEvent` and `FormulaBarEvent` directly. No re-export shims needed.

# Vendor the shared Rust spreadsheet-core engine (compiled to WASM) into
# `public/` so Vite copies it into `dist/` and index.html can load it. The
# React host renders the engine's *computed* values (see src/app/engine.ts);
# without this the grid is empty because the reducer stores raw strings with
# no formula evaluation. We copy from the sibling HTML demo's committed vendor
# copy — the single source of truth for the bundle — rather than committing a
# second ~890 KB duplicate here (public/spreadsheet-engine-wasm.js is gitignored).
ENGINE_SRC="$REPO_ROOT/code/programs/typescript/visicalc-html/vendor/spreadsheet-engine-wasm.js"
PUBLIC_DIR="$DEMO_DIR/public"
mkdir -p "$PUBLIC_DIR"
if [ -f "$ENGINE_SRC" ]; then
  echo "Vendoring engine bundle -> public/spreadsheet-engine-wasm.js"
  cp "$ENGINE_SRC" "$PUBLIC_DIR/spreadsheet-engine-wasm.js"
else
  echo "ERROR: engine bundle not found at $ENGINE_SRC" >&2
  exit 1
fi

echo "Done. Generated:"
ls -la "$OUT_DIR"
