"""Quartic-equation solving via Ferrari's method.

For ``a x⁴ + b x³ + c x² + d x + e = 0`` with rational coefficients:

1. **Rational-root theorem**: try all ±p/q candidates first.  When a
   rational root is found, deflate to a cubic and recurse through
   :func:`~cas_solve.cubic.solve_cubic`.

2. **Ferrari's method** (fallback when no rational root exists):
   depress to ``t⁴ + pt² + qt + r = 0`` via ``x = t − b/(4a)``, then
   introduce an auxiliary variable ``m`` satisfying a *resolvent cubic*.
   The resolvent cubic is solved via :func:`~cas_solve.cubic.solve_cubic`
   and the quartic factors into two quadratics.  Both quadratics are then
   solved via :func:`~cas_solve.quadratic.solve_quadratic`.

   If the resolvent cubic yields no usable real root (rational or
   Cardano-expressible), the quartic is returned as ``[]`` (unevaluated).
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import IRNode

from cas_solve.cubic import _dedup_and_sort, _eval_cubic, _fraction_to_ir, solve_cubic
from cas_solve.cubic import (
    _divisors,
    _fraction_lcm,
)
from cas_solve.quadratic import solve_quadratic


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def solve_quartic(
    a: Fraction, b: Fraction, c: Fraction, d: Fraction, e: Fraction
) -> list[IRNode] | str:
    """Solve ``a x⁴ + b x³ + c x² + d x + e = 0`` over Q (complex if needed).

    Returns a list of IR roots (up to 4 elements) or an empty list when
    the algorithm cannot produce a closed-form answer.
    """
    if a == 0:
        return solve_cubic(b, c, d, e)

    # ------------------------------------------------------------------
    # Step 1: rational root theorem
    # ------------------------------------------------------------------
    r = _find_rational_root_quartic(a, b, c, d, e)
    if r is not None:
        # Deflate by (x - r) to get a cubic.
        b2 = b + a * r
        c2 = c + b2 * r
        d2 = d + c2 * r
        remainder = e + d2 * r
        if remainder == 0:
            cubic_roots = solve_cubic(a, b2, c2, d2)
            r_ir = _fraction_to_ir(r)
            if isinstance(cubic_roots, str):
                return [r_ir]
            return _dedup_and_sort([r_ir] + cubic_roots)

    # ------------------------------------------------------------------
    # Step 2: Ferrari's method — depress the quartic
    # ------------------------------------------------------------------
    # Substitute x = t - b/(4a) to eliminate the cubic term.
    A = Fraction(1)  # monic after dividing by a
    B = Fraction(0)  # depressed (no t³ term)
    p = c / a - 3 * b ** 2 / (8 * a ** 2)
    q = b ** 3 / (8 * a ** 3) - b * c / (2 * a ** 2) + d / a
    r_coef = (
        -3 * b ** 4 / (256 * a ** 4)
        + b ** 2 * c / (16 * a ** 3)
        - b * d / (4 * a ** 2)
        + e / a
    )
    shift = -b / (4 * a)

    # Depressed quartic: t⁴ + p·t² + q·t + r_coef = 0
    if q == 0:
        # Biquadratic: t⁴ + p·t² + r_coef = 0 → let u = t²
        u_roots = solve_quadratic(Fraction(1), p, r_coef)
        if isinstance(u_roots, str):
            return []
        roots: list[IRNode] = []
        for u_root_ir in u_roots:
            # t = ±√u — we need to handle symbolically.
            # For rational u_roots, just use nested sqrt
            from symbolic_ir import IRApply, SQRT, NEG
            t_pos = IRApply(SQRT, (u_root_ir,))
            t_neg = IRApply(NEG, (t_pos,))
            from cas_solve.cubic import _add_shift
            roots.append(_add_shift(t_pos, shift))
            roots.append(_add_shift(t_neg, shift))
        return _dedup_and_sort(roots)

    # General Ferrari: resolvent cubic is
    # m³ + (5p/2)m² + (2p² - r_coef)m + (p³/2 - p·r_coef/2 - q²/8) = 0
    # (one common form; we use the Euler-Lagrange depressed resolvent)
    #
    # Resolvent cubic: 8m³ + 8p·m² + (2p² - 8r_coef)m - q² = 0
    ra = Fraction(8)
    rb = Fraction(8) * p
    rc = 2 * p ** 2 - 8 * r_coef
    rd_coef = -(q ** 2)

    resolvent_roots = solve_cubic(ra, rb, rc, rd_coef)
    if isinstance(resolvent_roots, str) or not resolvent_roots:
        return []

    # Find a rational (or at least IRInteger/IRRational) real root of the resolvent.
    from symbolic_ir import IRInteger, IRRational
    m_ir: IRNode | None = None
    for root_ir in resolvent_roots:
        if isinstance(root_ir, (IRInteger, IRRational)):
            m_ir = root_ir
            break

    if m_ir is None:
        # No usable rational root — return unevaluated.
        return []

    # Extract m as a Fraction.
    if isinstance(m_ir, IRInteger):
        m = Fraction(m_ir.value)
    else:  # IRRational
        m = Fraction(m_ir.numer, m_ir.denom)

    # Factor the depressed quartic into two quadratics using m:
    # t⁴ + p·t² + q·t + r_coef = (t² + m·t + α)(t² - m·t + β)
    # where α = p/2 + m²/2 - q/(2m) and β = p/2 + m²/2 + q/(2m).
    if m == 0:
        return []  # degenerate; shouldn't occur with correct resolvent

    alpha = p / 2 + m ** 2 / 2 - q / (2 * m)
    beta = p / 2 + m ** 2 / 2 + q / (2 * m)

    roots1 = solve_quadratic(Fraction(1), m, alpha)
    roots2 = solve_quadratic(Fraction(1), -m, beta)

    if isinstance(roots1, str):
        roots1 = []
    if isinstance(roots2, str):
        roots2 = []

    from cas_solve.cubic import _add_shift
    all_roots: list[IRNode] = []
    for t_ir in roots1 + roots2:
        all_roots.append(_add_shift(t_ir, shift))
    return _dedup_and_sort(all_roots)


# ---------------------------------------------------------------------------
# Rational root helpers
# ---------------------------------------------------------------------------


def _eval_quartic(
    a: Fraction,
    b: Fraction,
    c: Fraction,
    d: Fraction,
    e: Fraction,
    x: Fraction,
) -> Fraction:
    return a * x ** 4 + b * x ** 3 + c * x ** 2 + d * x + e


def _find_rational_root_quartic(
    a: Fraction, b: Fraction, c: Fraction, d: Fraction, e: Fraction
) -> Fraction | None:
    """Return a rational root of ``ax⁴ + bx³ + cx² + dx + e`` if one exists."""
    lcm = _fraction_lcm(a, b, c, d, e)
    A = int(a * lcm)
    E = int(e * lcm)

    if E == 0:
        return Fraction(0)

    p_divs = _divisors(abs(E))
    q_divs = _divisors(abs(A))

    for p_val in p_divs:
        for q_val in q_divs:
            for sign in (1, -1):
                r = Fraction(sign * p_val, q_val)
                if _eval_quartic(a, b, c, d, e, r) == 0:
                    return r
    return None
