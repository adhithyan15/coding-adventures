"""Substitute symbols or sub-expressions in symbolic IR trees.

Quick start::

    from cas_substitution import subst, replace_all
    from cas_pattern_matching import Blank, Pattern, Rule
    from symbolic_ir import IRApply, IRInteger, IRSymbol, ADD, MUL, POW

    # MACSYMA-style symbol substitution.
    subst(IRInteger(2), IRSymbol("x"),
          IRApply(POW, (IRSymbol("x"), IRInteger(2))))
    # Pow(2, 2) — un-simplified

    # Mathematica-style pattern substitution.
    rule = Rule(IRApply(POW, (Pattern("a", Blank()), IRInteger(2))),
                IRApply(MUL, (Pattern("a", Blank()), Pattern("a", Blank()))))
    replace_all(IRApply(POW, (IRSymbol("y"), IRInteger(2))), rule)
    # Mul(y, y)
"""

from cas_substitution.replace_all import replace_all, replace_all_many
from cas_substitution.subst import subst, subst_many

__all__ = [
    "replace_all",
    "replace_all_many",
    "subst",
    "subst_many",
]
