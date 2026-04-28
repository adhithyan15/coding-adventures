"""Inline optimizer for codegen-core's CIR output.

Runs two lightweight passes over the ``list[CIRInstr]`` produced by
the specialisation pass:

1. **Constant folding** — If both sources of a typed arithmetic or comparison
   instruction are literal values (int / float / bool), compute the result at
   compile time and replace the instruction with a ``const_<type>``.

2. **Dead-code elimination (DCE)** — Remove instructions whose destination
   register is never read.  Side-effectful instructions (``call_runtime``,
   ``call``, ``call_builtin``, ``io_out``, ``store_mem``, ``store_reg``,
   ``type_assert``, ``ret``, ``ret_void``, ``jmp``, ``jmp_if_true``,
   ``jmp_if_false``, ``label``) are always kept.

These two passes are sufficient to clean up the output of the specialisation
pass for simple functions.  Loop-invariant code motion, inlining, and SSA
optimizations are deferred to dedicated pass packages.

This module was originally ``jit_core.optimizer``.  It was moved here so
both ``jit-core`` and ``aot-core`` can import a shared implementation
without a backwards dependency on the JIT package.  ``jit_core.optimizer``
now re-exports ``run`` from here for backwards compatibility.

Design: duck-typed ``Optimizer`` protocol
-----------------------------------------
This module exposes a module-level ``run()`` function rather than a class.
``CodegenPipeline`` expects an ``Optimizer`` with a ``run(ir) -> ir``
method.  The adapter ``CIROptimizer`` below wraps the module-level
function into a class with the right interface.

Usage
-----
Most callers should use the module-level ``run()`` directly:

    from codegen_core.optimizer import cir_optimizer
    optimized = cir_optimizer.run(cir)

To plug into ``CodegenPipeline``:

    from codegen_core.optimizer.cir_optimizer import CIROptimizer
    pipeline = CodegenPipeline(backend=backend, optimizer=CIROptimizer())
"""

from __future__ import annotations

from typing import Any

from codegen_core.cir import CIRInstr

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
# Public entry point — module-level function (original jit-core API)
# ---------------------------------------------------------------------------

def run(cir: list[CIRInstr]) -> list[CIRInstr]:
    """Run all optimization passes and return the optimized CIR.

    Passes are run in order: constant folding, then DCE.  Both passes are
    idempotent; running them twice produces the same result.

    Parameters
    ----------
    cir:
        The unoptimized ``list[CIRInstr]`` from the specialisation pass.

    Returns
    -------
    list[CIRInstr]
        The optimized instruction list.  The input list is not modified.
    """
    cir = _constant_fold(cir)
    cir = _dead_code_eliminate(cir)
    return cir


# ---------------------------------------------------------------------------
# Class wrapper — satisfies the ``Optimizer[list[CIRInstr]]`` Protocol
# ---------------------------------------------------------------------------

class CIROptimizer:
    """Wraps the module-level ``run()`` as an ``Optimizer`` object.

    Use this class when constructing a ``CodegenPipeline`` that requires
    the ``Optimizer`` protocol:

        pipeline = CodegenPipeline(backend=b, optimizer=CIROptimizer())

    The module-level ``run()`` function is the canonical entry point for
    callers that just need the optimized list without a pipeline.
    """

    def run(self, cir: list[CIRInstr]) -> list[CIRInstr]:
        """Delegate to the module-level ``run()``."""
        return run(cir)


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
    # Extract the base op before the type suffix (e.g. "add_u8" → "add").
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
    """Infer an IIR type string from a Python literal folded at compile time."""
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
    # Collect all variable names that appear as sources anywhere in the list.
    # A destination that never appears as a source is dead.
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
