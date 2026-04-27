"""Trigonometric simplification, expansion, and reduction.

Quick start::

    from cas_trig import trig_simplify, trig_expand, trig_reduce
    from symbolic_ir import IRApply, IRInteger, IRSymbol

    SIN = IRSymbol("Sin")
    COS = IRSymbol("Cos")
    x = IRSymbol("x")

    # sin²(x) + cos²(x) → 1
    expr = IRApply(IRSymbol("Add"), (
        IRApply(IRSymbol("Pow"), (IRApply(SIN, (x,)), IRInteger(2))),
        IRApply(IRSymbol("Pow"), (IRApply(COS, (x,)), IRInteger(2))),
    ))
    trig_simplify(expr)   # IRInteger(1)

    # sin(2x) → 2 sin(x) cos(x)
    trig_expand(IRApply(SIN, (IRApply(IRSymbol("Mul"), (IRInteger(2), x)),)))
"""

from cas_trig.expand import trig_expand
from cas_trig.handlers import build_trig_handler_table
from cas_trig.reduce import trig_reduce
from cas_trig.simplify import trig_simplify
from cas_trig.special_values import lookup_special_value

__all__ = [
    "build_trig_handler_table",
    "lookup_special_value",
    "trig_expand",
    "trig_reduce",
    "trig_simplify",
]
