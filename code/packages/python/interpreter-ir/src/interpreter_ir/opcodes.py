"""Opcode category frozensets for InterpreterIR.

These sets let vm-core, jit-core, and passes classify instructions without
comparing against long lists of mnemonic strings.

Usage::

    from interpreter_ir.opcodes import ARITHMETIC_OPS, CMP_OPS
    if instr.op in ARITHMETIC_OPS:
        ...
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Type constants
# ---------------------------------------------------------------------------

CONCRETE_TYPES: frozenset[str] = frozenset(
    {"u8", "u16", "u32", "u64", "bool", "str"}
)

# The "unknown" type used by dynamically typed languages before profiling.
DYNAMIC_TYPE = "any"

# Sentinel placed by the vm-core profiler when multiple types are observed.
POLYMORPHIC_TYPE = "polymorphic"

# ---------------------------------------------------------------------------
# Opcode categories
# ---------------------------------------------------------------------------

ARITHMETIC_OPS: frozenset[str] = frozenset(
    {"add", "sub", "mul", "div", "mod", "neg"}
)

BITWISE_OPS: frozenset[str] = frozenset(
    {"and", "or", "xor", "not", "shl", "shr"}
)

CMP_OPS: frozenset[str] = frozenset(
    {"cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge"}
)

BRANCH_OPS: frozenset[str] = frozenset(
    {"jmp", "jmp_if_true", "jmp_if_false"}
)

CONTROL_OPS: frozenset[str] = frozenset(
    {"label", "ret", "ret_void"}
)

MEMORY_OPS: frozenset[str] = frozenset(
    {"load_reg", "store_reg", "load_mem", "store_mem"}
)

CALL_OPS: frozenset[str] = frozenset(
    {"call", "call_builtin"}
)

IO_OPS: frozenset[str] = frozenset(
    {"io_in", "io_out"}
)

COERCION_OPS: frozenset[str] = frozenset(
    {"cast", "type_assert"}
)

# All ops that produce a value (have a non-None dest).
_VALUE_EXTRA: frozenset[str] = frozenset(
    {"const", "load_reg", "load_mem", "call", "call_builtin", "io_in", "cast"}
)
VALUE_OPS: frozenset[str] = ARITHMETIC_OPS | BITWISE_OPS | CMP_OPS | _VALUE_EXTRA

# All ops that have side effects beyond producing a value.
SIDE_EFFECT_OPS: frozenset[str] = (
    BRANCH_OPS
    | CONTROL_OPS
    | frozenset({"store_reg", "store_mem", "io_out", "type_assert"})
)

# All known opcodes.
ALL_OPS: frozenset[str] = (
    ARITHMETIC_OPS
    | BITWISE_OPS
    | CMP_OPS
    | BRANCH_OPS
    | CONTROL_OPS
    | MEMORY_OPS
    | CALL_OPS
    | IO_OPS
    | COERCION_OPS
    | frozenset({"const"})
)
