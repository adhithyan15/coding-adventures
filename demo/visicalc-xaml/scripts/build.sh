#!/usr/bin/env bash
#
# build.sh — regenerate the FormulaBar XAML triple from the Mosaic
# pipeline and drop it into Generated/.
#
# Per the VisiCalc cross-backend demo plan (Phase 2 / VC2-xaml):
# this is the WinUI 3 / XAML cross-backend demo. The output is a
# triple: FormulaBar.xaml + FormulaBar.xaml.cs + FormulaBar.Event.cs.
# MainWindow.xaml hand-mounts the FormulaBar and a hand-written
# Grid section.
#
# Why not --emit-project? The full --emit-project mode generates a
# stand-alone single-component WinUI 3 shell, but we need a multi-
# component layout (FormulaBar + Grid). Plan item [M] (Phase 3 —
# multi-component artifact-builder shells) will extend the
# generator to handle this case; until then, the host shell here
# is hand-authored.
#
# Usage:
#   cd demo/visicalc-xaml
#   bash scripts/build.sh
#
# Then to actually run the app (WinUI 3 SDK / .NET 9 required;
# Windows-only):
#   winget install Microsoft.WindowsAppRuntime.1.7
#   dotnet build
#   dotnet run

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
OUT_DIR="$DEMO_DIR/Generated"
mkdir -p "$OUT_DIR"

echo "Compiling FormulaBar (XAML)..."
"$MOSAIC_COMPILE" --backend xaml \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC/FormulaBar.desktop.mll" \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar"

# TODO(VC2-xaml-grid): Grid pipeline emit is a follow-up. The
# `Grid` built-in primitive is only known by the React emitter;
# the XAML emitter would need its own table-builder lowering. For
# now the grid lives in MainWindow.xaml as hand-written
# StackPanel-of-Rows markup (visible alongside the FormulaBar).

echo "Done. Generated:"
ls -la "$OUT_DIR"
echo
echo "To run the demo (Windows + .NET 9 + WindowsAppRuntime 1.7):"
echo "  cd $DEMO_DIR"
echo "  dotnet build && dotnet run"
