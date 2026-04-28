"""Univariate polynomial factoring over algebraic number fields Q[√d].

Background
----------
The integers Q and even the rational numbers Q are not algebraically closed:
many polynomials that are irreducible over Q factor beautifully over an
algebraic extension.  The simplest algebraic extension of Q is

    Q[√d] = { p + q·√d  |  p, q ∈ Q }

for a square-free positive integer d.  An element of this field is called a
*quadratic algebraic number*.

Example: x⁴ + 1 is irreducible over Q (proven by Eisenstein + Sophie Germain),
but over Q[√2] it factors as:

    x⁴ + 1 = (x² + √2·x + 1)(x² − √2·x + 1)

This module implements factoring over Q[√d] for:

  - **Pattern 1**: depressed quartics of the form x⁴ + p·x² + q  →  two
    quadratic factors with coefficients in Q[√d] (see below for conditions).
  - **Pattern 2**: quadratics x² + bx + c  →  two linear factors when
    the discriminant b² − 4c equals d·t² for some rational t.
  - **Pattern 3**: trivial linear split  x² − d  →  (x − √d)(x + √d).

Algorithm overview for Pattern 1
----------------------------------
Suppose

    g = x⁴ + p·x² + q

and we hope to write  g = (x² + r√d·x + s)(x² − r√d·x + s).

Expanding the right-hand side:

    (x² + r√d·x + s)(x² − r√d·x + s)
        = x⁴  − r√d·x³ + s·x²
          + r√d·x³ − d·r²·x² + rs√d·x
          + s·x² − rs√d·x + s²
        = x⁴ + (2s − d·r²)x² + s²

So we need:

    2s − d·r² = p       (coefficient of x²)
    s²         = q       (constant term)

Solving:

    s  = ±√q            (requires q to be a perfect rational square)
    r² = (2s − p) / d   (requires (2s−p)/d to be a non-negative perfect
                         rational square)

We check both signs of s; the first that yields valid r wins.

Algorithm overview for Pattern 2 (quadratic splitting)
-------------------------------------------------------
For g = x² + bx + c, the roots are (−b ± √(b²−4c)) / 2.

If we adjoin √d, a root α = a + β√d exists iff:

    a = −b/2   (from the imaginary-part equation  β(2a + b) = 0)
    β² = (b²/4 − c) / d = (b² − 4c) / (4d)

So β is rational iff  (b² − 4c) / (4d) is a non-negative rational square.
Equivalently,  b² − 4c = d · (2β)² for rational 2β.

Then g = (x − (a + β√d))(x − (a − β√d)) = (x + b/2 − β√d)(x + b/2 + β√d).

Pattern 3 is the simplest case: x² − d = (x − √d)(x + √d).
This is b=0, c=−d, discriminant = 4d → (4d)/(4d) = 1 → β = 1/2·√(4d/d)... or
more directly just handled as a special case of the norm approach.
"""

from __future__ import annotations

import math
from fractions import Fraction

# ---------------------------------------------------------------------------
# Type aliases
# ---------------------------------------------------------------------------

# A polynomial coefficient is (rational_part, radical_part), meaning the
# actual coefficient is  rational_part + radical_part · √d.
# For coefficients in Q (no radical part), radical_part = 0.
AlgCoeff = tuple[Fraction, Fraction]

# A polynomial over Q[√d] is a list of AlgCoeff in ascending degree order:
# poly[0] is the constant term, poly[k] is the coefficient of x^k.
AlgPoly = list[AlgCoeff]


# ---------------------------------------------------------------------------
# Helper: perfect-rational-square test
# ---------------------------------------------------------------------------


def _is_rational_square(q: Fraction) -> Fraction | None:
    """Return √q as a Fraction if q is the square of a rational, else None.

    A rational number p/r is a perfect square of a rational iff both its
    numerator and denominator are perfect squares of integers.

    Examples::

        _is_rational_square(Fraction(4))   → Fraction(2)
        _is_rational_square(Fraction(1,4)) → Fraction(1, 2)
        _is_rational_square(Fraction(2))   → None
        _is_rational_square(Fraction(-1))  → None   (negative)

    Negative values never have real square roots, so return None.
    """
    if q < 0:
        return None
    if q == 0:
        return Fraction(0)

    p_int = q.numerator
    r_int = q.denominator

    # Integer square root of numerator
    sp = math.isqrt(p_int)
    if sp * sp != p_int:
        return None

    # Integer square root of denominator
    sr = math.isqrt(r_int)
    if sr * sr != r_int:
        return None

    return Fraction(sp, sr)


# ---------------------------------------------------------------------------
# Pattern 2 + 3: split a degree-2 polynomial over Q[√d]
# ---------------------------------------------------------------------------


def _try_split_quadratic(
    coeffs: list[int], d: int
) -> list[AlgPoly] | None:
    """Try to split a *monic* integer quadratic over Q[√d].

    Parameters
    ----------
    coeffs:
        Integer coefficient list ``[c, b, a]`` (ascending degree).
        We only handle monic (a=1) quadratics here; caller ensures this.
    d:
        The square-free positive integer that defines Q[√d].

    Returns
    -------
    Two ``AlgPoly`` factors ``[h1, h2]`` if the quadratic splits, else None.

    The two factors are conjugates:

        h1 = (x − (a_coeff + β√d)) = [−a_coeff − β√d,  1]
        h2 = (x − (a_coeff − β√d)) = [−a_coeff + β√d,  1]

    where the coefficients are ``AlgCoeff = (rational_part, radical_part)``.

    Mathematical derivation
    -----------------------
    For  g = x² + bx + c  to split over Q[√d], we need rational β ≠ 0 such
    that:

        b² − 4c = d · (2β)²

    From which  β = √((b² − 4c) / (4d)).

    The linear factor for root  −b/2 + β√d  is  (x + b/2 − β√d), and its
    conjugate for root  −b/2 − β√d  is  (x + b/2 + β√d).
    """
    if len(coeffs) != 3 or coeffs[2] != 1:
        return None  # Must be monic quadratic

    c_int, b_int = coeffs[0], coeffs[1]
    b = Fraction(b_int)
    c = Fraction(c_int)

    # Discriminant Δ = b² − 4c.
    disc = b * b - 4 * c

    # We want Δ = d·(2β)², so (2β)² = Δ/d.
    if disc == 0:
        return None  # Perfect square over Q — should already be linear

    ratio = disc / Fraction(d)
    two_beta_sq = _is_rational_square(ratio)
    if two_beta_sq is None:
        return None

    two_beta = two_beta_sq  # = 2β
    if two_beta == 0:
        return None  # Root in Q — no algebraic splitting needed

    # β = two_beta / 2 (rational part of the radical term)
    beta = two_beta / 2

    # a_coeff = −b/2  (rational shift)
    a_coeff = -b / 2

    # Factor h1 = x − (a_coeff + β√d) = x + (−a_coeff − β√d)
    # As AlgCoeff list (ascending degree): [(−a_coeff, −β), (1, 0)]
    h1: AlgPoly = [((-a_coeff, -beta)), (Fraction(1), Fraction(0))]
    # Factor h2 = x − (a_coeff − β√d) = x + (−a_coeff + β√d)
    h2: AlgPoly = [((-a_coeff, beta)), (Fraction(1), Fraction(0))]

    return [h1, h2]


# ---------------------------------------------------------------------------
# Pattern 1: split a depressed quartic x⁴ + p·x² + q over Q[√d]
# ---------------------------------------------------------------------------


def _try_split_depressed_quartic(
    coeffs: list[int], d: int
) -> list[AlgPoly] | None:
    """Try to split a *depressed monic quartic* x⁴ + p·x² + q over Q[√d].

    A depressed quartic has no x³ or x¹ terms, i.e. it has the form:

        g = x⁴ + p·x² + q

    We search for a factorisation into two monic quadratics:

        g = (x² + r√d·x + s)(x² − r√d·x + s)

    with rational r, s ∈ Q.

    Conditions (see module docstring for derivation)::

        s = ±√q          (s must be rational: q must be a perfect square)
        r = √((2s − p)/d) (r must be rational: (2s−p)/d must be ≥ 0 and a
                           perfect rational square)

    Parameters
    ----------
    coeffs:
        Integer coefficient list of the quartic, ascending degree.  Must be
        of the form  [q, 0, p, 0, 1].  If the polynomial has an x³ or x¹
        term this function returns None immediately.
    d:
        Square-free positive integer.

    Returns
    -------
    Two ``AlgPoly`` factors ``[h1, h2]`` on success, else None.
    """
    if len(coeffs) != 5:
        return None
    # Expect monic quartic [q, 0, p, 0, 1].
    if coeffs[4] != 1 or coeffs[3] != 0 or coeffs[1] != 0:
        return None  # Not a depressed monic quartic.

    q_int = coeffs[0]
    p_int = coeffs[2]

    p = Fraction(p_int)
    q = Fraction(q_int)

    # Try both signs of s.
    for sign in (1, -1):
        s_sq = _is_rational_square(q if sign == 1 else -q)
        if s_sq is None:
            continue

        # s_sq = |√q|, so s = sign * s_sq
        # If sign=1 we test s=+√q; sign=-1 tests s=-√q.
        # But _is_rational_square only works on non-negative q.
        # If q>0: sign=1 → s=√q, sign=-1 → s=-√q.
        # If q<0: _is_rational_square(-q) needs -q>0, so only sign=-1 works.
        s = Fraction(sign) * s_sq

        # Compute (2s − p)/d and check it is a non-negative rational square.
        numerator = 2 * s - p
        if numerator < 0:
            continue  # r² would be negative — no real factoring.

        r_sq = _is_rational_square(numerator / Fraction(d))
        if r_sq is None:
            continue
        if r_sq == 0:
            # r=0 means the two factors are the same x²+s; this only
            # works if g = (x²+s)², which is an exact square — the
            # original polynomial was already reducible over Q.
            continue

        r = r_sq  # ≥ 0; we'll emit + and − r√d

        # h1 = x² + r√d·x + s   (coefficients: [s, r√d, 1])
        # h2 = x² − r√d·x + s   (coefficients: [s, −r√d, 1])
        h1: AlgPoly = [
            (s, Fraction(0)),        # constant term s + 0·√d
            (Fraction(0), r),        # x-coefficient:  0 + r·√d
            (Fraction(1), Fraction(0)),  # x² coefficient: 1
        ]
        h2: AlgPoly = [
            (s, Fraction(0)),
            (Fraction(0), -r),       # x-coefficient:  0 − r·√d
            (Fraction(1), Fraction(0)),
        ]
        return [h1, h2]

    return None


# ---------------------------------------------------------------------------
# Generic dispatcher
# ---------------------------------------------------------------------------


def factor_over_extension(
    f_coeffs: list[int], d: int
) -> list[AlgPoly] | None:
    """Factor polynomial f over Q[√d].

    Given an integer polynomial ``f`` represented as a coefficient list
    (ascending degree, i.e. ``f_coeffs[k]`` is the coefficient of x^k) and
    a square-free positive integer ``d``, attempt to find a non-trivial
    factorisation of ``f`` over the algebraic number field Q[√d].

    The implementation checks three patterns (in order):

    1. **Depressed monic quartic**:  x⁴ + p·x² + q  →  two monic quadratics
       with coefficients  a + b√d.

    2. **Monic quadratic splitting**: x² + bx + c  →  two linear factors whose
       roots lie in Q[√d] (discriminant test).

    3. **Each irreducible Z-factor** is tested individually using the same
       two-pattern tests above, so composite inputs also split wherever possible.

    Parameters
    ----------
    f_coeffs:
        Coefficient list of f in ascending-degree order.  E.g. x⁴ + 1 is
        ``[1, 0, 0, 0, 1]``.
    d:
        Positive, square-free integer (the radical to adjoin).  For example
        d=2 adjoins √2, d=5 adjoins √5.

    Returns
    -------
    A list of ``AlgPoly`` factors on success.  Each factor is a list of
    ``(rational_part, radical_part)`` pairs (ascending degree) representing a
    polynomial with coefficients in Q[√d].

    Returns ``None`` if no splitting over Q[√d] was found (i.e. f is
    irreducible over Q[√d], or no algebraic factoring pattern applies).

    Degree ≤ 1 polynomials are always returned as None (trivially no split).

    Examples
    --------
    ::

        # x⁴ + 1 splits over Q[√2]
        factor_over_extension([1, 0, 0, 0, 1], 2)
        # [
        #   [(1, 0), (0, 1), (1, 0)],   # x² + √2·x + 1
        #   [(1, 0), (0, -1), (1, 0)],  # x² − √2·x + 1
        # ]

        # x² − 2 splits over Q[√2]
        factor_over_extension([-2, 0, 1], 2)
        # [
        #   [(1, -1), (1, 0)],   # x − √2  (i.e. −√2 + 1·x)
        #   [(1, 1), (1, 0)],    # x + √2
        # ]
    """
    from cas_factor import factor_integer_polynomial

    if not f_coeffs:
        return None
    deg = len(f_coeffs) - 1
    if deg <= 1:
        return None  # Degree 0 or 1 — trivially irreducible / constant.

    # Factor over Z first to get irreducible components.
    content, factors_z = factor_integer_polynomial(f_coeffs)

    # Collect all algebraic factors (from both content and Z-factors).
    result: list[AlgPoly] = []
    any_split = False

    # The content is an integer; an integer polynomial over Q[√d] cannot
    # split unless it's degree≥2. Content is always degree-0, so skip.
    # For each irreducible factor over Z, try to split it over Q[√d].
    for poly_coeffs, mult in factors_z:
        split = _try_split_single(poly_coeffs, d)
        if split is not None:
            # Add the factors (multiplicity copies).
            for _ in range(mult):
                result.extend(split)
            any_split = True
        else:
            # This factor is irreducible over Q[√d] too — keep it as-is,
            # but convert to AlgPoly format (all radical parts are 0).
            alg_poly: AlgPoly = [
                (Fraction(c), Fraction(0)) for c in poly_coeffs
            ]
            for _ in range(mult):
                result.append(alg_poly)

    if not any_split:
        return None  # No new splitting found over Q[√d].

    return result


def _try_split_single(
    poly_coeffs: list[int], d: int
) -> list[AlgPoly] | None:
    """Attempt to split a single irreducible Z-factor over Q[√d].

    Tries:
    1. Depressed monic quartic pattern.
    2. Monic quadratic splitting pattern.

    Returns a list of two AlgPoly factors, or None.
    """
    deg = len(poly_coeffs) - 1
    if deg < 2:
        return None  # Linear — irreducible.

    # Normalise to monic (positive leading coefficient already guaranteed
    # by factor_integer_polynomial; divide through if leading coeff ≠ 1).
    lead = poly_coeffs[-1]
    if lead != 1:
        # Not monic — try making it monic by checking the pattern directly.
        # For now we only handle monic polynomials in our patterns.
        # (A non-monic degree-2 could be factored, but factor_integer_poly
        #  guarantees primitiveness, so the lead is ±1 after normalisation.)
        if lead == -1:
            poly_coeffs = [-c for c in poly_coeffs]
        else:
            return None

    if deg == 2:
        return _try_split_quadratic(poly_coeffs, d)
    if deg == 4:
        return _try_split_depressed_quartic(poly_coeffs, d)

    # Degrees 3, 5+ — not handled in this phase.
    return None
