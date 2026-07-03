#!/usr/bin/env bash
# build.sh — regenerate FormulaBar + Grid composables from the
# shared `code/programs/mosaic/visicalc/*` sources via mosaic-compile.
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
#   cd code/programs/kotlin/visicalc-android
#   bash scripts/build.sh
#
# Then to run the demo:
#   ./gradlew installDebug   # (requires Android SDK + emulator)

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
    --package-search-path "$REPO_ROOT/code/packages:$REPO_ROOT/code/packages/mosaic" \
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

# UI30 touch variant: FormulaBar.touch.mll stacks the address label ABOVE a
# full-width input (Column) instead of the desktop Row. Same FormulaBar.mil
# interface, so MainActivity can swap FormulaBar <-> FormulaBarTouch at runtime
# against the identical dispatch contract. The `generate` helper can't be reused
# as-is because the Compose emitter names the composable after the .mil component
# (FormulaBar) and also emits the shared `sealed class FormulaBarEvent`; to let
# both composables coexist in package com.example.visicalc we (1) strip the
# DUPLICATE FormulaBarEvent from the touch output (it reuses the one in
# FormulaBar.kt) and (2) rename the composable to FormulaBarTouch. One event
# type, two layouts — mirrors the visicalc-compose demo.
echo "Compiling FormulaBar (Compose / Android, touch)..."
"$MOSAIC_COMPILE" --backend compose \
  --interface           "$SRC/FormulaBar.mil" \
  --layout              "$SRC/FormulaBar.touch.mll" \
  --style               "$SRC/FormulaBar.dark.msl" \
  --package-search-path "$REPO_ROOT/code/packages:$REPO_ROOT/code/packages/mosaic" \
  -o "$OUT_DIR/FormulaBarTouch.kt.raw"
{
  echo "// THIS FILE IS GENERATED.  Edit FormulaBar.{mil,touch.mll,dark.msl}"
  echo "// and re-run scripts/build.sh to regenerate."
  echo
  printf 'package com.example.visicalc\n\n'
  # Drop the sealed-class FormulaBarEvent block (lives in FormulaBar.kt, same
  # package). Its closing brace is the first column-0 `}`; nested data-class
  # braces are indented, so `^}` matches only the outer close. Then rename the fun.
  awk '/^sealed class FormulaBarEvent \{/{skip=1}
       skip && /^\}/{skip=0; next}
       skip{next}
       {print}' "$OUT_DIR/FormulaBarTouch.kt.raw" \
    | sed 's/fun FormulaBar(/fun FormulaBarTouch(/'
} > "$OUT_DIR/FormulaBarTouch.kt"
rm -f "$OUT_DIR/FormulaBarTouch.kt.raw"


# ── Engine (Rust → per-ABI .so via the NDK) ──────────────────────────────────
# Android's ART runtime has no JVM Foreign Function & Memory API (the path the
# Compose *Desktop* demo uses), so the shared engine is reached over JNI: this
# cross-compiles the `spreadsheet-android-jni` crate (a zero-dep jni-bridge shim
# over spreadsheet-core) to a `.so` per ABI and stages it into jniLibs/, where
# `System.loadLibrary("spreadsheet_android_jni")` (Engine.kt) loads it.
NDK_ROOT="${ANDROID_NDK_HOME:-${ANDROID_HOME:-$HOME/Library/Android/sdk}/ndk/28.2.13676358}"
TC="$(ls -d "$NDK_ROOT/toolchains/llvm/prebuilt/"*/ 2>/dev/null | head -1)"
MIN_API=26
if [ -n "$TC" ]; then
  for pair in "arm64-v8a:aarch64-linux-android" "x86_64:x86_64-linux-android"; do
    abi="${pair%%:*}"; target="${pair##*:}"
    LINKER="$TC/bin/${target}${MIN_API}-clang"
    [ -x "$LINKER" ] || continue
    upper="$(printf '%s' "$target" | tr 'a-z-' 'A-Z_')"
    under="$(printf '%s' "$target" | tr '-' '_')"
    echo "Cross-compiling engine for $abi ($target)..."
    ( cd "$REPO_ROOT/code/packages/rust"       && "$(command -v rustup)" target add "$target" >/dev/null 2>&1 || true
      env "CARGO_TARGET_${upper}_LINKER=$LINKER" "CC_${under}=$LINKER" "AR_${under}=$TC/bin/llvm-ar"         cargo build -q -p spreadsheet-android-jni --target "$target" --release )
    so="$REPO_ROOT/code/packages/rust/target/$target/release/libspreadsheet_android_jni.so"
    if [ -f "$so" ]; then
      mkdir -p "$DEMO_DIR/app/src/main/jniLibs/$abi"
      cp "$so" "$DEMO_DIR/app/src/main/jniLibs/$abi/"
      echo "  staged -> app/src/main/jniLibs/$abi/libspreadsheet_android_jni.so"
    fi
  done
else
  echo "WARN: Android NDK not found at $NDK_ROOT — skipping engine cross-compile."
fi

echo "Done. Generated:"
ls -la "$OUT_DIR" | grep -E '\.kt$'
echo
echo "To run the demo (Android SDK + emulator required):"
echo "  cd $DEMO_DIR"
echo "  ./gradlew installDebug"
