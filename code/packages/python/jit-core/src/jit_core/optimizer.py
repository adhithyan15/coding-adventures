"""Inline optimizer for jit-core's CIR output.

Runs two lightweight passes over the ``list[CIRInstr]`` produced by
``specialise()``:

1. **Constant folding** — If both sources of a typed arithmetic or comparison
   instruction are literal values (int / float / bool), compute the result at
   compile time and replace the instruction with a ``const_<type>``.

2. **Dead-code elimination (DCE)** — Remove instructions whose destination
   register is never read.  Side-effectful instructions (``call_runtime``,
   ``call``, ``call_builtin``, ``io_out``, ``store_mem``, ``store_reg``,
   ``type_assert``, ``ret``, ``ret_void``, ``jmp``, ``jmp_if_true``,
   ``jmp_if_false``) are always kept.

These two passes are sufficient to clean up the output of the specialization
pass for simple functions.  Loop-invariant code motion, inlining, and SSA
optimizations are deferred to a future ``ir-optimizer`` package.
"""

from __future__ import annotations

from typing import Any

from jit_core.cir import CIRInstr

# ---------------------------------------------------------------------------
# Side-effectful opcodes — always kept even if dest is unused
# ---------------------------------------------------------------------------

_SIDE_EFFECT_OPS: frozenset[str] = frozenset({
    "call_runtime",
    "call",
    "call_builtin",
    "io_out",
    "store_mem",
    "store_reg",
    "type_assert",
    "ret",
    "ret_void",
    "jmp",
    "jmp_if_true",
    "jmp_if_false",
    "label",
})

# Arithmetic ops whose result can be folded when both srcs are literals.
_FOLDABLE_OPS: dict[str, Any] = {
    "add": lambda a, b: a + b,
    "sub": lambda a, b: a - b,
    "mul": lambda a, b: a * b,
    "div": lambda a, b: a // b if isinstance(a, int) else a / b,
    "mod": lambda a, b: a % b,
    "and": lambda a, b: a & b,
    "or": lambda a, b: a | b,
    "xor": lambda a, b: a ^ b,
    "shl": lambda a, b: a << b,
    "shr": lambda a, b: a >> b,
    "cmp_eq": lambda a, b: a == b,
    "cmp_ne": lambda a, b: a != b,
    "cmp_lt": lambda a, b: a < b,
    "cmp_le": lambda a, b: a <= b,
    "cmp_gt": lambda a, b: a > b,
    "cmp_ge": lambda a, b: a >= b,
}


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------

def run(cir: list[CIRInstr]) -> list[CIRInstr]:
    """Run all optimization passes and return the optimized CIR.

    Passes are run in order: constant folding, then DCE.  Both passes are
    idempotent; running them twice produces the same result.
    """
    cir = _constant_fold(cir)
    cir = _dead_code_eliminate(cir)
    return cir


# ---------------------------------------------------------------------------
# Pass 1: constant folding
# ---------------------------------------------------------------------------

def _constant_fold(cir: list[CIRInstr]) -> list[CIRInstr]:
    result: list[CIRInstr] = []
    for instr in cir:
        folded = _try_fold(instr)
        result.append(folded if folded is not None else instr)
    return result


def _try_fold(instr: CIRInstr) -> CIRInstr | None:
    # Extract the base op (before the type suffix, e.g. "add_u8" → "add")
    base_op = instr.op.split("_")[0] if "_" in instr.op else instr.op
    folder = _FOLDABLE_OPS.get(base_op)
    if folder is None:
        return None
    if len(instr.srcs) != 2:
        return None
    a, b = instr.srcs[0], instr.srcs[1]
    if isinstance(a, str) or isinstance(b, str):
        return None  # not both literals
    try:
        value = folder(a, b)
    except (ZeroDivisionError, ValueError, OverflowError):
        return None
    const_type = instr.type if instr.type != "any" else _infer_literal_type(value)
    return CIRInstr(
        op=f"const_{const_type}", dest=instr.dest, srcs=[value], type=const_type
    )


def _infer_literal_type(value: object) -> str:
    if isinstance(value, bool):
        return "bool"
    if isinstance(value, int):
        if 0 <= value <= 255:
            return "u8"
        if 0 <= value <= 65535:
            return "u16"
        if 0 <= value <= 0xFFFF_FFFF:
            return "u32"
        return "u64"
    if isinstance(value, float):
        return "f64"
    if isinstance(value, str):
        return "str"
    return "any"


# ---------------------------------------------------------------------------
# Pass 2: dead-code elimination
# ---------------------------------------------------------------------------

def _dead_code_eliminate(cir: list[CIRInstr]) -> list[CIRInstr]:
    # Collect all variable names that appear as sources somewhere.
    used: set[str] = set()
    for instr in cir:
        for src in instr.srcs:
            if isinstance(src, str):
                used.add(src)

    result: list[CIRInstr] = []
    for instr in cir:
        if (
            instr.dest is not None
            and instr.dest not in used
            and instr.op not in _SIDE_EFFECT_OPS
        ):
            continue  # dead — dest produced but never consumed
        result.append(instr)
    return result
