#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
WEB="$HERE/host/web"
RUST="$HERE/../../../packages/rust"
WASM="$RUST/task-wasm"
EMIT="$WEB/.emit-webcomponent"

echo "[1/3] Building the TaskApp engine to WebAssembly..."
bash "$WASM/build-wasm.sh"

echo "[2/3] Emitting the real TaskApp Custom Element in both themes..."
rm -rf "$EMIT"
for theme in light dark; do
  (cd "$RUST" && cargo run -q -p mosaic-compile -- pkg "$HERE" \
    --backend webcomponent --theme "$theme" --output "$EMIT/$theme" --emit-project)
  mkdir -p "$WEB/webcomponent/generated/$theme"
  cp "$EMIT/$theme/webcomponent/TaskApp.js" "$WEB/webcomponent/generated/$theme/TaskApp.js"
  cp "$EMIT/$theme/webcomponent/main.js" "$WEB/webcomponent/generated/$theme/main.js"
done
rm -rf "$EMIT"

echo "[3/3] Staging the WASM engine for the parity host..."
mkdir -p "$WEB/webcomponent/public"
cp "$WASM/js/task-engine.mjs" "$WEB/src/task-engine.mjs"
cp "$WASM/pkg/task_engine.wasm" "$WEB/webcomponent/public/task_engine.wasm"

echo
echo "Ready. Run: cd \"$WEB\" && npm install && npm run build:webcomponent"
