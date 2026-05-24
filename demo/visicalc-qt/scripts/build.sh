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

# TODO(VC2-qt-grid): wire Grid once the Qt pipeline emitter
# supports the `Grid` built-in primitive. Until then, main.qml
# inlines a hand-written QtQuick.Controls TableView placeholder.

echo "Done. Generated:"
ls -la "$OUT_DIR"
echo
echo "To run the demo (Qt 6 SDK required):"
echo "  cd $DEMO_DIR"
echo "  qml main.qml"
