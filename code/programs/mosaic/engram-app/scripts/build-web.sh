#!/usr/bin/env bash
#
# Build Engram's web host into a deployable project.
#
#   1. compile the Engram engine to wasm,
#   2. emit the Mosaic app as a complete React/Vite project,
#   3. install the wasm runtime (loader + .wasm) into that project.
#
# Unlike task-app, Engram has no committed web host package: `mosaic-compile
# --emit-project` generates the whole thing — index.html, package.json,
# vite.config.ts, src/main.tsx — and `[host_assets]` supplies src/engram-host.ts.
# So this script produces a project rather than refreshing one.
#
# Step 3 is not optional. The emitted src/engram-host.ts imports
# "./engram-mosaic-host-wasm", and emission copies only the .d.ts — the loader
# itself lives in engram-wasm/js. Without the copy the build fails with:
#
#   Could not resolve "./engram-mosaic-host-wasm" from "src/engram-host.ts"
#
# That step previously existed only in build-all.ps1, which cannot run on the
# Linux CI runner. This is the cross-platform equivalent for the web backend.

set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: build-web.sh [--output DIR] [--theme NAME] [--build | --update-lock]

  --output DIR   Where to emit (default: <engram-app>/dist)
  --theme NAME   Style theme to bake in, e.g. light (default: the package default)
  --build        Also install from npm/web/package-lock.json (`npm ci`) and
                 run `npm run build`, producing dist/
  --update-lock  Only emit the project and regenerate npm/web/package-lock.json
                 from its package.json (skips the wasm build; installs nothing)

Emits a React/Vite project at <DIR>/react ready for `npm install && npm run dev`.
USAGE
}

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"   # the engram-app package
RUST="$(cd "$HERE/../../../packages/rust" && pwd)"
WASM="$RUST/engram-wasm"
LOCK="$HERE/npm/web/package-lock.json"
# shellcheck source=SCRIPTDIR/npm-lock.sh
source "$HERE/scripts/npm-lock.sh"

# `dist` rather than a new name: the root .gitignore already covers `dist/`,
# so the default run leaves no untracked clutter behind.
OUTPUT="$HERE/dist"
THEME=""
RUN_BUILD=0
UPDATE_LOCK=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output) OUTPUT="$2"; shift 2 ;;
    --theme)  THEME="$2";  shift 2 ;;
    --build)  RUN_BUILD=1; shift ;;
    --update-lock) UPDATE_LOCK=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "build-web.sh: unknown argument '$1'" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ "$UPDATE_LOCK" -eq 1 && "$RUN_BUILD" -eq 1 ]]; then
  echo "build-web.sh: --update-lock cannot be combined with --build" >&2
  exit 2
fi

# The lockfile depends only on package.json, so --update-lock skips the engine.
if [[ "$UPDATE_LOCK" -eq 0 ]]; then
  echo "[1/3] Building the Engram engine to wasm..."
  bash "$WASM/build-wasm.sh"
fi

echo "[2/3] Emitting the Mosaic app as a React project..."
rm -rf "$OUTPUT"
# Two explicit invocations rather than an array spread: expanding an empty array
# under `set -u` is an error on bash 3.2, which is what macOS ships, and this
# script has to run on a developer's Mac as well as the Linux CI runner.
if [[ -n "$THEME" ]]; then
  ( cd "$RUST" && cargo run -q -p mosaic-compile -- pkg "$HERE" \
      --backend react --output "$OUTPUT" --emit-project --theme "$THEME" )
else
  ( cd "$RUST" && cargo run -q -p mosaic-compile -- pkg "$HERE" \
      --backend react --output "$OUTPUT" --emit-project )
fi

APP="$OUTPUT/react"

if [[ "$UPDATE_LOCK" -eq 1 ]]; then
  update_lock "$APP" "$LOCK"
  exit 0
fi

echo "[3/3] Installing the wasm runtime into the emitted project..."
mkdir -p "$APP/src" "$APP/public"
cp "$WASM/js/engram-mosaic-host-wasm.mjs" "$APP/src/engram-mosaic-host-wasm.mjs"
cp "$WASM/pkg/engram_engine.wasm"         "$APP/public/engram_engine.wasm"

if [[ "$RUN_BUILD" -eq 1 ]]; then
  echo "[+] Installing from the committed lockfile..."
  install_from_lock "$APP" "$LOCK"
  echo "[+] Building the production bundle..."
  ( cd "$APP" && npm run build )
  # The engine has to reach the browser, not just the build. Vite copies public/
  # into dist/, and a missing wasm there is a runtime failure with a working
  # build — exactly the shape of bug this script's step 3 exists to prevent.
  cmp "$WASM/pkg/engram_engine.wasm" "$APP/dist/engram_engine.wasm"
  echo ""
  echo "Built: $APP/dist"
else
  echo ""
  echo "Ready. Run:  cd '$APP' && npm install && npm run dev"
fi
