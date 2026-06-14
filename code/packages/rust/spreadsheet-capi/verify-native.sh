#!/usr/bin/env bash
#
# verify-native.sh — prove the C ABI is callable from real native languages
# (beyond C): Swift (the SwiftUI path) and Dart/FFI (the Flutter path). Builds
# the shared library, then compiles + runs each language smoke against it.
# This is local verification (not a CI gate); CI runs `cargo test`.
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WS="$(cd "$DIR/.." && pwd)" # code/packages/rust
(cd "$WS" && cargo build -p spreadsheet-capi --release)
LIBDIR="$WS/target/release"
case "$(uname -s)" in
  Darwin) LIB="$LIBDIR/libspreadsheet_capi.dylib" ;;
  *)      LIB="$LIBDIR/libspreadsheet_capi.so" ;;
esac
TMP="$(mktemp -d)"

echo "── C ──";     bash "$DIR/build-capi.sh" >/dev/null && echo "C: see build-capi.sh"
echo "── Swift ──"; swiftc -import-objc-header "$DIR/include/spreadsheet.h" "$DIR/test/smoke.swift" -L "$LIBDIR" -lspreadsheet_capi -o "$TMP/swift" && DYLD_LIBRARY_PATH="$LIBDIR" "$TMP/swift"
echo "── Dart ──";  CAPI_LIB="$LIB" dart run "$DIR/test/smoke.dart"
echo "All native bindings verified."
