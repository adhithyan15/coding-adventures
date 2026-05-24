#!/usr/bin/env bash
#
# build.sh — regenerate the FormulaBar Dart widget from the Mosaic
# pipeline and drop it into lib/generated/.
#
# Per the VisiCalc cross-backend demo plan (Phase 2 / VC2-flutter):
# this is the Flutter cross-backend demo. The output is a Dart file
# containing a `FormulaBar extends StatelessWidget` class; lib/main.dart
# imports it and mounts it inside a MaterialApp.
#
# Today we only compile FormulaBar through the pipeline. The Grid
# Dart widget in lib/generated/Grid.dart is hand-written (the
# mosaic-emit-flutter pipeline emits a placeholder SizedBox.shrink()
# for the `Grid` built-in primitive — only the React emitter knows
# how to lower it fully). The hand-written Grid mimics what the
# eventual auto-generated Grid widget should look like.
#
# Usage:
#   cd demo/visicalc-flutter
#   bash scripts/build.sh
#
# Then to actually run the app (Flutter SDK required):
#   flutter pub get
#   flutter run        # or `flutter run -d chrome` / `-d macos` / etc.

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
OUT_DIR="$DEMO_DIR/lib/generated"
mkdir -p "$OUT_DIR"

echo "Compiling FormulaBar (Flutter)..."
"$MOSAIC_COMPILE" --backend flutter \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC/FormulaBar.desktop.mll" \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/formula_bar.dart"

# TODO(VC2-flutter-grid): switch to compiled Grid when the
# mosaic-emit-flutter pipeline learns the `Grid` built-in primitive.
# Today the Flutter emitter produces a placeholder SizedBox.shrink()
# for `Grid`, so lib/generated/grid.dart is hand-written to mirror
# what the eventual auto-generated widget should look like.

echo "Done. Generated:"
ls -la "$OUT_DIR"
echo
echo "To run the demo (Flutter SDK required):"
echo "  cd $DEMO_DIR"
echo "  flutter pub get"
echo "  flutter run"
