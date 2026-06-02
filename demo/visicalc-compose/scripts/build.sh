#!/usr/bin/env bash
# build.sh — regenerate the generated FormulaBar Composable from
# the Mosaic pipeline (mosaic-compile --backend compose).
#
# Per the VisiCalc cross-backend demo plan, this script runs the
# Compose emitter (code/packages/rust/mosaic-emit-compose/) against
# the canonical FormulaBar.{mil,desktop.mll,dark.msl} sources and
# drops the resulting Kotlin file into src/main/kotlin/generated/.
#
# Today we only compile FormulaBar through the pipeline.  The Grid
# Kotlin composable in src/main/kotlin/Grid.kt is still hand-written
# (the mosaic-emit-compose pipeline doesn't yet support the `Grid`
# built-in primitive — only the React + SwiftUI emitters do).  When
# the Compose Grid emitter lands (grid-emit-compose cycle), this
# script gains a second mosaic-compile invocation for Grid.
#
# Usage:
#   cd demo/visicalc-compose
#   bash scripts/build.sh
#
# Then to run the demo:
#   gradle --no-daemon run

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
OUT_DIR="$DEMO_DIR/src/main/kotlin/generated"
mkdir -p "$OUT_DIR"

echo "Compiling FormulaBar (Compose / Kotlin)..."
"$MOSAIC_COMPILE" --backend compose \
  --interface "$SRC/FormulaBar.mil" \
  --layout    "$SRC"   --variant desktop \
  --style     "$SRC/FormulaBar.dark.msl" \
  -o "$OUT_DIR/FormulaBar.kt"

# The Compose backend does not emit a `package` declaration (it
# doesn't know which package the host will live in).  Insert one
# matching the host's `Main.kt` so the generated file participates
# in the same Kotlin compilation unit.
TMP="$OUT_DIR/FormulaBar.kt.tmp"
{
  echo "// THIS FILE IS GENERATED.  Edit FormulaBar.{mil,desktop.mll,dark.msl}"
  echo "// and re-run scripts/build.sh to regenerate."
  echo
  printf 'package generated\n\n'
  cat "$OUT_DIR/FormulaBar.kt"
} > "$TMP"
mv "$TMP" "$OUT_DIR/FormulaBar.kt"

# TODO(grid-emit-compose): wire the Grid emitter here once the
# `Grid` built-in primitive is supported by mosaic-emit-compose.
# Until then, Grid.kt under src/main/kotlin/ stays hand-written.

echo "Done. Generated:"
ls -la "$OUT_DIR"
echo
echo "To run the demo:"
echo "  cd $DEMO_DIR"
echo "  gradle --no-daemon run"
