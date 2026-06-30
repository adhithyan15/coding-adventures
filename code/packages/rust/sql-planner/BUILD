#!/bin/sh
# BUILD — sql-planner (Linux/macOS)
#
# Installs leaf-to-root: lexer → sql-lexer → parser → grammar-tools → sql-backend
#   → sql-parser → sql-planner (this package)
set -e

WORKSPACE="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld

cd "$WORKSPACE"

# Build and test only this package.
cargo build --package coding-adventures-sql-planner
cargo test  --package coding-adventures-sql-planner
