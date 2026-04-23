"""Polynomial × asin/acos(linear) integration — Phase 12.

Integrates ``P(x) · asin(ax+b)`` and ``P(x) · acos(ax+b)`` for
``P ∈ Q[x]`` and ``a ∈ Q \\ {0}`` via integration by parts.

**asin IBP:**

    u = asin(ax+b),  dv = P(x) dx
    du = a/√(1−(ax+b)²) dx,  v = Q(x) = ∫P dx

    ∫ P·asin(ax+b) dx = Q(x)·asin(ax+b) − a · ∫ Q(x)/√(1−(ax+b)²) dx

**Residual via substitution t = ax+b:**

    a · ∫ Q(x)/√(1−(ax+b)²) dx  =  ∫ Q̃(t)/√(1−t²) dt

where Q̃(t) = Q((t−b)/a) (same-degree polynomial in t).

**Reduction formula** for monomials:

    ∫ tⁿ/√(1−t²) dt = −tⁿ⁻¹·√(1−t²)/n + (n−1)/n · ∫ tⁿ⁻²/√(1−t²) dt
    base: n=0 → asin(t),  n=1 → −√(1−t²)

By linearity:  ∫ Q̃(t)/√(1−t²) dt = A(t)·√(1−t²) + B(t)·asin(t)

where A(t) and B(t) are polynomials computed by ``_sqrt_integral_decompose``.

**Final results after back-substituting t = ax+b:**

    asin: [Q(x) − B(ax+b)] · asin(ax+b) − A(ax+b) · √(1−(ax+b)²)
    acos: Q(x) · acos(ax+b) + A(ax+b) · √(1−(ax+b)²) + B(ax+b) · asin(ax+b)

The acos result legitimately contains an asin term for deg(P) ≥ 1 because
d/dx acos = −d/dx asin, so acos IBP flips the sign of the residual and
the √ term also changes sign; the B·asin part survives as a separate term.

See ``code/specs/phase12-poly-asin-acos.md`` for the full derivation and
worked examples.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ACOS,
    ADD,
    ASIN,
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


def asin_poly_integral(
    poly: Poly,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return the IR for ``∫ poly(x) · asin(ax+b) dx``.

    Pre-conditions (caller's responsibility):
    - ``a ≠ 0``
    - ``poly`` is non-empty with Fraction coefficients.

    Always returns a closed-form IR.

    The result is:
        [Q(x) − B(ax+b)] · asin(ax+b) − A(ax+b) · √(1−(ax+b)²)

    where Q = ∫poly dx and A, B are computed from the residual integral
    ∫ Q̃(t)/√(1−t²) dt = A(t)·√(1−t²) + B(t)·asin(t).
    """
    p = _normalize(poly)
    if not p:
        return IRInteger(0)

    Q = _integrate_poly(p)
    A_x, B_x = _compute_AB(Q, a, b)

    arg_ir = linear_to_ir(a, b, x_sym)

    # asin coefficient: Q(x) − B(ax+b)
    # If B is zero, the coefficient is just Q(x).
    Q_ir = from_polynomial(Q, x_sym)
    B_x_norm = _normalize(B_x)
    if B_x_norm:
        B_ir = from_polynomial(B_x_norm, x_sym)
        asin_coef = IRApply(SUB, (Q_ir, B_ir))
    else:
        asin_coef = Q_ir

    asin_ir = IRApply(ASIN, (arg_ir,))
    main_term = IRApply(MUL, (asin_coef, asin_ir))

    # sqrt coefficient: A(ax+b) · √(1−(ax+b)²)
    A_x_norm = _normalize(A_x)
    sqrt_inner = IRApply(SUB, (IRInteger(1), IRApply(POW, (arg_ir, IRInteger(2)))))
    sqrt_ir = IRApply(SQRT, (sqrt_inner,))

    if A_x_norm:
        A_ir = from_polynomial(A_x_norm, x_sym)
        sqrt_term = IRApply(MUL, (A_ir, sqrt_ir))
        return IRApply(SUB, (main_term, sqrt_term))

    # A = 0 (can only happen for deg 0 poly): just the asin term.
    return main_term


def acos_poly_integral(
    poly: Poly,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return the IR for ``∫ poly(x) · acos(ax+b) dx``.

    Pre-conditions same as :func:`asin_poly_integral`.

    The result is:
        Q(x) · acos(ax+b) + A(ax+b) · √(1−(ax+b)²) + B(ax+b) · asin(ax+b)

    The sign flip from acos IBP (d/dx acos = −a/√(1−t²)) reverses the √
    term and turns the B·asin term into an addition rather than subtraction.
    When B = 0 (deg poly = 0) the asin term is omitted.
    """
    p = _normalize(poly)
    if not p:
        return IRInteger(0)

    Q = _integrate_poly(p)
    A_x, B_x = _compute_AB(Q, a, b)

    arg_ir = linear_to_ir(a, b, x_sym)

    # Main term: Q(x) · acos(ax+b)
    Q_ir = from_polynomial(Q, x_sym)
    acos_ir = IRApply(ACOS, (arg_ir,))
    main_term = IRApply(MUL, (Q_ir, acos_ir))

    sqrt_inner = IRApply(SUB, (IRInteger(1), IRApply(POW, (arg_ir, IRInteger(2)))))
    sqrt_ir = IRApply(SQRT, (sqrt_inner,))

    A_x_norm = _normalize(A_x)
    B_x_norm = _normalize(B_x)

    result: IRNode = main_term

    # Add A(ax+b) · √(1−(ax+b)²)
    if A_x_norm:
        A_ir = from_polynomial(A_x_norm, x_sym)
        sqrt_term = IRApply(MUL, (A_ir, sqrt_ir))
        result = IRApply(ADD, (result, sqrt_term))

    # Add B(ax+b) · asin(ax+b)  — only when B ≠ 0
    if B_x_norm:
        B_ir = from_polynomial(B_x_norm, x_sym)
        asin_ir = IRApply(ASIN, (arg_ir,))
        asin_term = IRApply(MUL, (B_ir, asin_ir))
        result = IRApply(ADD, (result, asin_term))

    return result


# ---------------------------------------------------------------------------
# Shared computation
# ---------------------------------------------------------------------------


def _compute_AB(
    Q: Poly,
    a: Fraction,
    b: Fraction,
) -> tuple[Poly, Poly]:
    """Return ``(A_x, B_x)`` such that

        ∫ Q̃(t)/√(1−t²) dt = A(t)·√(1−t²) + B(t)·asin(t)

    and then back-substituted as x-polynomials A(ax+b) and B(ax+b).

    Steps:
    1. Compose Q → Q̃(t) = Q((t−b)/a)   (change of variable)
    2. Decompose ∫ Q̃/√(1−t²) dt = A(t)·√ + B(t)·asin
    3. Compose A and B back: A_x(x) = A(ax+b), B_x(x) = B(ax+b)
    """
    Q_tilde = _compose_to_t(Q, a, b)
    A_t, B_t = _sqrt_integral_decompose(Q_tilde)
    A_x = _poly_compose_linear(A_t, a, b)
    B_x = _poly_compose_linear(B_t, a, b)
    return A_x, B_x


# ---------------------------------------------------------------------------
# Polynomial helpers
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
    """Compute Q((t−b)/a) as a t-polynomial.

    Uses Horner-like composition: substitute t → (t−b)/a into Q.
    The substitution polynomial is linear: [(−b/a), (1/a)] in ascending order.
    """
    if not Q:
        return ()
    # sub(t) = (t − b) / a = (−b/a) + (1/a)·t
    sub: Poly = (-b / a, Fraction(1) / a)
    # Horner: evaluate Q at sub using Horner's method in polynomial arithmetic.
    # Q(s) = c₀ + c₁·s + … + cₙ·sⁿ
    # Compute iteratively: result = cₙ, then result = result·sub + cₙ₋₁, …
    result: Poly = (Q[-1],)
    for c in reversed(Q[:-1]):
        result = _poly_add(_poly_mul(result, sub), (c,))
    return result


def _sqrt_integral_decompose(Q_tilde: Poly) -> tuple[Poly, Poly]:
    """Return ``(A, B)`` such that

        ∫ Q̃(t)/√(1−t²) dt = A(t)·√(1−t²) + B(t)·asin(t)

    Uses the reduction formula for each monomial, accumulated linearly:

        ∫ tⁿ/√(1−t²) dt = −tⁿ⁻¹/n · √(1−t²) + (n−1)/n · ∫ tⁿ⁻²/√(1−t²) dt

        base cases: n=0 → asin(t) [i.e. B₀=1, A=0]
                    n=1 → −√(1−t²) [i.e. A₋₁ monomial: const coeff −1, B=0]

    Returns coefficient tuples A(t), B(t) in ascending degree order.
    """
    if not Q_tilde:
        return (), ()

    # Memoise monomial results (A_coefs, B_coefs) for tⁿ/√(1−t²).
    memo: dict[int, tuple[Poly, Poly]] = {}

    def _monomial(n: int) -> tuple[Poly, Poly]:
        """∫ tⁿ/√(1−t²) dt = A_n(t)·√(1−t²) + B_n(t)·asin(t)."""
        if n in memo:
            return memo[n]
        if n == 0:
            # B = 1 (constant), A = 0.
            result = (), (Fraction(1),)
        elif n == 1:
            # A = −1 (constant coefficient of √), B = 0.
            result = (Fraction(-1),), ()
        else:
            # Reduction: ∫ tⁿ/√ = −tⁿ⁻¹/n·√ + (n−1)/n·∫ tⁿ⁻²/√
            # A contribution from −tⁿ⁻¹/n:  a monomial −1/n at degree n−1.
            A_new: Poly = (Fraction(0),) * (n - 1) + (Fraction(-1, n),)
            A_new = _normalize(A_new)
            A_rec, B_rec = _monomial(n - 2)
            coef = Fraction(n - 1, n)
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
    """Compute p(ax+b) as an x-polynomial.

    Uses Horner-like composition: substitute x → ax+b into p.
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


__all__ = ["asin_poly_integral", "acos_poly_integral"]
