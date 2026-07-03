"""Frobenius / power-series method for 2nd-order linear ODEs with a regular
singular point at ``x = 0``.

Track C1 of the macsyma-finish-plan: fallback solver for un-named regular
singular-point ODEs.  The named families (Bessel, Legendre, Hermite,
Chebyshev) are recognised structurally by Phase 21 in ``ode.py``; this
module catches anything those recognisers miss but that still admits a
Frobenius series solution.

Algorithm
---------
For a 2nd-order linear ODE

.. math::

    y'' + p(x) y' + q(x) y = 0

with a *regular singular point* at ``x = 0`` (so that ``x p(x)`` and
``x^2 q(x)`` are analytic at ``0``), we substitute the Frobenius series

.. math::

    y(x) = x^r \\sum_{n \\ge 0} a_n x^n

into the ODE.  Multiplying by ``x^2`` gives

.. math::

    x^2 y'' + x \\tilde P(x) y' + \\tilde Q(x) y = 0,

where ``\\tilde P(x) = x p(x) = \\sum_k p_k x^k`` and
``\\tilde Q(x) = x^2 q(x) = \\sum_k q_k x^k`` are both analytic.  The
coefficient of ``x^{n+r}`` (after substitution) is

.. math::

    a_n \\bigl[(n+r)(n+r-1) + p_0(n+r) + q_0 \\bigr]
      + \\sum_{k=1}^{n} a_{n-k} \\bigl[p_k(n-k+r) + q_k \\bigr] = 0.

Define the **indicial polynomial** ``F(s) = s(s-1) + p_0 s + q_0``.
Setting ``n = 0`` gives the **indicial equation** ``F(r) = 0``, whose
two roots ``r_1 \\ge r_2`` determine the leading exponents of the two
linearly independent Frobenius solutions.

For each ``n \\ge 1``,

.. math::

    a_n = - \\frac{1}{F(n + r)}
          \\sum_{k=1}^{n} a_{n-k} \\bigl[p_k(n-k+r) + q_k \\bigr].

The recurrence is well-defined for ``r = r_1`` (the larger root) provided
``F(n + r_1) \\ne 0`` for all ``n \\ge 1``.  This holds unless
``r_1 - r_2`` is a positive integer (then ``F(r_1 - (r_2 - r_1)) = 0`` for
the appropriate ``n``), or unless ``r_1 = r_2`` (logarithmic case).

Scope (deliberate)
------------------
This PR handles **only** the simplest reliable case:

1. Singular point at ``x = 0`` only.  (Translation to ``x_0 \\ne 0`` is a
   short extension but adds test surface — deferred.)
2. **Non-integer-difference** indicial roots only.  Equal roots and
   roots differing by a positive integer both lead to logarithmic terms
   in the second solution; we bail (return ``None``) so the caller can
   fall through to the unevaluated form.

The series is truncated at ``N = 10`` terms by default.

Why this is correct as a fallback
---------------------------------
The dispatcher in :func:`cas_ode.ode.solve_ode` tries the explicit named
families (Bessel, Legendre, Hermite, Chebyshev) *before* Frobenius.  So
the only ODEs that reach this helper are ones whose recurrence has no
known closed-form summation in our library — exactly the cases for which
a truncated power series is the best available answer.
"""

from __future__ import annotations

import math
from fractions import Fraction

from symbolic_ir import (
    ADD,
    EQUAL,
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
from symbolic_ir.nodes import D

# Default truncation length.  The recurrence is exact (Fraction arithmetic),
# but ten terms is enough to identify the leading behaviour and match a
# named series (Bessel, etc.) up to the precision the printer cares about.
_DEFAULT_TRUNCATION_N = 10


# ---------------------------------------------------------------------------
# Section 1 — Polynomial extraction in ``x`` (arbitrary degree)
# ---------------------------------------------------------------------------
#
# We need to read polynomial coefficients of P(x), Q(x), R(x) up to the
# truncation order N.  ``_try_polynomial_forcing`` in ``ode.py`` only
# supports degrees 0–2; here we generalise to arbitrary degree.
#
# Supported terms (after flattening the IR sum):
#   - rational constant            → contributes to coeff[0]
#   - bare x                       → contributes 1 to coeff[1]
#   - Mul(rational, x)             → contributes to coeff[1]
#   - bare Pow(x, k) with k≥0      → contributes 1 to coeff[k]
#   - Mul(rational, Pow(x, k))    → contributes to coeff[k]
#   - Mul(x, x), Mul(x, Pow(x,k)) → reduced to higher degree
#
# Anything else (rational functions of x, transcendentals) → None.


def _flatten_add(expr: IRNode) -> list[IRNode]:
    """Flatten an ``Add``/``Sub``/``Neg`` tree into a list of summands.

    Copy of the helper from :mod:`cas_ode.ode` — kept local so this module
    has no import-cycle risk and can be exercised in isolation.
    """
    if isinstance(expr, IRApply) and isinstance(expr.head, IRSymbol):
        if expr.head == ADD and len(expr.args) == 2:
            return _flatten_add(expr.args[0]) + _flatten_add(expr.args[1])
        if expr.head == SUB and len(expr.args) == 2:
            return _flatten_add(expr.args[0]) + [
                IRApply(NEG, (expr.args[1],))
            ]
        if expr.head == NEG and len(expr.args) == 1:
            inner = expr.args[0]
            if (
                isinstance(inner, IRApply)
                and inner.head == NEG
                and len(inner.args) == 1
            ):
                return _flatten_add(inner.args[0])
            inner_terms = _flatten_add(inner)
            return [IRApply(NEG, (t,)) for t in inner_terms]
    return [expr]


def _as_fraction(node: IRNode) -> Fraction | None:
    """Return the rational value of ``node`` if it is an integer or rational.

    Examples::

        _as_fraction(IRInteger(3))      → Fraction(3)
        _as_fraction(IRRational(1, 2))  → Fraction(1, 2)
        _as_fraction(IRSymbol("x"))     → None
    """
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    return None


def _degree_of_term(
    term: IRNode, x: IRSymbol
) -> tuple[Fraction, int] | None:
    """Return ``(coefficient, degree)`` for a monomial ``c·x^k``, else ``None``.

    Recognised shapes (after :func:`_flatten_add` strips ``Neg``):

    - ``IRInteger`` / ``IRRational``    → ``(value, 0)``
    - ``x``                             → ``(1, 1)``
    - ``Pow(x, k)`` with ``k`` int ≥ 0  → ``(1, k)``
    - ``Mul(c, x)`` / ``Mul(x, c)``      → ``(c, 1)``
    - ``Mul(c, Pow(x, k))`` and mirror   → ``(c, k)``
    - ``Mul(x, x)``                       → ``(1, 2)``
    - ``Mul(x, Pow(x, k))`` etc.         → ``(1, k+1)``
    - ``Neg(t)``                         → recurse and negate coefficient

    Any unrecognised shape returns ``None`` — the caller bails.
    """
    # Neg unwrap.
    if (
        isinstance(term, IRApply)
        and term.head == NEG
        and len(term.args) == 1
    ):
        inner = _degree_of_term(term.args[0], x)
        if inner is None:
            return None
        c, d = inner
        return (-c, d)

    # Constants.
    frac = _as_fraction(term)
    if frac is not None:
        return (frac, 0)

    # Bare x.
    if isinstance(term, IRSymbol):
        if term == x:
            return (Fraction(1), 1)
        return None  # foreign symbol

    if not isinstance(term, IRApply):
        return None

    # Pow(x, k).
    if term.head == POW and len(term.args) == 2:
        base, exp = term.args
        if base == x and isinstance(exp, IRInteger) and exp.value >= 0:
            return (Fraction(1), exp.value)
        return None

    # Mul(a, b).
    if term.head == MUL and len(term.args) == 2:
        a, b = term.args
        a_part = _degree_of_term(a, x)
        b_part = _degree_of_term(b, x)
        if a_part is None or b_part is None:
            return None
        ca, da = a_part
        cb, db = b_part
        return (ca * cb, da + db)

    return None


def _poly_coeffs(
    expr: IRNode, x: IRSymbol, max_deg: int
) -> list[Fraction] | None:
    """Extract polynomial coefficients of ``expr`` in ``x``, ``coeffs[k] = [x^k]``.

    Returns a list of length ``max_deg + 1`` with rational coefficients,
    or ``None`` if ``expr`` is not a polynomial in ``x`` (or has degree
    higher than ``max_deg``).

    Examples::

        _poly_coeffs(IRInteger(0), x, 3)               → [0, 0, 0, 0]
        _poly_coeffs(Pow(x, 2), x, 3)                  → [0, 0, 1, 0]
        _poly_coeffs(Sub(Pow(x, 2), Rational(1,4)), x, 3)
            → [-1/4, 0, 1, 0]
    """
    coeffs = [Fraction(0)] * (max_deg + 1)
    for term in _flatten_add(expr):
        pair = _degree_of_term(term, x)
        if pair is None:
            return None
        coeff, deg = pair
        if deg > max_deg:
            # Higher-order term — for series purposes we only care about
            # truncating at max_deg, but if we silently drop we'd produce
            # wrong recurrences.  Better to bail and let the caller fall
            # through.
            return None
        coeffs[deg] += coeff
    return coeffs


# ---------------------------------------------------------------------------
# Section 2 — Frobenius solver
# ---------------------------------------------------------------------------


def _collect_var2_coeffs(
    expr: IRNode, y: IRSymbol, x: IRSymbol
) -> tuple[IRNode, IRNode, IRNode] | None:
    """Inline copy of :func:`cas_ode.ode._collect_var2_coeffs`.

    Kept local so this module can be unit-tested without importing the
    full ``ode`` module (and so future refactors of one don't break the
    other).  See ``cas_ode.ode`` for the canonical implementation and
    full docstring.
    """
    # Local clones of helpers we need.
    y_prime = IRApply(D, (y, x))
    y_double = IRApply(D, (y_prime, x))

    def _split_out_factor(term: IRNode, target: IRNode) -> IRNode | None:
        if term == target:
            return IRInteger(1)
        if not isinstance(term, IRApply):
            return None
        if term.head == NEG:
            inner_k = _split_out_factor(term.args[0], target)
            if inner_k is not None:
                return IRApply(NEG, (inner_k,))
            return None
        if term.head == MUL:
            a, b = term.args
            if b == target:
                return a
            if a == target:
                return b
            coeff_b = _split_out_factor(b, target)
            if coeff_b is not None:
                return IRApply(MUL, (a, coeff_b))
            coeff_a = _split_out_factor(a, target)
            if coeff_a is not None:
                return IRApply(MUL, (coeff_a, b))
        return None

    p_parts: list[IRNode] = []
    q_parts: list[IRNode] = []
    r_parts: list[IRNode] = []
    for term in _flatten_add(expr):
        coeff_ypp = _split_out_factor(term, y_double)
        if coeff_ypp is not None:
            p_parts.append(coeff_ypp)
            continue
        coeff_yp = _split_out_factor(term, y_prime)
        if coeff_yp is not None:
            q_parts.append(coeff_yp)
            continue
        coeff_y = _split_out_factor(term, y)
        if coeff_y is not None:
            r_parts.append(coeff_y)
            continue
        return None
    if not p_parts:
        return None

    def _sum_parts(parts: list[IRNode]) -> IRNode:
        result = parts[0]
        for p in parts[1:]:
            result = IRApply(ADD, (result, p))
        return result

    P_node = _sum_parts(p_parts)
    Q_node = _sum_parts(q_parts) if q_parts else IRInteger(0)
    R_node = _sum_parts(r_parts) if r_parts else IRInteger(0)
    return (P_node, Q_node, R_node)


def _multiply_poly(
    a: list[Fraction], b: list[Fraction], max_deg: int
) -> list[Fraction]:
    """Polynomial multiplication truncated at degree ``max_deg``."""
    out = [Fraction(0)] * (max_deg + 1)
    for i, ai in enumerate(a):
        if i > max_deg or ai == 0:
            continue
        for j, bj in enumerate(b):
            if i + j > max_deg or bj == 0:
                continue
            out[i + j] += ai * bj
    return out


def _shift_poly_right(coeffs: list[Fraction], k: int) -> list[Fraction]:
    """Multiply a polynomial by ``x^k`` — i.e. shift coefficients right by ``k``.

    The result is truncated to the same length as the input.  Used to
    encode ``x * Q(x)`` and ``x^2 * R(x)`` when forming the Frobenius
    series ``tilde-P`` and ``tilde-Q``.
    """
    out = [Fraction(0)] * len(coeffs)
    for i, c in enumerate(coeffs):
        if i + k < len(out):
            out[i + k] = c
    return out


def _is_regular_singular(
    P: list[Fraction], Q: list[Fraction], R: list[Fraction]
) -> tuple[list[Fraction], list[Fraction]] | None:
    """Verify ``x = 0`` is a regular singular point and return ``(tildeP, tildeQ)``.

    For the standardised ODE ``y'' + p(x) y' + q(x) y = 0`` with
    ``p = Q/P``, ``q = R/P``, the singularity at 0 is regular iff
    ``x p(x)`` and ``x^2 q(x)`` are analytic at 0.  We work with
    polynomial coefficient lists; the test reduces to:

    - ``P[0] = 0`` (otherwise 0 is a *regular* point, not singular —
      caller should fall through).
    - Let ``m`` = order of vanishing of P at 0 (smallest index with
      P[m] ≠ 0).  Then ``x p(x) = x Q(x) / P(x)`` is analytic at 0 iff
      Q vanishes to order at least ``m - 1`` (so the factor of x in the
      numerator brings us to ``≥ m``).  Similarly ``x² q(x) = x² R(x) / P(x)``
      analytic iff R vanishes to order ≥ m - 2.

    For this PR we restrict to ``m ≤ 2`` (the most common case for
    second-order equations of the textbook form).  We then explicitly
    compute the truncated Taylor series of ``tilde P = x p`` and
    ``tilde Q = x² q`` by dividing through.

    Strategy
    --------
    Rather than dividing two polynomials symbolically (which can produce
    arbitrarily long expansions even if both are finite), we *normalise
    by setting* ``P_eff(x) = P(x) / x^m``.  Then

    .. math::

        \\tilde P(x) = x Q(x) / P(x)
                     = x^{1 - m} Q(x) / P_{eff}(x)

    For ``P_eff(0) \\ne 0`` we can invert ``P_eff`` as a power series.

    Equivalently — and more simply — we **assume** P, Q, R are already in
    the canonical Frobenius form where ``P(x) = x^2``, ``Q(x) = x · s(x)``
    for some analytic ``s``, and ``R(x)`` analytic.  That is the most
    common textbook presentation (and the one Bessel, Euler-Cauchy, etc.
    use).  If the user's ODE doesn't match this form exactly, we extend by
    dividing through.

    Returns
    -------
    ``(tildeP_coeffs, tildeQ_coeffs)`` where ``tildeP = x·p(x)`` and
    ``tildeQ = x²·q(x)`` are the analytic series, each truncated to the
    same length as the input lists.  Returns ``None`` if ``x = 0`` is not
    a regular singular point.
    """
    n = len(P)
    # Order of vanishing of P at 0.
    m = 0
    while m < n and P[m] == 0:
        m += 1
    if m == 0:
        # P(0) ≠ 0 — x=0 is a regular point, not singular.  Frobenius
        # is unnecessary; return None and let the caller fall through.
        return None
    if m > 2:
        # Higher-order vanishing of the y''-coefficient is unusual and
        # this PR doesn't try to handle it.
        return None

    # Compute P_eff(x) = P(x) / x^m  (analytic, with P_eff(0) ≠ 0).
    P_eff = [Fraction(0)] * n
    for i in range(m, n):
        P_eff[i - m] = P[i]
    if P_eff[0] == 0:
        return None  # shouldn't happen after the loop above, but defensive

    # Invert P_eff(x) as a power series ``inv_P_eff`` with the same
    # truncation order.  We use the standard recurrence
    #     inv[0]  = 1 / P_eff[0]
    #     inv[k]  = -(1/P_eff[0]) * Σ_{j=1..k} P_eff[j] * inv[k-j]
    inv_P_eff = [Fraction(0)] * n
    inv_P_eff[0] = Fraction(1) / P_eff[0]
    for k in range(1, n):
        s = Fraction(0)
        for j in range(1, k + 1):
            if j < n:
                s += P_eff[j] * inv_P_eff[k - j]
        inv_P_eff[k] = -inv_P_eff[0] * s

    # Build x * Q(x) / P(x) = x * Q(x) * (1/x^m) * inv_P_eff
    #                        = x^{1 - m} * Q(x) * inv_P_eff.
    # We need ``tilde P`` to be analytic at 0, i.e. exponent of x at the
    # leading term must be ≥ 0 after multiplying by x.
    #
    # Algorithm: form Q * inv_P_eff (truncated polynomial), then shift by
    # (1 - m) places — which is a right-shift by 1 if m=1, no shift if
    # m=2 ... wait, sign: x^{1-m}, so if m=1 the factor is x^0 (no
    # shift); if m=2 the factor is x^{-1} (left-shift by 1).  For
    # analyticity, the coefficient at index 0 of the unshifted product
    # must be zero when m=2 (so the left shift doesn't produce a 1/x
    # term).
    Q_inv = _multiply_poly(Q, inv_P_eff, n - 1)
    # tildeP[k] = Q_inv[k - (1 - m)] = Q_inv[k + m - 1]
    tildeP = [Fraction(0)] * n
    shift = m - 1  # how far to LEFT-shift Q_inv to obtain tildeP
    for k in range(n):
        idx = k + shift
        if idx < n:
            tildeP[k] = Q_inv[idx]
    # Sanity check: for analyticity, indices < shift of Q_inv must be 0.
    for i in range(shift):
        if Q_inv[i] != 0:
            return None

    # Similarly tildeQ = x^2 * q(x) = x^{2 - m} * R(x) * inv_P_eff.
    R_inv = _multiply_poly(R, inv_P_eff, n - 1)
    tildeQ = [Fraction(0)] * n
    shift2 = m - 2  # left-shift by m - 2 (0 if m=2, -1 if m=1)
    if shift2 >= 0:
        for k in range(n):
            idx = k + shift2
            if idx < n:
                tildeQ[k] = R_inv[idx]
        for i in range(shift2):
            if R_inv[i] != 0:
                return None
    else:
        # shift2 = -1: tildeQ = x * R_inv (right shift by 1).
        for k in range(1, n):
            tildeQ[k] = R_inv[k - 1]

    return (tildeP, tildeQ)


def _exact_sqrt_fraction(f: Fraction) -> Fraction | None:
    """Return the exact rational sqrt of ``f`` if both num/denom are perfect
    squares; else ``None``.  Duplicate of the helper in :mod:`cas_ode.ode`.
    """
    if f < 0:
        return None
    if f == 0:
        return Fraction(0)
    p = f.numerator
    q = f.denominator
    sp = math.isqrt(p)
    sq = math.isqrt(q)
    if sp * sp != p or sq * sq != q:
        return None
    return Fraction(sp, sq)


def _solve_indicial(
    p0: Fraction, q0: Fraction
) -> tuple[Fraction, Fraction] | None:
    """Solve ``r(r-1) + p0 r + q0 = 0`` for rational roots.

    Returns ``(r1, r2)`` with ``r1 >= r2`` (rational), or ``None`` if the
    roots are irrational or complex (out of scope for this PR — we want
    exact symbolic answers in the recurrence).
    """
    # r^2 + (p0 - 1) r + q0 = 0
    B = p0 - 1
    C = q0
    disc = B * B - 4 * C
    if disc < 0:
        return None  # complex roots; out of scope
    sqrt_disc = _exact_sqrt_fraction(disc)
    if sqrt_disc is None:
        return None  # irrational roots; out of scope
    r1 = (-B + sqrt_disc) / 2
    r2 = (-B - sqrt_disc) / 2
    if r1 < r2:
        r1, r2 = r2, r1
    return (r1, r2)


def _roots_differ_by_integer(r1: Fraction, r2: Fraction) -> bool:
    """Return True if ``r1 - r2`` is an integer (including 0).

    The logarithmic / equal-root cases are out of scope; we bail when
    this returns True.
    """
    diff = r1 - r2
    return diff.denominator == 1


def _build_series_ir(
    r: Fraction, a: list[Fraction], x: IRSymbol
) -> IRNode:
    """Build the IR for ``x^r · (a_0 + a_1 x + a_2 x^2 + ... + a_N x^N)``.

    Conventions:
    - Coefficients equal to 0 are skipped from the sum (but a_0 is
      always emitted, even if zero, so the printer shows the leading term).
    - The sum is built left-associatively with ``Add``.
    - ``x^r`` uses :class:`IRRational` when ``r`` has a denominator > 1;
      ``IRInteger`` otherwise.  ``r = 0`` collapses to ``1`` (no prefactor).
    """
    # Build the polynomial Σ a_k x^k.
    poly_terms: list[IRNode] = []
    for k, ak in enumerate(a):
        if ak == 0 and k > 0:
            continue
        # Build a_k as an IR literal.
        if ak >= 0:
            coeff_ir: IRNode = (
                IRInteger(ak.numerator)
                if ak.denominator == 1
                else IRRational(ak.numerator, ak.denominator)
            )
        else:
            ak_abs = -ak
            inner: IRNode = (
                IRInteger(ak_abs.numerator)
                if ak_abs.denominator == 1
                else IRRational(ak_abs.numerator, ak_abs.denominator)
            )
            coeff_ir = IRApply(NEG, (inner,))
        # Build x^k.
        if k == 0:
            term = coeff_ir
        elif k == 1:
            if ak == 1:
                term = x
            elif ak == -1:
                term = IRApply(NEG, (x,))
            else:
                term = IRApply(MUL, (coeff_ir, x))
        else:
            xk = IRApply(POW, (x, IRInteger(k)))
            if ak == 1:
                term = xk
            elif ak == -1:
                term = IRApply(NEG, (xk,))
            else:
                term = IRApply(MUL, (coeff_ir, xk))
        poly_terms.append(term)

    if not poly_terms:
        poly_terms = [IRInteger(0)]

    poly_ir: IRNode = poly_terms[0]
    for t in poly_terms[1:]:
        poly_ir = IRApply(ADD, (poly_ir, t))

    # Build x^r.
    if r == 0:
        return poly_ir
    if r == 1:
        return IRApply(MUL, (x, poly_ir))
    # r as IR literal (signed Fraction).
    if r > 0:
        r_ir: IRNode = (
            IRInteger(r.numerator)
            if r.denominator == 1
            else IRRational(r.numerator, r.denominator)
        )
    else:
        r_abs = -r
        r_pos: IRNode = (
            IRInteger(r_abs.numerator)
            if r_abs.denominator == 1
            else IRRational(r_abs.numerator, r_abs.denominator)
        )
        r_ir = IRApply(NEG, (r_pos,))
    x_pow_r = IRApply(POW, (x, r_ir))
    return IRApply(MUL, (x_pow_r, poly_ir))


def try_frobenius_series(
    expr: IRNode,
    y: IRSymbol,
    x: IRSymbol,
    N: int = _DEFAULT_TRUNCATION_N,
) -> IRNode | None:
    """Attempt to solve ``expr = 0`` as a Frobenius series at ``x = 0``.

    See module docstring for the algorithm.  Returns
    ``Equal(y, x^{r_1} · Σ a_k x^k)`` truncated at ``k = N`` on success,
    or ``None`` if any of the following hold:

    1. ``expr`` does not parse as a 2nd-order linear ODE in ``y``.
    2. ``P(x)``, ``Q(x)``, or ``R(x)`` is not a polynomial in ``x`` of
       degree ≤ ``N``.
    3. ``x = 0`` is a regular (non-singular) point — fall through to the
       analytic solvers.
    4. ``x = 0`` is an *irregular* singular point.
    5. Indicial roots are complex, irrational, equal, or differ by an
       integer (logarithmic / merged-root case — out of scope).

    Parameters
    ----------
    expr : IRNode
        The ODE expression that equals zero.
    y, x : IRSymbol
        Dependent and independent variable symbols.
    N : int, optional
        Truncation order — number of terms beyond ``a_0`` to compute.
        Defaults to 10.  The returned polynomial is degree at most ``N``.

    Returns
    -------
    ``IRApply(EQUAL, (y, series_ir))`` on success, ``None`` otherwise.
    """
    coeffs = _collect_var2_coeffs(expr, y, x)
    if coeffs is None:
        return None
    P_node, Q_node, R_node = coeffs

    # We need polynomials up to degree N to compute N recurrence steps.
    max_deg = N
    P_poly = _poly_coeffs(P_node, x, max_deg)
    if P_poly is None:
        return None
    Q_poly = _poly_coeffs(Q_node, x, max_deg)
    if Q_poly is None:
        return None
    R_poly = _poly_coeffs(R_node, x, max_deg)
    if R_poly is None:
        return None

    # Verify x=0 is a regular singular point and extract the analytic
    # series tildeP = x·p(x), tildeQ = x²·q(x).
    pq = _is_regular_singular(P_poly, Q_poly, R_poly)
    if pq is None:
        return None
    tildeP, tildeQ = pq

    p0 = tildeP[0]
    q0 = tildeQ[0]

    # Solve the indicial equation.
    roots = _solve_indicial(p0, q0)
    if roots is None:
        return None
    r1, r2 = roots
    if _roots_differ_by_integer(r1, r2):
        return None  # logarithmic / merged-root case out of scope

    # Compute the recurrence with a_0 = 1.
    # F(s) = s(s - 1) + p0 s + q0.
    def F(s: Fraction) -> Fraction:
        return s * (s - 1) + p0 * s + q0

    a: list[Fraction] = [Fraction(1)]
    for n in range(1, N + 1):
        denom = F(Fraction(n) + r1)
        if denom == 0:
            # Should not happen under the non-integer-difference guard,
            # but we defend against numerical surprises.
            return None
        rhs = Fraction(0)
        for k in range(1, n + 1):
            pk = tildeP[k] if k < len(tildeP) else Fraction(0)
            qk = tildeQ[k] if k < len(tildeQ) else Fraction(0)
            rhs += a[n - k] * (pk * (Fraction(n - k) + r1) + qk)
        a.append(-rhs / denom)

    series_ir = _build_series_ir(r1, a, x)
    return IRApply(EQUAL, (y, series_ir))
