#!/usr/bin/env bash
# run-ios.sh — build the VisiCalc SwiftUI app for the iOS Simulator (linking the
# iOS slice of the Rust engine), wrap it in a .app, install, and launch it.
# Proves the SAME SwiftUI code + Rust engine run on iOS. Requires Xcode + an
# installed iOS Simulator runtime.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR"

# Ensure the engine's iOS-sim slice is vendored.
bash scripts/build.sh >/dev/null

SIM="${1:-iPhone 17}"
SIM_ID="$(xcrun simctl list devices available | grep "$SIM (" | head -1 | grep -oE '[0-9A-F-]{36}')"
[ -n "$SIM_ID" ] || { echo "No available simulator matching '$SIM'"; exit 1; }
xcrun simctl boot "$SIM_ID" 2>/dev/null || true
open -a Simulator

DD="$(mktemp -d)/dd"
echo "Building for iOS Simulator…"
xcodebuild -scheme VisiCalc -destination "platform=iOS Simulator,id=$SIM_ID" -derivedDataPath "$DD" build >/dev/null

# Wrap the executable in a minimal .app (SwiftPM executables aren't .app bundles).
APP="$(mktemp -d)/VisiCalc.app"
mkdir -p "$APP"
cp "$DD/Build/Products/Debug-iphonesimulator/visicalc" "$APP/visicalc"
cat > "$APP/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>visicalc</string>
  <key>CFBundleIdentifier</key><string>com.codingadventures.visicalc</string>
  <key>CFBundleName</key><string>VisiCalc</string>
  <key>CFBundleDisplayName</key><string>VisiCalc</string>
  <key>CFBundleVersion</key><string>1.0</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSRequiresIPhoneOS</key><true/>
  <key>MinimumOSVersion</key><string>17.0</string>
  <key>UILaunchScreen</key><dict/>
  <key>UIDeviceFamily</key><array><integer>1</integer></array>
</dict></plist>
PLIST
codesign --force --sign - "$APP" >/dev/null 2>&1

xcrun simctl install "$SIM_ID" "$APP"
xcrun simctl launch "$SIM_ID" com.codingadventures.visicalc
echo "Launched VisiCalc on the iOS Simulator ($SIM)."
