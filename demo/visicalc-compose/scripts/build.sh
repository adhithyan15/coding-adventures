#!/usr/bin/env bash
# build.sh — regenerate the generated FormulaBar Composable from
# the Mosaic pipeline (mosaic-compile --backend compose).
#
# Per the VisiCalc cross-backend demo plan, this script runs the
# Compose emitter (code/packages/rust/mosaic-emit-compose/) against
# the canonical FormulaBar.{mil,desktop.mll,dark.msl} AND
# Grid.{mil,desktop.mll,dark.msl} sources and drops the resulting
# Kotlin files into src/main/kotlin/generated/.
#
# BOTH the FormulaBar and the Grid are now fully generated through the
# pipeline — there is no hand-written Grid composable anymore.  Grid is
# emitted from the shared `mosaic-pkg-grid::Grid` package composition
# (resolved via --package-search-path), and the Compose emitter inlines
# the `.msl` part-styles (width / height / background / border / padding
# / alignment / per-state highlight) onto each cell's `Modifier`, so the
# generated Grid renders as a real styled spreadsheet.
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

# Always (re)build mosaic-compile so emitter fixes reach the generated code.
# cargo is incremental, so this is ~free when nothing changed. The old
# `if [ ! -x "$MOSAIC_COMPILE" ]` guard skipped the build whenever ANY stale
# binary already existed, silently serving pre-fix output (this is what hid the
# Compose formula-bar textStyle fix — the demo kept using an old mosaic-compile).
echo "Building mosaic-compile..."
(cd "$REPO_ROOT/code/packages/rust" && cargo build -p mosaic-compile)

SRC="$REPO_ROOT/code/programs/mosaic/visicalc"
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

echo "Compiling Grid (Compose / Kotlin)..."
# UI34 — Grid is now generated from the shared visicalc/mosaic/
# Grid.{mil,desktop.mll,dark.msl} triple, same source the other six
# VC2-* demos consume.  Grid.desktop.mll is a UI34
# `pkg::mosaic-pkg-grid::Grid` one-liner; we pass --package-search-path
# so the resolver substitutes the package's full composition before
# the Compose emitter runs.
"$MOSAIC_COMPILE" --backend compose \
  --interface           "$SRC/Grid.mil" \
  --layout              "$SRC"   --variant desktop \
  --style               "$SRC/Grid.dark.msl" \
  --package-search-path "$REPO_ROOT/code/packages" \
  -o "$OUT_DIR/Grid.kt"

# Match the FormulaBar package-prefix shim — mosaic-emit-compose
# emits no `package` declaration so the host inserts one matching
# the generated/ directory layout.
TMP="$OUT_DIR/Grid.kt.tmp"
{
  echo "// THIS FILE IS GENERATED.  Edit Grid.{mil,desktop.mll,dark.msl}"
  echo "// and re-run scripts/build.sh to regenerate."
  echo
  printf 'package generated\n\n'
  cat "$OUT_DIR/Grid.kt"
} > "$TMP"
mv "$TMP" "$OUT_DIR/Grid.kt"

echo "Building the spreadsheet engine (Rust → dynamic lib) and vendoring it..."
# The C ABI dynamic library the Kotlin host loads through the Java FFM API (see
# Engine.kt), built from the spreadsheet-capi crate (cdylib) and copied into
# native/ (git-ignored). Java FFM's SymbolLookup.libraryLookup needs a
# .dylib/.so/.dll, the same dynamic packaging the Flutter demo uses.
RUST="$REPO_ROOT/code/packages/rust"
(cd "$RUST" && cargo build -p spreadsheet-capi --release)
mkdir -p "$DEMO_DIR/native"
case "$(uname -s)" in
  Darwin) LIB="libspreadsheet_capi.dylib" ;;
  *)      LIB="libspreadsheet_capi.so" ;;
esac
cp "$RUST/target/release/$LIB" "$DEMO_DIR/native/$LIB"

# Also stage the dylib into the Compose appResources layout so a packaged app
# (gradle createDistributable / packageDmg) is self-contained — it bundles the
# engine and finds it via compose.application.resources.dir (no CAPI_LIB needed).
# Compose wants a per-target subdir; pick it from the OS + CPU arch.
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)  TARGET_DIR="macos-arm64" ;;
  Darwin-x86_64) TARGET_DIR="macos-x64" ;;
  Linux-x86_64)  TARGET_DIR="linux-x64" ;;
  Linux-aarch64) TARGET_DIR="linux-arm64" ;;
  *)             TARGET_DIR="" ;;
esac
if [ -n "$TARGET_DIR" ]; then
  mkdir -p "$DEMO_DIR/appResources/$TARGET_DIR"
  cp "$RUST/target/release/$LIB" "$DEMO_DIR/appResources/$TARGET_DIR/$LIB"
fi

echo "Done. Generated composables + vendored engine:"
ls -la "$OUT_DIR"
ls -la "$DEMO_DIR/native"
echo
echo "Verify (headless: grid is engine-computed + recomputes; needs JDK 21+):"
echo "  cd $DEMO_DIR && bash scripts/verify.sh"
echo "Run the desktop app (needs JDK 21+ for the FFM API):"
echo "  cd $DEMO_DIR && gradle --no-daemon run"
