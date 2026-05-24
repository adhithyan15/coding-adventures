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
# Today we only compile FormulaBar through the pipeline. The Grid
# is hand-written in `Sources/VisiCalc/Generated/Grid.swift`
# (mosaic-emit-swiftui doesn't yet support the `Grid` built-in
# primitive — only the React emitter does). When the SwiftUI Grid
# emitter lands, this script gains a second `mosaic-compile
# --backend swiftui` line for Grid.
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

echo "Done. Generated:"
ls -la "$OUT_DIR"
echo
echo "To run the demo (Swift 5.9+ / Xcode 15+ required):"
echo "  cd $DEMO_DIR"
echo "  swift run"
