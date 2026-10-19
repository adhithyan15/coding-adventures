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
#   bash build-wasm.sh           # build, then record the inputs stamp
#   bash build-wasm.sh --check   # fail if the committed .wasm is stale (CI)
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
STAMP="$DST.inputs"

# The digest of every source the .wasm is built from (see inputs-digest.py).
inputs_digest() {
  (cd "$WORKSPACE" && cargo tree -q -p spreadsheet-wasm --target "$TARGET" \
      -e normal,build --prefix none --format "{p}") |
    python3 "$SCRIPT_DIR/inputs-digest.py"
}

wasm_digest() {
  python3 -c 'import hashlib, sys; print(hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest())' "$1"
}

# --check: the committed artifact must have been built from today's sources,
# and must be the artifact that stamp describes. Run by CI (BUILD_mac_and_linux).
if [ "${1:-}" = "--check" ]; then
  if [ ! -f "$DST" ] || [ ! -f "$STAMP" ]; then
    echo "error: $DST or its .inputs stamp is missing; run: bash build-wasm.sh" >&2
    exit 1
  fi
  expected_inputs="$(sed -n 's/^inputs-sha256 //p' "$STAMP")"
  expected_wasm="$(sed -n 's/^wasm-sha256 //p' "$STAMP")"
  actual_inputs="$(inputs_digest)"
  actual_wasm="$(wasm_digest "$DST")"
  if [ "$expected_inputs" != "$actual_inputs" ]; then
    echo "error: pkg/spreadsheet_engine.wasm is stale: a source it is built from changed." >&2
    echo "       Rebuild it (and the VisiCalc demo bundles) with: bash build-wasm.sh" >&2
    exit 1
  fi
  if [ "$expected_wasm" != "$actual_wasm" ]; then
    echo "error: pkg/spreadsheet_engine.wasm does not match its .inputs stamp; run: bash build-wasm.sh" >&2
    exit 1
  fi
  echo "pkg/spreadsheet_engine.wasm is current (inputs $actual_inputs)"
  exit 0
fi

mkdir -p "$OUT_DIR"

# Paths are remapped so the bytes do not depend on where the repository is
# checked out: two checkouts at different paths then build an identical .wasm.
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

SRC="$WORKSPACE/target/$TARGET/release/spreadsheet_wasm.wasm"
cp "$SRC" "$DST"
{
  echo "inputs-sha256 $(inputs_digest)"
  echo "wasm-sha256 $(wasm_digest "$DST")"
} > "$STAMP"

echo "Wrote $DST ($(wc -c < "$DST") bytes) and $STAMP"
echo "Smoke-test it with: node js/smoke.mjs"
echo "Then refresh the VisiCalc demo bundle:"
echo "  bash ../../../programs/typescript/visicalc-html/scripts/bundle-wasm-engine.sh"
