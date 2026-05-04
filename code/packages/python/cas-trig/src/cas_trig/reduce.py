"""TrigReduce: reduce powers of trig functions to multiple-angle form.

Applies the following reductions:
  - ``sin²(x)`` → ``(1 - cos(2x)) / 2``
  - ``cos²(x)`` → ``(1 + cos(2x)) / 2``
  - ``sin³(x)`` → ``(3·sin(x) - sin(3x)) / 4``
  - ``cos³(x)`` → ``(3·cos(x) + cos(3x)) / 4``
  - ``sin(x)·cos(x)`` → ``sin(2x) / 2``
  - General ``sinⁿ(x)`` and ``cosⁿ(x)`` for n ≤ 6 via hard-coded formulas.

For ``n > 6``, the expression is left unchanged (Phase 1 scope).
"""

from __future__ import annotations

from fractions import Fraction

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
    IRRational,
    IRSymbol,
)

SIN = IRSymbol("Sin")
COS = IRSymbol("Cos")

_MAX_ITER = 20


def trig_reduce(expr: IRNode) -> IRNode:
    """Reduce trig powers in ``expr`` to multiple-angle form.

    Applies power-reduction formulas recursively until stable, then
    canonicalises.
    """
    for _ in range(_MAX_ITER):
        prev = expr
        expr = _reduce_walk(expr)
        expr = canonical(expr)
        if expr == prev:
            break
    return expr


# ---------------------------------------------------------------------------
# Internal recursive walker
# ---------------------------------------------------------------------------


def _reduce_walk(node: IRNode) -> IRNode:
    """Recursively replace trig power expressions."""
    if not isinstance(node, IRApply):
        return node

    head_name = node.head.name if isinstance(node.head, IRSymbol) else ""

    # Recurse into args first
    new_args = tuple(_reduce_walk(a) for a in node.args)
    if new_args != node.args:
        node = IRApply(node.head, new_args)

    # Pow(Sin(x), n) or Pow(Cos(x), n)
    if head_name == "Pow" and len(node.args) == 2:
        base, exp = node.args
        if isinstance(exp, IRInteger) and exp.value >= 2:
            result = _reduce_pow(base, exp.value)
            if result is not None:
                return result

    # Mul(Sin(x), Cos(x)) → Sin(2x)/2
    if head_name == "Mul":
        result = _reduce_sin_cos_product(node.args)
        if result is not None:
            return result

    return node


def _reduce_pow(base: IRNode, n: int) -> IRNode | None:
    """Return the multiple-angle form of ``Sin(x)^n`` or ``Cos(x)^n``."""
    if not isinstance(base, IRApply):
        return None
    func_name = base.head.name if isinstance(base.head, IRSymbol) else ""
    if func_name not in ("Sin", "Cos"):
        return None
    if len(base.args) != 1:
        return None
    x = base.args[0]

    if func_name == "Sin":
        return _sin_power(x, n)
    return _cos_power(x, n)


# Power-reduction formulas for sin^n and cos^n (Phase 1: n ≤ 6)

def _sin_power(x: IRNode, n: int) -> IRNode | None:
    """Return exact multiple-angle form for ``sin^n(x)``."""
    if n == 2:
        # sin²(x) = (1 - cos(2x)) / 2
        return _frac(
            IRApply(SUB, (IRInteger(1), _cos_nx(2, x))),
            2,
        )
    if n == 3:
        # sin³(x) = (3 sin(x) - sin(3x)) / 4
        return _frac(
            IRApply(SUB, (
                IRApply(MUL, (IRInteger(3), IRApply(SIN, (x,)))),
                _sin_nx(3, x),
            )),
            4,
        )
    if n == 4:
        # sin⁴(x) = (3 - 4cos(2x) + cos(4x)) / 8
        return _frac(
            IRApply(ADD, (
                IRApply(SUB, (
                    IRInteger(3),
                    IRApply(MUL, (IRInteger(4), _cos_nx(2, x))),
                )),
                _cos_nx(4, x),
            )),
            8,
        )
    if n == 5:
        # sin⁵(x) = (10 sin(x) - 5 sin(3x) + sin(5x)) / 16
        return _frac(
            IRApply(ADD, (
                IRApply(SUB, (
                    IRApply(MUL, (IRInteger(10), IRApply(SIN, (x,)))),
                    IRApply(MUL, (IRInteger(5), _sin_nx(3, x))),
                )),
                _sin_nx(5, x),
            )),
            16,
        )
    if n == 6:
        # sin⁶(x) = (10 - 15cos(2x) + 6cos(4x) - cos(6x)) / 32
        return _frac(
            IRApply(SUB, (
                IRApply(ADD, (
                    IRApply(SUB, (
                        IRInteger(10),
                        IRApply(MUL, (IRInteger(15), _cos_nx(2, x))),
                    )),
                    IRApply(MUL, (IRInteger(6), _cos_nx(4, x))),
                )),
                _cos_nx(6, x),
            )),
            32,
        )
    return None  # n > 6 — Phase 2


def _cos_power(x: IRNode, n: int) -> IRNode | None:
    """Return exact multiple-angle form for ``cos^n(x)``."""
    if n == 2:
        # cos²(x) = (1 + cos(2x)) / 2
        return _frac(
            IRApply(ADD, (IRInteger(1), _cos_nx(2, x))),
            2,
        )
    if n == 3:
        # cos³(x) = (3 cos(x) + cos(3x)) / 4
        return _frac(
            IRApply(ADD, (
                IRApply(MUL, (IRInteger(3), IRApply(COS, (x,)))),
                _cos_nx(3, x),
            )),
            4,
        )
    if n == 4:
        # cos⁴(x) = (3 + 4cos(2x) + cos(4x)) / 8
        return _frac(
            IRApply(ADD, (
                IRApply(ADD, (
                    IRInteger(3),
                    IRApply(MUL, (IRInteger(4), _cos_nx(2, x))),
                )),
                _cos_nx(4, x),
            )),
            8,
        )
    if n == 5:
        # cos⁵(x) = (10 cos(x) + 5 cos(3x) + cos(5x)) / 16
        return _frac(
            IRApply(ADD, (
                IRApply(ADD, (
                    IRApply(MUL, (IRInteger(10), IRApply(COS, (x,)))),
                    IRApply(MUL, (IRInteger(5), _cos_nx(3, x))),
                )),
                _cos_nx(5, x),
            )),
            16,
        )
    if n == 6:
        # cos⁶(x) = (10 + 15cos(2x) + 6cos(4x) + cos(6x)) / 32
        return _frac(
            IRApply(ADD, (
                IRApply(ADD, (
                    IRApply(ADD, (
                        IRInteger(10),
                        IRApply(MUL, (IRInteger(15), _cos_nx(2, x))),
                    )),
                    IRApply(MUL, (IRInteger(6), _cos_nx(4, x))),
                )),
                _cos_nx(6, x),
            )),
            32,
        )
    return None  # n > 6 — Phase 2


def _reduce_sin_cos_product(
    args: tuple[IRNode, ...],
) -> IRNode | None:
    """Detect ``Mul(Sin(x), Cos(x))`` and return ``Sin(2x)/2``.

    Works for ``Mul`` with exactly two args (Sin and Cos of the same arg).
    Also handles scalar multiples like ``Mul(k, Sin(x), Cos(x))``.
    """
    if len(args) < 2:
        return None

    # Extract scalar and trig parts
    scalar: IRNode | None = None
    sin_arg: IRNode | None = None
    cos_arg: IRNode | None = None
    other: list[IRNode] = []

    for a in args:
        if isinstance(a, IRApply) and isinstance(a.head, IRSymbol):
            if a.head.name == "Sin" and len(a.args) == 1:
                if sin_arg is None:
                    sin_arg = a.args[0]
                    continue
            elif a.head.name == "Cos" and len(a.args) == 1:
                if cos_arg is None:
                    cos_arg = a.args[0]
                    continue
        other.append(a)

    if sin_arg is None or cos_arg is None:
        return None
    if sin_arg != cos_arg:
        return None  # Different arguments — cannot reduce

    # sin(x)·cos(x) = sin(2x)/2
    x = sin_arg
    sin_2x_half: IRNode = _frac(_sin_nx(2, x), 2)

    if not other:
        return sin_2x_half
    # Scalar coefficient: k · sin(x) · cos(x) = k · sin(2x)/2
    return IRApply(MUL, (*other, sin_2x_half))


# ---------------------------------------------------------------------------
# Helper constructors
# ---------------------------------------------------------------------------


def _sin_nx(n: int, x: IRNode) -> IRNode:
    """Return ``Sin(n·x)``."""
    return IRApply(SIN, (IRApply(MUL, (IRInteger(n), x)),))


def _cos_nx(n: int, x: IRNode) -> IRNode:
    """Return ``Cos(n·x)``."""
    return IRApply(COS, (IRApply(MUL, (IRInteger(n), x)),))


def _frac(numerator: IRNode, denominator: int) -> IRNode:
    """Return ``Mul(numerator, Rational(1, denominator))``."""
    return IRApply(MUL, (numerator, IRRational(1, denominator)))
