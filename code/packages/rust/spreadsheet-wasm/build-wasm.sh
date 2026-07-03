#!/usr/bin/env bash
#
# build-wasm.sh — compile the spreadsheet engine to a browser .wasm.
#
# Builds `spreadsheet-wasm` (the extern "C" ABI over spreadsheet-core-wasm)
# for `wasm32-unknown-unknown` in release mode and copies the result to
# `pkg/spreadsheet_engine.wasm`, the committed artifact the JS loader and the
# VisiCalc demos consume.
#
# Zero-dependency build: no wasm-bindgen, no wasm-pack — just `cargo build`
# with the wasm target and a copy. The ABI is hand-written `#[no_mangle]`
# exports; see src/lib.rs.
#
# Usage:
#   cd code/packages/rust/spreadsheet-wasm
#   bash build-wasm.sh
#
# Tooling note: nothing is installed globally in this repo — cargo and the
# wasm32 target come from mise. Run through `mise exec --` if needed:
#   mise exec -- bash build-wasm.sh
# One-time target install (if missing):
#   mise exec -- rustup target add wasm32-unknown-unknown

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="$(cd "$SCRIPT_DIR/.." && pwd)" # code/packages/rust
OUT_DIR="$SCRIPT_DIR/pkg"
TARGET=wasm32-unknown-unknown

mkdir -p "$OUT_DIR"

echo "Building spreadsheet-wasm for $TARGET (release)…"
(cd "$WORKSPACE" && cargo build -p spreadsheet-wasm --target "$TARGET" --release)

SRC="$WORKSPACE/target/$TARGET/release/spreadsheet_wasm.wasm"
DST="$OUT_DIR/spreadsheet_engine.wasm"
cp "$SRC" "$DST"

echo "Wrote $DST ($(wc -c < "$DST") bytes)"
echo "Smoke-test it with: node js/smoke.mjs"
