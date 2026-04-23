"""Polynomial × asinh/acosh(linear) integration — Phase 13.

Integrates ``P(x) · asinh(ax+b)`` and ``P(x) · acosh(ax+b)`` for
``P ∈ Q[x]`` and ``a ∈ Q \\ {0}`` via integration by parts.

**asinh IBP:**

    u = asinh(ax+b),  dv = P(x) dx
    du = a/√((ax+b)²+1) dx,  v = Q(x) = ∫P dx

    ∫ P·asinh(ax+b) dx = Q(x)·asinh(ax+b) − a · ∫ Q(x)/√((ax+b)²+1) dx

**Residual via substitution t = ax+b:**

    a · ∫ Q(x)/√((ax+b)²+1) dx  =  ∫ Q̃(t)/√(t²+1) dt

where Q̃(t) = Q((t−b)/a) (same-degree polynomial in t).

**Reduction formula** for monomials:

    ∫ tⁿ/√(t²+1) dt = (1/n)·tⁿ⁻¹·√(t²+1) − (n−1)/n · ∫ tⁿ⁻²/√(t²+1) dt
    base: n=0 → asinh(t),  n=1 → √(t²+1)

    Note: the leading coefficient is POSITIVE (+1/n) and the recursive term
    is SUBTRACTED, unlike the asin case where it is −1/n and added.

By linearity:  ∫ Q̃(t)/√(t²+1) dt = A(t)·√(t²+1) + B(t)·asinh(t)

where A(t) and B(t) are computed by ``_sqrt_decompose``.

**Final result after back-substituting t = ax+b:**

    ∫ P(x)·asinh(ax+b) dx = [Q(x) − B(ax+b)]·asinh(ax+b) − A(ax+b)·√((ax+b)²+1)

**acosh IBP:**

    d/dx acosh(ax+b) = a/√((ax+b)²−1)  (same positive sign as asinh)

The residual integral has the same structure with √(t²−1) instead of √(t²+1).
The reduction formula is identical:

    ∫ tⁿ/√(t²−1) dt = (1/n)·tⁿ⁻¹·√(t²−1) − (n−1)/n · ∫ tⁿ⁻²/√(t²−1) dt
    base: n=0 → acosh(t),  n=1 → √(t²−1)

**Final result:**

    ∫ P(x)·acosh(ax+b) dx = [Q(x) − B(ax+b)]·acosh(ax+b) − A(ax+b)·√((ax+b)²−1)

The same ``_sqrt_decompose`` function handles both families because the
reduction formula is algebraically identical; the IR head (ASINH vs ACOSH)
and the SQRT argument ((ax+b)²+1 vs (ax+b)²−1) are the only differences.

See ``code/specs/phase13-hyperbolic.md`` for the full derivation and worked
examples.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ACOSH,
    ADD,
    ASINH,
    MUL,
    POW,
    SQRT,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.polynomial_bridge import from_polynomial, linear_to_ir

# Polynomial represented as a tuple of Fraction coefficients in ascending
# degree order: (c₀, c₁, …, cₙ) encodes c₀ + c₁·t + … + cₙ·tⁿ.
Poly = tuple[Fraction, ...]


# ---------------------------------------------------------------------------
# Public entry points
# ---------------------------------------------------------------------------


def asinh_poly_integral(
    poly: Poly,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return the IR for ``∫ poly(x) · asinh(ax+b) dx``.

    Pre-conditions (caller's responsibility):
    - ``a ≠ 0``
    - ``poly`` is non-empty with Fraction coefficients.

    The result is:
        [Q(x) − B(ax+b)] · asinh(ax+b) − A(ax+b) · √((ax+b)²+1)

    where Q = ∫poly dx and A, B are from the residual ∫ Q̃(t)/√(t²+1) dt.
    """
    p = _normalize(poly)
    if not p:
        return IRInteger(0)

    Q = _integrate_poly(p)
    A_x, B_x = _compute_AB_plus(Q, a, b)
    arg_ir = linear_to_ir(a, b, x_sym)

    Q_ir = from_polynomial(Q, x_sym)
    B_x_norm = _normalize(B_x)
    if B_x_norm:
        B_ir = from_polynomial(B_x_norm, x_sym)
        asinh_coef = IRApply(SUB, (Q_ir, B_ir))
    else:
        asinh_coef = Q_ir

    asinh_ir = IRApply(ASINH, (arg_ir,))
    main_term = IRApply(MUL, (asinh_coef, asinh_ir))

    # √((ax+b)²+1)
    sqrt_inner = IRApply(ADD, (IRApply(POW, (arg_ir, IRInteger(2))), IRInteger(1)))
    sqrt_ir = IRApply(SQRT, (sqrt_inner,))

    A_x_norm = _normalize(A_x)
    if A_x_norm:
        A_ir = from_polynomial(A_x_norm, x_sym)
        sqrt_term = IRApply(MUL, (A_ir, sqrt_ir))
        return IRApply(SUB, (main_term, sqrt_term))

    return main_term


def acosh_poly_integral(
    poly: Poly,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return the IR for ``∫ poly(x) · acosh(ax+b) dx``.

    Pre-conditions same as :func:`asinh_poly_integral`.

    The result is:
        [Q(x) − B(ax+b)] · acosh(ax+b) − A(ax+b) · √((ax+b)²−1)

    The reduction formula for the residual ∫ Q̃(t)/√(t²−1) dt is identical
    to the asinh case; only the SQRT argument changes.
    """
    p = _normalize(poly)
    if not p:
        return IRInteger(0)

    Q = _integrate_poly(p)
    A_x, B_x = _compute_AB_minus(Q, a, b)
    arg_ir = linear_to_ir(a, b, x_sym)

    Q_ir = from_polynomial(Q, x_sym)
    B_x_norm = _normalize(B_x)
    if B_x_norm:
        B_ir = from_polynomial(B_x_norm, x_sym)
        acosh_coef = IRApply(SUB, (Q_ir, B_ir))
    else:
        acosh_coef = Q_ir

    acosh_ir = IRApply(ACOSH, (arg_ir,))
    main_term = IRApply(MUL, (acosh_coef, acosh_ir))

    # √((ax+b)²−1)
    sqrt_inner = IRApply(SUB, (IRApply(POW, (arg_ir, IRInteger(2))), IRInteger(1)))
    sqrt_ir = IRApply(SQRT, (sqrt_inner,))

    A_x_norm = _normalize(A_x)
    if A_x_norm:
        A_ir = from_polynomial(A_x_norm, x_sym)
        sqrt_term = IRApply(MUL, (A_ir, sqrt_ir))
        return IRApply(SUB, (main_term, sqrt_term))

    return main_term


# ---------------------------------------------------------------------------
# Shared computation
# ---------------------------------------------------------------------------


def _compute_AB_plus(Q: Poly, a: Fraction, b: Fraction) -> tuple[Poly, Poly]:
    """Return ``(A_x, B_x)`` for the asinh residual (√(t²+1) family).

    Reduction: Iₙ = (1/n)·tⁿ⁻¹·√(t²+1) − (n−1)/n · Iₙ₋₂  (minus sign).
    """
    Q_tilde = _compose_to_t(Q, a, b)
    A_t, B_t = _sqrt_plus_decompose(Q_tilde)
    A_x = _poly_compose_linear(A_t, a, b)
    B_x = _poly_compose_linear(B_t, a, b)
    return A_x, B_x


def _compute_AB_minus(Q: Poly, a: Fraction, b: Fraction) -> tuple[Poly, Poly]:
    """Return ``(A_x, B_x)`` for the acosh residual (√(t²−1) family).

    Reduction: Iₙ = (1/n)·tⁿ⁻¹·√(t²−1) + (n−1)/n · Iₙ₋₂  (PLUS sign).

    The sign difference from the asinh case arises because
    d/dt √(t²−1) = t/√(t²−1), so the IBP integration-by-parts step yields
    n·Iₙ = tⁿ⁻¹·√(t²−1) + (n−1)·Iₙ₋₂  (instead of minus).
    """
    Q_tilde = _compose_to_t(Q, a, b)
    A_t, B_t = _sqrt_minus_decompose(Q_tilde)
    A_x = _poly_compose_linear(A_t, a, b)
    B_x = _poly_compose_linear(B_t, a, b)
    return A_x, B_x


# ---------------------------------------------------------------------------
# Polynomial helpers (mirrors asin_poly_integral.py)
# ---------------------------------------------------------------------------


def _normalize(p: Poly) -> Poly:
    """Strip trailing zeros; return empty tuple for the zero polynomial."""
    lst = list(p)
    while lst and lst[-1] == Fraction(0):
        lst.pop()
    return tuple(lst)


def _integrate_poly(p: Poly) -> Poly:
    """Polynomial antiderivative (constant = 0): aᵢxⁱ → aᵢ/(i+1)·xⁱ⁺¹."""
    if not p:
        return ()
    result: list[Fraction] = [Fraction(0)]
    for i, c in enumerate(p):
        result.append(Fraction(c) / Fraction(i + 1))
    return _normalize(tuple(result))


def _poly_mul(p: Poly, q: Poly) -> Poly:
    """Multiply two polynomials."""
    if not p or not q:
        return ()
    deg = len(p) + len(q) - 2
    result = [Fraction(0)] * (deg + 1)
    for i, ci in enumerate(p):
        for j, cj in enumerate(q):
            result[i + j] += ci * cj
    return _normalize(tuple(result))


def _poly_add(p: Poly, q: Poly) -> Poly:
    """Add two polynomials."""
    n = max(len(p), len(q))
    result = [Fraction(0)] * n
    for i, c in enumerate(p):
        result[i] += c
    for i, c in enumerate(q):
        result[i] += c
    return _normalize(tuple(result))


def _poly_scale(c: Fraction, p: Poly) -> Poly:
    """Multiply a polynomial by a scalar."""
    return _normalize(tuple(c * ci for ci in p))


def _compose_to_t(Q: Poly, a: Fraction, b: Fraction) -> Poly:
    """Compute Q((t−b)/a) as a t-polynomial using Horner composition.

    The substitution linear polynomial is [(−b/a), (1/a)] in ascending order.
    """
    if not Q:
        return ()
    sub: Poly = (-b / a, Fraction(1) / a)
    result: Poly = (Q[-1],)
    for c in reversed(Q[:-1]):
        result = _poly_add(_poly_mul(result, sub), (c,))
    return result


def _sqrt_plus_decompose(Q_tilde: Poly) -> tuple[Poly, Poly]:
    """Return ``(A, B)`` such that ∫ Q̃(t)/√(t²+1) dt = A(t)·√(t²+1) + B(t)·asinh(t).

    Reduction formula:  Iₙ = (1/n)·tⁿ⁻¹·√(t²+1) − (n−1)/n · Iₙ₋₂  (MINUS)
    Base: n=0 → asinh(t),  n=1 → √(t²+1).

    Derived by IBP with u=tⁿ⁻¹, dv=t/√(t²+1) dt:
        n·Iₙ = tⁿ⁻¹·√(t²+1) − (n−1)·Iₙ₋₂
    The minus comes from ∫ tⁿ⁻²·(t²+1)/√ = Iₙ + Iₙ₋₂, bringing Iₙ to LHS.
    """
    if not Q_tilde:
        return (), ()

    memo: dict[int, tuple[Poly, Poly]] = {}

    def _monomial(n: int) -> tuple[Poly, Poly]:
        if n in memo:
            return memo[n]
        if n == 0:
            result = (), (Fraction(1),)
        elif n == 1:
            result = (Fraction(1),), ()
        else:
            A_new: Poly = (Fraction(0),) * (n - 1) + (Fraction(1, n),)
            A_new = _normalize(A_new)
            A_rec, B_rec = _monomial(n - 2)
            coef = Fraction(n - 1, n)
            A_total = _poly_add(A_new, _poly_scale(-coef, A_rec))
            B_total = _poly_scale(-coef, B_rec)
            result = _normalize(A_total), _normalize(B_total)
        memo[n] = result
        return result

    A_total: Poly = ()
    B_total: Poly = ()
    for deg, coef in enumerate(Q_tilde):
        if coef == Fraction(0):
            continue
        A_n, B_n = _monomial(deg)
        A_total = _poly_add(A_total, _poly_scale(coef, A_n))
        B_total = _poly_add(B_total, _poly_scale(coef, B_n))

    return _normalize(A_total), _normalize(B_total)


def _sqrt_minus_decompose(Q_tilde: Poly) -> tuple[Poly, Poly]:
    """Return ``(A, B)`` such that ∫ Q̃(t)/√(t²−1) dt = A(t)·√(t²−1) + B(t)·acosh(t).

    Reduction formula:  Iₙ = (1/n)·tⁿ⁻¹·√(t²−1) + (n−1)/n · Iₙ₋₂  (PLUS)
    Base: n=0 → acosh(t),  n=1 → √(t²−1).

    Derived by IBP with u=tⁿ⁻¹, dv=t/√(t²−1) dt:
        n·Iₙ = tⁿ⁻¹·√(t²−1) + (n−1)·Iₙ₋₂
    The PLUS comes from ∫ tⁿ⁻²·(t²−1)/√ = Iₙ − Iₙ₋₂, so moving Iₙ to LHS
    gives the +sign on Iₙ₋₂.  This contrasts with the +1 case above.
    """
    if not Q_tilde:
        return (), ()

    memo: dict[int, tuple[Poly, Poly]] = {}

    def _monomial(n: int) -> tuple[Poly, Poly]:
        if n in memo:
            return memo[n]
        if n == 0:
            result = (), (Fraction(1),)
        elif n == 1:
            result = (Fraction(1),), ()
        else:
            A_new: Poly = (Fraction(0),) * (n - 1) + (Fraction(1, n),)
            A_new = _normalize(A_new)
            A_rec, B_rec = _monomial(n - 2)
            coef = Fraction(n - 1, n)
            # PLUS sign — opposite from _sqrt_plus_decompose.
            A_total = _poly_add(A_new, _poly_scale(coef, A_rec))
            B_total = _poly_scale(coef, B_rec)
            result = _normalize(A_total), _normalize(B_total)
        memo[n] = result
        return result

    A_total: Poly = ()
    B_total: Poly = ()
    for deg, coef in enumerate(Q_tilde):
        if coef == Fraction(0):
            continue
        A_n, B_n = _monomial(deg)
        A_total = _poly_add(A_total, _poly_scale(coef, A_n))
        B_total = _poly_add(B_total, _poly_scale(coef, B_n))

    return _normalize(A_total), _normalize(B_total)


def _poly_compose_linear(p: Poly, a: Fraction, b: Fraction) -> Poly:
    """Compute p(ax+b) as an x-polynomial using Horner composition.

    The substitution polynomial is [b, a] in ascending order.
    """
    if not p:
        return ()
    sub: Poly = (b, a)
    result: Poly = (p[-1],)
    for c in reversed(p[:-1]):
        result = _poly_add(_poly_mul(result, sub), (c,))
    return result


# ---------------------------------------------------------------------------
# IR helpers
# ---------------------------------------------------------------------------


def _frac_ir(f: Fraction) -> IRNode:
    """Convert a Fraction to IRInteger or IRRational."""
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


__all__ = ["acosh_poly_integral", "asinh_poly_integral"]
