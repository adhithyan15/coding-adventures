#!/bin/sh
# BUILD — sql-optimizer (Linux/macOS)
#
# Dependency chain (leaf-to-root):
#   sql-lexer → sql-parser → sql-backend → sql-planner → sql-optimizer
set -e

WORKSPACE="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld

cd "$WORKSPACE"

cargo build --package coding-adventures-sql-optimizer
cargo test  --package coding-adventures-sql-optimizer
