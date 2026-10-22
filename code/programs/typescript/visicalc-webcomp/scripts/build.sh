#!/usr/bin/env bash
#
# build.sh — compile VisiCalc's FormulaBar and Grid to Web Components and
# write them into vendor/.
#
# Per the VisiCalc cross-backend demo plan (Phase 2 — VC2-webcomp): the page
# imports `vendor/{FormulaBar,Grid}.js` and opens from file://, so the
# compiled elements are committed. They used to be compiled into build/ and
# copied into vendor/ by hand; this writes them in place instead.
#
# Usage:
#   cd code/programs/typescript/visicalc-webcomp
#   bash scripts/build.sh           # regenerate vendor/FormulaBar.js, vendor/Grid.js
#   bash scripts/build.sh --check   # fail if vendor/ is stale (CI: BUILD_mac_and_linux)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEMO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$DEMO_DIR/../../../.." && pwd)"

MOSAIC_COMPILE="$REPO_ROOT/code/packages/rust/target/debug/mosaic-compile"

# Always (re)build mosaic-compile so emitter fixes reach the generated code.
# cargo is incremental, so this is ~free when nothing changed. The old
# `if [ ! -x "$MOSAIC_COMPILE" ]` guard skipped the build whenever ANY stale
# binary already existed, silently serving pre-fix output (this is what hid the
# Compose formula-bar textStyle fix — the demo kept using an old mosaic-compile).
echo "Building mosaic-compile..."
(cd "$REPO_ROOT/code/packages/rust" && cargo build -p mosaic-compile)

SRC="$REPO_ROOT/code/programs/mosaic/visicalc"
CHECK=0
if [ "${1:-}" = "--check" ]; then
  CHECK=1
  OUT_DIR="$(mktemp -d)"
  trap 'rm -rf "$OUT_DIR"' EXIT
else
  OUT_DIR="$DEMO_DIR/vendor"
fi
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
  --package-search-path "$REPO_ROOT/code/packages:$REPO_ROOT/code/packages/mosaic" \
  -o "$OUT_DIR/Grid.js"

if ((CHECK)); then
  stale=0
  for element in FormulaBar Grid; do
    if ! cmp -s "$OUT_DIR/$element.js" "$DEMO_DIR/vendor/$element.js"; then
      echo "error: vendor/$element.js is stale: the emitter's output changed." >&2
      stale=1
    fi
  done
  if ((stale)); then
    echo "       Regenerate with: bash code/programs/typescript/visicalc-webcomp/scripts/build.sh" >&2
    exit 1
  fi
  echo "vendor/FormulaBar.js and vendor/Grid.js are current"
  exit 0
fi

echo "Done. Regenerated:"
ls -la "$OUT_DIR"
echo
echo "Open $DEMO_DIR/index.html in a browser to view."
