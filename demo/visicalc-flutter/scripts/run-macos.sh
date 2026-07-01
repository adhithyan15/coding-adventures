#!/usr/bin/env bash
# run-macos.sh — build the VisiCalc Flutter app for macOS and launch it as a
# self-contained .app bundle.
#
# `flutter run -d macos` works because its working directory is the project
# (so `native/libspreadsheet_capi.dylib` is found relative to CWD). But the
# BUILT bundle, launched from Finder / `open`, has CWD `/` and cannot find the
# vendored engine — it would show Flutter's red "Failed to load dynamic library"
# page. This script makes the bundle self-contained: it copies the engine dylib
# into the app's Contents/Frameworks, which Engine._resolveLibraryPath checks
# first (relative to the executable), so the app loads the engine no matter where
# it is launched from.
#
# Usage:
#   cd demo/visicalc-flutter
#   bash scripts/run-macos.sh            # debug build
#   bash scripts/run-macos.sh --release  # release build
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR"

MODE="--debug"
BUILD_DIR="Debug"
if [ "${1:-}" = "--release" ]; then MODE="--release"; BUILD_DIR="Release"; fi

# Regenerate the mosaic widgets + (re)vendor the freshly built engine into native/.
bash scripts/build.sh >/dev/null

echo "Building the macOS app ($MODE)…"
flutter build macos "$MODE"

APP="build/macos/Build/Products/$BUILD_DIR/visicalc_flutter.app"
[ -d "$APP" ] || { echo "Build did not produce $APP"; exit 1; }

# Make the bundle self-contained: drop the engine dylib into Contents/Frameworks,
# where Engine._resolveLibraryPath looks (relative to the executable) before it
# falls back to the vendored source-tree native/ dir.
mkdir -p "$APP/Contents/Frameworks"
cp "native/libspreadsheet_capi.dylib" "$APP/Contents/Frameworks/"
echo "Bundled engine → $APP/Contents/Frameworks/libspreadsheet_capi.dylib"

echo "Launching…"
open "$APP"
echo "Launched $APP (self-contained; loads the engine from its own Frameworks)."
