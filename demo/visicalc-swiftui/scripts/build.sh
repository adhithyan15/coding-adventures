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

# Always (re)build mosaic-compile so emitter fixes reach the generated code.
# cargo is incremental, so this is ~free when nothing changed. The old
# `if [ ! -x "$MOSAIC_COMPILE" ]` guard skipped the build whenever ANY stale
# binary already existed, silently serving pre-fix output (this is what hid the
# Compose formula-bar textStyle fix — the demo kept using an old mosaic-compile).
echo "Building mosaic-compile..."
(cd "$REPO_ROOT/code/packages/rust" && cargo build -p mosaic-compile)

SRC="$REPO_ROOT/code/programs/mosaic/visicalc"
OUT_DIR="$DEMO_DIR/Sources/VisiCalc/Generated"
mkdir -p "$OUT_DIR"

# wrap_guard FILE OS — fence a generated Swift file in `#if os(OS) … #endif`
# so only the right platform's variant compiles. Both FormulaBar variants
# declare the same `FormulaBarView` type (they come from the same .mil), so the
# guards are what keep them from colliding when both files are in the target.
wrap_guard() {
  local f="$1" os="$2" tmp
  tmp="$(mktemp)"
  { printf '#if os(%s)\n' "$os"; cat "$f"; printf '#endif\n'; } > "$tmp"
  mv "$tmp" "$f"
}

# FormulaBar has a real per-platform LAYOUT: the desktop layout lays the address
# label + field out horizontally (HStack); the touch layout stacks them
# vertically (VStack) so the field gets full width on a phone. We generate BOTH
# from the shared FormulaBar.mil — desktop from FormulaBar.desktop.mll, touch
# from FormulaBar.touch.mll — and platform-guard each. ContentView just uses
# `FormulaBarView`; the active platform picks the matching layout. (Grid.touch
# is identical to Grid.desktop, so the Grid below needs no touch variant.)
echo "Compiling FormulaBar (SwiftUI, desktop → macOS)..."
"$MOSAIC_COMPILE" --backend swiftui \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC/FormulaBar.desktop.mll" \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.swift"
wrap_guard "$OUT_DIR/FormulaBar.swift" macOS

echo "Compiling FormulaBar (SwiftUI, touch → iOS)..."
"$MOSAIC_COMPILE" --backend swiftui \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC/FormulaBar.touch.mll" \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.touch.swift"
wrap_guard "$OUT_DIR/FormulaBar.touch.swift" iOS

echo "Compiling Grid (SwiftUI)..."
"$MOSAIC_COMPILE" --backend swiftui \
  --interface           "$SRC/Grid.mil" \
  --layout              "$SRC/Grid.desktop.mll" \
  --style               "$SRC/Grid.dark.msl" \
  --package-search-path "$REPO_ROOT/code/packages" \
  -o "$OUT_DIR/Grid.swift"

echo "Building the spreadsheet engine (Rust → static lib) and vendoring it..."
# The C ABI static library the app links (see Package.swift), built from the
# spreadsheet-capi crate per platform and copied into Vendor/ (git-ignored):
#   Vendor/macos   — the host target, for `swift run` / `swift test`.
#   Vendor/ios-sim — aarch64-apple-ios-sim, for the iOS Simulator build.
RUST="$REPO_ROOT/code/packages/rust"
mkdir -p "$DEMO_DIR/Vendor/macos" "$DEMO_DIR/Vendor/ios-sim"

(cd "$RUST" && cargo build -p spreadsheet-capi --release)
cp "$RUST/target/release/libspreadsheet_capi.a" "$DEMO_DIR/Vendor/macos/"

# iOS Simulator slice (skipped if the target isn't installed — macOS still works).
if rustup target list --installed 2>/dev/null | grep -q aarch64-apple-ios-sim; then
  (cd "$RUST" && cargo build -p spreadsheet-capi --release --target aarch64-apple-ios-sim)
  cp "$RUST/target/aarch64-apple-ios-sim/release/libspreadsheet_capi.a" "$DEMO_DIR/Vendor/ios-sim/"
else
  echo "  (iOS slice skipped — run 'rustup target add aarch64-apple-ios-sim' for iOS)"
fi

# Refresh the C header the CSpreadsheetEngine module exposes.
cp "$RUST/spreadsheet-capi/include/spreadsheet.h" \
   "$DEMO_DIR/Sources/CSpreadsheetEngine/include/spreadsheet.h"

echo "Done. Generated views + vendored engine."
echo
echo "Run (Swift 5.9+ / Xcode 15+ required):"
echo "  swift test                 # headless: grid is engine-computed + recomputes"
echo "  swift run                  # launch the macOS SwiftUI app"
echo "  bash scripts/run-ios.sh    # build + launch on the iOS Simulator"
