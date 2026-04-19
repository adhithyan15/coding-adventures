"""Hermite reduction — the rational part of rational-function integration.

Given a proper rational function ``N/D`` over Q (``deg N < deg D``),
Hermite reduction splits the antiderivative into two pieces:

    ∫ N/D dx  =  A/B        (the rational part, closed form)
               + ∫ C/E dx   (the log part, with E squarefree)

Any rational function has a rational part and a log part; Hermite
produces the rational part in closed form without ever needing to
factor ``D`` into irreducibles over Q. That matters — factoring is
expensive and only partially solvable over Q, whereas squarefree
factorization is cheap and complete (see ``polynomial.squarefree``).

After Hermite, the residual integrand ``C/E`` has a **squarefree**
denominator. The antiderivative of ``C/E`` is a sum of logs
(``c_i · log v_i``) — producing it in closed form is Rothstein–Trager's
job. For now the handler emits ``Integrate(C/E, x)`` unchanged.

The textbook reference is Bronstein, *Symbolic Integration I*,
Chapter 2. See ``code/specs/hermite-reduction.md`` for the scoped
spec and the worked out algorithm.
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import (
    Polynomial,
    add,
    deriv,
    divide,
    divmod_poly,
    extended_gcd,
    monic,
    multiply,
    normalize,
    squarefree,
    subtract,
)

Rational = tuple[Polynomial, Polynomial]


def hermite_reduce(num: Polynomial, den: Polynomial) -> tuple[Rational, Rational]:
    """Reduce ``num/den`` into a rational part plus a squarefree-denom log integrand.

    Returns ``((rat_num, rat_den), (log_num, log_den))`` where

    - ``rat_num / rat_den`` is the rational part of the antiderivative
      (so ``d/dx(rat_num / rat_den)`` is the "removable" piece of the
      integrand). If there is no rational part, ``rat_num = ()``.
    - ``log_num / log_den`` is the residual integrand; ``log_den`` is
      guaranteed squarefree. Its antiderivative is a sum of logs.

    Pre-condition: ``deg num < deg den``. The caller (``_integrate_rational``
    in the handler) splits off the polynomial part with ``divmod_poly``
    before calling.

    The denominator is normalised to monic internally — the leading
    coefficient is folded into the numerator first, so every
    polynomial manipulated inside this function is monic. That keeps
    :func:`polynomial.squarefree` and :func:`polynomial.extended_gcd`
    on their well-defined path.
    """
    if not normalize(den):
        raise ValueError("hermite_reduce: denominator is zero")

    # Normalise to monic denominator. Any leading coefficient of ``den``
    # is absorbed into ``num`` via a scalar divide. ``num`` might mix
    # Fractions and ints coming in; we coerce through ``Fraction`` once.
    lc = Fraction(den[-1])
    if lc != 1:
        den = monic(den)
        num = _poly_scale(num, Fraction(1) / lc)

    rat_num: Polynomial = ()
    rat_den: Polynomial = (Fraction(1),)

    # Peel one power of one squarefree factor per iteration. We
    # re-run ``squarefree`` each pass because the denominator changes
    # after every peel — after taking v^m down to v^(m-1), the layer
    # structure shifts. Re-running squarefree is O(deg²) per call and
    # the outer loop bound is Σ (m_i − 1) ≤ deg den, so the total is
    # comfortably polynomial for any realistic integrand.
    while True:
        factors = squarefree(den)
        # The highest multiplicity m such that factors[m-1] has degree > 0.
        m = 0
        for i in range(len(factors) - 1, -1, -1):
            if _degree(factors[i]) > 0:
                m = i + 1
                break
        if m <= 1:
            # den is squarefree (or constant). We're done peeling.
            break

        v = factors[m - 1]
        v_pow_m = _pow_poly(v, m)
        u = divide(den, v_pow_m)
        v_prime = deriv(v)
        uvp = multiply(u, v_prime)

        # Solve B·(u·v') + C·v = num with deg B < deg v. ``v`` is
        # squarefree so gcd(v, v') = 1; ``u`` is coprime to ``v`` by
        # construction; so gcd(u·v', v) = 1 — extended_gcd returns a
        # constant. Scale the cofactors by the inverse of that constant
        # and by ``num`` to get the particular solution we need.
        g, s_cof, t_cof = extended_gcd(uvp, v)
        g_const = Fraction(g[0])
        inv = Fraction(1) / g_const
        s_cof = _poly_scale(s_cof, inv)
        t_cof = _poly_scale(t_cof, inv)

        # (s_cof · num)·uvp + (t_cof · num)·v = num. Then reduce the
        # B-cofactor mod v to enforce ``deg B < deg v`` and push the
        # quotient into C (so the identity still holds).
        B_full = multiply(s_cof, num)
        C_full = multiply(t_cof, num)
        B_quot, B = divmod_poly(B_full, v)
        C = add(C_full, multiply(B_quot, uvp))

        # One step of Hermite: subtracts d/dx(-B / ((m-1)·v^(m-1)))
        # from the integrand.  The emitted rational piece is
        # -B / ((m-1) · v^(m-1)). The residual integrand is
        # ((m-1)·C + u·B') / ((m-1) · u · v^(m-1)); we push the
        # 1/(m-1) scalar into the numerator so the denominator
        # structure stays clean.
        scale = Fraction(1, m - 1)
        A = _poly_scale(B, -scale)
        v_pow_m1 = _pow_poly(v, m - 1)

        # Accumulate rational part: rat + A/v^(m-1).
        rat_num = add(multiply(rat_num, v_pow_m1), multiply(A, rat_den))
        rat_den = multiply(rat_den, v_pow_m1)

        # New integrand.
        B_prime = deriv(B)
        num = add(C, _poly_scale(multiply(u, B_prime), scale))
        den = multiply(u, v_pow_m1)

    return ((normalize(rat_num), normalize(rat_den)), (normalize(num), normalize(den)))


# ---------------------------------------------------------------------------
# Local helpers
# ---------------------------------------------------------------------------


def _degree(p: Polynomial) -> int:
    """Degree with the empty tuple as degree −1 (matches polynomial.degree)."""
    n = normalize(p)
    return len(n) - 1


def _pow_poly(p: Polynomial, n: int) -> Polynomial:
    """Repeated multiply. ``n`` is always small (human integrands)."""
    result: Polynomial = (Fraction(1),)
    for _ in range(n):
        result = multiply(result, p)
    return result


def _poly_scale(p: Polynomial, c: Fraction) -> Polynomial:
    """Multiply every coefficient by the scalar ``c``. Keeps Fraction exact."""
    return normalize(tuple(c * Fraction(coef) for coef in p))


# Re-exported so callers don't need to reach into ``polynomial`` just to
# run the correctness check used in tests.
__all__ = ["hermite_reduce", "Rational", "subtract"]
