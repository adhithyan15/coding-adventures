"""TrigExpand: expand compound trig arguments to sums/products.

Rewrites:
- ``sin(a + b)`` → ``sin(a)cos(b) + cos(a)sin(b)``
- ``cos(a + b)`` → ``cos(a)cos(b) - sin(a)sin(b)``
- ``sin(n·x)``  → Chebyshev recurrence for integer ``n``
- ``cos(n·x)``  → Chebyshev recurrence for integer ``n``
- ``sin(a - b)`` → ``sin(a)cos(b) - cos(a)sin(b)``
- ``cos(a - b)`` → ``cos(a)cos(b) + sin(a)sin(b)``

All other IR heads are recursed into but not rewritten.
"""

from __future__ import annotations

from cas_simplify import canonical
from symbolic_ir import (
    ADD,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRSymbol,
)

SIN = IRSymbol("Sin")
COS = IRSymbol("Cos")

_MAX_ITER = 20


def trig_expand(expr: IRNode) -> IRNode:
    """Fully expand compound trig arguments in ``expr``.

    Applies expansion recursively until no further expansions apply, then
    canonicalises the result.
    """
    for _ in range(_MAX_ITER):
        prev = expr
        expr = _expand_walk(expr)
        expr = canonical(expr)
        if expr == prev:
            break
    return expr


# ---------------------------------------------------------------------------
# Internal recursive walker
# ---------------------------------------------------------------------------


def _expand_walk(node: IRNode) -> IRNode:
    """Recursively apply angle-addition and multiple-angle expansions."""
    if not isinstance(node, IRApply):
        return node

    head_name = node.head.name if isinstance(node.head, IRSymbol) else ""

    # First recurse into args
    new_args = tuple(_expand_walk(a) for a in node.args)
    if new_args != node.args:
        node = IRApply(node.head, new_args)

    if head_name == "Sin" and len(node.args) == 1:
        return _expand_sin(node.args[0])
    if head_name == "Cos" and len(node.args) == 1:
        return _expand_cos(node.args[0])
    return node


def _expand_sin(arg: IRNode) -> IRNode:
    """Expand ``Sin(arg)`` if ``arg`` is a sum or integer multiple."""
    # sin(a + b) → sin(a)cos(b) + cos(a)sin(b)
    if isinstance(arg, IRApply) and isinstance(arg.head, IRSymbol):
        name = arg.head.name
        if name == "Add" and len(arg.args) == 2:
            a, b = arg.args[0], arg.args[1]
            return IRApply(ADD, (
                IRApply(MUL, (IRApply(SIN, (a,)), IRApply(COS, (b,)))),
                IRApply(MUL, (IRApply(COS, (a,)), IRApply(SIN, (b,)))),
            ))
        if name == "Sub" and len(arg.args) == 2:
            a, b = arg.args[0], arg.args[1]
            return IRApply(SUB, (
                IRApply(MUL, (IRApply(SIN, (a,)), IRApply(COS, (b,)))),
                IRApply(MUL, (IRApply(COS, (a,)), IRApply(SIN, (b,)))),
            ))
        if name == "Mul":
            n, x = _extract_integer_multiple(arg)
            if n is not None and n >= 2:
                return _chebyshev_sin(n, x)
        if name == "Neg" and len(arg.args) == 1:
            # sin(-x) = -sin(x)
            return IRApply(IRSymbol("Neg"), (IRApply(SIN, (arg.args[0],)),))
    return IRApply(SIN, (arg,))


def _expand_cos(arg: IRNode) -> IRNode:
    """Expand ``Cos(arg)`` if ``arg`` is a sum or integer multiple."""
    if isinstance(arg, IRApply) and isinstance(arg.head, IRSymbol):
        name = arg.head.name
        if name == "Add" and len(arg.args) == 2:
            a, b = arg.args[0], arg.args[1]
            return IRApply(SUB, (
                IRApply(MUL, (IRApply(COS, (a,)), IRApply(COS, (b,)))),
                IRApply(MUL, (IRApply(SIN, (a,)), IRApply(SIN, (b,)))),
            ))
        if name == "Sub" and len(arg.args) == 2:
            a, b = arg.args[0], arg.args[1]
            return IRApply(ADD, (
                IRApply(MUL, (IRApply(COS, (a,)), IRApply(COS, (b,)))),
                IRApply(MUL, (IRApply(SIN, (a,)), IRApply(SIN, (b,)))),
            ))
        if name == "Mul":
            n, x = _extract_integer_multiple(arg)
            if n is not None and n >= 2:
                return _chebyshev_cos(n, x)
        if name == "Neg" and len(arg.args) == 1:
            # cos(-x) = cos(x)
            return IRApply(COS, (arg.args[0],))
    return IRApply(COS, (arg,))


def _extract_integer_multiple(
    mul_node: IRApply,
) -> tuple[int, IRNode] | tuple[None, None]:
    """If ``mul_node`` is ``Mul(n, x)`` for integer ``n``, return ``(n, x)``.

    Returns ``(None, None)`` otherwise.
    """
    if not (isinstance(mul_node.head, IRSymbol) and mul_node.head.name == "Mul"):
        return None, None
    if len(mul_node.args) != 2:
        return None, None
    a, b = mul_node.args
    if isinstance(a, IRInteger) and a.value > 0:
        return a.value, b
    if isinstance(b, IRInteger) and b.value > 0:
        return b.value, a
    return None, None


def _chebyshev_sin(n: int, x: IRNode) -> IRNode:
    """Expand ``sin(n·x)`` using the Chebyshev recurrence.

    ``sin(n·x) = 2·cos(x)·sin((n-1)·x) - sin((n-2)·x)``

    Base cases:
    - ``sin(0·x) = 0``
    - ``sin(1·x) = sin(x)``
    - ``sin(2·x) = 2·sin(x)·cos(x)``
    """
    if n == 0:
        return IRInteger(0)
    if n == 1:
        return IRApply(SIN, (x,))
    if n == 2:
        return IRApply(MUL, (IRInteger(2), IRApply(SIN, (x,)), IRApply(COS, (x,))))

    # Recurrence for n ≥ 3
    sin_n_minus_1 = _chebyshev_sin(n - 1, x)
    sin_n_minus_2 = _chebyshev_sin(n - 2, x)
    return IRApply(SUB, (
        IRApply(MUL, (IRInteger(2), IRApply(COS, (x,)), sin_n_minus_1)),
        sin_n_minus_2,
    ))


def _chebyshev_cos(n: int, x: IRNode) -> IRNode:
    """Expand ``cos(n·x)`` using the Chebyshev recurrence.

    ``cos(n·x) = 2·cos(x)·cos((n-1)·x) - cos((n-2)·x)``

    Base cases:
    - ``cos(0·x) = 1``
    - ``cos(1·x) = cos(x)``
    - ``cos(2·x) = cos²(x) - sin²(x)``
    """
    if n == 0:
        return IRInteger(1)
    if n == 1:
        return IRApply(COS, (x,))
    if n == 2:
        return IRApply(SUB, (
            IRApply(POW, (IRApply(COS, (x,)), IRInteger(2))),
            IRApply(POW, (IRApply(SIN, (x,)), IRInteger(2))),
        ))

    # Recurrence for n ≥ 3
    cos_n_minus_1 = _chebyshev_cos(n - 1, x)
    cos_n_minus_2 = _chebyshev_cos(n - 2, x)
    return IRApply(SUB, (
        IRApply(MUL, (IRInteger(2), IRApply(COS, (x,)), cos_n_minus_1)),
        cos_n_minus_2,
    ))
