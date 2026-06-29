#!/usr/bin/env bash
#
# Compile the Engram WASM boundary to pkg/engram_engine.wasm.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT_DIR="$SCRIPT_DIR/pkg"
TARGET=wasm32-unknown-unknown

mkdir -p "$OUT_DIR"

if [[ -z "${CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER:-}" ]]; then
  SYSROOT="$(rustc --print sysroot)"
  RUST_LLD="$SYSROOT/lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe"
  if [[ -f "$RUST_LLD" ]]; then
    export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$RUST_LLD"
  elif command -v cygpath >/dev/null 2>&1; then
    RUST_LLD_UNIX="$(cygpath -u "$SYSROOT")/lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe"
    if [[ -f "$RUST_LLD_UNIX" ]]; then
      export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$(cygpath -w "$RUST_LLD_UNIX")"
    fi
  fi
fi

echo "Building engram-wasm for $TARGET (release)..."
(cd "$WORKSPACE" && cargo build -p engram-wasm --target "$TARGET" --release)

SRC="$WORKSPACE/target/$TARGET/release/engram_wasm.wasm"
DST="$OUT_DIR/engram_engine.wasm"
cp "$SRC" "$DST"

echo "Wrote $DST ($(wc -c < "$DST") bytes)"
echo "Smoke-test it with: node js/smoke.mjs"
