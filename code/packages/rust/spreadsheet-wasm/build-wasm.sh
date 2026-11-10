#!/usr/bin/env bash
#
# build-wasm.sh — compile the spreadsheet engine to a browser .wasm.
#
# Builds `spreadsheet-wasm` (the extern "C" ABI over spreadsheet-core-wasm)
# for `wasm32-unknown-unknown` in release mode and copies the result to
# `pkg/spreadsheet_engine.wasm`, the artifact the JS loader and the VisiCalc
# demos consume. The artifact is generated and git-ignored (BUILD-001): it is
# built, never committed, so there is nothing to keep "current".
#
# Zero-dependency build: no wasm-bindgen, no wasm-pack — just `cargo build`
# with the wasm target and a copy. The ABI is hand-written `#[no_mangle]`
# exports; see src/lib.rs.
#
# Usage:
#   cd code/packages/rust/spreadsheet-wasm
#   bash build-wasm.sh
#
# To build every demo engine plus the VisiCalc bundle in one go, run
# code/scripts/build-demo-wasm.sh instead.
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
DST="$OUT_DIR/spreadsheet_engine.wasm"

mkdir -p "$OUT_DIR"

# Paths are remapped so the bytes do not depend on where the repository is
# checked out: two checkouts at different paths then build an identical .wasm.
# (Nothing compares against a stored hash any more; this just keeps local
# builds comparable.)
# CARGO_ENCODED_RUSTFLAGS separates flags with 0x1f, so a path with spaces
# stays one flag; it also replaces any RUSTFLAGS in the environment, which
# would otherwise change the bytes.
SEP=$'\x1f'
ENCODED="--remap-path-prefix=$WORKSPACE=/workspace"
ENCODED="$ENCODED$SEP--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
ENCODED="$ENCODED$SEP--remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=/rustup"

echo "Building spreadsheet-wasm for $TARGET (release)…"
(cd "$WORKSPACE" && CARGO_ENCODED_RUSTFLAGS="$ENCODED" \
  cargo build -p spreadsheet-wasm --target "$TARGET" --release)

SRC="${CARGO_TARGET_DIR:-$WORKSPACE/target}/$TARGET/release/spreadsheet_wasm.wasm"
cp "$SRC" "$DST"

echo "Wrote $DST ($(wc -c < "$DST") bytes)"
echo "Smoke-test it with: node js/smoke.mjs"
echo "Then refresh the VisiCalc demo bundle:"
echo "  bash ../../../programs/typescript/visicalc-html/scripts/bundle-wasm-engine.sh"
