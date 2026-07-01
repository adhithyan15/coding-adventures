#!/usr/bin/env bash
#
# build.sh — regenerate the FormulaBar WebComponent bundle from the
# Mosaic pipeline (mosaic-compile --backend webcomponent).
#
# Per the VisiCalc cross-backend demo plan (Phase 2 — VC2-webcomp):
# this is the second cross-backend demo. The output is a JS file that
# defines `<mos-formula-bar>` as a custom element with shadow-DOM
# styling; index.html imports the bundle and mounts the element with
# HTML attributes.
#
# Today we only compile FormulaBar through the pipeline. The Grid
# fragment in index.html is hand-written (same gap as VC2-html — the
# `Grid` built-in primitive isn't supported by the
# mosaic-emit-webcomponent pipeline yet, only the React emitter knows
# how to lower it). When the WebComponent Grid emitter lands, this
# script gains a second `mosaic-compile --backend webcomponent ...`
# line for Grid and index.html switches to `<mos-grid>` instead of
# the hand-written `<table>` placeholder.
#
# Usage:
#   cd demo/visicalc-webcomp
#   bash scripts/build.sh
#
# Output:
#   demo/visicalc-webcomp/build/FormulaBar.js
#
# Open `demo/visicalc-webcomp/index.html` in any browser to view.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEMO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$DEMO_DIR/../.." && pwd)"

MOSAIC_COMPILE="$REPO_ROOT/code/packages/rust/target/debug/mosaic-compile"

# Always (re)build mosaic-compile so emitter fixes reach the generated code.
# cargo is incremental, so this is ~free when nothing changed. The old
# `if [ ! -x "$MOSAIC_COMPILE" ]` guard skipped the build whenever ANY stale
# binary already existed, silently serving pre-fix output (this is what hid the
# Compose formula-bar textStyle fix — the demo kept using an old mosaic-compile).
echo "Building mosaic-compile..."
(cd "$REPO_ROOT/code/packages/rust" && cargo build -p mosaic-compile)

SRC="$REPO_ROOT/demo/visicalc/mosaic"
OUT_DIR="$DEMO_DIR/build"
mkdir -p "$OUT_DIR"

echo "Compiling FormulaBar (WebComponent)..."
"$MOSAIC_COMPILE" --backend webcomponent \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC/FormulaBar.desktop.mll" \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.js"

echo "Compiling Grid (WebComponent)..."
# UI34 PR — Grid is now generated from the shared visicalc/mosaic/
# Grid.{mil,desktop.mll,dark.msl} triple (same source the React +
# HTML demos consume).  Grid.desktop.mll is a UI34
# `pkg::mosaic-pkg-grid::Grid (...)` one-liner; we pass
# --package-search-path so the resolver locates the package and
# substitutes its full composition before the WebComponent emitter
# runs.  The output registers `<mos-grid>` as a Custom Element with
# shadow DOM that reads list-typed slots via JSON.parse (so attribute
# values like `column-headers='["A","B","C"]'` work).
"$MOSAIC_COMPILE" --backend webcomponent \
  --interface           "$SRC/Grid.mil" \
  --layout              "$SRC/Grid.desktop.mll" \
  --style               "$SRC/Grid.dark.msl" \
  --package-search-path "$REPO_ROOT/code/packages" \
  -o "$OUT_DIR/Grid.js"

echo "Done. Generated:"
ls -la "$OUT_DIR"
echo
echo "Open $DEMO_DIR/index.html in a browser to view."
