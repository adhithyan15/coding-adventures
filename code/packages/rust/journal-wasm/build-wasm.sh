#!/usr/bin/env bash
#
# Compile the journal-wasm boundary to pkg/journal_engine.wasm.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT_DIR="$SCRIPT_DIR/pkg"
TARGET=wasm32-unknown-unknown

mkdir -p "$OUT_DIR"

# On Windows the host MSVC linker is unavailable in the sandbox; point cargo at
# rust-lld (mirrors task-wasm's build script).
if [[ -z "${CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER:-}" ]]; then
  SYSROOT="$(rustc --print sysroot)"
  RUST_LLD="$SYSROOT/lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe"
  if [[ -f "$RUST_LLD" ]]; then
    export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$RUST_LLD"
  fi
fi

echo "Building journal-wasm for $TARGET (release)..."
(cd "$WORKSPACE" && cargo build -p journal-wasm --target "$TARGET" --release)

SRC="$WORKSPACE/target/$TARGET/release/journal_wasm.wasm"
DST="$OUT_DIR/journal_engine.wasm"
cp "$SRC" "$DST"

echo "Wrote $DST ($(wc -c < "$DST") bytes)"
echo "Smoke-test it with: node js/smoke.mjs"
