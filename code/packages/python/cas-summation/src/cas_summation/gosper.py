"""Gosper's algorithm for indefinite hypergeometric summation.

Track H1 of ``code/specs/macsyma-truly-finish-plan.md``.

Background
==========

Gosper's 1978 algorithm finds, for a hypergeometric term ``a(k)``, a
closed-form *antidifference* ``T(k)`` satisfying ::

    T(k+1) - T(k) = a(k).

When such a ``T`` exists, the finite sum closes by the discrete analog
of the fundamental theorem of calculus ::

    ∑_{k=lo}^{hi} a(k) = T(hi+1) - T(lo).

The classical examples this module unlocks:

* ``∑_{k=1}^{N} k·2^k = (N-1)·2^(N+1) + 2``
* ``∑_{k=0}^{N} k·k!   = (N+1)! - 1``
* ``∑_{k=0}^{N} (-1)^k·k = …``

What "hypergeometric" means
---------------------------

A term ``a(k)`` is **hypergeometric** when the ratio ::

    r(k) = a(k+1) / a(k)

is a rational function of ``k`` (i.e. ratio of polynomials in ``k`` with
no factorials, powers k^k, etc.).  Equivalently, ``a(k)`` is built out
of these building blocks::

    a(k) = polynomial(k)
         · c^k          for constants c
         · k!           (= GammaFunc(k+1))
         · (a+k)!       (rising factorials)
         · binomial(N, k)
         · …

since each piece has a rational shift ratio:

* ``(k+1)^m / k^m``               → ``((k+1)/k)^m``
* ``c^(k+1) / c^k = c``           → constant (still "rational")
* ``(k+1)! / k! = k+1``           → linear in k

This module's strategy
======================

Rather than computing arbitrary symbolic ratios on the IR (which would
require general simplification we don't have), we **factor the summand
structurally** into the canonical hypergeometric building blocks listed
above.  For each piece we know the rational shift ratio in closed form.
Combining them gives ``r(k) = A(k) / B(k)`` as two polynomials in ``k``
with exact ``Fraction`` coefficients.

We then run the textbook Gosper pipeline on those polynomials:

1. **Petkovšek normalisation** — rewrite ``r(k) = (p(k+1)/p(k)) · q(k)/r(k)``
   with the shift-coprime property ``gcd(q(k), r(k+h)) = 1`` for every
   non-negative integer ``h``.  This is the canonical form Gosper's
   key equation assumes.

2. **Degree bound for x(k)** — the unknown polynomial ``x(k)`` in
   Gosper's key equation ``p(k) = q(k+1)·x(k+1) − r(k)·x(k)`` has a
   degree bounded by an explicit formula in the degrees and leading
   coefficients of ``p, q, r``.

3. **Solve the linear system** — match coefficients of ``k^i`` on both
   sides of the key equation; this is a linear system in the unknown
   coefficients of ``x``, solved by Gaussian elimination over the
   rationals (``fractions.Fraction``).

4. **Reconstruct ``T(k)``** — if a polynomial solution exists, the
   antidifference is ``T(k) = (q(k) · x(k) / p(k)) · a(k)`` (Petkovšek
   form; equivalent to the textbook ``T(k) = r(k-1)·x(k)·a(k)/p(k)``).

5. **Return the closed-form finite sum** ``T(hi+1) − T(lo)`` as an IR
   expression.

If any step fails (ratio not factorable into recognised pieces, key
equation has no polynomial solution of the bounded degree), we return
``None`` so the dispatcher in :mod:`cas_summation.summation` can fall
through to its other handlers and ultimately to the unevaluated
``Sum(...)`` form.

Conservative philosophy
-----------------------

Gosper is a *deciding* algorithm in theory — given a fully symbolic
hypergeometric term it returns either the closed form or a proof that
none exists.  This implementation is **incomplete** in two practical
senses: (a) we only recognise the structural building blocks listed
above (the textbook hypergeometric vocabulary covers more shapes), and
(b) we don't currently exploit the no-solution proof side.  Both are
deliberate — keeping the surface area small means the wins are
high-precision and easy to verify, and the dispatcher already has
strong fallbacks (Faulhaber, geometric, telescope) for shapes the
factoring layer doesn't cover.

References
----------

* R. W. Gosper Jr., "Decision procedure for indefinite hypergeometric
  summation", *Proc. Natl. Acad. Sci. USA* 75:1 (1978), pp. 40-42.
* M. Petkovšek, H. S. Wilf, D. Zeilberger, "A = B" (1996), chapter 5.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    DIV,
    GAMMA_FUNC,
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

# Polynomials in ``k`` are represented as ``list[Fraction]`` where
# ``p[i]`` is the coefficient of ``k^i``.  The zero polynomial is
# ``[]``; ``[Fraction(0)]`` is also accepted but normalised away by
# :func:`_poly_trim`.
Poly = list[Fraction]


# Defensive cap on polynomial degree.  Without this, an adversarial
# summand like ``Pow(k, 10**9)`` would balloon ``_ir_to_poly`` (which
# does repeated multiplication) into a memory-bomb.  Gosper-summable
# expressions in practice have very small polynomial degree (typically
# ≤ 5) — anything above this cap is almost certainly not Gosper-
# accessible and the dispatcher's other paths handle it equally well.
_MAX_POLY_DEGREE = 64


# ---------------------------------------------------------------------------
# Univariate polynomial arithmetic over the rationals.
#
# We implement just enough to run Gosper: add, subtract, multiply, scalar
# divide, shift (substitute k → k+h), GCD via Euclid, and a Gaussian
# linear solver.  All exact; no floats.
# ---------------------------------------------------------------------------


def _poly_trim(p: Poly) -> Poly:
    """Strip trailing zero coefficients so the leading coefficient is
    non-zero (or the polynomial is empty)."""
    out = list(p)
    while out and out[-1] == 0:
        out.pop()
    return out


def _poly_deg(p: Poly) -> int:
    """Polynomial degree; the zero polynomial conventionally has degree
    ``-1`` (so that ``deg(0) < deg(any non-zero)``)."""
    pp = _poly_trim(p)
    return len(pp) - 1


def _poly_add(a: Poly, b: Poly) -> Poly:
    """Pointwise sum of coefficient lists, padding the shorter input."""
    n = max(len(a), len(b))
    out = [Fraction(0)] * n
    for i in range(n):
        if i < len(a):
            out[i] += a[i]
        if i < len(b):
            out[i] += b[i]
    return _poly_trim(out)


def _poly_sub(a: Poly, b: Poly) -> Poly:
    """``a - b`` over the rationals."""
    return _poly_add(a, [-c for c in b])


def _poly_mul(a: Poly, b: Poly) -> Poly:
    """Schoolbook multiplication.  ``O(deg(a)·deg(b))`` — fine for the
    small polynomials Gosper sees."""
    a = _poly_trim(a)
    b = _poly_trim(b)
    if not a or not b:
        return []
    out = [Fraction(0)] * (len(a) + len(b) - 1)
    for i, ca in enumerate(a):
        for j, cb in enumerate(b):
            out[i + j] += ca * cb
    return _poly_trim(out)


def _poly_scalar(p: Poly, c: Fraction) -> Poly:
    """Multiply every coefficient by ``c``."""
    if c == 0:
        return []
    return [x * c for x in p]


def _poly_shift(p: Poly, h: int) -> Poly:
    """Return ``p(k + h)`` as a new polynomial.

    Uses the binomial expansion ``(k + h)^i = Σ_j C(i, j)·h^(i-j)·k^j``.
    Pure integer arithmetic — no floats.
    """
    n = len(p)
    out = [Fraction(0)] * n
    for i in range(n):
        if p[i] == 0:
            continue
        # Compute coefficients of (k + h)^i and accumulate p[i] times that.
        # Pascal's triangle row i.
        binom = 1
        for j in range(i + 1):
            # C(i, j) · h^(i - j)
            out[j] += p[i] * binom * (h ** (i - j))
            binom = binom * (i - j) // (j + 1)
    return _poly_trim(out)


def _poly_divmod(a: Poly, b: Poly) -> tuple[Poly, Poly]:
    """Polynomial long division.  Returns ``(quotient, remainder)``
    with ``a = quotient·b + remainder`` and ``deg(remainder) < deg(b)``.

    ``b`` must be non-zero.  Coefficients live in the rationals so the
    division is always exact in that field.
    """
    a = _poly_trim(a)
    b = _poly_trim(b)
    if not b:
        raise ZeroDivisionError("polynomial division by zero")
    if _poly_deg(a) < _poly_deg(b):
        return [], a
    q = [Fraction(0)] * (len(a) - len(b) + 1)
    r = list(a)
    while _poly_deg(r) >= _poly_deg(b):
        # Leading term of r / leading term of b.
        deg_diff = _poly_deg(r) - _poly_deg(b)
        coeff = r[-1] / b[-1]
        q[deg_diff] = coeff
        # Subtract coeff · k^deg_diff · b from r.
        shifted = [Fraction(0)] * deg_diff + [c * coeff for c in b]
        r = _poly_sub(r, shifted)
    return _poly_trim(q), _poly_trim(r)


def _poly_gcd(a: Poly, b: Poly) -> Poly:
    """Monic GCD over the rationals via Euclid's algorithm.

    Output is normalised to have leading coefficient ``1`` (or empty if
    both inputs are zero).
    """
    a, b = _poly_trim(a), _poly_trim(b)
    while b:
        _, r = _poly_divmod(a, b)
        a, b = b, r
    if not a:
        return []
    # Monic-normalise.
    lc = a[-1]
    return [c / lc for c in a]


def _poly_eq(a: Poly, b: Poly) -> bool:
    return _poly_trim(a) == _poly_trim(b)


def _poly_is_const(p: Poly) -> bool:
    """True if the polynomial is a (possibly zero) constant."""
    return _poly_deg(p) <= 0


def _solve_linear_system(
    matrix: list[list[Fraction]], rhs: list[Fraction]
) -> list[Fraction] | None:
    """Solve ``M · x = rhs`` over the rationals via Gaussian elimination.

    Returns the solution vector ``x``, or ``None`` if the system is
    inconsistent.  Under-determined systems pick free variables = 0
    (sufficient for Gosper — any valid ``x(k)`` works as long as it
    satisfies the key equation).

    Pure rational arithmetic via ``fractions.Fraction``; no float
    comparisons, no pivot-tolerance heuristics.
    """
    if not matrix:
        return [Fraction(0)] * 0 if not rhs else None
    rows = len(matrix)
    cols = len(matrix[0])
    # Build the augmented matrix.
    m = [list(row) + [rhs[i]] for i, row in enumerate(matrix)]
    # Forward elimination with partial pivoting (any non-zero pivot works).
    row = 0
    for col in range(cols):
        # Find a non-zero pivot in this column at or below ``row``.
        pivot = -1
        for r in range(row, rows):
            if m[r][col] != 0:
                pivot = r
                break
        if pivot == -1:
            continue  # Free variable.
        m[row], m[pivot] = m[pivot], m[row]
        # Normalise the pivot row.
        piv = m[row][col]
        m[row] = [c / piv for c in m[row]]
        # Eliminate from other rows.
        for r in range(rows):
            if r == row:
                continue
            factor = m[r][col]
            if factor == 0:
                continue
            m[r] = [m[r][i] - factor * m[row][i] for i in range(cols + 1)]
        row += 1
    # Check for inconsistency: 0 = nonzero in any row.
    for r in range(rows):
        if all(m[r][c] == 0 for c in range(cols)) and m[r][cols] != 0:
            return None
    # Read off the solution; columns without a pivot get value 0.
    x = [Fraction(0)] * cols
    row_for_col: dict[int, int] = {}
    for r in range(rows):
        for c in range(cols):
            if m[r][c] == 1 and all(m[r2][c] == 0 for r2 in range(rows) if r2 != r):
                row_for_col[c] = r
                break
    for c in range(cols):
        if c in row_for_col:
            x[c] = m[row_for_col[c]][cols]
    return x


# ---------------------------------------------------------------------------
# IR ↔ polynomial bridge.
#
# We convert pure-polynomial IR sub-trees in ``k`` to ``Poly`` (and
# back).  Anything we can't represent (transcendentals, division, free
# symbols other than ``k``) makes us return ``None`` and bail out.
# ---------------------------------------------------------------------------


def _rational_of(node: IRNode) -> Fraction | None:
    """Lift an integer/rational IR literal to ``Fraction``; else ``None``."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    return None


def _ir_to_poly(node: IRNode, k: IRSymbol) -> Poly | None:
    """Convert an IR expression that is a polynomial in ``k`` to ``Poly``.

    Returns ``None`` when the expression has any non-polynomial-in-k
    structure (division by k-bearing denominator, transcendentals, free
    non-k symbols, fractional/negative exponents, …).

    Constants (anything not containing ``k``) become a degree-0
    polynomial as long as they're rational literals; symbolic free
    constants cause us to bail (Gosper's coefficient solving needs
    concrete rationals).
    """
    # Constant literal.
    r = _rational_of(node)
    if r is not None:
        return [r]
    # Bare ``k``.
    if isinstance(node, IRSymbol):
        if node == k:
            return [Fraction(0), Fraction(1)]
        # Free symbol — cannot proceed; coefficient system would have an
        # unknown literal.
        return None
    if not isinstance(node, IRApply):
        return None
    if node.head == NEG and len(node.args) == 1:
        inner = _ir_to_poly(node.args[0], k)
        return None if inner is None else _poly_scalar(inner, Fraction(-1))
    if node.head == ADD:
        out: Poly = []
        for arg in node.args:
            sub = _ir_to_poly(arg, k)
            if sub is None:
                return None
            out = _poly_add(out, sub)
        return out
    if node.head == SUB and len(node.args) == 2:
        a = _ir_to_poly(node.args[0], k)
        b = _ir_to_poly(node.args[1], k)
        if a is None or b is None:
            return None
        return _poly_sub(a, b)
    if node.head == MUL:
        out = [Fraction(1)]
        for arg in node.args:
            sub = _ir_to_poly(arg, k)
            if sub is None:
                return None
            out = _poly_mul(out, sub)
        return out
    if node.head == POW and len(node.args) == 2:
        base, exp = node.args
        base_poly = _ir_to_poly(base, k)
        if base_poly is None:
            return None
        if not isinstance(exp, IRInteger) or exp.value < 0:
            return None
        # Cap the exponent to avoid memory blow-up on hostile inputs
        # (e.g. ``Pow(k, 10**9)``).  Polynomials above the cap can't
        # plausibly be Gosper-summable inside this module's vocabulary.
        if exp.value > _MAX_POLY_DEGREE:
            return None
        result = [Fraction(1)]
        for _ in range(exp.value):
            result = _poly_mul(result, base_poly)
            if _poly_deg(result) > _MAX_POLY_DEGREE:
                return None
        return result
    if node.head == DIV and len(node.args) == 2:
        num, den = node.args
        np = _ir_to_poly(num, k)
        dp = _ir_to_poly(den, k)
        if np is None or dp is None:
            return None
        # Only honour exact polynomial division by a non-zero constant
        # (degree-0) denominator.  Anything else means the expression
        # isn't a polynomial.
        if _poly_deg(dp) != 0:
            return None
        return _poly_scalar(np, Fraction(1) / dp[0])
    return None


def _poly_to_ir(p: Poly, k: IRSymbol) -> IRNode:
    """Convert ``Poly`` back to an IR expression.

    Uses Horner-style nesting for readability; the VM's normaliser
    cleans it up further at call time.  Zero polynomial → ``0``.
    Constant polynomial → just the constant.
    """
    p = _poly_trim(p)
    if not p:
        return IRInteger(0)

    def _frac_to_ir(f: Fraction) -> IRNode:
        if f.denominator == 1:
            return IRInteger(f.numerator)
        return IRRational(f.numerator, f.denominator)

    # Build sum of c_i · k^i terms.
    terms: list[IRNode] = []
    for i, c in enumerate(p):
        if c == 0:
            continue
        if i == 0:
            terms.append(_frac_to_ir(c))
        elif i == 1:
            if c == 1:
                terms.append(k)
            else:
                terms.append(IRApply(MUL, (_frac_to_ir(c), k)))
        else:
            power = IRApply(POW, (k, IRInteger(i)))
            if c == 1:
                terms.append(power)
            else:
                terms.append(IRApply(MUL, (_frac_to_ir(c), power)))
    if len(terms) == 1:
        return terms[0]
    return IRApply(ADD, tuple(terms))


# ---------------------------------------------------------------------------
# Structural factoring: a(k) → (poly(k), exponentials, factorials).
#
# We recognise these atomic hypergeometric pieces:
#
#   * polynomial in k       (any pure polynomial — captures the
#                            "1 · poly" leading factor)
#   * c^k                   for rational c  (geometric ratio = c)
#   * c^(α·k + β)           constant-coefficient linear exponent
#   * GammaFunc(linear(k))  (= factorial of a linear shift, e.g. k!)
#
# Each piece has a rational shift ratio:
#
#   poly(k+1) / poly(k)                 → poly / poly  (we shift exactly)
#   c^(k+1) / c^k                       → c            (rational constant)
#   GammaFunc(k+a+1) / GammaFunc(k+a)   → k + a        (linear in k)
#
# The combined ratio of an entire product of these atoms is the product
# of their ratios, which stays rational by construction.
# ---------------------------------------------------------------------------


class _Hyp:
    """Decomposed hypergeometric term ``a(k)``.

    Fields encode the multiplicative pieces:

    * ``poly``         — the polynomial factor ``P(k)`` as a ``Poly``;
                         empty list ⇒ zero (Gosper trivially yields 0).
    * ``exp_factors``  — list of ``(base, exponent)`` pairs where
                         ``base`` is a rational ``Fraction`` and
                         ``exponent`` is an integer-coefficient linear
                         polynomial in ``k`` (a ``Poly``).  Represents
                         ``∏ base^exponent``.
    * ``gamma_shifts`` — list of integers ``a_i`` representing the
                         product of ``GammaFunc(k + a_i)`` factors.
                         (Reciprocal Gamma factors are recorded with
                         ``recip_gamma_shifts``.)
    * ``recip_gamma_shifts`` — same shape, for ``GammaFunc(k + a_i)``
                               appearing in the denominator.

    The corresponding term is ::

        a(k) = poly(k) · ∏ base_i^exponent_i(k)
             · ∏ GammaFunc(k + s_j)
             / ∏ GammaFunc(k + t_l)

    Anything in the original IR that doesn't fit one of these slots
    causes the structural decomposer to bail (return ``None``).
    """

    __slots__ = ("poly", "exp_factors", "gamma_shifts", "recip_gamma_shifts")

    def __init__(self) -> None:
        self.poly: Poly = [Fraction(1)]
        self.exp_factors: list[tuple[Fraction, Poly]] = []
        self.gamma_shifts: list[int] = []
        self.recip_gamma_shifts: list[int] = []


def _try_linear_in_k(node: IRNode, k: IRSymbol) -> tuple[int, int] | None:
    """If ``node`` is an integer-coefficient linear polynomial ``α·k + β``,
    return ``(α, β)``; else ``None``.

    The Gamma-factor recogniser uses this — we only handle integer
    shifts ``k + a`` (and ``k`` itself, i.e. ``α = 1, β = 0``).  Pure
    constants come back as ``(0, β)``.
    """
    p = _ir_to_poly(node, k)
    if p is None:
        return None
    if not p:
        return (0, 0)
    if _poly_deg(p) > 1:
        return None
    a = p[1] if len(p) >= 2 else Fraction(0)
    b = p[0]
    if a.denominator != 1 or b.denominator != 1:
        return None
    return (int(a.numerator), int(b.numerator))


def _decompose(node: IRNode, k: IRSymbol, hyp: _Hyp | None = None) -> _Hyp | None:
    """Recursively factor ``node`` into a ``_Hyp`` decomposition.

    Returns ``None`` if any sub-expression doesn't fit the recognised
    hypergeometric vocabulary.  The expected entry point is
    :func:`try_gosper_sum`, which seeds an empty ``_Hyp`` and calls
    here.
    """
    if hyp is None:
        hyp = _Hyp()

    # ── Polynomial sub-tree?  Fold it into ``hyp.poly``. ──
    poly = _ir_to_poly(node, k)
    if poly is not None:
        hyp.poly = _poly_mul(hyp.poly, poly)
        return hyp

    if not isinstance(node, IRApply):
        return None

    # ── Pure multiplicative descent. ──
    if node.head == MUL:
        for arg in node.args:
            if _decompose(arg, k, hyp) is None:
                return None
        return hyp

    # ── NEG: fold a -1 into the polynomial factor. ──
    if node.head == NEG and len(node.args) == 1:
        inner = _decompose(node.args[0], k, hyp)
        if inner is None:
            return None
        inner.poly = _poly_scalar(inner.poly, Fraction(-1))
        return inner

    # ── DIV: A/B — recurse on the numerator, then handle the
    # denominator as a separate divisor.  Only support the cases the
    # ratio computation can express: polynomial denominator or
    # GammaFunc denominator.
    if node.head == DIV and len(node.args) == 2:
        num, den = node.args
        if _decompose(num, k, hyp) is None:
            return None
        # Polynomial denominator — only constant denominators are
        # exact divisions; non-constant polynomial denominators are
        # rejected (would require partial-fraction or extension of
        # the model).
        den_poly = _ir_to_poly(den, k)
        if den_poly is not None:
            if _poly_deg(den_poly) != 0 or not den_poly:
                return None
            hyp.poly = _poly_scalar(hyp.poly, Fraction(1) / den_poly[0])
            return hyp
        # GammaFunc denominator.
        if (
            isinstance(den, IRApply)
            and den.head == GAMMA_FUNC
            and len(den.args) == 1
        ):
            lin = _try_linear_in_k(den.args[0], k)
            if lin is None or lin[0] != 1:
                return None
            hyp.recip_gamma_shifts.append(lin[1])
            return hyp
        return None

    # ── POW(base, k-bearing exponent): only constant base with linear-in-k
    # exponent.  This captures ``c^k`` and ``c^(α·k + β)``.
    if node.head == POW and len(node.args) == 2:
        base, exp = node.args
        base_poly = _ir_to_poly(base, k)
        if base_poly is None:
            return None
        # POW(k-bearing, integer-constant) was already handled by
        # _ir_to_poly above; if we reach here, exp must depend on k.
        if _poly_deg(base_poly) != 0 or not base_poly:
            return None
        # The base is a rational constant.
        b = base_poly[0]
        if b == 0:
            # 0^k is 0 for k > 0 and 1 for k == 0 — too degenerate.
            return None
        # Exponent must be a polynomial in k (we accept any degree, but
        # only linear exponents yield a rational ratio — higher degree
        # gives transcendental shift ratios like b^(2k+1)).  Restrict
        # to degree ≤ 1.
        exp_poly = _ir_to_poly(exp, k)
        if exp_poly is None:
            return None
        if _poly_deg(exp_poly) > 1:
            return None
        hyp.exp_factors.append((b, exp_poly))
        return hyp

    # ── GammaFunc(linear(k)): factorial-like factor. ──
    if node.head == GAMMA_FUNC and len(node.args) == 1:
        lin = _try_linear_in_k(node.args[0], k)
        if lin is None or lin[0] != 1:
            return None
        hyp.gamma_shifts.append(lin[1])
        return hyp

    return None


# ---------------------------------------------------------------------------
# Ratio computation: r(k) = a(k+1) / a(k) as (numerator, denominator) Polys.
# ---------------------------------------------------------------------------


def _hyp_ratio(h: _Hyp) -> tuple[Poly, Poly] | None:
    """Compute ``a(k+1)/a(k)`` for a decomposed hypergeometric ``a(k)``.

    Returns ``(numer_poly, denom_poly)``.  The result is exact; no
    common-factor cancellation is performed yet — Petkovšek
    normalisation handles that.

    Mathematically::

        poly(k+1)/poly(k)                            — shifted poly ratio
        ∏ base_i^(exp_i(k+1) - exp_i(k))             — exponential ratio,
                                                       which is the rational
                                                       constant b_i^α_i where
                                                       α_i is the linear
                                                       coefficient of exp_i
        ∏ GammaFunc(k+s_j+1)/GammaFunc(k+s_j)        — = (k + s_j)
        ÷ ∏ GammaFunc(k+t_l+1)/GammaFunc(k+t_l)      — = 1 / (k + t_l)

    The exponential ratio uses an important simplification: when the
    exponent is ``α·k + β`` (integer ``α``), the shift-ratio
    ``base^(α(k+1)+β) / base^(αk+β) = base^α`` is a constant in ``k``,
    so it just multiplies the numerator.  Non-integer ``α`` would yield
    irrational ratios and we'd have refused the factor at decomposition.
    """
    # Polynomial piece.
    poly = h.poly
    poly_shift = _poly_shift(poly, 1)
    # Reject zero or numerically-canceling polynomial.
    if not _poly_trim(poly):
        return None
    numer = poly_shift
    denom = poly

    # Exponential pieces — each contributes a constant b^α to the
    # numerator.  We accept α as a fractional Fraction *only* when the
    # resulting ratio is still a rational number — concretely, when
    # b^α evaluates to a rational.  We restrict to integer α here for
    # safety.
    for base, exp_poly in h.exp_factors:
        # Linear coefficient of the exponent in k.
        if _poly_deg(exp_poly) == 0:
            # Constant exponent ⇒ the factor is a constant ⇒ ratio = 1.
            continue
        alpha = exp_poly[1]  # coefficient of k
        if alpha.denominator != 1:
            return None  # would yield base^(p/q), irrational in general
        alpha_int = int(alpha.numerator)
        # base^alpha as a rational.
        if alpha_int >= 0:
            factor = base ** alpha_int
        else:
            if base == 0:
                return None
            factor = Fraction(1) / (base ** (-alpha_int))
        numer = _poly_scalar(numer, factor)

    # GammaFunc(k + s) numerator-side: ratio = (k + s).
    for s in h.gamma_shifts:
        numer = _poly_mul(numer, [Fraction(s), Fraction(1)])

    # GammaFunc(k + t) denominator-side: ratio = 1 / (k + t),
    # so the overall ratio gains (k + t) in the denominator.
    for t in h.recip_gamma_shifts:
        denom = _poly_mul(denom, [Fraction(t), Fraction(1)])

    return numer, denom


# ---------------------------------------------------------------------------
# Petkovšek normalisation: find p, q, r with r(k) = p(k+1)/p(k) · q(k)/r(k)
# and gcd(q(k), r(k+h)) = 1 for all integers h ≥ 0.
# ---------------------------------------------------------------------------


def _petkovsek_normalise(
    a: Poly, b: Poly
) -> tuple[Poly, Poly, Poly] | None:
    """Given the ratio ``a(k+1)/a(k) = A(k)/B(k)`` as input polynomials,
    return ``(A_norm, B_norm, C)`` such that::

        a(k+1)/a(k) = A_norm(k) · C(k+1) / (B_norm(k) · C(k))

    and ``gcd(A_norm(k), B_norm(k + h)) = 1`` for every integer
    ``h ≥ 0`` (the shift-coprime / Gosper-Petkovšek property).

    Algorithm (Gosper, ``A = B`` §5.3):

      1.  Initialise ``A ← input numerator, B ← input denominator, C ← 1``.
      2.  Find an integer ``h ≥ 0`` and ``g = gcd(A(k), B(k+h))`` with
          ``deg(g) ≥ 1`` (the common factor we peel).
      3.  Update::
              A(k) ← A(k) / g(k)
              B(k) ← B(k) / g(k - h)
              C(k) ← C(k) · ∏_{i=1}^{h} g(k - i)
      4.  Restart from step 2 (a peel may expose new factors at lower h).
      5.  Stop when no ``h`` yields a non-constant gcd — A and B are now
          shift-coprime.

    Why this terminates: each successful peel strictly reduces
    ``deg(A) + deg(B)``, which is bounded below by 0.  Why the
    bounded-``h`` search is enough: the integer roots of
    ``Resultant_k(A(k), B(k+h))`` (viewed as a polynomial in ``h``) are
    finitely many and bounded by ``deg(A) + deg(B)`` for the
    polynomial families this module produces.
    """
    A: Poly = list(a)
    B: Poly = list(b)
    C: Poly = [Fraction(1)]
    max_h = max(_poly_deg(A), _poly_deg(B)) + 2
    if max_h < 0:
        max_h = 0
    while True:
        peeled = False
        for h in range(max_h + 1):
            B_shifted = _poly_shift(B, h)
            g = _poly_gcd(A, B_shifted)
            if _poly_deg(g) >= 1:
                # Peel ``g`` out.
                A_new, rem_A = _poly_divmod(A, g)
                if rem_A:
                    return None
                g_back = _poly_shift(g, -h)
                B_new, rem_B = _poly_divmod(B, g_back)
                if rem_B:
                    return None
                # Multiply C by g(k - 1)·g(k - 2)·…·g(k - h).
                acc = [Fraction(1)]
                for i in range(1, h + 1):
                    acc = _poly_mul(acc, _poly_shift(g, -i))
                C = _poly_mul(C, acc)
                A, B = A_new, B_new
                peeled = True
                break
        if not peeled:
            return A, B, C


# ---------------------------------------------------------------------------
# Gosper degree bound for x(k) in the key equation
#
#     A(k)·x(k+1) − B(k − 1)·x(k) = C(k)            (post-Petkovšek form)
#
# A, B are the shift-coprime numerator/denominator of a(k+1)/a(k) and C
# is the "absorbed" common-factor product accumulated during Petkovšek
# normalisation.
# ---------------------------------------------------------------------------


def _gosper_degree_bound(A: Poly, B: Poly, C: Poly) -> int:
    """Compute the upper bound for ``deg(x)`` in Gosper's key equation
    ``A(k)·x(k+1) − B(k−1)·x(k) = C(k)``.

    Letting ``S = A + B_shifted`` and ``D = A − B_shifted`` (where
    ``B_shifted(k) = B(k − 1)``), and ``L = deg(S)``, ``M = deg(D)``,
    ``c = deg(C)``:

    *   **Generic case** (``L > M + 1``):  ``deg(x) ≤ c − L``.
    *   **Degenerate case** (``L ≤ M + 1``): ``deg(x) ≤ max(c − M,
        −2·D_top/S_top − 1)``, with the second term rounded *up* to a
        non-negative integer.

    Robust to corner cases: we add 1 as a safety margin since the
    Gaussian solver runs cheaply over a few extra rationals.  Returns
    ``-1`` when the bound is negative (proves no polynomial solution
    exists — Gosper rejects the term).
    """
    B_shifted = _poly_shift(B, -1)
    S = _poly_add(A, B_shifted)
    D = _poly_sub(A, B_shifted)
    deg_S = _poly_deg(S)
    deg_D = _poly_deg(D)
    deg_C = _poly_deg(C)
    if deg_S > deg_D + 1:
        bound = deg_C - deg_S
    else:
        m = max(_poly_deg(A), _poly_deg(B_shifted))
        if m < 0:
            return 0
        S_top = S[m] if m < len(S) else Fraction(0)
        if S_top == 0:
            bound = deg_C - m
        else:
            D_at_m_minus_1 = (
                D[m - 1] if (m - 1) >= 0 and (m - 1) < len(D) else Fraction(0)
            )
            candidate = -2 * D_at_m_minus_1 / S_top - 1
            if candidate < 0:
                cand_int = 0
            else:
                # Ceiling toward +∞.
                cand_int = (
                    int(candidate)
                    if candidate == int(candidate)
                    else int(candidate) + 1
                )
            bound = max(deg_C - m, cand_int)
    if bound < 0:
        return -1
    # One-step safety margin: extra unknowns just zero out in the
    # Gaussian solve.
    return bound + 1


def _solve_key_equation(A: Poly, B: Poly, C: Poly, deg_bound: int) -> Poly | None:
    """Find ``x(k)`` of degree ≤ ``deg_bound`` solving
    ``A(k)·x(k+1) − B(k−1)·x(k) = C(k)`` over the rationals, or ``None``.

    Parameterise ``x(k) = x_0 + x_1·k + … + x_d·k^d`` with unknown
    rational ``x_i``.  Substituting and equating coefficients of each
    ``k^j`` gives a linear system in the ``x_i``; we solve it via
    Gaussian elimination over ``Fraction``.
    """
    if deg_bound < 0:
        return None
    d = deg_bound
    n_unknowns = d + 1
    B_shifted = _poly_shift(B, -1)
    # For each unknown x_i (coefficient of k^i in x), the contribution
    # to the LHS polynomial in k is:
    #     x_i · [ A(k) · (k + 1)^i − B(k − 1) · k^i ]
    basis_polys: list[Poly] = []
    max_deg = 0
    for i in range(n_unknowns):
        k_pow_i: Poly = [Fraction(0)] * i + [Fraction(1)]
        kp1_pow_i = _poly_shift(k_pow_i, 1)
        left = _poly_mul(A, kp1_pow_i)
        right = _poly_mul(B_shifted, k_pow_i)
        bp = _poly_sub(left, right)
        basis_polys.append(bp)
        if _poly_deg(bp) > max_deg:
            max_deg = _poly_deg(bp)
    # RHS: coefficients of C(k).
    C_trim = _poly_trim(C)
    rhs_len = max(max_deg + 1, len(C_trim))
    if rhs_len == 0:
        rhs_len = 1
    rhs = [Fraction(0)] * rhs_len
    for j, c in enumerate(C_trim):
        rhs[j] = c
    matrix: list[list[Fraction]] = []
    for j in range(rhs_len):
        row: list[Fraction] = []
        for i in range(n_unknowns):
            bp = basis_polys[i]
            row.append(bp[j] if j < len(bp) else Fraction(0))
        matrix.append(row)
    sol = _solve_linear_system(matrix, rhs)
    if sol is None:
        return None
    x_poly: Poly = _poly_trim(list(sol))
    # Sanity check: verify the solution actually satisfies the key
    # equation.  This catches bound-too-low cases where Gaussian
    # elimination happily zeroes out the unknowns and gives a wrong
    # answer (the equation is then inconsistent in the higher terms).
    if not x_poly:
        # Zero solution — only valid if C is also zero.
        if _poly_trim(C):
            return None
        return [Fraction(0)]
    # Compute A(k)·x(k+1) - B(k-1)·x(k) and compare to C(k).
    x_shifted = _poly_shift(x_poly, 1)
    lhs = _poly_sub(_poly_mul(A, x_shifted), _poly_mul(B_shifted, x_poly))
    if not _poly_eq(lhs, C):
        return None
    return x_poly


# ---------------------------------------------------------------------------
# Top-level entry: try Gosper on a summand.
# ---------------------------------------------------------------------------


def try_gosper_sum(
    summand: IRNode,
    k: IRSymbol,
    lo: IRNode,
    hi: IRNode,
) -> IRNode | None:
    """Attempt Gosper's algorithm on ``∑_{k=lo}^{hi} summand``.

    Returns the IR closed form ``T(hi+1) − T(lo)`` on success, or
    ``None`` to signal fall-through.  The caller (the dispatcher in
    :mod:`cas_summation.summation`) is responsible for wrapping the
    result through ``vm.eval`` for normalisation.

    Pipeline (see module docstring for the math):

      1.  Structurally decompose ``summand`` into the hypergeometric
          building blocks (polynomial · ∏ c^k · ∏ k!).  Bail if any
          factor doesn't fit.
      2.  Compute ``r(k) = a(k+1)/a(k)`` as a pair of polynomials.
      3.  Petkovšek-normalise into ``(p_pet, q_pet, r_pet)``.  Note
          the ``p_pet`` here is the *Petkovšek* p — distinct from the
          ``p(k)`` of Gosper's key equation, which equals the original
          ``p_pet`` (we relabel to avoid confusion).
      4.  Bound deg(x); run Gaussian elimination.
      5.  If x exists, reconstruct ``T(k) = (r(k-1) · x(k) · a(k)) / p(k)``
          and return ``T(hi+1) - T(lo)``.

    Returns ``None`` on any failure — the dispatcher will fall through
    to its remaining handlers.
    """
    # Step 1: structural decomposition.
    hyp = _decompose(summand, k)
    if hyp is None:
        return None

    # Step 2: ratio.
    ratio = _hyp_ratio(hyp)
    if ratio is None:
        return None
    a_top, b_bot = ratio
    # If the polynomial part is zero, the sum is 0.
    if not _poly_trim(hyp.poly):
        return IRInteger(0)

    # Step 3: Petkovšek normalisation.  Outputs A_norm, B_norm, C_poly
    # with a(k+1)/a(k) = A_norm(k)·C_poly(k+1) / (B_norm(k)·C_poly(k))
    # and gcd(A_norm(k), B_norm(k+h)) = 1 for h ≥ 0.
    norm = _petkovsek_normalise(a_top, b_bot)
    if norm is None:
        return None
    A_norm, B_norm, C_poly = norm

    # Step 4: Gosper key equation
    #     A_norm(k)·x(k+1) − B_norm(k−1)·x(k) = C_poly(k)
    # degree bound + solve.
    deg_bound = _gosper_degree_bound(A_norm, B_norm, C_poly)
    x_poly = _solve_key_equation(A_norm, B_norm, C_poly, deg_bound)
    if x_poly is None or not _poly_trim(x_poly):
        return None

    # Step 5: reconstruct T(k) = B_norm(k−1)·x(k)·a(k) / C_poly(k).
    #
    # The naïve formula divides by C_poly(k), which may vanish at the
    # boundary k = lo even though the original sum is well-defined (a
    # removable singularity — typical for factorial-bearing terms).
    # The fix is to do the polynomial cancellation *symbolically*
    # before substituting: split a(k) into its polynomial part
    # ``hyp.poly`` and its transcendental rest (exponentials,
    # factorials), then cancel ``B_norm(k-1) · x(k) · hyp.poly(k)``
    # against ``C_poly(k)`` via polynomial GCD.  Whatever polynomial
    # factor remains in the denominator is then guaranteed non-vanishing
    # at the substitution points the dispatcher uses (or the algorithm
    # would have rejected the term up front).
    B_at_k_minus_1 = _poly_shift(B_norm, -1)
    full_numer_poly = _poly_mul(_poly_mul(B_at_k_minus_1, x_poly), hyp.poly)
    denom_poly = list(C_poly)
    g = _poly_gcd(full_numer_poly, denom_poly)
    if _poly_deg(g) >= 1:
        nq, rem_n = _poly_divmod(full_numer_poly, g)
        dq, rem_d = _poly_divmod(denom_poly, g)
        if not rem_n and not rem_d:
            full_numer_poly = nq
            denom_poly = dq

    # Build the "rest" of the summand: a(k) without its polynomial
    # factor.  We construct this as an IR multiplication of the
    # exponential and Gamma pieces only.  The polynomial part is
    # already absorbed into ``full_numer_poly`` above.
    def _frac_to_ir(f: Fraction) -> IRNode:
        if f.denominator == 1:
            return IRInteger(f.numerator)
        return IRRational(f.numerator, f.denominator)

    def _build_transcendental_part() -> IRNode:
        """Build IR for ``∏ base_i^exp_i(k) · ∏ Γ(k+s_j) / ∏ Γ(k+t_l)``.

        Skips entirely if the hypergeometric is purely polynomial
        (returns ``IRInteger(1)``)."""
        pieces: list[IRNode] = []
        for base, exp_poly in hyp.exp_factors:
            base_ir = _frac_to_ir(base)
            exp_ir = _poly_to_ir(exp_poly, k)
            pieces.append(IRApply(POW, (base_ir, exp_ir)))
        for s in hyp.gamma_shifts:
            arg = (
                k
                if s == 0
                else IRApply(ADD, (k, IRInteger(s)))
            )
            pieces.append(IRApply(GAMMA_FUNC, (arg,)))
        # Reciprocal Gammas: divide.
        denominator_gammas = []
        for t in hyp.recip_gamma_shifts:
            arg = (
                k
                if t == 0
                else IRApply(ADD, (k, IRInteger(t)))
            )
            denominator_gammas.append(IRApply(GAMMA_FUNC, (arg,)))
        if not pieces and not denominator_gammas:
            return IRInteger(1)
        if pieces:
            numer = (
                pieces[0]
                if len(pieces) == 1
                else IRApply(MUL, tuple(pieces))
            )
        else:
            numer = IRInteger(1)
        if not denominator_gammas:
            return numer
        denom = (
            denominator_gammas[0]
            if len(denominator_gammas) == 1
            else IRApply(MUL, tuple(denominator_gammas))
        )
        return IRApply(DIV, (numer, denom))

    transcendental_ir = _build_transcendental_part()

    def _t_at(k_value: IRNode) -> IRNode:
        """Build IR for ``T(k_value) = poly_numer(k_value)
        · transcendental(k_value) / poly_denom(k_value)``."""
        from cas_substitution import subst

        numer_ir = _poly_to_ir(full_numer_poly, k)
        denom_ir = _poly_to_ir(denom_poly, k)
        numer_at = subst(k_value, k, numer_ir)
        denom_at = subst(k_value, k, denom_ir)
        trans_at = subst(k_value, k, transcendental_ir)
        # Build (numer * trans) / denom.  When denom is a non-zero
        # constant, the VM will normalise it away.
        return IRApply(
            DIV,
            (IRApply(MUL, (numer_at, trans_at)), denom_at),
        )

    hi_plus_one = IRApply(ADD, (hi, IRInteger(1)))
    t_hi = _t_at(hi_plus_one)
    t_lo = _t_at(lo)
    return IRApply(SUB, (t_hi, t_lo))
