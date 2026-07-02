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
#   cd code/programs/dart/visicalc-flutter
#   bash scripts/build.sh
#
# Then to actually run the app (Flutter SDK required):
#   flutter pub get
#   flutter run        # or `flutter run -d chrome` / `-d macos` / etc.

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
OUT_DIR="$DEMO_DIR/lib/generated"
mkdir -p "$OUT_DIR"

echo "Compiling FormulaBar (Flutter, desktop)..."
"$MOSAIC_COMPILE" --backend flutter \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC/FormulaBar.desktop.mll" \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/formula_bar.dart"

# UI30 touch variant: FormulaBar.touch.mll stacks the address label ABOVE a
# full-width input (Column) instead of the desktop Row. Same FormulaBar.mil
# interface, so main.dart can swap FormulaBar <-> FormulaBarTouch at runtime
# against the identical dispatch contract. The Flutter emitter names the widget
# after the .mil component (FormulaBar) and also emits the shared
# `sealed class FormulaBarEvent` + subclasses; to let both widgets coexist we
# (1) strip the DUPLICATE event classes from the touch output and instead
# `import 'formula_bar.dart'` to reuse them, and (2) rename the widget +
# constructor to FormulaBarTouch. Result: one event type, two layouts.
echo "Compiling FormulaBar (Flutter, touch)..."
"$MOSAIC_COMPILE" --backend flutter \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC/FormulaBar.touch.mll" \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/formula_bar_touch.dart.raw"
# awk: inject `import 'formula_bar.dart';` right after the material import, and
# drop the event-class block (from `sealed class FormulaBarEvent {` up to, but
# excluding, `class FormulaBar extends StatelessWidget {`). sed: rename the
# widget class + its constructor. (No newlines in the sed replacement, so it is
# portable across BSD/macOS + GNU sed.)
awk '
  /^import '\''package:flutter\/material.dart'\'';/ { print; print "import '\''formula_bar.dart'\'';"; next }
  /^sealed class FormulaBarEvent \{/ { skip = 1 }
  /^class FormulaBar extends StatelessWidget \{/ { skip = 0 }
  skip { next }
  { print }
' "$OUT_DIR/formula_bar_touch.dart.raw" \
  | sed -e 's/class FormulaBar extends StatelessWidget/class FormulaBarTouch extends StatelessWidget/' \
        -e 's/const FormulaBar(/const FormulaBarTouch(/' \
  > "$OUT_DIR/formula_bar_touch.dart"
rm -f "$OUT_DIR/formula_bar_touch.dart.raw"

echo "Compiling Grid (Flutter)..."
# UI34 — Grid is now generated from the shared visicalc/mosaic/
# Grid.{mil,desktop.mll,dark.msl} triple (same source the React +
# HTML + WebComponent + SwiftUI + Qt demos consume).
# Grid.desktop.mll is a UI34 `pkg::mosaic-pkg-grid::Grid` one-liner;
# we pass --package-search-path so the resolver substitutes the
# package's full composition before the Flutter emitter runs.
"$MOSAIC_COMPILE" --backend flutter \
  --interface           "$SRC/Grid.mil" \
  --layout              "$SRC/Grid.desktop.mll" \
  --style               "$SRC/Grid.dark.msl" \
  --package-search-path "$REPO_ROOT/code/packages" \
  -o "$OUT_DIR/grid.dart"

echo "Building the spreadsheet engine (Rust → dynamic lib) and vendoring it..."
# The C ABI dynamic library that dart:ffi loads at runtime (see lib/engine.dart),
# built from the spreadsheet-capi crate (cdylib) and copied into native/
# (git-ignored). dart:ffi's DynamicLibrary.open needs a .dylib/.so/.dll, not the
# static .a the SwiftUI/Qt demos link — same engine, dynamic packaging.
RUST="$REPO_ROOT/code/packages/rust"
(cd "$RUST" && cargo build -p spreadsheet-capi --release)
mkdir -p "$DEMO_DIR/native"
case "$(uname -s)" in
  Darwin) LIB="libspreadsheet_capi.dylib" ;;
  *)      LIB="libspreadsheet_capi.so" ;;
esac
cp "$RUST/target/release/$LIB" "$DEMO_DIR/native/$LIB"

echo "Done. Generated widgets + vendored engine:"
ls -la "$OUT_DIR"
ls -la "$DEMO_DIR/native"
echo
echo "To run the demo (Flutter SDK required):"
echo "  cd $DEMO_DIR"
echo "  flutter test               # headless: grid is engine-computed + recomputes"
echo "  flutter pub get && flutter run -d macos   # launch the desktop app"
