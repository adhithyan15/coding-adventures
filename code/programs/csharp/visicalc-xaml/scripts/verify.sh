#!/usr/bin/env bash
#
# verify.sh — headless proof that the XAML VisiCalc demo does REAL formula work
# on the shared Rust engine, with no WinUI in the loop (WinUI is Windows-only).
#
# It runs the cross-platform console harness in test/, which compiles the
# WinUI-free Engine.cs together with Program.cs and P/Invokes the vendored engine
# library — so the .NET ↔ P/Invoke ↔ C ABI ↔ Rust path is verifiable on macOS /
# Linux / Windows alike. This is the .NET equivalent of the SwiftUI demo's
# `swift test`, the Qt demo's tst_model, and the Compose demo's verify.sh.
#
# Requires: .NET 9 SDK. Run scripts/build.sh first so native/libspreadsheet_capi.*
# exists.
#
# Usage:
#   cd code/programs/csharp/visicalc-xaml && bash scripts/verify.sh

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

echo "Running the .NET P/Invoke smoke (test/EngineSmoke.csproj)..."
dotnet run --project test/EngineSmoke.csproj -c Release
