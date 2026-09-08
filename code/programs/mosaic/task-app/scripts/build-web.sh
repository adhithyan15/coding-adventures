#!/usr/bin/env bash
#
# Build the task-app web host:
#   1. build the task-core engine to wasm,
#   2. emit the TaskApp React component into the committed host package (host/web/src),
#   3. copy the wasm runtime (loader + .wasm) into the package.
#
# The host itself (package.json, main.tsx, persistence.ts, index.html, …) is a real,
# committed npm package — this script only refreshes the generated + copied artifacts.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)" # the task-app package dir
WEB="$HERE/host/web"
RUST="$(cd "$HERE/../../../packages/rust" && pwd)"
WASM="$RUST/task-wasm"

echo "[1/3] Building the engine to wasm..."
bash "$WASM/build-wasm.sh"

echo "[2/3] Emitting the TaskApp component into the web host..."
# `mosaic-compile pkg` emits into <output>/react/, so emit to a scratch dir and
# copy just the component file into the host's src (main.tsx imports ./TaskApp).
mkdir -p "$WEB/src" "$WEB/public"
# Both themes are emitted, as TaskApp.light.tsx / TaskApp.dark.tsx. mosstyle bakes
# colours into each component's *inline* styles, so there is no CSS variable to flip
# at runtime — the host picks a whole component instead (see src/theme.ts).
EMIT="$WEB/.emit"
rm -rf "$EMIT"
for THEME in light dark; do
  ( cd "$RUST" && cargo run -q -p mosaic-compile -- pkg "$HERE" \
      --backend react --theme "$THEME" --output "$EMIT/$THEME" )
  cp "$EMIT/$THEME/react/TaskApp.tsx" "$WEB/src/TaskApp.$THEME.tsx"
done
rm -rf "$EMIT"

echo "[3/3] Copying the wasm runtime..."
cp "$WASM/js/task-engine.mjs"   "$WEB/src/task-engine.mjs"
cp "$WASM/pkg/task_engine.wasm" "$WEB/public/task_engine.wasm"

echo ""
echo "Ready. Run:  cd '$WEB' && npm install && npm run dev   (http://localhost:5173)"
