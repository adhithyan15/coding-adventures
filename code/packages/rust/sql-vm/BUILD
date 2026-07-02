#!/bin/sh
# BUILD — sql-vm (Rust)
#
# Stack-machine bytecode VM that executes Programs from sql-codegen against a
# Backend (sql-backend).  Sixth stage of the Mini-SQLite pipeline.
#
# Dependencies (leaf → root order):
#   sql-backend, sql-planner, sql-optimizer, sql-codegen, sql-vm

set -e

WORKSPACE="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld

cd "$WORKSPACE"

echo "[sql-vm] Building package..."
cargo build --package coding-adventures-sql-vm
echo "[sql-vm] Build complete."

echo "[sql-vm] Running tests..."
cargo test --package coding-adventures-sql-vm
echo "[sql-vm] Tests complete."
