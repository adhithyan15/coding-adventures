"""Polynomial × log(linear) integration — Phase 3e.

Integrates ``p(x) · log(a·x + b)`` for ``p ∈ Q[x]`` and ``a ∈ Q \\ {0}``
via integration by parts:

    ∫ p(x)·log(a·x + b) dx  =  P(x)·log(a·x + b)  −  ∫ P(x)·a/(a·x + b) dx

where ``P(x) = ∫ p(x) dx`` (polynomial antiderivative, constant = 0).

The residual integral reduces entirely to polynomial arithmetic via long
division of ``P(x)·a`` by ``(a·x + b)``:

    P(x)·a = Q(x)·(a·x + b) + r        (r = P(−b/a)·a ∈ Q)

    ∫ P(x)·a/(a·x + b) dx  =  S(x)  +  P(−b/a)·log(a·x + b)

where ``S(x) = ∫ Q(x) dx`` is a polynomial.  The final answer is:

    ∫ p(x)·log(a·x + b) dx  =  [P(x) − P(−b/a)] · log(a·x + b) − S(x)

The constant ``P(−b/a)`` is the log coefficient evaluated at the
singularity ``x = −b/a`` — it cancels the log divergence exactly.

See ``code/specs/phase3-transcendental.md`` for the full derivation and
worked examples.
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import (
    Polynomial,
    divmod_poly,
    multiply,
    normalize,
)
from symbolic_ir import LOG, MUL, SUB, IRApply, IRInteger, IRNode, IRSymbol

from symbolic_vm.polynomial_bridge import from_polynomial, linear_to_ir


def log_poly_integral(
    poly: Polynomial,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return the IR for ``∫ poly(x) · log(a·x + b) dx``.

    Pre-conditions (caller's responsibility):
    - ``a ≠ 0``
    - ``poly`` is a polynomial with ``Fraction`` coefficients.

    Always returns a closed-form IR.
    """
    p = tuple(Fraction(c) for c in normalize(poly))
    if not p:
        return IRInteger(0)

    # P(x) = ∫ p(x) dx  (polynomial antiderivative, constant = 0).
    P = _integrate_poly(p)

    # r/a = P(−b/a) — the constant correction to the log coefficient.
    root = -b / a
    r_over_a = _poly_eval_exact(P, root)   # = P(−b/a)

    # Q(x) such that P(x)·a = Q(x)·(a·x + b) + r.
    Pa = multiply(P, (a,))                      # P(x) · a
    Pa_minus_r = _subtract_constant(Pa, r_over_a * a)  # P(x)·a − r
    divisor: Polynomial = (b, a)                # constant-first: a·x + b
    Q_quot, _ = divmod_poly(Pa_minus_r, divisor)
    Q = normalize(Q_quot)

    # S(x) = ∫ Q(x) dx.
    S = _integrate_poly(Q) if Q else ()

    # log coefficient: P(x) − P(−b/a).
    log_coef = _subtract_constant(P, r_over_a)

    # Build IR: [P − P(−b/a)]·log(a·x + b) − S(x).
    log_arg = linear_to_ir(a, b, x_sym)
    log_ir = IRApply(LOG, (log_arg,))
    log_term = _poly_times_log(log_coef, log_ir, x_sym)

    if not normalize(S):
        return log_term

    s_ir = from_polynomial(S, x_sym)
    return IRApply(SUB, (log_term, s_ir))


# ---------------------------------------------------------------------------
# Private helpers
# ---------------------------------------------------------------------------


def _integrate_poly(p: Polynomial) -> Polynomial:
    """Polynomial antiderivative: ``aᵢ·xⁱ → aᵢ/(i+1)·xⁱ⁺¹``."""
    if not p:
        return ()
    result: list[Fraction] = [Fraction(0)]
    for i, c in enumerate(p):
        result.append(Fraction(c) / Fraction(i + 1))
    return normalize(tuple(result))


def _poly_eval_exact(p: Polynomial, x_val: Fraction) -> Fraction:
    """Evaluate polynomial ``p`` at ``x_val`` using Horner's method."""
    result = Fraction(0)
    for c in reversed(p):
        result = result * x_val + Fraction(c)
    return result


def _subtract_constant(p: Polynomial, c: Fraction) -> Polynomial:
    """Return ``p(x) − c`` as a ``Polynomial`` tuple."""
    if not p:
        return ((-c),) if c != 0 else ()
    lst = [Fraction(coef) for coef in p]
    lst[0] = lst[0] - c
    return normalize(tuple(lst))


def _poly_times_log(coef_poly: Polynomial, log_ir: IRNode, x_sym: IRSymbol) -> IRNode:
    """Build IR for ``coef_poly(x) · log_ir``."""
    n = normalize(coef_poly)
    if not n:
        return IRInteger(0)
    coef_ir = from_polynomial(n, x_sym)
    if coef_ir == IRInteger(1):
        return log_ir
    return IRApply(MUL, (coef_ir, log_ir))


__all__ = ["log_poly_integral"]
