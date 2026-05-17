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
"$MOSAIC_COMPILE" --backend react \
  --interface "$DEMO_DIR/mosaic/Grid.mil" \
  --layout    "$DEMO_DIR/mosaic/Grid.desktop.mll" \
  --style     "$DEMO_DIR/mosaic/Grid.dark.msl" \
  -o "$OUT_DIR/Grid.tsx"

echo "Compiling FormulaBar..."
"$MOSAIC_COMPILE" --backend react \
  --interface "$DEMO_DIR/mosaic/FormulaBar.mil" \
  --layout    "$DEMO_DIR/mosaic/FormulaBar.desktop.mll" \
  --style     "$DEMO_DIR/mosaic/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.tsx"

# NOTE: The pipeline emitter inlines the event union directly into the
# .tsx file as a non-exported `type` declaration. The host app's
# `state.ts` defines its own copies of the event shapes so they don't
# need to be re-exported from the generated files. If/when the pipeline
# emitter starts using `export type ...`, the host can switch to
# importing directly.

echo "Done. Generated:"
ls -la "$OUT_DIR"
