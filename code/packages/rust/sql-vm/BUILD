#!/usr/bin/env bash
# BUILD — sql-vm (Rust)
#
# Stack-machine bytecode VM that executes Programs from sql-codegen against a
# Backend (sql-backend).  Sixth stage of the Mini-SQLite pipeline.
#
# Dependencies (leaf → root order):
#   sql-backend, sql-planner, sql-optimizer, sql-codegen, sql-vm

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "[sql-vm] Building package..."
cd "$SCRIPT_DIR"

cargo build --package coding-adventures-sql-vm
echo "[sql-vm] Build complete."

echo "[sql-vm] Running tests..."
cargo test --package coding-adventures-sql-vm
echo "[sql-vm] Tests complete."
