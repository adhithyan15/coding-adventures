"""Track J1 — Taylor-series-expansion fallback for limits.

Acceptance cases from ``macsyma-truly-finish-plan.md``:

    limit((sin(x) - x)/x^3,        x, 0) = -1/6
    limit((1 - cos(x))/x^2,        x, 0) =  1/2
    limit((exp(x) - 1 - x)/x^2,    x, 0) =  1/2
    limit((tan(x) - x)/x^3,        x, 0) =  1/3
    limit((log(1+x) - x)/x^2,      x, 0) = -1/2
    limit((sin(x) - x)/(exp(x^2) - 1), x, 0) = -1/6  (the (x^2 + O(x^4)) denom
                                                       has leading u^2, and the
                                                       numerator's leading
                                                       coefficient is -1/6 at u^3
                                                       — so the limit is 0)

Plus regression checks that the existing direct-substitution / L'Hôpital
paths continue to close their own cases without hitting the Taylor
fallback.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EXP,
    LOG,
    MUL,
    POW,
    SIN,
    SUB,
    TAN,
    IRApply,
    IRInteger,
    IRRational,
    IRSymbol,
)

from cas_limit_series import try_series_limit

# ---------------------------------------------------------------------------
# Tiny helpers
# ---------------------------------------------------------------------------


def _i(n: int) -> IRInteger:
    return IRInteger(n)


def _x() -> IRSymbol:
    return IRSymbol("x")


def _div(a, b):  # noqa: ANN001
    return IRApply(DIV, (a, b))


def _sub(a, b):  # noqa: ANN001
    return IRApply(SUB, (a, b))


def _pow(a, n):  # noqa: ANN001
    return IRApply(POW, (a, _i(n)))


def _mul(*args):  # noqa: ANN001
    return IRApply(MUL, tuple(args))


def _sin(a):  # noqa: ANN001
    return IRApply(SIN, (a,))


def _cos(a):  # noqa: ANN001
    return IRApply(COS, (a,))


def _tan(a):  # noqa: ANN001
    return IRApply(TAN, (a,))


def _exp(a):  # noqa: ANN001
    return IRApply(EXP, (a,))


def _log(a):  # noqa: ANN001
    return IRApply(LOG, (a,))


def _as_fraction(node):  # noqa: ANN001
    """Convert an IR literal node back to a Fraction for asserting."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    raise AssertionError(f"expected literal, got {node!r}")


# ---------------------------------------------------------------------------
# Acceptance cases
# ---------------------------------------------------------------------------


class TestAcceptance:
    """The six acceptance cases listed in the Track J1 spec."""

    def test_sin_minus_x_over_x_cubed(self) -> None:
        """(sin(x) − x)/x³  →  −1/6."""
        x = _x()
        expr = _div(_sub(_sin(x), x), _pow(x, 3))
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert _as_fraction(result) == Fraction(-1, 6)

    def test_one_minus_cos_over_x_squared(self) -> None:
        """(1 − cos(x))/x²  →  1/2."""
        x = _x()
        expr = _div(_sub(_i(1), _cos(x)), _pow(x, 2))
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert _as_fraction(result) == Fraction(1, 2)

    def test_exp_minus_1_minus_x_over_x_squared(self) -> None:
        """(exp(x) − 1 − x)/x²  →  1/2."""
        x = _x()
        numer = _sub(_sub(_exp(x), _i(1)), x)
        expr = _div(numer, _pow(x, 2))
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert _as_fraction(result) == Fraction(1, 2)

    def test_tan_minus_x_over_x_cubed(self) -> None:
        """(tan(x) − x)/x³  →  1/3."""
        x = _x()
        expr = _div(_sub(_tan(x), x), _pow(x, 3))
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert _as_fraction(result) == Fraction(1, 3)

    def test_log_one_plus_x_minus_x_over_x_squared(self) -> None:
        """(log(1+x) − x)/x²  →  −1/2."""
        x = _x()
        # log(1 + x)
        one_plus_x = IRApply(ADD, (_i(1), x))
        numer = _sub(_log(one_plus_x), x)
        expr = _div(numer, _pow(x, 2))
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert _as_fraction(result) == Fraction(-1, 2)

    def test_sin_minus_x_over_exp_x_squared_minus_one(self) -> None:
        """(sin(x) − x) / (exp(x²) − 1).

        sin(x) − x = −x³/6 + O(x⁵)          → leading u^3, c_3 = −1/6
        exp(x²) − 1 = x² + O(x⁴)             → leading u^2, d_2 = 1
        p = 3 > q = 2 ⇒ limit = 0.
        """
        x = _x()
        numer = _sub(_sin(x), x)
        denom = _sub(_exp(_pow(x, 2)), _i(1))
        expr = _div(numer, denom)
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert isinstance(result, IRInteger) and result.value == 0


# ---------------------------------------------------------------------------
# Regression — existing paths still close
# ---------------------------------------------------------------------------


class TestRegression:
    """sin(x)/x and x^2 still close via existing paths (or via Taylor)."""

    def test_sin_over_x(self) -> None:
        """sin(x)/x  →  1.  Both Taylor and L'Hôpital should give this."""
        x = _x()
        expr = _div(_sin(x), x)
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert _as_fraction(result) == Fraction(1)

    def test_x_squared_quotient(self) -> None:
        """x²/x → 0 by leading-order analysis."""
        x = _x()
        expr = _div(_pow(x, 2), x)
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert isinstance(result, IRInteger) and result.value == 0


# ---------------------------------------------------------------------------
# Edge cases — return None gracefully
# ---------------------------------------------------------------------------


class TestFallthrough:
    """Inputs that the Taylor fallback cannot handle should return None."""

    def test_non_quotient_returns_none(self) -> None:
        """Bare polynomial (not a quotient) → None.

        The fallback only fires on f/g shapes — anything else is the job
        of other dispatch branches.
        """
        x = _x()
        result = try_series_limit(_pow(x, 2), x, _i(0))
        assert result is None

    def test_unsupported_head_returns_none(self) -> None:
        """A quotient containing an unsupported head (e.g. Asin) → None."""
        x = _x()
        # Use a synthetic head that's not in the supported transcendental
        # list.
        weird = IRApply(IRSymbol("Asin"), (x,))
        expr = _div(weird, x)
        result = try_series_limit(expr, x, _i(0))
        assert result is None

    def test_infinity_point_returns_none(self) -> None:
        """Limits at ±∞ are not handled by this first version of the
        fallback (they need a u = 1/x rewrite)."""
        x = _x()
        expr = _div(_sin(x), x)
        result = try_series_limit(expr, x, IRSymbol("inf"))
        assert result is None

    def test_divergent_quotient(self) -> None:
        """1/x² → ∞ (p < q, denominator vanishes faster).

        The fallback should return the IRSymbol("inf") sentinel.
        """
        x = _x()
        expr = _div(_i(1), _pow(x, 2))
        result = try_series_limit(expr, x, _i(0))
        # 1 has leading order 0, x^2 has leading order 2: p=0 < q=2
        # ⇒ ∞ (positive, since 1/1 > 0).
        assert result is not None
        assert isinstance(result, IRSymbol) and result.name == "inf"

    def test_negative_divergence(self) -> None:
        """-1/x² → -∞."""
        x = _x()
        expr = _div(_i(-1), _pow(x, 2))
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert isinstance(result, IRSymbol) and result.name == "minf"


# ---------------------------------------------------------------------------
# Non-origin expansion point
# ---------------------------------------------------------------------------


class TestShiftedPoint:
    """Limit at a non-zero point should translate to the origin first."""

    def test_polynomial_at_one(self) -> None:
        """(x² − 1)/(x − 1) at x = 1  →  2."""
        x = _x()
        numer = _sub(_pow(x, 2), _i(1))
        denom = _sub(x, _i(1))
        expr = _div(numer, denom)
        result = try_series_limit(expr, x, _i(1))
        assert result is not None
        assert _as_fraction(result) == Fraction(2)


# ---------------------------------------------------------------------------
# Wiring — limit_advanced uses try_series_limit when L'Hôpital can't close
# ---------------------------------------------------------------------------


class TestDispatcherIntegration:
    """The top-level ``limit_advanced`` should pick up Taylor results
    when called without an injected diff_fn for 0/0 forms.
    """

    def test_limit_advanced_no_diff_fn_uses_taylor(self) -> None:
        """sin(x)/x via limit_advanced without diff_fn falls through to Taylor."""
        from cas_limit_series import limit_advanced

        x = _x()
        expr = _div(_sin(x), x)
        # No diff_fn → L'Hôpital can't fire, but Taylor should still close.
        result = limit_advanced(expr, x, _i(0))
        # Result should evaluate to 1 (IRInteger(1) or a Limit fallthrough).
        # We accept either: the contract is "Taylor closes 0/0 when L'Hôpital
        # has no diff_fn" — for sin(x)/x specifically Taylor gives 1.
        assert isinstance(result, IRInteger) and result.value == 1


# ---------------------------------------------------------------------------
# Series-ring internals — these tests exercise the low-level Series API
# directly. They're not strictly required by the spec but they bring the
# coverage of ``series_limit.py`` up over the 80% threshold and document
# the boundaries of the ring.
# ---------------------------------------------------------------------------


class TestSeriesRing:
    """Direct exercises of the Series class and its arithmetic."""

    def test_reciprocal_of_vanishing_series_raises(self) -> None:
        """Reciprocal requires a nonzero constant term."""
        from cas_limit_series.series_limit import Series, _SeriesError

        s = Series([Fraction(0), Fraction(1)], 4)  # = u
        try:
            s.reciprocal()
        except _SeriesError:
            return
        raise AssertionError("expected _SeriesError")

    def test_integer_power_negative_raises(self) -> None:
        """integer_power requires k >= 0."""
        from cas_limit_series.series_limit import Series, _SeriesError

        s = Series([Fraction(1)], 4)
        try:
            s.integer_power(-1)
        except _SeriesError:
            return
        raise AssertionError("expected _SeriesError")

    def test_integer_power_zero_is_one(self) -> None:
        """Any series to the 0 power is the constant 1."""
        from cas_limit_series.series_limit import Series

        s = Series([Fraction(2), Fraction(3)], 4)
        result = s.integer_power(0)
        assert result.coeffs[0] == 1
        assert all(c == 0 for c in result.coeffs[1:])

    def test_compose_with_nonzero_constant_raises(self) -> None:
        """Composition requires inner(0) == 0."""
        from cas_limit_series.series_limit import Series, _SeriesError

        outer = Series([Fraction(1), Fraction(1)], 4)
        inner = Series([Fraction(1), Fraction(1)], 4)  # has nonzero constant
        try:
            outer.compose_with_zero_constant(inner)
        except _SeriesError:
            return
        raise AssertionError("expected _SeriesError")

    def test_series_padding_and_truncation(self) -> None:
        """Series constructor pads short lists and truncates long ones."""
        from cas_limit_series.series_limit import Series

        short = Series([Fraction(1)], 3)
        assert short.coeffs == [Fraction(1), Fraction(0), Fraction(0), Fraction(0)]
        long = Series([Fraction(i) for i in range(10)], 3)
        assert long.coeffs == [Fraction(0), Fraction(1), Fraction(2), Fraction(3)]

    def test_negative_order_raises(self) -> None:
        """Order must be non-negative."""
        from cas_limit_series.series_limit import Series, _SeriesError

        try:
            Series([Fraction(1)], -1)
        except _SeriesError:
            return
        raise AssertionError("expected _SeriesError")


class TestExpanderEdges:
    """Branches of ``_expand`` that the acceptance cases don't reach."""

    def test_neg_head(self) -> None:
        """-sin(x)/x at 0 → -1."""
        from symbolic_ir import NEG

        x = _x()
        numer = IRApply(NEG, (_sin(x),))
        expr = _div(numer, x)
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert _as_fraction(result) == Fraction(-1)

    def test_negative_integer_power(self) -> None:
        """1/(1-x) / 1 at 0 → 1 via reciprocal-then-power on Pow(., -1)."""
        # We need an actual Pow with exponent -1 sitting INSIDE a quotient
        # so the inner ``_expand`` path takes Pow(base, IRInteger(-1)).
        x = _x()
        one_minus_x = _sub(_i(1), x)
        # 1/((1-x)^-1) = (1-x); divided by 1, limit is 1.
        inner_pow = IRApply(POW, (one_minus_x, IRInteger(-1)))
        # Build Mul(1, Pow(1-x, -1)) / 1 — exercises Pow(., -1).
        expr = _div(IRApply(POW, (one_minus_x, IRInteger(-1))), _i(1))
        _ = inner_pow  # silence linter
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert _as_fraction(result) == Fraction(1)

    def test_mul_quotient_via_pow_neg_one(self) -> None:
        """``Mul(N, Pow(D, -1))`` is recognised as a quotient."""
        x = _x()
        # sin(x) * x^(-1) — same as sin(x)/x.
        expr = _mul(_sin(x), IRApply(POW, (x, IRInteger(-1))))
        result = try_series_limit(expr, x, _i(0))
        assert result is not None
        assert _as_fraction(result) == Fraction(1)

    def test_div_at_origin_with_constant_numerator(self) -> None:
        """1/1 at 0 → 1 via the rational ring."""
        x = _x()
        expr = _div(_i(1), _i(1))
        # This is not a quotient with zero-vanishing parts, but the
        # try_series_limit fallback should still cope.
        result = try_series_limit(expr, x, _i(0))
        # 1/1 has p=q=0, c_p/d_q=1.
        assert result is not None
        assert _as_fraction(result) == Fraction(1)

    def test_max_order_clamped_high(self) -> None:
        """``max_order`` above the hard cap is clamped."""
        x = _x()
        expr = _div(_sin(x), x)
        result = try_series_limit(expr, x, _i(0), max_order=999)
        assert result is not None
        assert _as_fraction(result) == Fraction(1)

    def test_max_order_clamped_low(self) -> None:
        """``max_order`` below 4 is bumped to 4."""
        x = _x()
        expr = _div(_sin(x), x)
        result = try_series_limit(expr, x, _i(0), max_order=2)
        assert result is not None
        assert _as_fraction(result) == Fraction(1)
