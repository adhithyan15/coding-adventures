#!/usr/bin/env bash
#
# build.sh — regenerate the FormulaBar SwiftUI view from the Mosaic
# pipeline and drop it into Sources/VisiCalc/Generated/.
#
# Per the VisiCalc cross-backend demo plan (Phase 2 / VC2-swiftui):
# this is the SwiftUI cross-backend demo. The output is a Swift
# file declaring `FormulaBarView: View`; ContentView.swift imports
# the package and mounts the view inside a VStack.
#
# UI34 rewire — Grid is now also generated through the pipeline
# from the shared `demo/visicalc/mosaic/Grid.{mil,desktop.mll,dark.msl}`
# triple.  Grid.desktop.mll is a UI34 `pkg::mosaic-pkg-grid::Grid`
# one-liner; we pass --package-search-path so the resolver
# substitutes the package's full composition before the SwiftUI
# emitter runs.
#
# Usage:
#   cd demo/visicalc-swiftui
#   bash scripts/build.sh
#
# Then to actually run the app (Swift 5.9+ / Xcode 15+ required):
#   swift run                  # macOS terminal target
#   open Package.swift         # open in Xcode for iOS / macOS app
#
# NOTE: there's a known emitter glitch in the generated FormulaBar
# (the `.onSubmit` handler emits `.commit(value: formula)` while
# the `FormulaBarEvent` enum's `commit` case carries no associated
# value). The Swift compiler will reject this. Track as
# UI31-swiftui-commit-arity in a follow-up; meanwhile, hand-patch
# the generated file or wait for the emitter fix before `swift run`.

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
OUT_DIR="$DEMO_DIR/Sources/VisiCalc/Generated"
mkdir -p "$OUT_DIR"

echo "Compiling FormulaBar (SwiftUI)..."
"$MOSAIC_COMPILE" --backend swiftui \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC/FormulaBar.desktop.mll" \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.swift"

echo "Compiling Grid (SwiftUI)..."
"$MOSAIC_COMPILE" --backend swiftui \
  --interface           "$SRC/Grid.mil" \
  --layout              "$SRC/Grid.desktop.mll" \
  --style               "$SRC/Grid.dark.msl" \
  --package-search-path "$REPO_ROOT/code/packages" \
  -o "$OUT_DIR/Grid.swift"

echo "Done. Generated:"
ls -la "$OUT_DIR"
echo
echo "To run the demo (Swift 5.9+ / Xcode 15+ required):"
echo "  cd $DEMO_DIR"
echo "  swift run"
