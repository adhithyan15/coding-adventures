#!/usr/bin/env bash
# make-app.sh — wrap the built executable in a minimal VisiCalc.app bundle so
# the demo is double-clickable (and a first-class macOS app, not a bare
# `swift run` process). Run after `bash scripts/build.sh`.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR"
swift build -c release
APP="$DIR/VisiCalc.app"
rm -rf "$APP"; mkdir -p "$APP/Contents/MacOS"
cp .build/release/visicalc "$APP/Contents/MacOS/VisiCalc"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>VisiCalc</string>
  <key>CFBundleDisplayName</key><string>VisiCalc</string>
  <key>CFBundleIdentifier</key><string>com.codingadventures.visicalc</string>
  <key>CFBundleVersion</key><string>1.0</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleExecutable</key><string>VisiCalc</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
echo "Built $APP — run with: open VisiCalc.app"
