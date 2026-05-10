"""Polynomial / power-of-index summation using Faulhaber's formula.

Faulhaber's formula gives the exact closed form for the "power sum":

    S(n, m) = Σ_{k=1}^{n} k^m

as a polynomial of degree (m+1) in n.  The first six formulas (m = 0…5)
are hard-coded here as IR-tree builders because they appear so frequently
in practice.

General-bounds reduction
------------------------
For a sum with lower bound *a* and upper bound *b*:

    Σ_{k=a}^{b} k^m  =  S(b, m)  −  S(a−1, m)

When a = 1 the second term vanishes: S(0, m) = 0 for every m ≥ 0.
When a = 0 the second term is S(−1, m), which is 0 for odd m and −1 for
even m (not needed — we handle lo=0 via the same formula because 0^m=0
for m≥1, and for m=0 we add 1 to account for the k=0 term).

Usage::

    from fractions import Fraction
    from cas_summation.poly_sum import poly_sum_ir
    from symbolic_ir import IRSymbol

    n = IRSymbol("n")
    # Σ_{k=1}^{n} k^2  →  IR for n*(n+1)*(2*n+1)/6
    expr = poly_sum_ir(m=2, coeff=Fraction(1), lo_val=1, hi=n)
"""

from fractions import Fraction

from symbolic_ir import (
    ADD,
    DIV,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
)

# ---------------------------------------------------------------------------
# IR literal helpers
# ---------------------------------------------------------------------------


def _int(n: int) -> IRNode:
    """Lift a Python int to an IR integer literal."""
    return IRInteger(n)


def _frac(c: Fraction) -> IRNode:
    """Lift a Fraction to its canonical IR literal."""
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def _add(*args: IRNode) -> IRNode:
    """Build a left-associative chain of ADD nodes."""
    result = args[0]
    for a in args[1:]:
        result = IRApply(ADD, (result, a))
    return result


def _mul(*args: IRNode) -> IRNode:
    """Build a left-associative chain of MUL nodes."""
    result = args[0]
    for a in args[1:]:
        result = IRApply(MUL, (result, a))
    return result


def _div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _pow(base: IRNode, exp: int) -> IRNode:
    return IRApply(POW, (base, _int(exp)))


def _neg(a: IRNode) -> IRNode:
    return IRApply(NEG, (a,))


def _sub(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(SUB, (a, b))


# ---------------------------------------------------------------------------
# S(n, m): Faulhaber closed forms  Σ_{k=1}^n k^m
# ---------------------------------------------------------------------------
#
# Each function builds an IR tree that evaluates to the sum when *n* is
# substituted.  The formulas are verified by differencing (S(n,m) −
# S(n−1,m) = n^m) and by spot-checking concrete values:
#
#   S(4, 0) = 4          formula: n = 4 ✓
#   S(4, 1) = 10         formula: 4·5/2 = 10 ✓
#   S(4, 2) = 30         formula: 4·5·9/6 = 30 ✓
#   S(4, 3) = 100        formula: (4·5/2)^2 = 100 ✓
#   S(4, 4) = 354        formula: 4·5·9·(3·16+3·4−1)/30 = 4·5·9·55/30 = 9900/30... wait
#             actual: 1+16+81+256=354
#             4·5·9·(48+12-1)/30 = 4·5·9·59/30 = 10620/30 = 354 ✓
#   S(4, 5) = 1300       formula: 16·25·(32+8-1)/12 = 400·39/12 = 15600/12 = 1300 ✓


def _S0(n: IRNode) -> IRNode:
    """Σ_{k=1}^n k^0 = n."""
    return n


def _S1(n: IRNode) -> IRNode:
    """Σ_{k=1}^n k^1 = n·(n+1)/2."""
    # n·(n+1) / 2
    return _div(_mul(n, _add(n, _int(1))), _int(2))


def _S2(n: IRNode) -> IRNode:
    """Σ_{k=1}^n k^2 = n·(n+1)·(2n+1)/6."""
    two_n_plus_1 = _add(_mul(_int(2), n), _int(1))
    return _div(_mul(n, _mul(_add(n, _int(1)), two_n_plus_1)), _int(6))


def _S3(n: IRNode) -> IRNode:
    """Σ_{k=1}^n k^3 = [n·(n+1)/2]^2."""
    half = _div(_mul(n, _add(n, _int(1))), _int(2))
    return _pow(half, 2)


def _S4(n: IRNode) -> IRNode:
    """Σ_{k=1}^n k^4 = n·(n+1)·(2n+1)·(3n^2+3n−1) / 30."""
    #   (3n^2 + 3n − 1)
    inner = _sub(_add(_mul(_int(3), _pow(n, 2)), _mul(_int(3), n)), _int(1))
    two_n_plus_1 = _add(_mul(_int(2), n), _int(1))
    return _div(
        _mul(n, _mul(_add(n, _int(1)), _mul(two_n_plus_1, inner))),
        _int(30),
    )


def _S5(n: IRNode) -> IRNode:
    """Σ_{k=1}^n k^5 = n^2·(n+1)^2·(2n^2+2n−1) / 12."""
    #   (2n^2 + 2n − 1)
    inner = _sub(_add(_mul(_int(2), _pow(n, 2)), _mul(_int(2), n)), _int(1))
    return _div(
        _mul(_pow(n, 2), _mul(_pow(_add(n, _int(1)), 2), inner)),
        _int(12),
    )


# Map exponent → Faulhaber builder.
_FAULHABER: dict[int, object] = {
    0: _S0,
    1: _S1,
    2: _S2,
    3: _S3,
    4: _S4,
    5: _S5,
}


def faulhaber_ir(m: int, n: IRNode) -> IRNode | None:
    """IR tree for Σ_{k=1}^n k^m using Faulhaber's formula, or None if m>5.

    Parameters
    ----------
    m:
        Non-negative integer exponent.  Supported range: 0 ≤ m ≤ 5.
    n:
        An IR node representing the upper bound.  May be symbolic or
        a concrete integer node.

    Returns
    -------
    IRNode | None
        The Faulhaber polynomial in *n*, or ``None`` when m > 5.

    Examples
    --------
    >>> from symbolic_ir import IRSymbol
    >>> n = IRSymbol("n")
    >>> faulhaber_ir(1, n)   # n*(n+1)/2
    IRApply(Div, (IRApply(Mul, (n, IRApply(Add, (n, 1)))), 2))
    """
    if m < 0 or m > 5:
        return None
    return _FAULHABER[m](n)  # type: ignore[operator]


# ---------------------------------------------------------------------------
# poly_sum_ir: Σ_{k=lo}^{hi} coeff·k^m
# ---------------------------------------------------------------------------


def poly_sum_ir(
    m: int,
    coeff: Fraction,
    lo_val: int,
    hi: IRNode,
) -> IRNode | None:
    """IR for ``coeff * Σ_{k=lo}^{hi} k^m`` using Faulhaber, or None.

    This handles the common case where the lower bound *lo* is a small
    concrete integer and the upper bound *hi* is symbolic (e.g. ``n``).
    If Faulhaber can't handle the exponent (m > 5) the function returns None.

    The reduction formula is:

        Σ_{k=lo}^{hi} k^m  =  S(hi, m)  −  S(lo−1, m)

    For lo=1: S(lo−1, m) = S(0, m) = 0 (because all formulas vanish at n=0).
    For lo=0: Σ_{k=0}^{hi} k^m = S(hi, m) + 0^m.
              0^0 = 1, 0^m = 0 for m≥1 — so Σ_{k=0} = S(hi, m) for m≥1,
              and S(hi, 0) + 1 for m=0.

    Parameters
    ----------
    m:
        The exponent.  Must be 0 ≤ m ≤ 5.
    coeff:
        A rational scalar multiplier.
    lo_val:
        Concrete lower bound as a Python int (0 or 1 are most common).
    hi:
        IR node for the upper bound.

    Returns
    -------
    IRNode | None
    """
    s_hi = faulhaber_ir(m, hi)
    if s_hi is None:
        return None

    # Compute S(lo−1, m) at the concrete lower endpoint.
    # S is defined as Σ_{k=1}^n k^m, so S(0, m) = 0 always, and
    # S(-1, m) = 0^m (the k=0 term is not in S, so S(-1) = 0 as well
    # for the reduction).
    lo_minus_1 = lo_val - 1

    if lo_minus_1 <= 0:
        # S(0, m) = 0 and S(-1, m) = 0 for our convention: the Faulhaber
        # S sums from k=1, so any upper bound ≤ 0 gives 0.
        s_lo_minus_1: IRNode | None = None  # sentinel for "zero"
    else:
        # lo > 1: evaluate S(lo-1, m) with the concrete integer.
        s_lo_minus_1 = faulhaber_ir(m, _int(lo_minus_1))

    # When lo=0 and m=0 we must add 1 (the k=0 term: 0^0=1).
    # When lo=0 and m≥1 the k=0 term is 0, no correction needed.
    extra_zero_term: IRNode | None = None
    if lo_val == 0 and m == 0:
        extra_zero_term = _int(1)

    # Build the difference S(hi) − S(lo−1).
    diff = s_hi if s_lo_minus_1 is None else _sub(s_hi, s_lo_minus_1)

    if extra_zero_term is not None:
        diff = _add(diff, extra_zero_term)

    if coeff == Fraction(1):
        return diff
    return _mul(_frac(coeff), diff)
