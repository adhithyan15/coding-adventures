"""Special-value lookup table for sin, cos, tan at rational multiples of π.

For exact values we return IR expressions using ``IRInteger``, ``IRRational``,
and ``IRApply(SQRT, ...)`` where needed.

Supported exact values:
  sin/cos/tan at: 0, π/6, π/4, π/3, π/2, 2π/3, 3π/4, 5π/6, π,
                  7π/6, 5π/4, 4π/3, 3π/2, 5π/3, 7π/4, 11π/6, 2π
  and all their negatives.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import IRApply, IRInteger, IRNode, IRRational, IRSymbol

# Heads
SIN = IRSymbol("Sin")
COS = IRSymbol("Cos")
TAN = IRSymbol("Tan")
SQRT = IRSymbol("Sqrt")
MUL = IRSymbol("Mul")
NEG = IRSymbol("Neg")
DIV = IRSymbol("Div")

# Common exact values
ZERO = IRInteger(0)
ONE = IRInteger(1)
NEG_ONE = IRInteger(-1)


def _half() -> IRNode:
    return IRRational(1, 2)


def _neg_half() -> IRNode:
    return IRRational(-1, 2)


def _sqrt2_over_2() -> IRNode:
    """√2/2 as Mul(Sqrt(2), Rational(1,2))."""
    return IRApply(MUL, (IRApply(SQRT, (IRInteger(2),)), IRRational(1, 2)))


def _neg_sqrt2_over_2() -> IRNode:
    return IRApply(NEG, (_sqrt2_over_2(),))


def _sqrt3_over_2() -> IRNode:
    """√3/2 as Mul(Sqrt(3), Rational(1,2))."""
    return IRApply(MUL, (IRApply(SQRT, (IRInteger(3),)), IRRational(1, 2)))


def _neg_sqrt3_over_2() -> IRNode:
    return IRApply(NEG, (_sqrt3_over_2(),))


def _sqrt3() -> IRNode:
    return IRApply(SQRT, (IRInteger(3),))


def _neg_sqrt3() -> IRNode:
    return IRApply(NEG, (_sqrt3(),))


def _inv_sqrt3() -> IRNode:
    """1/√3 = √3/3 as Mul(Sqrt(3), Rational(1,3))."""
    return IRApply(MUL, (IRApply(SQRT, (IRInteger(3),)), IRRational(1, 3)))


# ---------------------------------------------------------------------------
# Lookup tables  (keyed by (function_name, numerator, denominator) of angle/π)
# ---------------------------------------------------------------------------
# The key is (func, p, q) meaning func(p*π/q).
# E.g. sin(π/6) → key = ("sin", 1, 6)


def _make_sin_table() -> dict[tuple[int, int], IRNode]:
    """Return a mapping ``(p, q) → sin(p·π/q)`` for the standard angles."""
    t: dict[tuple[int, int], IRNode] = {
        (0, 1): ZERO,          # sin(0) = 0
        (1, 6): _half(),       # sin(π/6) = 1/2
        (1, 4): _sqrt2_over_2(),  # sin(π/4) = √2/2
        (1, 3): _sqrt3_over_2(),  # sin(π/3) = √3/2
        (1, 2): ONE,            # sin(π/2) = 1
        (2, 3): _sqrt3_over_2(),  # sin(2π/3) = √3/2
        (3, 4): _sqrt2_over_2(),  # sin(3π/4) = √2/2
        (5, 6): _half(),       # sin(5π/6) = 1/2
        (1, 1): ZERO,           # sin(π) = 0
        (7, 6): _neg_half(),   # sin(7π/6) = -1/2
        (5, 4): _neg_sqrt2_over_2(),  # sin(5π/4) = -√2/2
        (4, 3): _neg_sqrt3_over_2(),  # sin(4π/3) = -√3/2
        (3, 2): NEG_ONE,        # sin(3π/2) = -1
        (5, 3): _neg_sqrt3_over_2(),  # sin(5π/3) = -√3/2
        (7, 4): _neg_sqrt2_over_2(),  # sin(7π/4) = -√2/2
        (11, 6): _neg_half(),  # sin(11π/6) = -1/2
        (2, 1): ZERO,           # sin(2π) = 0
    }
    return t


def _make_cos_table() -> dict[tuple[int, int], IRNode]:
    t: dict[tuple[int, int], IRNode] = {
        (0, 1): ONE,            # cos(0) = 1
        (1, 6): _sqrt3_over_2(),  # cos(π/6) = √3/2
        (1, 4): _sqrt2_over_2(),  # cos(π/4) = √2/2
        (1, 3): _half(),        # cos(π/3) = 1/2
        (1, 2): ZERO,           # cos(π/2) = 0
        (2, 3): _neg_half(),    # cos(2π/3) = -1/2
        (3, 4): _neg_sqrt2_over_2(),  # cos(3π/4) = -√2/2
        (5, 6): _neg_sqrt3_over_2(),  # cos(5π/6) = -√3/2
        (1, 1): NEG_ONE,        # cos(π) = -1
        (7, 6): _neg_sqrt3_over_2(),  # cos(7π/6) = -√3/2
        (5, 4): _neg_sqrt2_over_2(),  # cos(5π/4) = -√2/2
        (4, 3): _neg_half(),    # cos(4π/3) = -1/2
        (3, 2): ZERO,           # cos(3π/2) = 0
        (5, 3): _half(),        # cos(5π/3) = 1/2
        (7, 4): _sqrt2_over_2(),  # cos(7π/4) = √2/2
        (11, 6): _sqrt3_over_2(),  # cos(11π/6) = √3/2
        (2, 1): ONE,            # cos(2π) = 1
    }
    return t


def _make_tan_table() -> dict[tuple[int, int], IRNode]:
    t: dict[tuple[int, int], IRNode] = {
        (0, 1): ZERO,           # tan(0) = 0
        (1, 6): _inv_sqrt3(),   # tan(π/6) = 1/√3
        (1, 4): ONE,            # tan(π/4) = 1
        (1, 3): _sqrt3(),       # tan(π/3) = √3
        # tan(π/2) = undefined
        (2, 3): _neg_sqrt3(),   # tan(2π/3) = -√3
        (3, 4): NEG_ONE,        # tan(3π/4) = -1
        (5, 6): _neg_inv_sqrt3(),  # tan(5π/6) = -1/√3
        (1, 1): ZERO,           # tan(π) = 0
        (7, 6): _inv_sqrt3(),   # tan(7π/6) = 1/√3
        (5, 4): ONE,            # tan(5π/4) = 1
        (4, 3): _sqrt3(),       # tan(4π/3) = √3
        # tan(3π/2) = undefined
        (5, 3): _neg_sqrt3(),   # tan(5π/3) = -√3
        (7, 4): NEG_ONE,        # tan(7π/4) = -1
        (11, 6): _neg_inv_sqrt3(),  # tan(11π/6) = -1/√3
        (2, 1): ZERO,           # tan(2π) = 0
    }
    return t


def _neg_inv_sqrt3() -> IRNode:
    return IRApply(NEG, (_inv_sqrt3(),))


_SIN_TABLE = _make_sin_table()
_COS_TABLE = _make_cos_table()
_TAN_TABLE = _make_tan_table()

_TABLE: dict[str, dict[tuple[int, int], IRNode]] = {
    "Sin": _SIN_TABLE,
    "Cos": _COS_TABLE,
    "Tan": _TAN_TABLE,
}


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def lookup_special_value(func_name: str, arg: IRNode) -> IRNode | None:
    """Return the exact value of ``func_name(arg)`` if ``arg`` is a
    recognised rational multiple of π, otherwise ``None``.

    Recognises the following argument forms:
    - ``IRSymbol("%pi")``      → π/1
    - ``IRRational(p, q) * %pi``  → p·π/q  (Mul form)
    - ``IRInteger(n) * %pi``  → n·π/1  (Mul form)
    - ``IRApply(Neg, (%pi,))`` → -π   (→ use periodicity)
    """
    table = _TABLE.get(func_name)
    if table is None:
        return None

    frac = _arg_as_pi_fraction(arg)
    if frac is None:
        return None

    # Reduce modulo 2 (period is 2π → fraction of π period is 2)
    # frac is p/q where the angle is frac·π
    frac = frac % Fraction(2)  # 0 ≤ frac < 2
    p, q = frac.numerator, frac.denominator
    return table.get((p, q))


def _arg_as_pi_fraction(arg: IRNode) -> Fraction | None:
    """Return the coefficient c such that ``arg = c·π``, or ``None``."""
    PI = "%pi"

    # Bare %pi → coefficient 1
    if isinstance(arg, IRSymbol) and arg.name == PI:
        return Fraction(1)

    # Neg(%pi) → coefficient -1
    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Neg"
        and len(arg.args) == 1
        and isinstance(arg.args[0], IRSymbol)
        and arg.args[0].name == PI
    ):
        return Fraction(-1)

    # Mul(coefficient, %pi) or Mul(%pi, coefficient)
    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Mul"
        and len(arg.args) == 2
    ):
        a, b = arg.args[0], arg.args[1]
        # One arg should be %pi, the other should be a numeric coefficient
        if isinstance(b, IRSymbol) and b.name == PI:
            a, b = b, a  # swap so a is %pi
        if isinstance(a, IRSymbol) and a.name == PI:
            c = _to_fraction(b)
            if c is not None:
                return c

    # Neg(Mul(coefficient, %pi)) → negative coefficient
    if (
        isinstance(arg, IRApply)
        and isinstance(arg.head, IRSymbol)
        and arg.head.name == "Neg"
        and len(arg.args) == 1
    ):
        inner = _arg_as_pi_fraction(arg.args[0])
        if inner is not None:
            return -inner

    return None


def _to_fraction(node: IRNode) -> Fraction | None:
    """Convert a constant IR node to a Fraction, or None."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    return None
