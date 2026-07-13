#!/usr/bin/env bash
#
# Assemble the runnable task-app web project into dist/react:
#   1. build the task-core engine to wasm,
#   2. emit the React project from the Mosaic package,
#   3. overlay the web host wiring (our main.tsx) + the wasm runtime.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)" # the task-app package dir
RUST="$(cd "$HERE/../../../packages/rust" && pwd)"
WASM="$RUST/task-wasm"
OUT="$HERE/dist"

echo "[1/3] Building the engine to wasm..."
bash "$WASM/build-wasm.sh"

echo "[2/3] Emitting the React project..."
( cd "$RUST" && cargo run -q -p mosaic-compile -- pkg "$HERE" \
    --backend react --output "$OUT" --emit-project )

echo "[3/3] Overlaying host wiring + wasm runtime..."
RS="$OUT/react/src"
RP="$OUT/react/public"
mkdir -p "$RP"
cp "$HERE/host/web/main.tsx"          "$RS/main.tsx"          # replaces the generated shell
cp "$HERE/host/web/task-engine.d.ts"  "$RS/task-engine.d.ts"
cp "$WASM/js/task-engine.mjs"         "$RS/task-engine.mjs"
cp "$WASM/pkg/task_engine.wasm"       "$RP/task_engine.wasm"

echo ""
echo "Ready. Run:  cd '$OUT/react' && npm install && npm run dev   (http://localhost:5173)"
