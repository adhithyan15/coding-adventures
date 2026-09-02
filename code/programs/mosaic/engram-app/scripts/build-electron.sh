#!/usr/bin/env bash
#
# Build the Engram desktop app from the Mosaic package.
#
# The sibling of build-web.sh, and deliberately shaped the same way: compile the
# engine to wasm, emit the app as a complete project, install the wasm runtime
# into it, and optionally build and package. Anything Engram-specific that the
# generic Mosaic emitter should not know about is applied here, after emission.
#
# ## Why Electron is the first native target
#
# Engram's engine is already wasm, and the Electron host loads that same engine
# rather than a platform-native library. So this needs no C++ toolchain, no
# Xcode, no JDK, no Flutter SDK -- only Node, which every runner already has.
# The Qt, SwiftUI, Compose, XAML and Flutter backends each need their own
# toolchain and are separate work.
#
# ## Why not build-all.ps1
#
# That script already knows how to assemble several native backends, but it is
# PowerShell, so the Linux CI runner cannot execute it. That is exactly why no
# lane could produce a web bundle before build-web.sh existed, and it is why
# this is a shell script rather than another PowerShell entry point.
#
# ## Packaging
#
# The emitted project has no electron-builder configuration -- and should not:
# application id, icons, and target formats are product decisions, not something
# a generic UI emitter can know. They are injected here.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"
RUST="$REPO/code/packages/rust"
WASM="$RUST/engram-wasm"

OUTPUT="$HERE/dist-electron"
RUN_BUILD=0
RUN_PACKAGE=0

usage() {
  cat <<'USAGE'
build-electron.sh — build the Engram desktop app from the Mosaic package

  --output DIR   Where to emit (default: <engram-app>/dist-electron)
  --build        Also run `npm install` and `npm run build`
  --package      Also produce a distributable with electron-builder
                 (implies --build; builds for the CURRENT platform only)
  -h, --help     Show this message
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output)  OUTPUT="$2"; shift 2 ;;
    --build)   RUN_BUILD=1; shift ;;
    --package) RUN_BUILD=1; RUN_PACKAGE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

echo "[1/4] Building the Engram engine to wasm..."
bash "$WASM/build-wasm.sh"

echo "[2/4] Emitting the Mosaic app as an Electron project..."
rm -rf "$OUTPUT"
( cd "$RUST" && cargo run -q -p mosaic-compile -- pkg "$HERE" \
    --backend electron --output "$OUTPUT" --emit-project )

APP="$OUTPUT/electron"

echo "[3/4] Installing the wasm runtime into the emitted project..."
mkdir -p "$APP/src" "$APP/public" "$APP/electron"
cp "$WASM/js/engram-mosaic-host-wasm.mjs" "$APP/src/engram-mosaic-host-wasm.mjs"
cp "$WASM/pkg/engram_engine.wasm"         "$APP/public/engram_engine.wasm"
# The Electron main process loads the engine too, from beside its own bundle
# rather than through the renderer's public/ directory.
cp "$WASM/js/engram-mosaic-host-wasm.mjs" "$APP/electron/engram-mosaic-host-wasm.mjs"
cp "$WASM/pkg/engram_engine.wasm"         "$APP/electron/engram_engine.wasm"

echo "[4/4] Adding packaging configuration..."
# electron-builder reads its config from package.json's "build" key. Injecting
# it here keeps product decisions -- appId, icons, target formats -- out of the
# generic emitter, which has no business knowing them.
#
# `files` is explicit rather than a wildcard: electron-builder's default sweeps
# the whole directory including node_modules and source, producing an installer
# several times larger than it needs to be, with the app's TypeScript sources
# inside it.
python3 - "$APP/package.json" <<'PY'
import json, sys

path = sys.argv[1]
with open(path) as handle:
    pkg = json.load(handle)

pkg["name"] = "engram"
pkg["productName"] = "Engram"
pkg["description"] = "Spaced repetition study app"
pkg.setdefault("version", "0.0.0")
pkg["author"] = "coding-adventures"
pkg["license"] = "MIT"

pkg.setdefault("devDependencies", {})["electron-builder"] = "^25.1.8"
pkg.setdefault("scripts", {})["package"] = "electron-builder --publish never"

pkg["build"] = {
    "appId": "dev.codingadventures.engram",
    "productName": "Engram",
    "directories": {"output": "release"},
    "files": [
        "dist/**/*",
        "dist-electron/**/*",
        "electron/engram_engine.wasm",
        "electron/engram-mosaic-host-wasm.mjs",
        "electron/host.js",
        "package.json",
    ],
    # One portable format per platform. Signing and notarisation need
    # credentials this build does not have, so the macOS target is a plain
    # zip rather than a dmg an unsigned installer would refuse to open.
    "linux": {"target": ["AppImage"], "category": "Education"},
    "mac": {"target": ["zip"], "category": "public.app-category.education"},
    "win": {"target": ["portable"]},
}

with open(path, "w") as handle:
    json.dump(pkg, handle, indent=2)
    handle.write("\n")
print(f"  configured electron-builder in {path}")
PY

if [[ "$RUN_BUILD" -eq 1 ]]; then
  echo "[+] Building..."
  ( cd "$APP" && npm install --no-audit --no-fund && npm run build )
  # The engine has to reach the shipped app, not merely the build directory.
  # Vite copies public/ into dist/, and a missing wasm there is a runtime
  # failure behind a successful build -- the same shape of bug the web lane
  # checks for, and the reason a stale committed artifact went unnoticed for
  # two months.
  cmp "$WASM/pkg/engram_engine.wasm" "$APP/dist/engram_engine.wasm"
  echo "  engine verified in dist/"
fi

if [[ "$RUN_PACKAGE" -eq 1 ]]; then
  echo "[+] Packaging with electron-builder..."
  ( cd "$APP" && npx --yes electron-builder --publish never )

  # Verify the engine actually reached the packaged app.
  #
  # electron-builder collects files by an explicit `files` list, and everything
  # ends up inside `app.asar` -- an archive, so the wasm is invisible to `ls`
  # and to `find`. A wrong entry in that list produces an installer that builds,
  # installs, and launches, and then cannot import a deck: the same failure the
  # web lane checks for, one layer further from view.
  #
  # Two copies are expected: dist/ for the renderer and electron/ for the main
  # process, which loads the engine from beside its own bundle.
  ASAR="$(find "$APP/release" -name app.asar -print -quit 2>/dev/null || true)"
  if [[ -z "$ASAR" ]]; then
    echo "error: no app.asar in the packaged output; cannot verify the engine" >&2
    exit 1
  fi
  ENGINES="$(npx --yes @electron/asar list "$ASAR" | grep -c 'engram_engine\.wasm' || true)"
  if [[ "$ENGINES" -lt 2 ]]; then
    echo "error: expected the engine in both dist/ and electron/, found $ENGINES copy/copies in $ASAR" >&2
    echo "       the app would launch and then fail to import a deck" >&2
    npx --yes @electron/asar list "$ASAR" | grep -v '^/node_modules' >&2
    exit 1
  fi
  echo "  engine verified inside app.asar ($ENGINES copies)"

  echo ""
  echo "Packaged: $APP/release"
  ls -la "$APP/release" 2>/dev/null | head -20 || true
elif [[ "$RUN_BUILD" -eq 1 ]]; then
  echo ""
  echo "Built: $APP  (run 'npm start' there, or re-run with --package)"
else
  echo ""
  echo "Ready. Run:  cd '$APP' && npm install && npm run build && npm start"
fi
