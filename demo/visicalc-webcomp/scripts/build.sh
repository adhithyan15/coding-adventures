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

if [ ! -x "$MOSAIC_COMPILE" ]; then
  echo "Building mosaic-compile..."
  (cd "$REPO_ROOT/code/packages/rust" && cargo build -p mosaic-compile)
fi

SRC="$REPO_ROOT/demo/visicalc/mosaic"
OUT_DIR="$DEMO_DIR/build"
mkdir -p "$OUT_DIR"

echo "Compiling FormulaBar (WebComponent)..."
"$MOSAIC_COMPILE" --backend webcomponent \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC/FormulaBar.desktop.mll" \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.js"

# TODO(VC2-webcomp-grid): wire Grid once the WebComponent pipeline
# emitter supports the `Grid` built-in primitive. Until then,
# index.html uses a hand-written static-HTML <table> placeholder.

echo "Done. Generated:"
ls -la "$OUT_DIR"
echo
echo "Open $DEMO_DIR/index.html in a browser to view."
