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
LOCK="$HERE/npm/electron/package-lock.json"
# shellcheck source=SCRIPTDIR/npm-lock.sh
source "$HERE/scripts/npm-lock.sh"

OUTPUT="$HERE/dist-electron"
RUN_BUILD=0
RUN_PACKAGE=0
UPDATE_LOCK=0

usage() {
  cat <<'USAGE'
build-electron.sh — build the Engram desktop app from the Mosaic package

  --output DIR   Where to emit (default: <engram-app>/dist-electron)
  --build        Also install from npm/electron/package-lock.json (`npm ci`)
                 and run `npm run build`
  --package      Also produce a distributable with electron-builder
                 (implies --build; builds for the CURRENT platform only)
  --update-lock  Only emit the project and regenerate
                 npm/electron/package-lock.json from its package.json
                 (skips the wasm build; installs nothing)
  -h, --help     Show this message
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output)  OUTPUT="$2"; shift 2 ;;
    --build)   RUN_BUILD=1; shift ;;
    --package) RUN_BUILD=1; RUN_PACKAGE=1; shift ;;
    --update-lock) UPDATE_LOCK=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ "$UPDATE_LOCK" -eq 1 && "$RUN_BUILD" -eq 1 ]]; then
  echo "--update-lock cannot be combined with --build or --package" >&2
  exit 2
fi

# The lockfile depends only on package.json, so --update-lock skips the engine.
if [[ "$UPDATE_LOCK" -eq 0 ]]; then
  echo "[1/4] Building the Engram engine to wasm..."
  bash "$WASM/build-wasm.sh"
fi

echo "[2/4] Emitting the Mosaic app as an Electron project..."
rm -rf "$OUTPUT"
( cd "$RUST" && cargo run -q -p mosaic-compile -- pkg "$HERE" \
    --backend electron --output "$OUTPUT" --emit-project )

APP="$OUTPUT/electron"

if [[ "$UPDATE_LOCK" -eq 0 ]]; then
  echo "[3/4] Installing the wasm runtime into the emitted project..."
  mkdir -p "$APP/src" "$APP/public" "$APP/electron"
  cp "$WASM/js/engram-mosaic-host-wasm.mjs" "$APP/src/engram-mosaic-host-wasm.mjs"
  cp "$WASM/pkg/engram_engine.wasm"         "$APP/public/engram_engine.wasm"
  # The Electron main process loads the engine too, from beside its own bundle
  # rather than through the renderer's public/ directory.
  cp "$WASM/js/engram-mosaic-host-wasm.mjs" "$APP/electron/engram-mosaic-host-wasm.mjs"
  cp "$WASM/pkg/engram_engine.wasm"         "$APP/electron/engram_engine.wasm"
fi

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

# The packaging tools are devDependencies at exact versions, so they come from
# the committed lockfile like everything else. They used to be fetched by
# `npx --yes` at whatever version the registry served that day -- unpinned code
# that builds the installer and opens it again to check it.
#
# Keep each on one line of the form dev["name"] = "version": tests/npm_lockfiles.rs
# reads them from this file to check the lock covers them.
dev = pkg.setdefault("devDependencies", {})
dev["electron-builder"] = "25.1.8"
dev["@electron/asar"] = "4.3.0"
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
    # Package the Electron runtime the build step below verified, instead of
    # letting electron-builder download its own copy (see the --package step
    # below).
    "electronDist": "node_modules/electron/dist",
}

with open(path, "w") as handle:
    json.dump(pkg, handle, indent=2)
    handle.write("\n")
print(f"  configured electron-builder in {path}")
PY

if [[ "$UPDATE_LOCK" -eq 1 ]]; then
  update_lock "$APP" "$LOCK"
  exit 0
fi

if [[ "$RUN_BUILD" -eq 1 ]]; then
  echo "[+] Installing from the committed lockfile..."
  install_from_lock "$APP" "$LOCK"
  echo "[+] Building..."
  ( cd "$APP" && npm run build )
  # The engine has to reach the shipped app, not merely the build directory.
  # Vite copies public/ into dist/, and a missing wasm there is a runtime
  # failure behind a successful build -- the same shape of bug the web lane
  # checks for, and the reason a stale committed artifact went unnoticed for
  # two months.
  cmp "$WASM/pkg/engram_engine.wasm" "$APP/dist/engram_engine.wasm"
  echo "  engine verified in dist/"
fi

if [[ "$RUN_PACKAGE" -eq 1 ]]; then
  # The Electron runtime is the largest piece of code in the installer, and the
  # lockfile does not cover it: the electron npm package is only a launcher
  # that fetches the binary separately. Left alone, electron-builder downloads
  # that zip itself and trusts it as served.
  #
  # Electron's own install.js fetches the same zip but checks it against
  # checksums.json, which ships INSIDE the lockfile-pinned electron package --
  # so the binary is pinned by the lock transitively:
  #
  #   package-lock.json --sha512--> electron-42.5.0.tgz --contains--> checksums.json
  #                                                     --sha256--> electron-v42.5.0-<os>-<arch>.zip
  #
  # `electronDist` above then points electron-builder at that verified copy.
  #
  # install.js is started through a small Node wrapper that first deletes the
  # environment variables that would change what it does:
  #
  #   electron_use_remote_checksums      fetch SHASUMS256.txt from the network
  #   npm_config_electron_use_remote_... instead of using the pinned file
  #   ELECTRON_INSTALL_PLATFORM / _ARCH  fetch another platform's runtime,
  #   npm_config_platform / _arch        which would package a broken app
  #
  # The deletion happens inside Node, case-insensitively, because Windows
  # environment names are case-insensitive: `env -u` would remove only the one
  # spelling it is given.
  #
  # A dropped connection mid-download is retried; install.js skips the fetch
  # once a verified runtime is in place. A checksum mismatch fails every
  # attempt the same way, so retrying never lets a bad zip through.
  echo "[+] Fetching the Electron runtime, verified against the pinned checksums..."
  for attempt in 1 2 3; do
    if ( cd "$APP" && node -e '
      const unsafe = /^(npm_config_)?(electron_use_remote_checksums|platform|arch)$|^electron_install_(platform|arch)$/i;
      for (const name of Object.keys(process.env)) if (unsafe.test(name)) delete process.env[name];
      require("./node_modules/electron/install.js");' ); then
      break
    fi
    if [[ "$attempt" -eq 3 ]]; then
      echo "error: could not fetch a verified Electron runtime" >&2
      exit 1
    fi
    echo "  attempt $attempt failed; retrying" >&2
  done
  # The runtime in dist/ must be the version the lock pinned, fully extracted.
  if ! ( cd "$APP" && node -e '
      const fs = require("fs");
      const want = require("./node_modules/electron/package.json").version;
      const have = fs.readFileSync("node_modules/electron/dist/version", "utf8").trim().replace(/^v/, "");
      if (have !== want || !fs.existsSync("node_modules/electron/path.txt")) {
        console.error(`dist/ holds Electron ${have || "(nothing)"}, expected ${want}`);
        process.exit(1);
      }' ); then
    echo "error: node_modules/electron/dist is not the lock-pinned Electron runtime" >&2
    exit 1
  fi

  echo "[+] Packaging with electron-builder..."
  ( cd "$APP" && npx --no-install electron-builder --publish never )

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
  # Run from inside the project: `npx --no-install` finds the pinned
  # @electron/asar only in the project's own node_modules, and from anywhere
  # else it refuses rather than downloading.
  ENGINES="$( (cd "$APP" && npx --no-install @electron/asar list "$ASAR") | grep -c 'engram_engine\.wasm' || true)"
  if [[ "$ENGINES" -lt 2 ]]; then
    echo "error: expected the engine in both dist/ and electron/, found $ENGINES copy/copies in $ASAR" >&2
    echo "       the app would launch and then fail to import a deck" >&2
    (cd "$APP" && npx --no-install @electron/asar list "$ASAR") | grep -v '^/node_modules' >&2
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
