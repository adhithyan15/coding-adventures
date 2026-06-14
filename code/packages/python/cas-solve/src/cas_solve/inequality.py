"""Polynomial inequality solving in one real variable — Phase 27.

Solves inequalities of the form ``p(x) op 0`` (or equivalently
``lhs op rhs`` normalised to ``lhs − rhs op 0``) where ``op`` is one of
``<``, ``>``, ``≤``, ``≥`` and ``p(x)`` is a polynomial of degree 1–4
with rational coefficients.

Algorithm
---------

1.  **Normalise direction** — compute ``f = lhs − rhs`` and record
    ``want_positive`` (``True`` for ``>`` / ``≥``) and ``strict`` (``True``
    for ``<`` / ``>``).

2.  **Extract polynomial** — use ``symbolic_vm.polynomial_bridge.to_rational``
    (deferred import; falls back to ``None`` when the bridge is absent).

3.  **Find real roots numerically** — convert the ascending-degree Fraction
    coefficients to a descending-degree float list and call
    ``nsolve_poly`` from :mod:`cas_solve.durand_kerner`.  Filter roots
    to those whose imaginary part is negligible (``|Im| < 1e-8``).

    For degrees 1 and 2 the exact Fraction roots are also computed so that
    the IR output is exact (``IRInteger`` / ``IRRational``) rather than
    ``IRFloat``.

4.  **Sign analysis** — for each open interval between consecutive real
    roots evaluate ``f`` at the midpoint (or at ``root − 1`` / ``root + 1``
    for unbounded outer intervals) and record the sign.

5.  **Build result** — collect all intervals where the sign satisfies the
    desired condition.  Roots are included in non-strict inequalities and
    excluded in strict ones.  Each interval becomes one IR node:

    * ``Less(x, a)``                          — ``(−∞, a)``
    * ``LessEqual(x, a)``                     — ``(−∞, a]``
    * ``Greater(x, a)``                       — ``(a, +∞)``
    * ``GreaterEqual(x, a)``                  — ``[a, +∞)``
    * ``And(Greater(x, a), Less(x, b))``      — ``(a, b)``
    * ``And(GreaterEqual(x, a), LessEqual(x, b))`` — ``[a, b]``
    * (mixed open/closed variants similarly)
    * ``GreaterEqual(IRInteger(0), IRInteger(0))`` — all reals (trivially
      true condition, returned when the solution is the entire real line)
    * ``[]`` (empty list) — no real solutions

Public API
----------

    try_solve_inequality(ineq_ir, var) → list[IRNode] | None
"""

from __future__ import annotations

from fractions import Fraction
from typing import TYPE_CHECKING

from symbolic_ir import (
    AND,
    GREATER,
    GREATER_EQUAL,
    LESS,
    LESS_EQUAL,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

if TYPE_CHECKING:
    pass

# ---------------------------------------------------------------------------
# IR construction helpers
# ---------------------------------------------------------------------------


def _frac_ir(c: Fraction) -> IRNode:
    """Fraction → IRInteger (denom 1) or IRRational."""
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def _make_less(x: IRSymbol, a: IRNode, strict: bool) -> IRNode:
    """Return ``Less(x, a)`` or ``LessEqual(x, a)``."""
    head = LESS if strict else LESS_EQUAL
    return IRApply(head, (x, a))


def _make_greater(x: IRSymbol, a: IRNode, strict: bool) -> IRNode:
    """Return ``Greater(x, a)`` or ``GreaterEqual(x, a)``."""
    head = GREATER if strict else GREATER_EQUAL
    return IRApply(head, (x, a))


def _make_interval(
    x: IRSymbol,
    lo: IRNode | None,
    hi: IRNode | None,
    lo_strict: bool,
    hi_strict: bool,
) -> IRNode:
    """Return the condition node for one interval.

    ``lo = None`` means unbounded below; ``hi = None`` means unbounded above.

    Examples::

        _make_interval(x, None, a, True, True)    →  Less(x, a)
        _make_interval(x, a, None, False, True)   →  GreaterEqual(x, a)
        _make_interval(x, a, b, True, False)      →  And(Greater(x,a), LessEqual(x,b))
    """
    if lo is None and hi is None:
        # All reals — represented as a trivially-true condition.
        return IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))
    if lo is None:
        return _make_less(x, hi, hi_strict)  # type: ignore[arg-type]
    if hi is None:
        return _make_greater(x, lo, lo_strict)
    lo_cond = _make_greater(x, lo, lo_strict)
    hi_cond = _make_less(x, hi, hi_strict)
    return IRApply(AND, (lo_cond, hi_cond))


# ---------------------------------------------------------------------------
# Polynomial evaluation helpers
# ---------------------------------------------------------------------------


def _poly_eval_float(coeffs_asc: tuple[Fraction, ...], x_val: float) -> float:
    """Evaluate the polynomial at ``x_val``.

    ``coeffs_asc`` is in ascending degree order: ``(c₀, c₁, …, cₙ)``
    so that ``p(x) = c₀ + c₁·x + … + cₙ·xⁿ``.
    Horner's method applied in reverse (descending traversal).
    """
    result = 0.0
    for c in reversed(coeffs_asc):
        result = result * x_val + float(c)
    return result


def _sign_of(val: float) -> int:
    """Return +1, 0, or -1."""
    if abs(val) < 1e-9:
        return 0
    return 1 if val > 0 else -1


# ---------------------------------------------------------------------------
# Exact root extraction for degrees 1–2
# ---------------------------------------------------------------------------


def _exact_roots_deg1(coeffs: tuple[Fraction, ...]) -> list[Fraction]:
    """Return the one rational root of a degree-1 polynomial."""
    b, a = coeffs[0], coeffs[1]  # ascending: c₀ = b, c₁ = a
    if a == Fraction(0):
        return []
    return [-b / a]


def _exact_roots_deg2(coeffs: tuple[Fraction, ...]) -> list[Fraction] | None:
    """Return exact Fraction roots of a degree-2 polynomial.

    Returns ``None`` if the discriminant is not a perfect square (irrational
    roots) — the caller then falls back to numerical roots only.

    Returns ``[]`` if the discriminant is negative (no real roots).
    """
    c, b, a = coeffs[0], coeffs[1], coeffs[2]  # ascending
    disc = b * b - Fraction(4) * a * c
    if disc < 0:
        return []
    if disc == 0:
        return [-b / (Fraction(2) * a)]
    # Try to extract integer sqrt of disc numerator/denominator.
    import math  # noqa: PLC0415

    n, d = disc.numerator, disc.denominator
    sqrt_n = math.isqrt(n)
    sqrt_d = math.isqrt(d)
    if sqrt_n * sqrt_n == n and sqrt_d * sqrt_d == d:
        sqrt_disc = Fraction(sqrt_n, sqrt_d)
        two_a = Fraction(2) * a
        r1 = (-b + sqrt_disc) / two_a
        r2 = (-b - sqrt_disc) / two_a
        roots = sorted([r1, r2])
        return roots
    return None  # irrational roots → caller uses floats


# ---------------------------------------------------------------------------
# Numeric real-root finding
# ---------------------------------------------------------------------------


def _real_roots_numeric(coeffs_asc: tuple[Fraction, ...]) -> list[float]:
    """Find real roots numerically using Durand-Kerner.

    ``coeffs_asc`` is in ascending degree order.
    Returns sorted list of real roots (imaginary-part threshold: 1e-8).
    """
    # nsolve_poly wants *descending* order.
    coeffs_desc = [float(c) for c in reversed(coeffs_asc)]
    try:
        from cas_solve.durand_kerner import nsolve_poly  # noqa: PLC0415
    except ImportError:
        return []
    roots_complex = nsolve_poly(coeffs_desc)
    real_roots = [r.real for r in roots_complex if abs(r.imag) < 1e-8]
    return sorted(real_roots)


# ---------------------------------------------------------------------------
# Root → exact IR node conversion
# ---------------------------------------------------------------------------


def _float_to_ir(val: float) -> IRNode:
    """Convert a float root to IR.  Use IRFloat (not IRRational) for
    irrational values; the caller may later replace with exact form."""
    return IRFloat(val)


def _frac_roots_to_ir(frac_roots: list[Fraction]) -> list[IRNode]:
    return [_frac_ir(r) for r in frac_roots]


# ---------------------------------------------------------------------------
# Core sign-analysis function
# ---------------------------------------------------------------------------


def _solve_poly_ineq(
    coeffs_asc: tuple[Fraction, ...],
    want_positive: bool,
    strict: bool,
    var: IRSymbol,
    exact_roots_ir: list[IRNode] | None,
    numeric_roots: list[float],
) -> list[IRNode]:
    """Build the solution list given polynomial coefficients and roots.

    ``exact_roots_ir``: if provided, these IR nodes (sorted ascending by
    float value) are used as the boundary points in the output conditions.
    If ``None``, ``IRFloat`` nodes are used.

    ``numeric_roots``: float values of real roots (sorted ascending) —
    used for sign testing and interval boundaries when exact form is absent.
    """
    # Build parallel lists of (float_value, ir_node) for each root.
    if exact_roots_ir is not None and len(exact_roots_ir) == len(numeric_roots):
        boundary_ir = exact_roots_ir
    else:
        # Fall back to IRFloat for boundaries.
        boundary_ir = [_float_to_ir(r) for r in numeric_roots]

    n_roots = len(numeric_roots)
    # Deduplicate numerically-close roots (within 1e-6 of each other).
    if n_roots >= 2:
        deduped_floats: list[float] = [numeric_roots[0]]
        deduped_ir: list[IRNode] = [boundary_ir[0]]
        for i in range(1, n_roots):
            if numeric_roots[i] - deduped_floats[-1] > 1e-6:
                deduped_floats.append(numeric_roots[i])
                deduped_ir.append(boundary_ir[i])
        numeric_roots = deduped_floats
        boundary_ir = deduped_ir
        n_roots = len(numeric_roots)

    # Build test points: one per open interval between boundaries.
    # Intervals: (−∞, r₀), (r₀, r₁), …, (rₙ₋₁, +∞)
    def test_point(i: int) -> float:
        """Mid-point of interval i (0-based).

        Interval 0 is (−∞, r₀), interval n_roots is (rₙ₋₁, +∞).
        """
        if n_roots == 0:
            return 0.0
        if i == 0:
            return numeric_roots[0] - 1.0
        if i == n_roots:
            return numeric_roots[-1] + 1.0
        return (numeric_roots[i - 1] + numeric_roots[i]) / 2.0

    n_intervals = n_roots + 1
    result: list[IRNode] = []

    for i in range(n_intervals):
        tp = test_point(i)
        sign = _sign_of(_poly_eval_float(coeffs_asc, tp))

        # Does this interval satisfy the inequality?
        satisfies = (sign > 0) if want_positive else (sign < 0)
        if not satisfies:
            continue

        # Build the interval condition.
        lo_ir = boundary_ir[i - 1] if i > 0 else None
        hi_ir = boundary_ir[i] if i < n_roots else None

        # Strict boundary: open ends; non-strict boundary: closed ends.
        lo_strict = strict
        hi_strict = strict

        result.append(_make_interval(var, lo_ir, hi_ir, lo_strict, hi_strict))

    # Handle non-strict: roots where f = 0 also satisfy f ≥ 0 or f ≤ 0.
    # These are automatically captured because:
    #   • For a simple root, the adjacent intervals have opposite signs, and
    #     at least one satisfies the inequality.  The root is the boundary of
    #     a half-open (strict) interval — but we need to INCLUDE the root for
    #     non-strict.  That happens because we set lo_strict/hi_strict = strict
    #     (which is False for non-strict), so we already generate closed
    #     intervals that include the root.
    # Nothing extra to do here.

    # Degenerate result: if every interval satisfied (all reals).
    #
    # We can only collapse to "all reals" when the **roots themselves** are
    # also part of the solution:
    #
    #   • No roots (n_roots == 0): the polynomial never crosses zero, so
    #     the entire real line is trivially included in the open intervals.
    #   • Non-strict inequality (not strict): f = 0 satisfies both f ≥ 0
    #     and f ≤ 0, so the roots are included already.
    #
    # For a STRICT inequality with at least one root we must NOT collapse:
    # e.g. (x−1)² > 0 is satisfied on (−∞,1) and (1,+∞) — all open
    # intervals — but NOT at x = 1 where f = 0.  The correct output is
    # [Less(x, 1), Greater(x, 1)], not the all-reals sentinel.
    if len(result) == n_intervals and (n_roots == 0 or not strict):
        # All intervals included → solution is the entire real line.
        # Return a single trivially-true condition.
        return [IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))]

    return result


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def try_solve_inequality(
    ineq_ir: IRNode,
    var: IRSymbol,
) -> list[IRNode] | None:
    """Try to solve a polynomial inequality in one real variable.

    ``ineq_ir`` must be ``IRApply`` with head in
    ``{Less, Greater, LessEqual, GreaterEqual}``.

    Returns a list of condition IR nodes (each representing one disjoint
    interval in the solution set), or ``None`` if the pattern is not
    recognised or the polynomial bridge is unavailable.

    Special return values:

    * ``[]``   — no real solutions.
    * ``[GreaterEqual(0, 0)]``  — the entire real line is the solution.
    """
    # --- Validate structure ---
    if not isinstance(ineq_ir, IRApply):
        return None
    head = ineq_ir.head
    if not isinstance(head, IRSymbol):
        return None
    if head.name not in {"Less", "Greater", "LessEqual", "GreaterEqual"}:
        return None
    if len(ineq_ir.args) != 2:
        return None

    lhs, rhs = ineq_ir.args
    fname = head.name
    want_positive = fname in {"Greater", "GreaterEqual"}
    strict = fname in {"Less", "Greater"}

    # --- Build f = lhs − rhs ---
    if isinstance(rhs, IRInteger) and rhs.value == 0:
        f_ir = lhs
    elif isinstance(lhs, IRInteger) and lhs.value == 0:
        # 0 op rhs  →  invert and use −rhs op 0 (flip direction)
        # This is rare; we rewrite as rhs op' 0 instead.
        # 0 > rhs  ↔  rhs < 0  →  swap sides, flip op
        # Rather than rewrite: just negate by setting f = rhs and flipping.
        f_ir = rhs
        want_positive = not want_positive
    else:
        f_ir = IRApply(SUB, (lhs, rhs))

    # --- Extract polynomial coefficients ---
    try:
        from symbolic_vm.polynomial_bridge import to_rational  # noqa: PLC0415
    except ImportError:
        return None

    result = to_rational(f_ir, var)
    if result is None:
        return None
    num_frac, den_frac = result
    if den_frac != (Fraction(1),):
        return None  # rational function, not polynomial
    coeffs = num_frac  # ascending degree

    deg = len(coeffs) - 1
    if deg < 1:
        # Constant polynomial: either always true or always false.
        const_val = float(coeffs[0]) if coeffs else 0.0
        sign = _sign_of(const_val)
        satisfies = (sign > 0) if want_positive else (sign < 0)
        if satisfies:
            return [IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))]
        # strict/non-strict doesn't matter for a constant
        # (0 op 0 gives all reals for non-strict, no sol for strict)
        if not strict and sign == 0:
            return [IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))]
        return []

    if deg > 4:
        return None  # unsupported degree

    # --- Find exact roots (for deg 1–2) and numeric roots ---
    exact_ir: list[IRNode] | None = None
    numeric: list[float] = []

    if deg == 1:
        frac_roots = _exact_roots_deg1(coeffs)
        exact_ir = _frac_roots_to_ir(frac_roots)
        numeric = [float(r) for r in frac_roots]

    elif deg == 2:
        frac_roots = _exact_roots_deg2(coeffs)
        if frac_roots is not None:
            exact_ir = _frac_roots_to_ir(frac_roots)
            numeric = [float(r) for r in frac_roots]
        else:
            # Irrational roots — use Durand-Kerner.
            numeric = _real_roots_numeric(coeffs)
            exact_ir = None  # will use IRFloat boundaries

    else:
        # deg 3 or 4: always use numeric roots.
        numeric = _real_roots_numeric(coeffs)
        exact_ir = None

    return _solve_poly_ineq(coeffs, want_positive, strict, var, exact_ir, numeric)
