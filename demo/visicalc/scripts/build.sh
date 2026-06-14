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
#   cd demo/visicalc
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
REPO_ROOT="$(cd "$DEMO_DIR/../.." && pwd)"

MOSAIC_COMPILE="$REPO_ROOT/code/packages/rust/target/debug/mosaic-compile"

# Build mosaic-compile if it doesn't exist or is older than the source.
if [ ! -x "$MOSAIC_COMPILE" ]; then
  echo "Building mosaic-compile..."
  (cd "$REPO_ROOT/code/packages/rust" && cargo build -p mosaic-compile)
fi

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
  --interface           "$DEMO_DIR/mosaic/Grid.mil" \
  --layout              "$DEMO_DIR/mosaic/Grid.desktop.mll" \
  --style               "$DEMO_DIR/mosaic/Grid.dark.msl" \
  --package-search-path "$REPO_ROOT/code/packages" \
  -o "$OUT_DIR/Grid.tsx"

echo "Compiling FormulaBar..."
"$MOSAIC_COMPILE" --backend react \
  --interface "$DEMO_DIR/mosaic/FormulaBar.mil" \
  --layout    "$DEMO_DIR/mosaic/FormulaBar.desktop.mll" \
  --style     "$DEMO_DIR/mosaic/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.tsx"

# The pipeline emitter writes `export type {Component}Event = ...` into
# each generated .tsx file, so the host (src/app/state.ts) imports
# `GridEvent` and `FormulaBarEvent` directly. No re-export shims needed.

echo "Done. Generated:"
ls -la "$OUT_DIR"
