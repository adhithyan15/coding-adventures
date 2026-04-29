#!/usr/bin/env bash
set -euo pipefail

# ── jit-core BUILD ──────────────────────────────────────────────────────────
# JIT compilation engine for InterpreterIR.
#
# Depends on:
#   interpreter-ir  — IIRFunction, IIRInstr, SlotState, Operand
#   vm-core         — VMCore, Value, VMError
#
# Build this package alone:
#   cargo build -p jit-core
#
# Run tests:
#   cargo test -p jit-core
# ─────────────────────────────────────────────────────────────────────────────

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

echo "==> Building interpreter-ir (dependency)"
cargo build -p interpreter-ir

echo "==> Building vm-core (dependency)"
cargo build -p vm-core

echo "==> Building jit-core"
cargo build -p jit-core

echo "==> Testing jit-core"
cargo test -p jit-core

echo "✓ jit-core build + tests complete"
