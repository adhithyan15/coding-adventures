#!/usr/bin/env bash
#
# build.sh — regenerate the FormulaBar HTML fragment from the Mosaic
# pipeline (mosaic-compile --backend html).
#
# Per the VisiCalc cross-backend demo plan (Phase 2 — VC2-html):
# this is the simplest possible cross-backend demo — open `index.html`
# in a browser and see VisiCalc.
#
# Today we only compile FormulaBar through the pipeline. The Grid
# fragment is hand-written in index.html as a static HTML <table>
# placeholder, because the `Grid` built-in primitive isn't yet wired
# into the mosaic-emit-html pipeline (only the React emitter knows
# how to lower it). When the HTML Grid emitter lands, this script
# gains a second `mosaic-compile --backend html ...` line for Grid
# and index.html switches to including `build/Grid.html` instead of
# its hand-written placeholder.
#
# Usage:
#   cd demo/visicalc-html
#   bash scripts/build.sh
#
# Output:
#   demo/visicalc-html/build/FormulaBar.html
#
# Open `demo/visicalc-html/index.html` in any browser to view.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEMO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$DEMO_DIR/../.." && pwd)"

MOSAIC_COMPILE="$REPO_ROOT/code/packages/rust/target/debug/mosaic-compile"

if [ ! -x "$MOSAIC_COMPILE" ]; then
  echo "Building mosaic-compile..."
  (cd "$REPO_ROOT/code/packages/rust" && cargo build -p mosaic-compile)
fi

SRC="$REPO_ROOT/demo/visicalc/mosaic"
OUT_DIR="$DEMO_DIR/build"
mkdir -p "$OUT_DIR"

echo "Compiling FormulaBar (HTML, desktop variant)..."
"$MOSAIC_COMPILE" --backend html \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC"   --variant desktop \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.html"

# UI30 multi-variant proof: same .mil + .msl, different .mll —
# layout pivots from Row (desktop) to Column (touch) without any
# host-code change.  See FormulaBar.touch.mll's header comment.
echo "Compiling FormulaBar (HTML, touch variant)..."
"$MOSAIC_COMPILE" --backend html \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC"   --variant touch \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.touch.html"

# TODO(VC2-html-grid): wire Grid once the HTML pipeline emitter
# supports the `Grid` built-in primitive. Until then, the index.html
# uses a hand-written static-HTML <table> placeholder.

echo "Done. Generated:"
ls -la "$OUT_DIR"
echo
echo "Open $DEMO_DIR/index.html in a browser to view."
