"""Bivariate Hensel lifting for factoring over ℚ[x, y].

This module extends ``cas_factor`` from univariate Z[x] into bivariate
Q[x, y] using the textbook **bivariate Hensel lift** around a
substitution point ``y = y₀``.  The algorithm is generic — one routine,
many polynomial shapes — and replaces the per-shape recognisers that
would otherwise proliferate (``x^2+xy-2y^2``, ``x^3-x^2y-xy^2+y^3``,
etc.).

Mathematical background
-----------------------
Given a bivariate polynomial ``f(x, y) ∈ ℚ[x, y]`` that we want to
factor:

1. **Substitute** ``y = y₀`` for a small integer ``y₀`` (we try 0, 1, -1,
   2, -2, …).  The substitution must be *lucky*: the univariate image
   ``f(x, y₀)`` must be **squarefree** and its degree in ``x`` must equal
   ``deg_x(f)`` (otherwise the leading coefficient vanishes at ``y₀`` and
   the lift cannot reconstruct it).
2. **Factor the univariate image** ``f₀(x) = f(x, y₀)`` over ℚ using the
   existing :func:`cas_factor.factor_integer_polynomial` machinery.
3. **Lift** the factors back to ℚ[x, y] by Hensel's lemma.  At step
   ``k``, we know ``f ≡ g_k · h_k (mod (y - y₀)ᵏ)`` and we compute
   correction polynomials ``Δg, Δh ∈ ℚ[x]`` such that

       g_{k+1} = g_k + Δg · (y - y₀)ᵏ
       h_{k+1} = h_k + Δh · (y - y₀)ᵏ

   satisfy ``f ≡ g_{k+1} · h_{k+1} (mod (y - y₀)^{k+1})``.  The
   corrections solve the **univariate Diophantine equation**

       g₀(x) · Δh(x) + h₀(x) · Δg(x) ≡ e_k(x)   (mod g₀ · h₀, in ℚ[x])

   where ``e_k(x)`` is the coefficient of ``(y - y₀)ᵏ`` in
   ``(f - g_k · h_k)``.  Because we're over the field ℚ, the existence
   of ``Δg, Δh`` reduces to ``gcd(g₀, h₀) = 1`` — exactly the
   squarefreeness condition we enforced in step 1.

4. **Stop** when ``k > deg_y(f)``: the lifted factors are then exact (no
   further y-powers can appear).  Multiply ``g · h`` out and verify
   equality with ``f``; if equal, return ``[g, h]``.

5. **Multi-factor case**: when ``f₀`` has more than two univariate
   factors, we iterate the two-factor lift: at each round, peel off one
   factor against the product of the rest, then recurse on the rest.

Representation
--------------
A bivariate polynomial is a ``dict[tuple[int, int], Fraction]`` mapping
the exponent pair ``(i, j)`` to the coefficient of ``x^i · y^j``.
Zero coefficients are stripped (the empty dict is the zero polynomial).
This sparse representation suits Hensel's mix of high-degree-in-x,
low-degree-in-y data well.

Univariate polynomials in ℚ[x] use ``list[Fraction]`` in ascending
degree order, matching the convention of :mod:`cas_factor.polynomial`
but with rational coefficients.

Worked example
--------------
``f(x, y) = x² + xy - 2y²`` factors as ``(x + 2y)(x - y)``.

1. Substitute ``y = 0``: ``f₀(x) = x²``.  This is *not* squarefree —
   try ``y₀ = 1`` next.
2. Substitute ``y = 1``: ``f₀(x) = x² + x - 2 = (x + 2)(x - 1)``.
   Squarefree, degree 2 = deg_x(f).  ✓
3. Univariate factor: ``g₀(x) = x + 2``, ``h₀(x) = x - 1``.
4. Lift in powers of ``(y - 1)``:
   - At ``k = 1``: error ``e_1 = ((f - g₀h₀) shifted) coef of (y-1)``.
     Solve ``(x+2)·Δh + (x-1)·Δg = e_1`` for Δg, Δh of degree 0.
   - Continue until ``k > deg_y(f) = 2``.
5. The lifted factors are ``g(x, y) = x + 2y``, ``h(x, y) = x - y``.

Public API
----------
::

    try_bivariate_hensel(
        f: dict[tuple[int, int], Fraction],
    ) -> list[dict[tuple[int, int], Fraction]] | None

Returns a list of irreducible bivariate factors (each with at least one
non-constant term) whose product equals ``f``, or ``None`` if no
non-trivial factorisation was found.

Returns ``None`` (falls through cleanly) when:

- The polynomial is univariate (single variable in x or y).
- No lucky substitution point exists within the search range.
- The univariate image is irreducible.
- The reconstructed factors fail the final verification multiplication.
"""

from __future__ import annotations

from fractions import Fraction

from cas_factor.factor import factor_integer_polynomial

# A bivariate polynomial as a sparse dict {(i, j): coef}.  Key (i, j)
# represents the term ``coef · x^i · y^j``.  Empty dict = zero polynomial.
BiPoly = dict[tuple[int, int], Fraction]

# Univariate polynomial over ℚ as ascending-degree list of Fractions.
# Trailing zeros are stripped at every operation.
UniQPoly = list[Fraction]

# How many small integer values of y₀ to try when searching for a lucky
# substitution point.  In practice 0, ±1, ±2, … cover virtually every
# polynomial that arises from a graduate-engineering CAS workload.  We
# cap to keep the search bounded.
_MAX_Y0_SEARCH = 8


# ---------------------------------------------------------------------------
# Univariate Q[x] helpers — small, self-contained, mirror polynomial.py
# but use Fractions throughout because Hensel lifts may introduce
# denominators that don't clear back to Z[x].
# ---------------------------------------------------------------------------


def _u_normalize(p: UniQPoly) -> UniQPoly:
    """Strip trailing-zero (high-degree) coefficients."""
    out = list(p)
    while out and out[-1] == 0:
        out.pop()
    return out


def _u_degree(p: UniQPoly) -> int:
    """Degree of a univariate polynomial; ``-1`` for the zero polynomial."""
    p = _u_normalize(p)
    return len(p) - 1 if p else -1


def _u_add(a: UniQPoly, b: UniQPoly) -> UniQPoly:
    """Sum of two univariate Q-polynomials."""
    n = max(len(a), len(b))
    out: UniQPoly = [Fraction(0)] * n
    for i, c in enumerate(a):
        out[i] += c
    for i, c in enumerate(b):
        out[i] += c
    return _u_normalize(out)


def _u_sub(a: UniQPoly, b: UniQPoly) -> UniQPoly:
    """Difference ``a - b`` of two univariate Q-polynomials."""
    n = max(len(a), len(b))
    out: UniQPoly = [Fraction(0)] * n
    for i, c in enumerate(a):
        out[i] += c
    for i, c in enumerate(b):
        out[i] -= c
    return _u_normalize(out)


def _u_mul(a: UniQPoly, b: UniQPoly) -> UniQPoly:
    """Product of two univariate Q-polynomials."""
    if not a or not b:
        return []
    out: UniQPoly = [Fraction(0)] * (len(a) + len(b) - 1)
    for i, ca in enumerate(a):
        if ca == 0:
            continue
        for j, cb in enumerate(b):
            out[i + j] += ca * cb
    return _u_normalize(out)


def _u_scale(a: UniQPoly, s: Fraction) -> UniQPoly:
    """Multiply a univariate Q-polynomial by a scalar."""
    if s == 0:
        return []
    return _u_normalize([c * s for c in a])


def _u_divmod(a: UniQPoly, b: UniQPoly) -> tuple[UniQPoly, UniQPoly]:
    """Polynomial division ``a = q · b + r`` in ℚ[x].

    Returns ``(quotient, remainder)`` with ``deg(r) < deg(b)``.  Both
    inputs and outputs are in ascending-coefficient order.

    The standard long-division loop: subtract a scalar multiple of ``b``
    shifted to align with the current leading term of ``a``, reducing
    degree by one each iteration.  Termination requires ``deg(b) ≥ 0``.
    """
    a = _u_normalize(a)
    b = _u_normalize(b)
    if not b:
        raise ZeroDivisionError("division by zero polynomial")
    db = len(b) - 1
    lc_b = b[-1]
    # Highest-degree quotient coefficient first; reverse at the end.
    q_rev: UniQPoly = []
    rem = list(a)
    while len(rem) - 1 >= db and rem:
        shift = len(rem) - 1 - db
        c = rem[-1] / lc_b
        q_rev.append(c)
        # Subtract c · b · x^shift from rem.
        for k, bk in enumerate(b):
            rem[shift + k] -= c * bk
        # Strip the now-zero leading term (and any further trailing zeros).
        while rem and rem[-1] == 0:
            rem.pop()
    q = list(reversed(q_rev))
    return _u_normalize(q), _u_normalize(rem)


def _u_gcd_ext(a: UniQPoly, b: UniQPoly) -> tuple[UniQPoly, UniQPoly, UniQPoly]:
    """Extended Euclidean algorithm in ℚ[x].

    Returns ``(g, s, t)`` such that ``s·a + t·b = g`` and ``g`` is a
    **monic** GCD (or zero if both inputs are zero).  The Bézout
    coefficients ``s, t`` solve the univariate diophantine identity at
    the heart of each Hensel lift step.
    """
    old_r, r = _u_normalize(a), _u_normalize(b)
    old_s: UniQPoly = [Fraction(1)]
    s: UniQPoly = []
    old_t: UniQPoly = []
    t: UniQPoly = [Fraction(1)]

    while r:
        q, _ = _u_divmod(old_r, r)
        old_r, r = r, _u_sub(old_r, _u_mul(q, r))
        old_s, s = s, _u_sub(old_s, _u_mul(q, s))
        old_t, t = t, _u_sub(old_t, _u_mul(q, t))

    g = old_r
    if g and g[-1] != 1:
        # Make GCD monic; rescale Bézout coefficients accordingly.
        inv = Fraction(1) / g[-1]
        g = _u_scale(g, inv)
        old_s = _u_scale(old_s, inv)
        old_t = _u_scale(old_t, inv)
    return g, old_s, old_t


def _u_diophantine(g0: UniQPoly, h0: UniQPoly, c: UniQPoly) -> tuple[
    UniQPoly, UniQPoly
] | None:
    """Solve ``u · g₀ + v · h₀ = c`` in ℚ[x] with ``deg u < deg h₀``,
    ``deg v < deg g₀``.

    Requires ``gcd(g₀, h₀) = 1`` (a non-zero constant) — this is the
    coprimality assumption that the lucky-substitution check guarantees.

    Algorithm: compute Bézout ``s · g₀ + t · h₀ = 1`` via extended GCD,
    then scale by ``c`` and reduce::

        s · c · g₀ + t · c · h₀ = c
        Let u = (s · c) mod h₀, with quotient q so s · c = q · h₀ + u.
        Then v = (t · c + q · g₀) automatically has deg < deg g₀ once we
        reduce mod g₀.

    Verification (mod g₀ · h₀):
        u · g₀ + v · h₀ = (s·c − q·h₀) · g₀ + (t·c + q·g₀) · h₀
                        = s·c·g₀ + t·c·h₀ = c.    ✓

    Returns ``None`` if ``gcd(g₀, h₀) ≠ 1`` (would indicate an unlucky
    substitution slipped through the squarefreeness check).
    """
    g, s, t = _u_gcd_ext(g0, h0)
    if _u_degree(g) != 0:
        return None
    # Normalise so the constant GCD is 1.
    inv = Fraction(1) / g[0]
    s = _u_scale(s, inv)
    t = _u_scale(t, inv)
    sc = _u_mul(s, c)
    q, u = _u_divmod(sc, h0)
    tc = _u_mul(t, c)
    v_raw = _u_add(tc, _u_mul(q, g0))
    _q2, v = _u_divmod(v_raw, g0)
    return u, v


# ---------------------------------------------------------------------------
# Bivariate polynomial helpers
# ---------------------------------------------------------------------------


def _bi_normalize(p: BiPoly) -> BiPoly:
    """Drop entries with zero coefficient."""
    return {k: v for k, v in p.items() if v != 0}


def _bi_degree_x(p: BiPoly) -> int:
    """Highest exponent of ``x`` appearing in any non-zero term."""
    p = _bi_normalize(p)
    if not p:
        return -1
    return max(i for (i, _j) in p)


def _bi_degree_y(p: BiPoly) -> int:
    """Highest exponent of ``y`` appearing in any non-zero term."""
    p = _bi_normalize(p)
    if not p:
        return -1
    return max(j for (_i, j) in p)


def _bi_add(a: BiPoly, b: BiPoly) -> BiPoly:
    """Sum of two bivariate polynomials."""
    out: BiPoly = dict(a)
    for k, v in b.items():
        out[k] = out.get(k, Fraction(0)) + v
    return _bi_normalize(out)


def _bi_sub(a: BiPoly, b: BiPoly) -> BiPoly:
    """Difference ``a − b`` of two bivariate polynomials."""
    out: BiPoly = dict(a)
    for k, v in b.items():
        out[k] = out.get(k, Fraction(0)) - v
    return _bi_normalize(out)


def _bi_mul(a: BiPoly, b: BiPoly) -> BiPoly:
    """Product of two bivariate polynomials.

    Iterate the Cartesian product of monomials; sum the
    ``(i1+i2, j1+j2) ↦ c1·c2`` contributions into an output map.
    """
    out: BiPoly = {}
    for (i1, j1), c1 in a.items():
        if c1 == 0:
            continue
        for (i2, j2), c2 in b.items():
            if c2 == 0:
                continue
            k = (i1 + i2, j1 + j2)
            out[k] = out.get(k, Fraction(0)) + c1 * c2
    return _bi_normalize(out)


def _bi_substitute_y(p: BiPoly, y0: Fraction) -> UniQPoly:
    """Substitute ``y = y₀`` and return the resulting polynomial in ``x``.

    For each monomial ``c · x^i · y^j`` the contribution to coefficient
    ``[x^i]`` is ``c · y₀^j``.  We sum these into a list indexed by ``i``.
    """
    deg_x = _bi_degree_x(p)
    if deg_x < 0:
        return []
    out: UniQPoly = [Fraction(0)] * (deg_x + 1)
    for (i, j), c in p.items():
        out[i] += c * (y0 ** j)
    return _u_normalize(out)


def _bi_uni_x(p: UniQPoly) -> BiPoly:
    """Embed a univariate polynomial in ``x`` into the bivariate ring.

    ``a₀ + a₁ x + a₂ x² + …`` becomes ``{(0,0): a₀, (1,0): a₁, …}``.
    """
    out: BiPoly = {}
    for i, c in enumerate(p):
        if c != 0:
            out[(i, 0)] = Fraction(c)
    return out


def _bi_coeff_at_y_power(p: BiPoly, k: int) -> UniQPoly:
    """Return the univariate-in-x coefficient of ``y^k`` in ``p``.

    Used during the lift to read off the residual ``[y^k] (f − g·h)``
    that drives the next correction step.
    """
    deg_x = max((i for (i, j) in p if j == k), default=-1)
    if deg_x < 0:
        return []
    out: UniQPoly = [Fraction(0)] * (deg_x + 1)
    for (i, j), c in p.items():
        if j == k:
            out[i] += c
    return _u_normalize(out)


def _bi_shift_y(p: BiPoly, y0: Fraction) -> BiPoly:
    """Rewrite ``p`` as a polynomial in ``(y − y₀)`` instead of ``y``.

    The substitution ``y ↦ (y - y₀) + y₀`` expands every ``y^j`` via the
    binomial theorem.  We loop over each input monomial and distribute
    the binomial expansion onto the output.

    Used to recentre the lift around ``y = y₀ ≠ 0`` so that the algorithm
    can always treat the lifting parameter as ``y'`` with ``y' = 0``.
    """
    if y0 == 0:
        return dict(p)
    out: BiPoly = {}
    # Precompute (y - y0)^j? No — we want y^j = ((y-y0) + y0)^j expressed
    # in the NEW variable y'.  So new exponent ranges from 0 to j.
    # Treat (y-y0)^j as the polynomial whose monomials we will distribute.
    for (i, j), c in p.items():
        if c == 0:
            continue
        # Binomial expansion: y^j = ((y-y0)+y0)^j = Σ_{m=0}^{j} C(j,m) (y-y0)^m y0^(j-m)
        # In the new variable y' = y - y0, the coefficient of y'^m is C(j,m) * y0^(j-m).
        from math import comb
        for m in range(j + 1):
            coeff = c * comb(j, m) * (y0 ** (j - m))
            k = (i, m)
            out[k] = out.get(k, Fraction(0)) + coeff
    return _bi_normalize(out)


# ---------------------------------------------------------------------------
# Univariate ℚ-factoring via cas_factor.factor_integer_polynomial
# ---------------------------------------------------------------------------


def _factor_uni_q(p: UniQPoly) -> list[UniQPoly] | None:
    """Factor a univariate ℚ[x] polynomial into a flat list of factors.

    Clears denominators to integer coefficients, calls the existing
    integer factorisation pipeline, and remaps each factor back to
    ℚ-coefficients.  Multiplicity ``m`` is expanded to ``m`` copies of
    the factor in the output list (Hensel deals with multi-factor input
    by iterated two-factor lift).

    Returns ``None`` if the input is trivial (degree < 1) or if every
    factor is the input itself (irreducible).  Returns the factor list
    otherwise.

    Note: the returned factors have rational coefficients (we don't
    enforce primitivity in the output — the lift handles arbitrary
    monic-or-not factors via the leading-coefficient division step).
    """
    p = _u_normalize(p)
    if len(p) < 2:
        return None
    # Clear denominators.
    denom_lcm = 1
    from math import gcd
    for c in p:
        d = c.denominator
        denom_lcm = denom_lcm * d // gcd(denom_lcm, d)
    int_p = [int(c * denom_lcm) for c in p]
    content, factors = factor_integer_polynomial(int_p)
    if not factors:
        return None
    flat: list[UniQPoly] = []
    for coeffs, mult in factors:
        for _ in range(mult):
            flat.append([Fraction(c) for c in coeffs])
    # If only one factor and it equals the input, irreducible.
    if len(flat) == 1:
        f0 = flat[0]
        # Scale to compare: input ≈ content * f0 / denom_lcm
        scale = Fraction(content, denom_lcm)
        scaled = _u_scale(f0, scale)
        if scaled == p:
            return None
    # Distribute the leading content into the first factor so that the
    # product is exactly p (over ℚ).  All other factors stay as returned.
    if flat:
        scale = Fraction(content, denom_lcm)
        flat[0] = _u_scale(flat[0], scale)
    return flat


# ---------------------------------------------------------------------------
# The two-factor bivariate Hensel lift
# ---------------------------------------------------------------------------


def _two_factor_lift(
    f: BiPoly, g0: UniQPoly, h0: UniQPoly, deg_y: int
) -> tuple[BiPoly, BiPoly] | None:
    """Lift ``f ≡ g₀ · h₀ (mod y)`` to exact bivariate factors.

    Preconditions
    -------------
    - ``f(x, 0) = g₀(x) · h₀(x)`` exactly in ℚ[x].
    - ``gcd(g₀, h₀) = 1`` in ℚ[x] (squarefreeness of the image).
    - ``deg_y`` is the total y-degree of ``f``; the lift terminates at
      step ``deg_y + 1``.

    Loop invariant
    --------------
    At the start of iteration ``k`` we have bivariate ``g, h`` with::

        f ≡ g · h   (mod y^k)

    and ``[y^0] g = g₀``, ``[y^0] h = h₀`` (the base factors never change
    at the y^0 layer).

    Step
    ----
    1. Compute ``error = f − g · h``.
    2. Read off ``e_k = [y^k] error``.  If zero, no correction needed for
       this layer; advance to ``k+1``.
    3. Solve ``u · g₀ + v · h₀ = e_k`` in ℚ[x].
    4. Update ``g := g + v · y^k``, ``h := h + u · y^k``.

    Termination
    -----------
    After ``k = deg_y + 1`` iterations, the residual ``f − g·h`` has all
    y-coefficients equal to zero (degree-by-degree induction).  We then
    cross-verify the product equals ``f`` and return.

    Returns ``None`` if the diophantine solver fails (would indicate the
    coprimality precondition was violated).
    """
    g: BiPoly = _bi_uni_x(g0)
    h: BiPoly = _bi_uni_x(h0)

    # Lift one y-layer at a time.
    for k in range(1, deg_y + 1):
        error = _bi_sub(f, _bi_mul(g, h))
        if not error:
            break
        e_k = _bi_coeff_at_y_power(error, k)
        if not e_k:
            continue
        solved = _u_diophantine(g0, h0, e_k)
        if solved is None:
            return None
        u, v = solved
        # Add v · y^k to g; u · y^k to h.
        for i, c in enumerate(v):
            if c == 0:
                continue
            key = (i, k)
            g[key] = g.get(key, Fraction(0)) + c
        for i, c in enumerate(u):
            if c == 0:
                continue
            key = (i, k)
            h[key] = h.get(key, Fraction(0)) + c
        g = _bi_normalize(g)
        h = _bi_normalize(h)

    # Final verification.
    if _bi_mul(g, h) != _bi_normalize(f):
        return None
    return g, h


# ---------------------------------------------------------------------------
# Top-level: try_bivariate_hensel
# ---------------------------------------------------------------------------


def _y0_candidates() -> list[int]:
    """Enumeration order for ``y₀`` substitution points.

    Tries 0, 1, -1, 2, -2, …  up to ``_MAX_Y0_SEARCH`` total candidates.
    Small magnitudes minimise coefficient growth in the substitution,
    keeping the univariate factorisation fast.
    """
    out = [0]
    i = 1
    while len(out) < _MAX_Y0_SEARCH:
        out.append(i)
        if len(out) < _MAX_Y0_SEARCH:
            out.append(-i)
        i += 1
    return out


def _is_lucky(p: BiPoly, image: UniQPoly) -> bool:
    """Check that the univariate image is *lucky*: full x-degree and
    squarefree.

    A lucky substitution preserves the structure needed for Hensel:

    - **Full degree**: ``deg_x(image) == deg_x(p)``.  Otherwise the
      leading-coefficient-in-x of ``p`` vanished at ``y₀`` and the lift
      cannot recover it.
    - **Squarefree**: ``gcd(image, image') = constant``.  Otherwise two
      lifted factors would share a univariate factor at the base layer
      and the diophantine solver's coprimality assumption fails.
    """
    if _u_degree(image) != _bi_degree_x(p):
        return False
    if _u_degree(image) < 1:
        return False
    # Squarefree check: gcd with derivative is a constant.
    deriv: UniQPoly = []
    for i in range(1, len(image)):
        deriv.append(Fraction(i) * image[i])
    deriv = _u_normalize(deriv)
    if not deriv:
        return False
    g, _, _ = _u_gcd_ext(image, deriv)
    return _u_degree(g) == 0


def try_bivariate_hensel(f: BiPoly) -> list[BiPoly] | None:
    """Attempt to factor a bivariate polynomial via Hensel lifting.

    Parameters
    ----------
    f : BiPoly
        A bivariate polynomial as ``dict[(i, j) ↦ Fraction]`` with key
        ``(i, j)`` meaning ``x^i · y^j``.  Coefficients are rationals.

    Returns
    -------
    list[BiPoly] | None
        A flat list of irreducible bivariate factors whose product
        equals ``f``, or ``None`` if no non-trivial factorisation was
        found.  ``None`` is returned when:

        - ``f`` is degenerate (zero or single-variable).
        - No lucky ``y₀`` substitution gives a squarefree univariate
          image of full x-degree (we try a bounded list of small
          integers).
        - The univariate image is irreducible over ℚ.
        - The lifted factors fail final-product verification (would
          indicate a bug — should not occur in practice).

    Algorithm
    ---------
    See module docstring.  In short: pick a small integer ``y₀`` with
    a lucky image; factor the image univariately; lift each factor
    back to bivariate by solving a univariate diophantine equation per
    y-layer.

    Multi-factor handling
    ---------------------
    When the univariate image splits into ``r ≥ 2`` factors, we iterate
    the two-factor lift: peel off the first factor against the product
    of the remaining ``r − 1``, then recurse on the bivariate
    "remainder" factor.  This avoids the engineering complexity of a
    proper r-way diophantine solver while still handling typical
    graduate-engineering inputs (most multi-factor cases are r = 2 or
    3).
    """
    f = _bi_normalize(f)
    if not f:
        return None
    # Bivariate-only guard: must mention both x and y.
    if _bi_degree_y(f) < 1:
        return None  # univariate in x — fall through to existing handler
    if _bi_degree_x(f) < 1:
        return None  # univariate in y — caller should swap variables

    deg_y = _bi_degree_y(f)

    for y0 in _y0_candidates():
        # Shift to recentre around y = y₀ if non-zero.  The lift always
        # operates in the "y₀ = 0" frame; we undo the shift at the end.
        y0_frac = Fraction(y0)
        f_shifted = _bi_shift_y(f, y0_frac)
        image = _bi_substitute_y(f_shifted, Fraction(0))
        if not _is_lucky(f_shifted, image):
            continue

        # Factor the univariate image over ℚ.
        uni_factors = _factor_uni_q(image)
        if uni_factors is None or len(uni_factors) < 2:
            continue

        # Iteratively peel off one factor at a time.
        remaining_bi = f_shifted
        bi_factors: list[BiPoly] = []
        remaining_uni = list(uni_factors)
        success = True

        while len(remaining_uni) >= 2:
            g0 = remaining_uni[0]
            # h0 = product of the rest.
            h0: UniQPoly = [Fraction(1)]
            for q in remaining_uni[1:]:
                h0 = _u_mul(h0, q)

            lifted = _two_factor_lift(remaining_bi, g0, h0, deg_y)
            if lifted is None:
                success = False
                break
            g_bi, h_bi = lifted
            bi_factors.append(g_bi)
            remaining_bi = h_bi
            remaining_uni = remaining_uni[1:]

        if not success:
            continue
        # The last remaining factor is whatever's left.
        bi_factors.append(remaining_bi)

        # Unshift each factor back to the original y-frame.
        if y0 != 0:
            neg_y0 = Fraction(-y0)
            bi_factors = [_bi_shift_y(fac, neg_y0) for fac in bi_factors]

        # Verify product reconstructs f.
        prod: BiPoly = {(0, 0): Fraction(1)}
        for fac in bi_factors:
            prod = _bi_mul(prod, fac)
        if prod != _bi_normalize(f):
            continue

        # Filter out trivial factors (constants).  A non-constant factor
        # must mention at least one variable.
        non_trivial: list[BiPoly] = []
        scalar = Fraction(1)
        for fac in bi_factors:
            if _bi_degree_x(fac) == 0 and _bi_degree_y(fac) == 0:
                # Pure constant; fold into the scalar.
                if fac:
                    scalar *= next(iter(fac.values()))
            else:
                non_trivial.append(fac)

        if len(non_trivial) < 2:
            continue  # only one real factor — not a useful split

        if scalar != 1:
            non_trivial[0] = _bi_mul(non_trivial[0], {(0, 0): scalar})

        return non_trivial

    return None


# ---------------------------------------------------------------------------
# n-variate Hensel lifting — Track K1
# ---------------------------------------------------------------------------
#
# Strategy (one generic algorithm — NOT per-variable-count helpers):
#
#   1.  Pick a "main" variable v_0 (always index 0 in our sparse-tuple
#       representation; the caller is responsible for ordering vars lex
#       so x < y < z and v_0 = x).
#   2.  Substitute v_1..v_{n-1} with small integer values (the
#       "specialisation point") to reduce f down to a univariate
#       polynomial in v_0.
#   3.  Factor the univariate image via the existing
#       ``factor_integer_polynomial`` chain (rational roots + Kronecker
#       + BZH).
#   4.  Lift the univariate factors back to the full n-variate ring one
#       variable at a time.  At step k (k = 1, …, n−1) we have factors
#       of ``f|_{v_{k+1}=a_{k+1}, …, v_{n−1}=a_{n−1}}`` in
#       Q[v_0, …, v_{k−1}] and lift them to factors of
#       ``f|_{v_{k+1}=a_{k+1}, …}`` in Q[v_0, …, v_k] by Hensel-style
#       expansion in powers of ``(v_k − a_k)``.
#   5.  Each lift step solves a *coefficient-ring* diophantine equation
#       ``A·u + B·v = c`` in Q[v_0, …, v_{k−1}].  Recursively: when the
#       coefficient ring has ≥ 2 variables, specialise v_1..v_{k−1} to
#       reduce to Q[v_0], solve via the existing ``_u_diophantine``, then
#       lift back by the same Hensel-style expansion — base case
#       k = 1 hits ``_u_diophantine`` directly.
#   6.  Verify the final product equals the input; if not, return
#       ``None`` and let the caller fall through.
#
# Representation:
#   ``NPoly = dict[tuple[int, ...], Fraction]`` — tuple of length n,
#   where entry i is the exponent of variable v_i.  Empty dict is zero.
#
# Bounded resource discipline:
#   - At most _MAX_N_SPECIALISATION lucky-point tuples are tried.
#   - Recursion depth is bounded by n (number of variables).
#   - Each lift loop is bounded by ``deg_{v_k}(f) + 1`` iterations.

# An n-variate polynomial as a sparse dict ``{(e_0, …, e_{n−1}): coef}``.
# Tuple length equals the number of variables; the variable count must
# be passed alongside this dict because the empty polynomial doesn't
# carry it.
NPoly = dict[tuple[int, ...], Fraction]

# Bounded search for lucky n-variate specialisation tuples.  Each
# attempt costs O(univariate factor + lift).  10 tuples comfortably
# covers the acceptance cases without unbounded growth on adversarial
# inputs.
_MAX_N_SPECIALISATION = 10


def _n_normalize(p: NPoly) -> NPoly:
    """Drop zero coefficients."""
    return {k: v for k, v in p.items() if v != 0}


def _n_zero(num_vars: int) -> NPoly:  # noqa: ARG001 — kept for symmetry
    """The zero polynomial in ``num_vars`` variables (an empty dict)."""
    return {}


def _n_one(num_vars: int) -> NPoly:
    """The constant ``1`` polynomial."""
    return {tuple([0] * num_vars): Fraction(1)}


def _n_const(num_vars: int, c: Fraction) -> NPoly:
    """The constant ``c`` polynomial."""
    if c == 0:
        return {}
    return {tuple([0] * num_vars): c}


def _n_degree_in(p: NPoly, var_idx: int) -> int:
    """Highest exponent of variable ``var_idx`` in any non-zero term."""
    p = _n_normalize(p)
    if not p:
        return -1
    return max(k[var_idx] for k in p)


def _n_total_degree(p: NPoly) -> int:
    """Highest total degree (sum of exponents) of any non-zero term."""
    p = _n_normalize(p)
    if not p:
        return -1
    return max(sum(k) for k in p)


def _n_add(a: NPoly, b: NPoly) -> NPoly:
    """Sum of two n-variate polynomials."""
    out: NPoly = dict(a)
    for k, v in b.items():
        out[k] = out.get(k, Fraction(0)) + v
    return _n_normalize(out)


def _n_sub(a: NPoly, b: NPoly) -> NPoly:
    """Difference ``a − b``."""
    out: NPoly = dict(a)
    for k, v in b.items():
        out[k] = out.get(k, Fraction(0)) - v
    return _n_normalize(out)


def _n_mul(a: NPoly, b: NPoly) -> NPoly:
    """Product of two n-variate polynomials.

    Standard Cartesian-product accumulator: each pair of monomials
    contributes ``c1·c2`` at exponent-tuple ``k1 + k2``.
    """
    out: NPoly = {}
    for k1, c1 in a.items():
        if c1 == 0:
            continue
        for k2, c2 in b.items():
            if c2 == 0:
                continue
            k = tuple(a_ + b_ for a_, b_ in zip(k1, k2, strict=True))
            out[k] = out.get(k, Fraction(0)) + c1 * c2
    return _n_normalize(out)


def _n_substitute_var(p: NPoly, var_idx: int, value: Fraction) -> NPoly:
    """Substitute ``v_{var_idx} = value`` and drop that variable.

    Returns a polynomial in ``num_vars − 1`` variables (the tuple is
    shortened by removing position ``var_idx``).  Each monomial
    ``c · ∏ v_i^{e_i}`` contributes ``c · value^{e_{var_idx}}`` to the
    corresponding shorter monomial.
    """
    out: NPoly = {}
    for k, c in p.items():
        new_key = k[:var_idx] + k[var_idx + 1 :]
        contrib = c * (value ** k[var_idx])
        if contrib == 0:
            continue
        out[new_key] = out.get(new_key, Fraction(0)) + contrib
    return _n_normalize(out)


def _n_substitute_var_keep(p: NPoly, var_idx: int, value: Fraction) -> NPoly:
    """Substitute ``v_{var_idx} = value`` keeping the tuple shape.

    Unlike :func:`_n_substitute_var`, the output polynomial has the same
    number of variables as the input — the substituted variable's slot
    is just forced to exponent 0 in every monomial.  Useful when we want
    to compare polynomials in the same ring after substituting one of
    them.
    """
    out: NPoly = {}
    for k, c in p.items():
        new_key = k[:var_idx] + (0,) + k[var_idx + 1 :]
        contrib = c * (value ** k[var_idx])
        if contrib == 0:
            continue
        out[new_key] = out.get(new_key, Fraction(0)) + contrib
    return _n_normalize(out)


def _n_coeff_at_power(p: NPoly, var_idx: int, k: int) -> NPoly:
    """Return the (n−1)-variate coefficient of ``v_{var_idx}^k`` in ``p``.

    Used during the Hensel lift in variable ``v_{var_idx}`` to read off
    the residual at each power level.  The returned tuples drop the
    ``var_idx`` position.
    """
    out: NPoly = {}
    for key, c in p.items():
        if key[var_idx] == k:
            new_key = key[:var_idx] + key[var_idx + 1 :]
            out[new_key] = out.get(new_key, Fraction(0)) + c
    return _n_normalize(out)


def _n_embed_var(p: NPoly, var_idx: int, num_vars_out: int) -> NPoly:
    """Embed an (n−1)-variate polynomial into n-variate by inserting a
    zero exponent at position ``var_idx``.

    Inverse of :func:`_n_substitute_var` at ``value = 0``: takes a poly
    in ``num_vars_out − 1`` variables and lifts it into the n-variate
    ring with all ``v_{var_idx}`` exponents set to 0.
    """
    out: NPoly = {}
    for k, c in p.items():
        # k has length num_vars_out - 1
        new_key = k[:var_idx] + (0,) + k[var_idx:]
        out[new_key] = c
    return _n_normalize(out)


def _n_to_univariate(p: NPoly, var_idx: int) -> UniQPoly:
    """Convert a polynomial that genuinely depends on only one variable
    to a ``UniQPoly``.

    Pre: every non-zero monomial has zero exponent on every variable
    except ``v_{var_idx}``.  Used at the base of the lift recursion to
    hand off to ``_u_diophantine``.
    """
    if not p:
        return []
    max_e = max(k[var_idx] for k in p)
    out: UniQPoly = [Fraction(0)] * (max_e + 1)
    for k, c in p.items():
        # Other coordinates must be zero by precondition.
        out[k[var_idx]] += c
    return _u_normalize(out)


def _u_to_n(p: UniQPoly, var_idx: int, num_vars: int) -> NPoly:
    """Embed a univariate polynomial into the n-variate ring as a
    polynomial in ``v_{var_idx}``."""
    out: NPoly = {}
    for e, c in enumerate(p):
        if c == 0:
            continue
        key = tuple(e if i == var_idx else 0 for i in range(num_vars))
        out[key] = Fraction(c)
    return out


def _n_shift_var(p: NPoly, var_idx: int, value: Fraction) -> NPoly:
    """Rewrite ``p`` as a polynomial in ``(v_{var_idx} − value)``.

    The substitution ``v ↦ (v - value) + value`` distributes through
    each monomial's ``v^e`` factor via the binomial theorem.  Mirror of
    :func:`_bi_shift_y` but in the n-variate sparse representation.
    """
    if value == 0:
        return dict(p)
    from math import comb

    out: NPoly = {}
    for k, c in p.items():
        if c == 0:
            continue
        e = k[var_idx]
        # v^e = ((v - value) + value)^e = Σ C(e, m) (v - value)^m value^(e-m)
        for m in range(e + 1):
            coeff = c * comb(e, m) * (value ** (e - m))
            new_key = k[:var_idx] + (m,) + k[var_idx + 1 :]
            out[new_key] = out.get(new_key, Fraction(0)) + coeff
    return _n_normalize(out)


def _n_only_uses_var(p: NPoly, var_idx: int) -> bool:
    """True if every non-zero monomial of ``p`` has zero exponent on every
    variable except ``v_{var_idx}``.

    Used as a pre-condition check at the base of the recursive
    diophantine: at depth 0 we expect the polynomial to live in a
    single variable.
    """
    for k in p:
        for i, e in enumerate(k):
            if i != var_idx and e != 0:
                return False
    return True


# ---------------------------------------------------------------------------
# Recursive coefficient-ring diophantine
# ---------------------------------------------------------------------------


def _n_diophantine(
    g0: NPoly,
    h0: NPoly,
    c: NPoly,
    num_vars: int,
    main_var: int,
    active_vars: tuple[int, ...],
) -> tuple[NPoly, NPoly] | None:
    """Solve ``u·g0 + v·h0 = c`` in ``Q[active_vars]`` ⊂ ``Q[v_0, …, v_{n−1}]``.

    Parameters
    ----------
    g0, h0, c : NPoly
        Polynomials in the n-variate ring.  Only coordinates listed in
        ``active_vars`` may be non-zero; all others must equal zero on
        every monomial.
    main_var : int
        The "x" variable in the recursive structure; always index 0 in
        ``active_vars`` (the leading entry).
    active_vars : tuple[int, ...]
        The subset of variable indices whose ring we're solving in.
        Length-1 means univariate base case; longer means recurse on
        the trailing variable.

    Algorithm
    ---------
    *Base case* (``len(active_vars) == 1``): convert each NPoly to
    ``UniQPoly``, call ``_u_diophantine``, lift the result back.

    *Recursive case*: let ``w = active_vars[-1]`` be the variable to
    eliminate.  Pick a small integer ``w_0`` such that ``g0|_{w=w_0}``
    and ``h0|_{w=w_0}`` remain coprime when projected to the
    coefficient ring of one fewer variable.  Then:

      1. Solve at ``w = w_0`` recursively to get ``u_0, v_0`` in
         ``Q[active_vars − {w}]``.
      2. Lift ``u_0, v_0`` to ``Q[active_vars]`` by adding corrections
         at successive powers of ``(w − w_0)``.  Each correction is
         again a recursive diophantine in ``Q[active_vars − {w}]``.

    Returns ``None`` if no lucky ``w_0`` produces a solvable system, or
    if the recursion's base case fails.

    Bounded resources
    -----------------
    - Specialisation search: at most ``_MAX_N_SPECIALISATION`` tuples
      per recursive level.
    - Recursion depth: ``len(active_vars)`` ≤ ``n``.
    - Lift loop: ``deg_w(c) + 1`` iterations per level.
    """
    # Base case: univariate ring — convert and call _u_diophantine.
    if len(active_vars) == 1:
        only = active_vars[0]
        # All inputs must be polynomials in just v_{only}.
        if not (
            _n_only_uses_var(g0, only)
            and _n_only_uses_var(h0, only)
            and _n_only_uses_var(c, only)
        ):
            return None
        g0_u = _n_to_univariate(g0, only)
        h0_u = _n_to_univariate(h0, only)
        c_u = _n_to_univariate(c, only)
        solved = _u_diophantine(g0_u, h0_u, c_u)
        if solved is None:
            return None
        u_u, v_u = solved
        return (
            _u_to_n(u_u, only, num_vars),
            _u_to_n(v_u, only, num_vars),
        )

    # Recursive case: eliminate the last variable in active_vars.
    w = active_vars[-1]
    rest = active_vars[:-1]

    # Bound the lift by w-degree across g0, h0, c — the residual after
    # full lift cannot exceed the max input w-degree.
    max_w_deg = max(
        _n_degree_in(g0, w),
        _n_degree_in(h0, w),
        _n_degree_in(c, w),
        0,
    )

    for w0_int in _y0_candidates()[:_MAX_N_SPECIALISATION]:
        w0 = Fraction(w0_int)
        # Shift to recentre around w = w_0 (so lift is in (w − w_0)).
        g0_shift = _n_shift_var(g0, w, w0)
        h0_shift = _n_shift_var(h0, w, w0)
        c_shift = _n_shift_var(c, w, w0)
        # Project to w = 0 in the shifted ring.
        g0_base = _n_coeff_at_power(g0_shift, w, 0)
        h0_base = _n_coeff_at_power(h0_shift, w, 0)
        c_base = _n_coeff_at_power(c_shift, w, 0)
        # Re-embed without the w slot — these now live in num_vars but
        # with w-coordinate = 0 implicit.
        g0_base_n = _n_embed_var(g0_base, w, num_vars)
        h0_base_n = _n_embed_var(h0_base, w, num_vars)
        c_base_n = _n_embed_var(c_base, w, num_vars)

        # Solve recursively at w = w_0.
        solved = _n_diophantine(
            g0_base_n, h0_base_n, c_base_n, num_vars, main_var, rest
        )
        if solved is None:
            continue
        u, v = solved  # both in the smaller ring, embedded back at w=0

        # Lift in powers of (w - w_0).
        success = True
        for k in range(1, max_w_deg + 1):
            # Compute residual = c_shift − (u·g0_shift + v·h0_shift),
            # read off [w^k] coefficient.
            prod = _n_add(_n_mul(u, g0_shift), _n_mul(v, h0_shift))
            err = _n_sub(c_shift, prod)
            if not err:
                break
            e_k = _n_coeff_at_power(err, w, k)
            if not e_k:
                continue
            e_k_n = _n_embed_var(e_k, w, num_vars)
            sub = _n_diophantine(
                g0_base_n, h0_base_n, e_k_n, num_vars, main_var, rest
            )
            if sub is None:
                success = False
                break
            du, dv = sub
            # Multiply du and dv by (w - w_0)^k.  In the shifted ring
            # that's just w^k (since shift was applied), so embed each
            # in the n-variate ring shifting w-coord from 0 to k.
            for key, coef in list(du.items()):
                new_key = key[:w] + (k,) + key[w + 1 :]
                u[new_key] = u.get(new_key, Fraction(0)) + coef
            for key, coef in list(dv.items()):
                new_key = key[:w] + (k,) + key[w + 1 :]
                v[new_key] = v.get(new_key, Fraction(0)) + coef
            u = _n_normalize(u)
            v = _n_normalize(v)

        if not success:
            continue

        # Verify: u·g0_shift + v·h0_shift == c_shift in the shifted ring.
        check = _n_add(_n_mul(u, g0_shift), _n_mul(v, h0_shift))
        if check != _n_normalize(c_shift):
            continue

        # Unshift u and v back to the original w-frame.
        if w0 != 0:
            u = _n_shift_var(u, w, -w0)
            v = _n_shift_var(v, w, -w0)
        return u, v

    return None


# ---------------------------------------------------------------------------
# n-variate two-factor lift
# ---------------------------------------------------------------------------


def _n_two_factor_lift(
    f: NPoly,
    g0: NPoly,
    h0: NPoly,
    num_vars: int,
    main_var: int,
    lift_var: int,
    coeff_vars: tuple[int, ...],
) -> tuple[NPoly, NPoly] | None:
    """Lift ``f ≡ g0 · h0 (mod v_{lift_var})`` to exact factors in
    ``Q[v_0, …, v_{n−1}]`` (where the active subring during this lift
    is ``main_var ∪ coeff_vars ∪ {lift_var}``).

    Preconditions
    -------------
    - ``f(.., lift_var = 0, ..) = g0 · h0`` in ``Q[main_var, *coeff_vars]``.
    - ``gcd(g0|_{coeff_vars=specialised}, h0|_{coeff_vars=specialised}) = 1``
      in ``Q[main_var]`` (checked indirectly via the diophantine
      solver succeeding).
    - ``f``'s ``lift_var``-degree is bounded; the lift terminates after
      ``deg_{lift_var}(f) + 1`` iterations.

    Loop invariant
    --------------
    At the start of iteration ``k`` we have polynomials ``g, h`` with
    ``f ≡ g · h (mod v_{lift_var}^k)``.  Each iteration solves a
    coefficient-ring diophantine ``Δg · h0 + Δh · g0 = e_k`` (the
    coefficient of ``v_{lift_var}^k`` in the current residual).

    Returns ``None`` if the diophantine solver fails or final
    verification mismatches.
    """
    g: NPoly = dict(g0)
    h: NPoly = dict(h0)

    deg_lift = _n_degree_in(f, lift_var)
    active = (main_var,) + tuple(coeff_vars)

    for k in range(1, deg_lift + 1):
        error = _n_sub(f, _n_mul(g, h))
        if not error:
            break
        e_k = _n_coeff_at_power(error, lift_var, k)
        if not e_k:
            continue
        # e_k lives in num_vars − 1 implicit dims (lift_var dropped).
        # Re-embed at lift_var = 0 so all inputs share the n-variate ring.
        e_k_n = _n_embed_var(e_k, lift_var, num_vars)
        solved = _n_diophantine(
            g0, h0, e_k_n, num_vars, main_var, active
        )
        if solved is None:
            return None
        du, dv = solved
        # Multiply du, dv by v_{lift_var}^k.
        for key, coef in list(du.items()):
            new_key = key[:lift_var] + (k,) + key[lift_var + 1 :]
            # `du` was solved with v in slot of h0 — i.e. it's the
            # correction to h (matching the bivariate convention).
            h[new_key] = h.get(new_key, Fraction(0)) + coef
        for key, coef in list(dv.items()):
            new_key = key[:lift_var] + (k,) + key[lift_var + 1 :]
            g[new_key] = g.get(new_key, Fraction(0)) + coef
        g = _n_normalize(g)
        h = _n_normalize(h)

    if _n_mul(g, h) != _n_normalize(f):
        return None
    return g, h


# ---------------------------------------------------------------------------
# Top-level n-variate Hensel
# ---------------------------------------------------------------------------


def _n_specialisation_candidates(num_aux: int) -> list[tuple[int, ...]]:
    """Enumerate small integer tuples of length ``num_aux`` as
    candidate specialisation points for the auxiliary variables.

    Order: try the all-ones tuple first (usually well-conditioned),
    then mixes of 1, 2, −1, 3, −2.  Capped at
    ``_MAX_N_SPECIALISATION`` total tuples.
    """
    if num_aux == 0:
        return [()]
    primitives: list[int] = [1, 2, -1, 3, -2]
    tuples: list[tuple[int, ...]] = []
    # Tier 1: constant tuples (1,1,…), (2,2,…), …
    for v in primitives:
        tuples.append(tuple([v] * num_aux))
        if len(tuples) >= _MAX_N_SPECIALISATION:
            return tuples
    # Tier 2: vary one coordinate at a time off the (1,1,…) base.
    base = [1] * num_aux
    for i in range(num_aux):
        for v in primitives[1:]:
            cand = list(base)
            cand[i] = v
            tuples.append(tuple(cand))
            if len(tuples) >= _MAX_N_SPECIALISATION:
                return tuples
    return tuples[:_MAX_N_SPECIALISATION]


def _is_lucky_uni(p_n: NPoly, image: UniQPoly, main_var: int) -> bool:
    """Lucky-image check for the n-variate specialisation.

    The image must (a) have full ``v_{main_var}``-degree (the leading
    coefficient survived the specialisation) and (b) be squarefree so
    the diophantine solver's coprimality assumption holds.
    """
    if _u_degree(image) != _n_degree_in(p_n, main_var):
        return False
    if _u_degree(image) < 1:
        return False
    deriv: UniQPoly = []
    for i in range(1, len(image)):
        deriv.append(Fraction(i) * image[i])
    deriv = _u_normalize(deriv)
    if not deriv:
        return False
    g, _, _ = _u_gcd_ext(image, deriv)
    return _u_degree(g) == 0


def try_n_variate_hensel(f: NPoly, num_vars: int) -> list[NPoly] | None:
    """Attempt to factor an n-variate polynomial via iterated bivariate
    Hensel lifting.

    Parameters
    ----------
    f : NPoly
        Sparse-dict polynomial in ``Q[v_0, …, v_{num_vars − 1}]``.
        Every key is a tuple of length ``num_vars``.
    num_vars : int
        Total number of variables.  Must be ≥ 1.

    Returns
    -------
    list[NPoly] | None
        A list of irreducible n-variate factors whose product equals
        ``f``, or ``None`` if no factorisation was found.

    Falls through (``None``) when:

    - ``num_vars < 2`` — the univariate path handles this elsewhere.
    - No lucky specialisation tuple gives a squarefree univariate image
      of full ``v_0``-degree (bounded search of
      ``_MAX_N_SPECIALISATION`` tuples).
    - The univariate image is irreducible.
    - Some lift step or the final product verification fails.

    For ``num_vars == 2`` the implementation can be checked against
    :func:`try_bivariate_hensel` (this routine returns identical
    factorings, up to ordering).
    """
    f = _n_normalize(f)
    if not f:
        return None
    if num_vars < 2:
        return None
    # f must non-trivially depend on at least 2 variables.
    if _n_degree_in(f, 0) < 1:
        return None
    if not any(_n_degree_in(f, i) >= 1 for i in range(1, num_vars)):
        return None

    main_var = 0
    aux_vars = list(range(1, num_vars))

    for spec_tuple in _n_specialisation_candidates(len(aux_vars)):
        spec = {aux_vars[i]: Fraction(spec_tuple[i]) for i in range(len(aux_vars))}

        # Shift f so each auxiliary variable is recentred to 0 at
        # spec.  After shifting, the "0-point" of the lift is the
        # origin in the shifted frame.
        f_shift = f
        for v_i, w_i in spec.items():
            f_shift = _n_shift_var(f_shift, v_i, w_i)
        f_shift = _n_normalize(f_shift)

        # Specialise all aux vars to 0 in the shifted frame → univariate
        # in v_main.
        f_uni_npoly = f_shift
        for v_i in aux_vars:
            f_uni_npoly = _n_substitute_var_keep(f_uni_npoly, v_i, Fraction(0))
        if not _n_only_uses_var(f_uni_npoly, main_var):
            continue
        image = _n_to_univariate(f_uni_npoly, main_var)
        if not _is_lucky_uni(f_shift, image, main_var):
            continue

        uni_factors = _factor_uni_q(image)
        if uni_factors is None or len(uni_factors) < 2:
            continue

        # Embed each univariate factor in the n-variate ring at main_var.
        n_factors_current: list[NPoly] = [
            _u_to_n(u, main_var, num_vars) for u in uni_factors
        ]

        # Lift one auxiliary variable at a time.
        success = True
        # At each lift step, the "current f" is f_shift with all aux vars
        # AFTER the current one substituted to 0, and aux vars UP TO
        # current one free.  We re-derive this fresh at each step from
        # the original f_shift to avoid drift.
        for lift_idx, lift_var in enumerate(aux_vars):
            # Build f at this stage: f_shift specialised at aux vars
            # AFTER lift_var to zero.
            f_stage = f_shift
            for later in aux_vars[lift_idx + 1 :]:
                f_stage = _n_substitute_var_keep(f_stage, later, Fraction(0))
            f_stage = _n_normalize(f_stage)

            coeff_vars = tuple(aux_vars[:lift_idx])

            # Iteratively peel one factor at a time.
            remaining = f_stage
            new_factors: list[NPoly] = []
            remaining_factors = list(n_factors_current)
            while len(remaining_factors) >= 2:
                g0 = remaining_factors[0]
                h0 = _n_one(num_vars)
                for q in remaining_factors[1:]:
                    h0 = _n_mul(h0, q)
                lifted = _n_two_factor_lift(
                    remaining, g0, h0, num_vars, main_var, lift_var, coeff_vars
                )
                if lifted is None:
                    success = False
                    break
                g_lift, h_lift = lifted
                new_factors.append(g_lift)
                remaining = h_lift
                remaining_factors = remaining_factors[1:]
            if not success:
                break
            new_factors.append(remaining)
            n_factors_current = new_factors

        if not success:
            continue

        # Unshift each factor back to the original frame.
        result = list(n_factors_current)
        for v_i, w_i in spec.items():
            result = [_n_shift_var(fac, v_i, -w_i) for fac in result]
        result = [_n_normalize(fac) for fac in result]

        # Verify product reconstructs f.
        prod: NPoly = _n_one(num_vars)
        for fac in result:
            prod = _n_mul(prod, fac)
        if prod != _n_normalize(f):
            continue

        # Drop trivial constant factors; fold their scalar into factor 0.
        non_trivial: list[NPoly] = []
        scalar = Fraction(1)
        for fac in result:
            if _n_total_degree(fac) <= 0:
                # Pure constant.
                if fac:
                    scalar *= next(iter(fac.values()))
            else:
                non_trivial.append(fac)
        if len(non_trivial) < 2:
            continue
        if scalar != 1:
            non_trivial[0] = _n_mul(
                non_trivial[0], _n_const(num_vars, scalar)
            )
        return non_trivial

    return None
