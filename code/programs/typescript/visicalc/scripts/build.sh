#!/usr/bin/env bash
set -euo pipefail

if [ -d "$HOME/.cargo/bin" ] && ! command -v cargo >/dev/null 2>&1; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEMO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$DEMO_DIR/../../../.." && pwd)"
RUST="$REPO_ROOT/code/packages/rust"
OUT="$DEMO_DIR/src/components"
mkdir -p "$OUT" "$DEMO_DIR/public"
cargo build --manifest-path "$RUST/Cargo.toml" -p mosaic-compile
rustup target add wasm32-unknown-unknown
cargo build --manifest-path "$RUST/Cargo.toml" -p visicalc-mosaic-app --target wasm32-unknown-unknown
MOSAIC_COMPILE="$RUST/target/debug/mosaic-compile"
if [ -f "${MOSAIC_COMPILE}.exe" ]; then
  MOSAIC_COMPILE="${MOSAIC_COMPILE}.exe"
fi
for THEME in light dark; do
  "$MOSAIC_COMPILE" pkg "$REPO_ROOT/code/programs/mosaic/visicalc" \
    --backend react --theme "$THEME" --output "$OUT/$THEME"
done
cp "$RUST/target/wasm32-unknown-unknown/debug/visicalc_mosaic_app.wasm" "$DEMO_DIR/public/visicalc_mosaic_app.wasm"
