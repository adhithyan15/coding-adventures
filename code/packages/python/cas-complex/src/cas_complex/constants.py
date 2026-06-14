"""ImaginaryUnit constant and related IR sentinels."""
from __future__ import annotations

from symbolic_ir import ADD, MUL, NEG, POW, IRApply, IRNode, IRSymbol

#: The imaginary unit ``i`` as a pre-bound IR symbol.
#:
#: This is a singleton — every reference to the imaginary unit in IR trees
#: uses this exact object so that ``is``-comparisons and hash-based
#: collections work correctly.
IMAGINARY_UNIT: IRSymbol = IRSymbol("ImaginaryUnit")

# Common IR building-block symbols used by this package.
_ADD = ADD
_MUL = MUL
_NEG = NEG
_POW = POW
_LIST = IRSymbol("List")
_SQRT = IRSymbol("Sqrt")
_EXP = IRSymbol("Exp")
_ATAN = IRSymbol("Atan")
_ATAN2 = IRSymbol("Atan2")


def is_imaginary_unit(node: IRNode) -> bool:
    """Return True if ``node`` is the ImaginaryUnit symbol."""
    return isinstance(node, IRSymbol) and node.name == "ImaginaryUnit"


def make_neg(node: IRNode) -> IRNode:
    """Wrap ``node`` in ``Neg(...)``; fold ``Neg(Neg(x))`` → ``x``."""
    is_neg = (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "Neg"
    )
    if is_neg:
        return node.args[0]
    return IRApply(_NEG, (node,))


def make_mul(a: IRNode, b: IRNode) -> IRNode:
    """Build ``Mul(a, b)``."""
    return IRApply(_MUL, (a, b))


def make_add(a: IRNode, b: IRNode) -> IRNode:
    """Build ``Add(a, b)``."""
    return IRApply(_ADD, (a, b))


def make_pow(base: IRNode, exp: IRNode) -> IRNode:
    """Build ``Pow(base, exp)``."""
    return IRApply(_POW, (base, exp))
