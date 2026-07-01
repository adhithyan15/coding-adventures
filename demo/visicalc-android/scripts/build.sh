#!/usr/bin/env bash
# build.sh — regenerate FormulaBar + Grid composables from the
# shared `demo/visicalc/mosaic/*` sources via mosaic-compile.
#
# UI34 PR — visicalc-android joins the rest of the cross-backend
# matrix.  Both composables are now generated from the same
# {FormulaBar,Grid}.{mil,desktop.mll,dark.msl} triple every other
# VC2-* demo consumes.  Grid.desktop.mll is a UI34
# `pkg::mosaic-pkg-grid::Grid` one-liner, so the package resolver
# substitutes the canonical Grid + Cell composition before the
# Compose emitter runs.
#
# Output package: `com.example.visicalc` — matches the host's
# Activity package so the generated composables participate in
# the same Kotlin compilation unit without ceremony imports.
#
# Usage:
#   cd demo/visicalc-android
#   bash scripts/build.sh
#
# Then to run the demo:
#   ./gradlew installDebug   # (requires Android SDK + emulator)

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

SRC="$REPO_ROOT/demo/visicalc/mosaic"
OUT_DIR="$DEMO_DIR/app/src/main/java/com/example/visicalc"
mkdir -p "$OUT_DIR"

# Helper — wraps `mosaic-compile` and prepends the `package` line
# the Compose emitter doesn't know to emit.
generate() {
  local name="$1"
  local layout="$2"

  echo "Compiling $name (Compose / Android)..."
  "$MOSAIC_COMPILE" --backend compose \
    --interface           "$SRC/${name}.mil" \
    --layout              "$layout" \
    --style               "$SRC/${name}.dark.msl" \
    --package-search-path "$REPO_ROOT/code/packages" \
    -o "$OUT_DIR/${name}.kt"

  local tmp="$OUT_DIR/${name}.kt.tmp"
  {
    echo "// THIS FILE IS GENERATED.  Edit ${name}.{mil,desktop.mll,dark.msl}"
    echo "// and re-run scripts/build.sh to regenerate."
    echo
    printf 'package com.example.visicalc\n\n'
    cat "$OUT_DIR/${name}.kt"
  } > "$tmp"
  mv "$tmp" "$OUT_DIR/${name}.kt"
}

generate "FormulaBar" "$SRC/FormulaBar.desktop.mll"
generate "Grid"       "$SRC/Grid.desktop.mll"

echo "Done. Generated:"
ls -la "$OUT_DIR" | grep -E '\.kt$'
echo
echo "To run the demo (Android SDK + emulator required):"
echo "  cd $DEMO_DIR"
echo "  ./gradlew installDebug"
