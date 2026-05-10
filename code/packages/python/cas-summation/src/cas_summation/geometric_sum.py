"""Geometric series summation.

A *geometric series* has the form  Σ_{k=a}^{b} c · r^k  where the
base *r* and coefficient *c* are both independent of the summation index *k*.

Finite sum (b < ∞)
------------------
Using the standard formula for a geometric progression:

    Σ_{k=a}^{b} r^k  =  r^a · (r^(b−a+1) − 1) / (r − 1)     (r ≠ 1)

When r = 1 every term equals 1, so the sum is just (b − a + 1).

With a coefficient c:

    Σ_{k=a}^{b} c · r^k  =  c · r^a · (r^(b−a+1) − 1) / (r − 1)

Infinite sum (b = ∞)
--------------------
For |r| < 1 the geometric series converges:

    Σ_{k=a}^{∞} r^k  =  r^a / (1 − r)

This module returns the formal closed form without checking convergence,
matching historical MACSYMA's behaviour.  The caller is responsible for
ensuring the series converges.

Special case lo = 0, infinite:

    Σ_{k=0}^{∞} r^k  =  1 / (1 − r)          (r^0 = 1 absorbed)

Usage::

    from symbolic_ir import IRSymbol, IRRational, IRInteger
    from cas_summation.geometric_sum import geometric_sum_ir

    r = IRRational(1, 2)   # r = 1/2
    k = IRSymbol("k")
    # sum(1/2^k, k, 0, %inf)  →  1 / (1 - 1/2) = 2
    expr = geometric_sum_ir(
        coeff=IRInteger(1), base=r, lo=IRInteger(0), hi=None, is_infinite=True
    )
"""

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
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _int(n: int) -> IRNode:
    return IRInteger(n)


def _pow(base: IRNode, exp: IRNode) -> IRNode:
    return IRApply(POW, (base, exp))


def _mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _sub(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(SUB, (a, b))


def _add(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(ADD, (a, b))


def _neg(a: IRNode) -> IRNode:
    return IRApply(NEG, (a,))


# ---------------------------------------------------------------------------
# Public function
# ---------------------------------------------------------------------------


def geometric_sum_ir(
    coeff: IRNode,
    base: IRNode,
    lo: IRNode,
    hi: IRNode | None,
    is_infinite: bool,
) -> IRNode:
    """IR for  coeff · Σ_{k=lo}^{hi} base^k.

    Parameters
    ----------
    coeff:
        A constant (w.r.t. the index) scalar factor.  Use ``IRInteger(1)``
        when there is no coefficient.
    base:
        The geometric ratio *r* (constant w.r.t. the index).
    lo:
        Lower bound IR node (inclusive).
    hi:
        Upper bound IR node (inclusive).  Ignored when ``is_infinite=True``.
    is_infinite:
        If ``True``, build the infinite-series formula r^lo / (1 − r).
        The caller is responsible for verifying convergence (|r| < 1).

    Returns
    -------
    IRNode
        An IR tree representing the closed form of the sum.

    Derivation (finite)
    -------------------
    Let L = lo, B = base.

        Σ_{k=L}^{H} B^k = B^L + B^(L+1) + … + B^H
                        = B^L · (1 + B + … + B^(H−L))
                        = B^L · (B^(H−L+1) − 1) / (B − 1)

    Examples
    --------
    >>> from symbolic_ir import IRSymbol, IRRational, IRInteger
    >>> # sum((1/2)^k, k, 0, inf) = 1/(1 - 1/2) = 2
    >>> geometric_sum_ir(IRInteger(1), IRRational(1,2), IRInteger(0), None, True)
    """
    if is_infinite:
        # Formula: coeff · base^lo / (1 − base)
        one_minus_base = _sub(_int(1), base)
        # base^lo: simplify when lo is zero
        if isinstance(lo, IRInteger) and lo.value == 0:
            sum_part: IRNode = _div(_int(1), one_minus_base)
        else:
            sum_part = _div(_pow(base, lo), one_minus_base)
    else:
        # Finite formula: coeff · base^lo · (base^(hi−lo+1) − 1) / (base − 1)
        assert hi is not None, "hi must be provided for finite geometric sum"
        span_plus_1 = _add(_sub(hi, lo), _int(1))  # (hi − lo + 1)
        numerator = _sub(_pow(base, span_plus_1), _int(1))
        denominator = _sub(base, _int(1))
        ratio_part = _div(numerator, denominator)  # (r^n − 1)/(r − 1)
        base_lo = _pow(base, lo)
        sum_part = _mul(base_lo, ratio_part)

    # Apply the leading coefficient.
    if isinstance(coeff, IRInteger) and coeff.value == 1:
        return sum_part
    return _mul(coeff, sum_part)
