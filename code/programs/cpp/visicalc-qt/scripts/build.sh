#!/usr/bin/env bash
#
# build.sh — regenerate the FormulaBar/Grid QML from the Mosaic pipeline AND
# build+vendor the Rust spreadsheet engine the demo computes on.
#
# Per the VisiCalc cross-backend demo plan (Phase 2 / VC2-qt): this is the Qt
# cross-backend demo, now wired to the shared Rust `spreadsheet-core` engine
# through its C ABI (spreadsheet-capi) — the same engine the SwiftUI demo links
# natively and the web demos run as WebAssembly. The generated QML renders the
# values the engine computes; editing the formula bar writes through to the
# engine and recomputes.
#
# This script does two things:
#   1. Regenerates build/{FormulaBar,Grid}.qml via `mosaic-compile --backend qt`.
#   2. Builds the spreadsheet-capi crate to a static library and vendors it
#      (plus its C header) into Vendor/ — git-ignored, exactly like the SwiftUI
#      demo's Vendor/.
#
# Usage:
#   cd demo/visicalc-qt
#   bash scripts/build.sh
#
# Then build & run. The engine-backed binary is the real demo:
#   qmake && make && ./visicalc_qt_app            # qmake (no CMake needed)
#   # or, if you have CMake:
#   cmake -B build-cmake && cmake --build build-cmake && ./build-cmake/visicalc_qt_app
#
# And the headless proof (the Qt equivalent of `swift test`):
#   cd test && qmake && make && ./tst_model
#
# (`qml main.qml` still opens the layout for QML iteration, but it can't expose
# the C++ model or link the engine, so its grid is empty — use the binary.)

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

echo "Building the spreadsheet engine (Rust → static lib) and vendoring it..."
# The C ABI static library the Qt app + headless test link (see visicalc-qt.pro
# / CMakeLists.txt). Built from the spreadsheet-capi crate and copied into
# Vendor/ (git-ignored), along with the C header the C++ model includes.
(cd "$REPO_ROOT/code/packages/rust" && cargo build -p spreadsheet-capi --release)
mkdir -p "$DEMO_DIR/Vendor"
cp "$REPO_ROOT/code/packages/rust/target/release/libspreadsheet_capi.a" "$DEMO_DIR/Vendor/"
cp "$REPO_ROOT/code/packages/rust/spreadsheet-capi/include/spreadsheet.h" "$DEMO_DIR/Vendor/spreadsheet.h"

echo "Done. Generated QML + vendored engine:"
ls -la "$OUT_DIR"
ls -la "$DEMO_DIR/Vendor"
echo
echo "To build & run the demo (Qt 6 SDK required):"
echo "  cd $DEMO_DIR"
echo "  qmake && make && ./visicalc_qt_app           # the engine-backed GUI"
echo "  (cd test && qmake && make && ./tst_model)    # headless engine proof"
