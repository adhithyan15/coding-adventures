#!/usr/bin/env bash
#
# verify-native.sh — prove the C ABI is callable from every native language the
# cross-backend VisiCalc demos use, computing identical results. Builds the
# shared library, then compiles + runs each language smoke against it:
#
#   C       → the path Qt/C++ uses
#   Swift   → the path SwiftUI uses
#   Dart    → the path Flutter uses (dart:ffi)
#   .NET    → the path XAML/WinUI uses (P/Invoke)
#   Kotlin  → the path Compose/Android uses (Java FFM, no hand-written JNI)
#
# Local verification (not a CI gate); CI runs `cargo test`. Each language is
# skipped (with a note) if its toolchain isn't installed.
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WS="$(cd "$DIR/.." && pwd)" # code/packages/rust
(cd "$WS" && cargo build -p spreadsheet-capi --release)
LIBDIR="$WS/target/release"
case "$(uname -s)" in
  Darwin) LIB="$LIBDIR/libspreadsheet_capi.dylib" ;;
  *)      LIB="$LIBDIR/libspreadsheet_capi.so" ;;
esac
export CAPI_LIB="$LIB"
TMP="$(mktemp -d)"
have() { command -v "$1" >/dev/null 2>&1; }

echo "── C ──"
cc -I "$DIR/include" "$DIR/test/smoke.c" "$LIB" -o "$TMP/c" && "$TMP/c"

echo "── Swift ──"
if have swiftc; then
  swiftc -import-objc-header "$DIR/include/spreadsheet.h" "$DIR/test/smoke.swift" -L "$LIBDIR" -lspreadsheet_capi -o "$TMP/swift" \
    && DYLD_LIBRARY_PATH="$LIBDIR" "$TMP/swift"
else echo "(skipped: swiftc not installed)"; fi

echo "── Dart ──"
if have dart; then dart run "$DIR/test/smoke.dart"; else echo "(skipped: dart not installed)"; fi

echo "── .NET ──"
if have dotnet; then dotnet run --project "$DIR/test/dotnet"; else echo "(skipped: dotnet not installed)"; fi

echo "── Kotlin ──"
if have kotlinc && have java; then
  kotlinc "$DIR/test/smoke.kt" -include-runtime -d "$TMP/kotlin.jar" 2>/dev/null \
    && java --enable-preview --enable-native-access=ALL-UNNAMED -jar "$TMP/kotlin.jar"
else echo "(skipped: kotlinc/java not installed)"; fi

echo
echo "All available native bindings verified."
