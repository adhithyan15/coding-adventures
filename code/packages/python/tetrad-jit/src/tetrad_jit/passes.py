"""JIT optimization passes (TET05).

Two passes run in order:

1. **Constant folding** — evaluates binary and unary operations on
   known-constant virtual variables at compile time.

   ``v0 = const 10; v1 = const 5; v2 = add v0, v1``
   becomes ``v0 = const 10; v1 = const 5; v2 = const 15``.

2. **Dead code elimination (DCE)** — removes IR instructions whose
   destination is never used as a source in a live instruction.

   ``v3 = add v1, v2   (v3 appears nowhere else)``  → removed.

Both passes leave side-effect instructions (``store_var``, ``io_out``,
``jmp``, ``jz``, ``jnz``, ``ret``, ``call``, ``deopt``) untouched.
Labels are never removed.
"""

from __future__ import annotations

from tetrad_jit.ir import (
    BINARY_OPS,
    SIDE_EFFECT_OPS,
    IRInstr,
    evaluate_op,
)

__all__ = ["constant_fold", "dead_code_eliminate", "optimize"]


# ---------------------------------------------------------------------------
# Pass 1: Constant folding
# ---------------------------------------------------------------------------


def constant_fold(ir: list[IRInstr]) -> list[IRInstr]:
    """Replace operations on known constants with their folded results.

    Maintains ``values: dict[str, int | None]`` — maps each virtual variable
    to its compile-time value if known, or ``None`` if unknown.  A source may
    also be a raw ``int`` (for immediate operands like ``ADD_IMM``); those are
    always known.

    Unary ops:
      ``not v``         → bitwise NOT of v (& 0xFF)
      ``logical_not v`` → 1 if v==0 else 0

    Binary ops (from ``BINARY_OPS``):
      Uses ``evaluate_op(op, a, b)`` for consistent u8 semantics.
    """
    values: dict[str, int | None] = {}
    result: list[IRInstr] = []

    def _known(src: str | int) -> int | None:
        if isinstance(src, int):
            return src
        return values.get(src)

    for instr in ir:
        if instr.op == "const" and instr.dst is not None:
            values[instr.dst] = instr.srcs[0] if isinstance(instr.srcs[0], int) else None
            result.append(instr)

        elif instr.op in BINARY_OPS and instr.dst is not None:
            a = _known(instr.srcs[0])
            b = _known(instr.srcs[1])
            if a is not None and b is not None:
                folded = evaluate_op(instr.op, a, b)
                values[instr.dst] = folded
                result.append(IRInstr(
                    op="const", dst=instr.dst, srcs=[folded], ty="u8",
                    comment=f"folded {instr.op}({a},{b})",
                ))
            else:
                values[instr.dst] = None
                result.append(instr)

        elif instr.op == "not" and instr.dst is not None:
            a = _known(instr.srcs[0])
            if a is not None:
                folded = (~a) & 0xFF
                values[instr.dst] = folded
                result.append(IRInstr(
                    op="const", dst=instr.dst, srcs=[folded], ty="u8",
                    comment=f"folded not({a})",
                ))
            else:
                values[instr.dst] = None
                result.append(instr)

        elif instr.op == "logical_not" and instr.dst is not None:
            a = _known(instr.srcs[0])
            if a is not None:
                folded = 0 if a != 0 else 1
                values[instr.dst] = folded
                result.append(IRInstr(
                    op="const", dst=instr.dst, srcs=[folded], ty="u8",
                    comment=f"folded logical_not({a})",
                ))
            else:
                values[instr.dst] = None
                result.append(instr)

        else:
            if instr.dst is not None:
                values[instr.dst] = None
            result.append(instr)

    return result


# ---------------------------------------------------------------------------
# Pass 2: Dead code elimination
# ---------------------------------------------------------------------------


def dead_code_eliminate(ir: list[IRInstr]) -> list[IRInstr]:
    """Remove IR instructions whose result is never used.

    A variable is *live* if it appears as a source (``srcs`` element) in any
    instruction that is kept.  Side-effect instructions (``SIDE_EFFECT_OPS``)
    and ``label`` instructions are always kept.

    The pass iterates to fixpoint because removing a dead instruction may
    make its sources dead too.
    """
    changed = True
    while changed:
        # Collect all used variables in a single forward scan.
        live: set[str] = set()
        for instr in ir:
            for src in instr.srcs:
                if isinstance(src, str) and not src.startswith("lbl_"):
                    live.add(src)

        new_ir: list[IRInstr] = []
        changed = False
        for instr in ir:
            keep = (
                instr.op in SIDE_EFFECT_OPS
                or instr.op == "label"
                or instr.op == "param"       # always keep param definitions
                or instr.dst is None         # no-dst instructions (side effects)
                or instr.dst in live
            )
            if keep:
                new_ir.append(instr)
            else:
                changed = True               # removed at least one instruction
        ir = new_ir

    return ir


# ---------------------------------------------------------------------------
# Combined optimizer
# ---------------------------------------------------------------------------


def optimize(ir: list[IRInstr]) -> list[IRInstr]:
    """Run all optimization passes in order and return the optimized IR."""
    ir = constant_fold(ir)
    ir = dead_code_eliminate(ir)
    return ir
