"""Tetrad JIT intermediate representation (SSA-like IR).

Between bytecode and x86-64, the JIT uses a simple SSA-based IR.  Each IR
instruction operates on **virtual variables** (e.g. ``v0``, ``v1``) rather
than the accumulator and explicit registers.  This makes optimization passes
easier to implement: constant folding only needs to track a values dict;
dead-code elimination only needs a liveness set.

IR instruction set
------------------

+------------------+----------------------------------------------+
| Op               | Semantics                                    |
+==================+==============================================+
| ``const``        | dst = srcs[0] (integer literal)              |
+------------------+----------------------------------------------+
| ``param``        | dst = Nth argument (src[0] = N)              |
+------------------+----------------------------------------------+
| ``load_var``     | dst = locals[srcs[0]] (or globals)           |
+------------------+----------------------------------------------+
| ``store_var``    | locals[srcs[0]] = srcs[1]                    |
+------------------+----------------------------------------------+
| ``add``          | dst = (srcs[0] + srcs[1]) % 256             |
+------------------+----------------------------------------------+
| ``sub``          | dst = (srcs[0] - srcs[1]) % 256             |
+------------------+----------------------------------------------+
| ``mul``          | dst = (srcs[0] * srcs[1]) % 256             |
+------------------+----------------------------------------------+
| ``div``          | dst = srcs[0] // srcs[1]                    |
+------------------+----------------------------------------------+
| ``mod``          | dst = srcs[0] % srcs[1]                     |
+------------------+----------------------------------------------+
| ``and``          | dst = srcs[0] & srcs[1]                     |
+------------------+----------------------------------------------+
| ``or``           | dst = srcs[0] | srcs[1]                     |
+------------------+----------------------------------------------+
| ``xor``          | dst = srcs[0] ^ srcs[1]                     |
+------------------+----------------------------------------------+
| ``not``          | dst = ~srcs[0] & 0xFF                       |
+------------------+----------------------------------------------+
| ``shl``          | dst = (srcs[0] << srcs[1]) & 0xFF           |
+------------------+----------------------------------------------+
| ``shr``          | dst = srcs[0] >> srcs[1]                    |
+------------------+----------------------------------------------+
| ``cmp_eq``       | dst = 1 if srcs[0] == srcs[1] else 0        |
+------------------+----------------------------------------------+
| ``cmp_ne``       | dst = 1 if srcs[0] != srcs[1] else 0        |
+------------------+----------------------------------------------+
| ``cmp_lt``       | dst = 1 if srcs[0] < srcs[1] else 0         |
+------------------+----------------------------------------------+
| ``cmp_le``       | dst = 1 if srcs[0] <= srcs[1] else 0        |
+------------------+----------------------------------------------+
| ``cmp_gt``       | dst = 1 if srcs[0] > srcs[1] else 0         |
+------------------+----------------------------------------------+
| ``cmp_ge``       | dst = 1 if srcs[0] >= srcs[1] else 0        |
+------------------+----------------------------------------------+
| ``logical_not``  | dst = 0 if srcs[0] != 0 else 1              |
+------------------+----------------------------------------------+
| ``jmp``          | unconditional jump to label srcs[0]          |
+------------------+----------------------------------------------+
| ``jz``           | if srcs[0]==0 jump to label srcs[1]          |
+------------------+----------------------------------------------+
| ``jnz``          | if srcs[0]!=0 jump to label srcs[1]          |
+------------------+----------------------------------------------+
| ``label``        | branch target; srcs[0] is the label name     |
+------------------+----------------------------------------------+
| ``ret``          | return srcs[0]                              |
+------------------+----------------------------------------------+
| ``io_in``        | dst = read_io()                             |
+------------------+----------------------------------------------+
| ``io_out``       | write srcs[0] to io                         |
+------------------+----------------------------------------------+
| ``deopt``        | bail to interpreter                         |
+------------------+----------------------------------------------+
| ``call``         | dst = call func_idx(srcs[1..])              |
+------------------+----------------------------------------------+

``srcs`` entries can be:
- ``str``: a virtual variable name (``"v0"``)
- ``int``: an integer constant
- ``str`` starting with ``"lbl_"``: a label for jumps
"""

from __future__ import annotations

from dataclasses import dataclass, field

__all__ = [
    "ARITHMETIC_OPS",
    "BINARY_OPS",
    "CMP_OPS",
    "SIDE_EFFECT_OPS",
    "IRInstr",
    "evaluate_op",
]

# ---------------------------------------------------------------------------
# Instruction categories
# ---------------------------------------------------------------------------

ARITHMETIC_OPS: frozenset[str] = frozenset({
    "add", "sub", "mul", "div", "mod",
    "and", "or", "xor", "shl", "shr",
})

CMP_OPS: frozenset[str] = frozenset({
    "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge",
})

BINARY_OPS: frozenset[str] = ARITHMETIC_OPS | CMP_OPS

SIDE_EFFECT_OPS: frozenset[str] = frozenset({
    "store_var", "io_out", "jmp", "jz", "jnz", "ret", "deopt", "call",
})


# ---------------------------------------------------------------------------
# IR instruction
# ---------------------------------------------------------------------------


@dataclass
class IRInstr:
    """One instruction in the Tetrad JIT IR.

    ``op``      — operation name (see table above).
    ``dst``     — destination virtual variable name, or ``None`` for ops with
                  side effects only (``store_var``, ``io_out``, ``ret``, …).
    ``srcs``    — list of source virtual variable names or integer literals.
    ``ty``      — type annotation: ``"u8"`` (concrete) or ``"unknown"``.
    ``comment`` — optional debug comment appended to the IR dump.
    """

    op: str
    dst: str | None
    srcs: list[str | int]
    ty: str
    comment: str = field(default="")

    def __repr__(self) -> str:  # pragma: no cover
        lhs = f"{self.dst} = " if self.dst else ""
        srcs = ", ".join(str(s) for s in self.srcs)
        cmt = f"  # {self.comment}" if self.comment else ""
        return f"{lhs}{self.op} {srcs}  [{self.ty}]{cmt}"


# ---------------------------------------------------------------------------
# Constant evaluator (used by the constant-folding pass)
# ---------------------------------------------------------------------------


def evaluate_op(op: str, a: int, b: int) -> int:  # noqa: PLR0911
    """Evaluate a binary arithmetic or comparison operation on two constants.

    All arithmetic results are taken mod 256 (u8 wrap).
    Division and modulo by zero return 0 (deopt not needed for constants).
    """
    match op:
        case "add":
            return (a + b) % 256
        case "sub":
            return (a - b) % 256
        case "mul":
            return (a * b) % 256
        case "div":
            return a // b if b != 0 else 0
        case "mod":
            return a % b if b != 0 else 0
        case "and":
            return a & b
        case "or":
            return a | b
        case "xor":
            return a ^ b
        case "shl":
            return (a << b) & 0xFF
        case "shr":
            return a >> b
        case "cmp_eq":
            return 1 if a == b else 0
        case "cmp_ne":
            return 1 if a != b else 0
        case "cmp_lt":
            return 1 if a < b else 0
        case "cmp_le":
            return 1 if a <= b else 0
        case "cmp_gt":
            return 1 if a > b else 0
        case "cmp_ge":
            return 1 if a >= b else 0
        case _:
            return 0
