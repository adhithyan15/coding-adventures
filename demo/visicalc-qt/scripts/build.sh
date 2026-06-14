#!/usr/bin/env bash
#
# build.sh — regenerate FormulaBar.qml from the Mosaic pipeline
# and drop it into the demo's build/ directory.
#
# Per the VisiCalc cross-backend demo plan (Phase 2 / VC2-qt):
# this is the Qt cross-backend demo. The output is a QML file
# defining `FormulaBar.qml` as a top-level Item with property/signal
# declarations; main.qml imports it via the local QML module
# (declared in qmldir).
#
# Today we only compile FormulaBar through the pipeline. The Grid
# is hand-written in main.qml as inline QtQuick.Controls 2 widgets
# (the mosaic-emit-qt pipeline doesn't yet support the `Grid`
# built-in primitive — only the React emitter knows how to lower
# it fully). When the Qt Grid emitter lands, this gains a second
# `mosaic-compile --backend qt` invocation.
#
# Usage:
#   cd demo/visicalc-qt
#   bash scripts/build.sh
#
# Then to actually run the app (Qt 6 SDK required):
#   qml main.qml
#   # or via CMake:
#   cmake -B build && cmake --build build && ./build/visicalc-qt

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

echo "Compiling FormulaBar (Qt / QML)..."
"$MOSAIC_COMPILE" --backend qt \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC/FormulaBar.desktop.mll" \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.qml"

echo "Compiling Grid (Qt / QML)..."
# UI34 — Grid is now generated from the shared visicalc/mosaic/
# Grid.{mil,desktop.mll,dark.msl} triple (same source the React +
# HTML + WebComponent + SwiftUI demos consume).  Grid.desktop.mll
# is a UI34 `pkg::mosaic-pkg-grid::Grid` one-liner; we pass
# --package-search-path so the resolver substitutes the package's
# full composition before the Qt emitter runs.  The output is a
# QML Item with `Repeater` loops, `Loader` if/else gates, and
# typed signals — exposed to main.qml through the local `qml/`
# module by adding a `Grid 1.0 ../build/Grid.qml` line to qmldir.
"$MOSAIC_COMPILE" --backend qt \
  --interface           "$SRC/Grid.mil" \
  --layout              "$SRC/Grid.desktop.mll" \
  --style               "$SRC/Grid.dark.msl" \
  --package-search-path "$REPO_ROOT/code/packages" \
  -o "$OUT_DIR/Grid.qml"

echo "Done. Generated:"
ls -la "$OUT_DIR"
echo
echo "To run the demo (Qt 6 SDK required):"
echo "  cd $DEMO_DIR"
echo "  qml main.qml"
