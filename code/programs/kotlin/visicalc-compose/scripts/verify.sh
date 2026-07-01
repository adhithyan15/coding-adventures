#!/usr/bin/env bash
#
# verify.sh — headless proof that the Compose VisiCalc demo does REAL formula
# work on the shared Rust engine, with no Compose/Gradle in the loop.
#
# It compiles the engine glue (Engine.kt) plus the smoke harness
# (test/EngineSmoke.kt) — both plain Kotlin, no Compose — with kotlinc, then
# runs them on the JVM with the Java FFM API enabled, loading the vendored
# engine library. This is the Compose equivalent of the SwiftUI demo's
# `swift test`, the Qt demo's tst_model, and the Flutter demo's `flutter test`.
#
# Requires: JDK 21+ (the FFM API is preview in 21, stable in 22) and kotlinc.
# Run scripts/build.sh first so native/libspreadsheet_capi.* exists.
#
# Usage:
#   cd demo/visicalc-compose && bash scripts/verify.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEMO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$DEMO_DIR"

case "$(uname -s)" in
  Darwin) LIB="native/libspreadsheet_capi.dylib" ;;
  *)      LIB="native/libspreadsheet_capi.so" ;;
esac
if [ ! -f "$LIB" ]; then
  echo "Engine library $LIB not found — run scripts/build.sh first." >&2
  exit 1
fi
export CAPI_LIB="$DEMO_DIR/$LIB"

OUT="$(mktemp -d)/smoke.jar"
echo "Compiling Engine.kt + test/EngineSmoke.kt..."
kotlinc src/main/kotlin/Engine.kt test/EngineSmoke.kt -include-runtime -d "$OUT"

echo "Running the FFM smoke (JDK 21 preview FFM)..."
java --enable-preview --enable-native-access=ALL-UNNAMED -cp "$OUT" EngineSmokeKt
