"""Tests for cas_solve.inequality — Phase 27 polynomial inequality solving.

These tests exercise every code path within ``inequality.py`` without
depending on ``symbolic_vm`` (which is *not* in cas-solve's dependency list).

Strategy
--------

The module under test calls ``symbolic_vm.polynomial_bridge.to_rational``
via a **deferred import** inside ``try_solve_inequality``.  We inject a fake
``symbolic_vm.polynomial_bridge`` module into ``sys.modules`` so that the
import succeeds with a controlled ``to_rational``.  The fixture
``patch_bridge(coeffs)`` accepts ascending-degree Fraction coefficients and
makes ``to_rational`` return them for *any* expression — this lets us test
the sign-analysis and interval-building logic independently of the actual
polynomial bridge.

For the pure helper functions (``_frac_ir``, ``_poly_eval_float``, etc.) and
the core algorithm (``_solve_poly_ineq``), no mocking is needed at all.
"""

from __future__ import annotations

import sys
import types
from contextlib import contextmanager
from fractions import Fraction

import pytest
from symbolic_ir import (
    AND,
    GREATER,
    GREATER_EQUAL,
    LESS,
    LESS_EQUAL,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRSymbol,
)

from cas_solve.inequality import (
    _exact_roots_deg1,
    _exact_roots_deg2,
    _frac_ir,
    _make_interval,
    _poly_eval_float,
    _sign_of,
    _solve_poly_ineq,
    try_solve_inequality,
)

# ---------------------------------------------------------------------------
# Shared symbols
# ---------------------------------------------------------------------------

X = IRSymbol("x")
ZERO = IRInteger(0)
ONE = IRInteger(1)

# ---------------------------------------------------------------------------
# Bridge-patching context manager
# ---------------------------------------------------------------------------


@contextmanager
def patch_bridge(coeffs_asc: tuple):
    """Patch polynomial_bridge so ``to_rational`` returns *coeffs_asc*.

    The coefficients are converted to Fraction automatically.
    ``den`` is always ``(Fraction(1),)`` — polynomial (not rational function).
    """
    fracs = tuple(Fraction(c) for c in coeffs_asc)

    def _to_rational(expr, var):  # noqa: ARG001
        return fracs, (Fraction(1),)

    mock_mod = types.ModuleType("symbolic_vm.polynomial_bridge")
    mock_mod.to_rational = _to_rational
    mock_vm = types.ModuleType("symbolic_vm")

    old_vm = sys.modules.get("symbolic_vm")
    old_bridge = sys.modules.get("symbolic_vm.polynomial_bridge")

    sys.modules["symbolic_vm"] = mock_vm
    sys.modules["symbolic_vm.polynomial_bridge"] = mock_mod

    try:
        yield
    finally:
        if old_vm is None:
            sys.modules.pop("symbolic_vm", None)
        else:
            sys.modules["symbolic_vm"] = old_vm

        if old_bridge is None:
            sys.modules.pop("symbolic_vm.polynomial_bridge", None)
        else:
            sys.modules["symbolic_vm.polynomial_bridge"] = old_bridge


@contextmanager
def patch_bridge_none():
    """Patch polynomial_bridge so ``to_rational`` always returns ``None``."""
    mock_mod = types.ModuleType("symbolic_vm.polynomial_bridge")
    mock_mod.to_rational = lambda expr, var: None  # noqa: ARG005

    old_bridge = sys.modules.get("symbolic_vm.polynomial_bridge")
    old_vm = sys.modules.get("symbolic_vm")

    sys.modules["symbolic_vm"] = types.ModuleType("symbolic_vm")
    sys.modules["symbolic_vm.polynomial_bridge"] = mock_mod

    try:
        yield
    finally:
        if old_vm is None:
            sys.modules.pop("symbolic_vm", None)
        else:
            sys.modules["symbolic_vm"] = old_vm

        if old_bridge is None:
            sys.modules.pop("symbolic_vm.polynomial_bridge", None)
        else:
            sys.modules["symbolic_vm.polynomial_bridge"] = old_bridge


def _ineq(head, lhs=None, rhs=None):
    """Build a simple IRApply inequality node.

    ``head`` is one of LESS, GREATER, LESS_EQUAL, GREATER_EQUAL.
    ``lhs`` defaults to ``X``; ``rhs`` defaults to ``ZERO``.
    """
    lhs = lhs if lhs is not None else X
    rhs = rhs if rhs is not None else ZERO
    return IRApply(head, (lhs, rhs))


# ===========================================================================
# 1. _frac_ir
# ===========================================================================


class TestFracIr:
    """_frac_ir: Fraction → IRInteger or IRRational."""

    def test_whole_positive(self) -> None:
        assert _frac_ir(Fraction(3)) == IRInteger(3)

    def test_whole_negative(self) -> None:
        assert _frac_ir(Fraction(-7)) == IRInteger(-7)

    def test_zero(self) -> None:
        assert _frac_ir(Fraction(0)) == IRInteger(0)

    def test_proper_fraction(self) -> None:
        assert _frac_ir(Fraction(1, 2)) == IRRational(1, 2)

    def test_negative_fraction(self) -> None:
        result = _frac_ir(Fraction(-3, 4))
        assert isinstance(result, IRRational)
        assert result.numer == -3
        assert result.denom == 4

    def test_improper_fraction_reduces(self) -> None:
        # Fraction(6, 3) = Fraction(2) → should give IRInteger(2)
        assert _frac_ir(Fraction(6, 3)) == IRInteger(2)


# ===========================================================================
# 2. _poly_eval_float
# ===========================================================================


class TestPolyEvalFloat:
    """_poly_eval_float: Horner evaluation with ascending-degree coefficients."""

    def test_constant(self) -> None:
        """p(x) = 5 → always 5."""
        coeffs = (Fraction(5),)
        assert _poly_eval_float(coeffs, 0.0) == pytest.approx(5.0)
        assert _poly_eval_float(coeffs, 3.7) == pytest.approx(5.0)

    def test_linear(self) -> None:
        """p(x) = -1 + x → p(1) = 0, p(2) = 1."""
        coeffs = (Fraction(-1), Fraction(1))
        assert _poly_eval_float(coeffs, 1.0) == pytest.approx(0.0)
        assert _poly_eval_float(coeffs, 2.0) == pytest.approx(1.0)

    def test_quadratic(self) -> None:
        """p(x) = x² - 1 → p(1) = 0, p(0) = -1, p(-1) = 0."""
        coeffs = (Fraction(-1), Fraction(0), Fraction(1))
        assert _poly_eval_float(coeffs, 1.0) == pytest.approx(0.0)
        assert _poly_eval_float(coeffs, 0.0) == pytest.approx(-1.0)
        assert _poly_eval_float(coeffs, -1.0) == pytest.approx(0.0)

    def test_cubic(self) -> None:
        """p(x) = x³ - x = x(x-1)(x+1) → p(0)=0, p(2)=6."""
        coeffs = (Fraction(0), Fraction(-1), Fraction(0), Fraction(1))
        assert _poly_eval_float(coeffs, 0.0) == pytest.approx(0.0)
        assert _poly_eval_float(coeffs, 2.0) == pytest.approx(6.0)
        assert _poly_eval_float(coeffs, -1.0) == pytest.approx(0.0)


# ===========================================================================
# 3. _sign_of
# ===========================================================================


class TestSignOf:
    """_sign_of: returns +1, 0, or -1."""

    def test_positive(self) -> None:
        assert _sign_of(3.5) == 1

    def test_negative(self) -> None:
        assert _sign_of(-0.5) == -1

    def test_near_zero(self) -> None:
        """Values within 1e-9 of zero count as zero."""
        assert _sign_of(0.0) == 0
        assert _sign_of(5e-10) == 0
        assert _sign_of(-5e-10) == 0

    def test_small_positive(self) -> None:
        assert _sign_of(1e-8) == 1


# ===========================================================================
# 4. _exact_roots_deg1
# ===========================================================================


class TestExactRootsDeg1:
    """_exact_roots_deg1: exact rational root of ax + b = 0."""

    def test_simple(self) -> None:
        """x - 1 = 0 → x = 1."""
        root = _exact_roots_deg1((Fraction(-1), Fraction(1)))
        assert root == [Fraction(1)]

    def test_rational_root(self) -> None:
        """2x + 3 = 0 → x = -3/2."""
        root = _exact_roots_deg1((Fraction(3), Fraction(2)))
        assert root == [Fraction(-3, 2)]

    def test_zero_coefficient(self) -> None:
        """If leading coefficient is zero, no root (constant)."""
        roots = _exact_roots_deg1((Fraction(5), Fraction(0)))
        assert roots == []

    def test_negative_slope(self) -> None:
        """-3x + 6 = 0 → x = 2."""
        root = _exact_roots_deg1((Fraction(6), Fraction(-3)))
        assert root == [Fraction(2)]


# ===========================================================================
# 5. _exact_roots_deg2
# ===========================================================================


class TestExactRootsDeg2:
    """_exact_roots_deg2: exact rational roots of ax² + bx + c = 0."""

    def test_two_integer_roots(self) -> None:
        """x² - 3x + 2 = 0 → {1, 2}."""
        roots = _exact_roots_deg2((Fraction(2), Fraction(-3), Fraction(1)))
        assert roots == [Fraction(1), Fraction(2)]

    def test_double_root(self) -> None:
        """x² - 2x + 1 = 0 → {1} (double)."""
        roots = _exact_roots_deg2((Fraction(1), Fraction(-2), Fraction(1)))
        assert roots == [Fraction(1)]

    def test_no_real_roots(self) -> None:
        """x² + 1 = 0 → [] (no real roots)."""
        roots = _exact_roots_deg2((Fraction(1), Fraction(0), Fraction(1)))
        assert roots == []

    def test_irrational_roots(self) -> None:
        """x² - 2 = 0 → irrational, returns None."""
        roots = _exact_roots_deg2((Fraction(-2), Fraction(0), Fraction(1)))
        assert roots is None

    def test_symmetric_roots(self) -> None:
        """x² - 1 = 0 → {-1, 1}."""
        roots = _exact_roots_deg2((Fraction(-1), Fraction(0), Fraction(1)))
        assert roots == [Fraction(-1), Fraction(1)]


# ===========================================================================
# 6. _make_interval
# ===========================================================================


class TestMakeInterval:
    """_make_interval: construct the correct IR interval condition."""

    def test_unbounded_below_strict(self) -> None:
        """(−∞, a) → Less(x, a)."""
        result = _make_interval(X, None, ONE, True, True)
        assert result == IRApply(LESS, (X, ONE))

    def test_unbounded_below_nonstrict(self) -> None:
        """(−∞, a] → LessEqual(x, a)."""
        result = _make_interval(X, None, ONE, False, False)
        assert result == IRApply(LESS_EQUAL, (X, ONE))

    def test_unbounded_above_strict(self) -> None:
        """(a, +∞) → Greater(x, a)."""
        result = _make_interval(X, ONE, None, True, True)
        assert result == IRApply(GREATER, (X, ONE))

    def test_unbounded_above_nonstrict(self) -> None:
        """[a, +∞) → GreaterEqual(x, a)."""
        result = _make_interval(X, ONE, None, False, False)
        assert result == IRApply(GREATER_EQUAL, (X, ONE))

    def test_bounded_strict(self) -> None:
        """(a, b) → And(Greater(x, a), Less(x, b))."""
        a, b = IRInteger(-1), IRInteger(1)
        result = _make_interval(X, a, b, True, True)
        expected = IRApply(AND, (IRApply(GREATER, (X, a)), IRApply(LESS, (X, b))))
        assert result == expected

    def test_bounded_nonstrict(self) -> None:
        """[a, b] → And(GreaterEqual(x, a), LessEqual(x, b))."""
        a, b = IRInteger(1), IRInteger(2)
        result = _make_interval(X, a, b, False, False)
        expected = IRApply(
            AND,
            (IRApply(GREATER_EQUAL, (X, a)), IRApply(LESS_EQUAL, (X, b))),
        )
        assert result == expected

    def test_both_none_all_reals(self) -> None:
        """(−∞, +∞) → GreaterEqual(0, 0)."""
        result = _make_interval(X, None, None, True, True)
        assert result == IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))

    def test_bounded_mixed(self) -> None:
        """(a, b] → And(Greater(x, a), LessEqual(x, b))."""
        a, b = IRInteger(-3), IRInteger(3)
        result = _make_interval(X, a, b, True, False)  # lo strict, hi nonstrict
        expected = IRApply(
            AND,
            (IRApply(GREATER, (X, a)), IRApply(LESS_EQUAL, (X, b))),
        )
        assert result == expected


# ===========================================================================
# 7. _solve_poly_ineq — core algorithm (no mocking needed)
# ===========================================================================


class TestSolvePolyIneqCore:
    """_solve_poly_ineq: sign analysis + interval construction.

    This tests the heart of the inequality solver by passing coefficients
    and pre-computed roots directly, bypassing the polynomial bridge and
    root-finding layers.
    """

    # Shorthand for "exact_roots_ir built from fractions"
    @staticmethod
    def _ir(fracs: list[Fraction]) -> list:
        return [_frac_ir(f) for f in fracs]

    def test_linear_greater(self) -> None:
        """x - 1 > 0 → [Greater(x, 1)]."""
        coeffs = (Fraction(-1), Fraction(1))
        roots_f = [Fraction(1)]
        roots_n = [1.0]
        result = _solve_poly_ineq(
            coeffs, want_positive=True, strict=True, var=X,
            exact_roots_ir=self._ir(roots_f),
            numeric_roots=roots_n,
        )
        assert len(result) == 1
        assert result[0] == IRApply(GREATER, (X, IRInteger(1)))

    def test_linear_less_equal(self) -> None:
        """x - 1 <= 0 → [LessEqual(x, 1)]."""
        coeffs = (Fraction(-1), Fraction(1))
        roots_f = [Fraction(1)]
        roots_n = [1.0]
        result = _solve_poly_ineq(
            coeffs, want_positive=False, strict=False, var=X,
            exact_roots_ir=self._ir(roots_f),
            numeric_roots=roots_n,
        )
        assert len(result) == 1
        assert result[0] == IRApply(LESS_EQUAL, (X, IRInteger(1)))

    def test_quad_two_roots_greater(self) -> None:
        """x² - 1 > 0 → [Less(x, -1), Greater(x, 1)]."""
        coeffs = (Fraction(-1), Fraction(0), Fraction(1))
        roots_f = [Fraction(-1), Fraction(1)]
        roots_n = [-1.0, 1.0]
        result = _solve_poly_ineq(
            coeffs, want_positive=True, strict=True, var=X,
            exact_roots_ir=self._ir(roots_f),
            numeric_roots=roots_n,
        )
        assert len(result) == 2
        assert result[0] == IRApply(LESS, (X, IRInteger(-1)))
        assert result[1] == IRApply(GREATER, (X, IRInteger(1)))

    def test_quad_two_roots_greater_equal(self) -> None:
        """x² - 1 >= 0 → [LessEqual(x, -1), GreaterEqual(x, 1)]."""
        coeffs = (Fraction(-1), Fraction(0), Fraction(1))
        roots_f = [Fraction(-1), Fraction(1)]
        roots_n = [-1.0, 1.0]
        result = _solve_poly_ineq(
            coeffs, want_positive=True, strict=False, var=X,
            exact_roots_ir=self._ir(roots_f),
            numeric_roots=roots_n,
        )
        assert len(result) == 2
        assert result[0] == IRApply(LESS_EQUAL, (X, IRInteger(-1)))
        assert result[1] == IRApply(GREATER_EQUAL, (X, IRInteger(1)))

    def test_quad_interval_less(self) -> None:
        """x² - 1 < 0 → [And(Greater(x,-1), Less(x,1))]."""
        coeffs = (Fraction(-1), Fraction(0), Fraction(1))
        roots_f = [Fraction(-1), Fraction(1)]
        roots_n = [-1.0, 1.0]
        result = _solve_poly_ineq(
            coeffs, want_positive=False, strict=True, var=X,
            exact_roots_ir=self._ir(roots_f),
            numeric_roots=roots_n,
        )
        assert len(result) == 1
        inner = result[0]
        assert isinstance(inner, IRApply) and inner.head is AND
        lo, hi = inner.args
        assert lo == IRApply(GREATER, (X, IRInteger(-1)))
        assert hi == IRApply(LESS, (X, IRInteger(1)))

    def test_quad_interval_less_equal(self) -> None:
        """x² - 3x + 2 <= 0 → [And(GreaterEqual(x,1), LessEqual(x,2))]."""
        coeffs = (Fraction(2), Fraction(-3), Fraction(1))
        roots_f = [Fraction(1), Fraction(2)]
        roots_n = [1.0, 2.0]
        result = _solve_poly_ineq(
            coeffs, want_positive=False, strict=False, var=X,
            exact_roots_ir=self._ir(roots_f),
            numeric_roots=roots_n,
        )
        assert len(result) == 1
        inner = result[0]
        assert isinstance(inner, IRApply) and inner.head is AND
        lo, hi = inner.args
        assert lo == IRApply(GREATER_EQUAL, (X, IRInteger(1)))
        assert hi == IRApply(LESS_EQUAL, (X, IRInteger(2)))

    def test_all_reals_trivially_true(self) -> None:
        """x² + 1 > 0 — no real roots, always positive → all reals."""
        coeffs = (Fraction(1), Fraction(0), Fraction(1))
        result = _solve_poly_ineq(
            coeffs, want_positive=True, strict=True, var=X,
            exact_roots_ir=[],
            numeric_roots=[],
        )
        assert len(result) == 1
        assert result[0] == IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))

    def test_no_solution(self) -> None:
        """x² + 1 < 0 — no real roots, never negative → empty list."""
        coeffs = (Fraction(1), Fraction(0), Fraction(1))
        result = _solve_poly_ineq(
            coeffs, want_positive=False, strict=True, var=X,
            exact_roots_ir=[],
            numeric_roots=[],
        )
        assert result == []

    def test_double_root_strict_two_intervals(self) -> None:
        """(x-1)² > 0 — double root at 1; f>0 except at the root itself.

        _solve_poly_ineq sees two open intervals (−∞,1) and (1,+∞) both
        positive.  Result: [Less(x,1), Greater(x,1)].
        """
        coeffs = (Fraction(1), Fraction(-2), Fraction(1))
        roots_f = [Fraction(1)]
        roots_n = [1.0]
        result = _solve_poly_ineq(
            coeffs, want_positive=True, strict=True, var=X,
            exact_roots_ir=self._ir(roots_f),
            numeric_roots=roots_n,
        )
        # Both open intervals satisfy the strict inequality.
        assert len(result) == 2

    def test_double_root_nonstrict_all_reals(self) -> None:
        """(x-1)² >= 0 — always non-negative → all reals sentinel."""
        coeffs = (Fraction(1), Fraction(-2), Fraction(1))
        roots_f = [Fraction(1)]
        roots_n = [1.0]
        result = _solve_poly_ineq(
            coeffs, want_positive=True, strict=False, var=X,
            exact_roots_ir=self._ir(roots_f),
            numeric_roots=roots_n,
        )
        assert len(result) == 1
        assert result[0] == IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))

    def test_float_boundaries_when_no_exact(self) -> None:
        """When exact_roots_ir=None, boundaries become IRFloat."""
        coeffs = (Fraction(-2), Fraction(0), Fraction(1))  # x²-2 > 0
        # exact roots are irrational → pass None to force IRFloat boundaries
        import math
        sqrt2 = math.sqrt(2)
        roots_n = sorted([-sqrt2, sqrt2])
        result = _solve_poly_ineq(
            coeffs, want_positive=True, strict=True, var=X,
            exact_roots_ir=None,
            numeric_roots=roots_n,
        )
        assert len(result) == 2
        # Boundaries should be IRFloat (not IRInteger/IRRational)
        lo_cond = result[0]  # Less(x, -√2)
        assert isinstance(lo_cond, IRApply) and lo_cond.head is LESS
        assert isinstance(lo_cond.args[1], IRFloat)


# ===========================================================================
# 8. try_solve_inequality — full integration via mock bridge
# ===========================================================================


class TestLinearIneq:
    """try_solve_inequality: linear polynomial x + b op 0."""

    def test_x_minus_1_greater(self) -> None:
        """x - 1 > 0 → [Greater(x, 1)]."""
        # coeffs for x - 1: ascending (c0=-1, c1=1)
        with patch_bridge((-1, 1)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result == [IRApply(GREATER, (X, IRInteger(1)))]

    def test_x_minus_1_greater_equal(self) -> None:
        """x - 1 >= 0 → [GreaterEqual(x, 1)]."""
        with patch_bridge((-1, 1)):
            result = try_solve_inequality(_ineq(GREATER_EQUAL), X)
        assert result == [IRApply(GREATER_EQUAL, (X, IRInteger(1)))]

    def test_x_minus_1_less(self) -> None:
        """x - 1 < 0 → [Less(x, 1)]."""
        with patch_bridge((-1, 1)):
            result = try_solve_inequality(_ineq(LESS), X)
        assert result == [IRApply(LESS, (X, IRInteger(1)))]

    def test_x_minus_1_less_equal(self) -> None:
        """x - 1 <= 0 → [LessEqual(x, 1)]."""
        with patch_bridge((-1, 1)):
            result = try_solve_inequality(_ineq(LESS_EQUAL), X)
        assert result == [IRApply(LESS_EQUAL, (X, IRInteger(1)))]

    def test_rational_root(self) -> None:
        """2x + 3 < 0 → [Less(x, -3/2)]."""
        # ascending: (3, 2) → root at -3/2
        with patch_bridge((3, 2)):
            result = try_solve_inequality(_ineq(LESS), X)
        assert result is not None
        assert len(result) == 1
        node = result[0]
        assert isinstance(node, IRApply) and node.head is LESS
        assert node.args[1] == IRRational(-3, 2)

    def test_negative_slope(self) -> None:
        """-x + 2 > 0 → root at 2; positive for x<2 → [Less(x, 2)]."""
        # ascending: (2, -1) → root at 2
        with patch_bridge((2, -1)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result == [IRApply(LESS, (X, IRInteger(2)))]


class TestQuadIneqTwoRoots:
    """Quadratic polynomial with two distinct rational roots."""

    def test_x2_minus_1_greater(self) -> None:
        """x² - 1 > 0 → [Less(x,-1), Greater(x,1)]."""
        with patch_bridge((-1, 0, 1)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result is not None
        assert len(result) == 2
        assert result[0] == IRApply(LESS, (X, IRInteger(-1)))
        assert result[1] == IRApply(GREATER, (X, IRInteger(1)))

    def test_x2_minus_1_greater_equal(self) -> None:
        """x² - 1 >= 0 → [LessEqual(x,-1), GreaterEqual(x,1)]."""
        with patch_bridge((-1, 0, 1)):
            result = try_solve_inequality(_ineq(GREATER_EQUAL), X)
        assert result is not None
        assert len(result) == 2
        assert result[0] == IRApply(LESS_EQUAL, (X, IRInteger(-1)))
        assert result[1] == IRApply(GREATER_EQUAL, (X, IRInteger(1)))

    def test_x2_minus_1_less(self) -> None:
        """x² - 1 < 0 → [And(Greater(x,-1), Less(x,1))]."""
        with patch_bridge((-1, 0, 1)):
            result = try_solve_inequality(_ineq(LESS), X)
        assert result is not None
        assert len(result) == 1
        inner = result[0]
        assert inner.head is AND
        assert inner.args[0] == IRApply(GREATER, (X, IRInteger(-1)))
        assert inner.args[1] == IRApply(LESS, (X, IRInteger(1)))

    def test_x2_minus_1_less_equal(self) -> None:
        """x² - 1 <= 0 → [And(GreaterEqual(x,-1), LessEqual(x,1))]."""
        with patch_bridge((-1, 0, 1)):
            result = try_solve_inequality(_ineq(LESS_EQUAL), X)
        assert result is not None
        assert len(result) == 1
        inner = result[0]
        assert inner.head is AND
        assert inner.args[0] == IRApply(GREATER_EQUAL, (X, IRInteger(-1)))
        assert inner.args[1] == IRApply(LESS_EQUAL, (X, IRInteger(1)))

    def test_x2_minus_3x_plus_2_less_equal(self) -> None:
        """x² - 3x + 2 <= 0 → [And(GreaterEqual(x,1), LessEqual(x,2))]."""
        with patch_bridge((2, -3, 1)):
            result = try_solve_inequality(_ineq(LESS_EQUAL), X)
        assert result is not None
        assert len(result) == 1
        inner = result[0]
        assert inner.head is AND
        assert inner.args[0] == IRApply(GREATER_EQUAL, (X, IRInteger(1)))
        assert inner.args[1] == IRApply(LESS_EQUAL, (X, IRInteger(2)))

    def test_x2_minus_3x_plus_2_less(self) -> None:
        """x² - 3x + 2 < 0 → [And(Greater(x,1), Less(x,2))]."""
        with patch_bridge((2, -3, 1)):
            result = try_solve_inequality(_ineq(LESS), X)
        assert result is not None
        assert len(result) == 1
        inner = result[0]
        assert inner.head is AND
        assert inner.args[0] == IRApply(GREATER, (X, IRInteger(1)))
        assert inner.args[1] == IRApply(LESS, (X, IRInteger(2)))


class TestQuadIneqDoubleRoot:
    """Quadratic with a double root: (x-1)² = x² - 2x + 1."""

    def test_double_root_strict(self) -> None:
        """(x-1)² > 0 → two open half-lines (x<1 and x>1)."""
        with patch_bridge((1, -2, 1)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result is not None
        # Both intervals (−∞,1) and (1,+∞) satisfy the strict inequality.
        assert len(result) == 2

    def test_double_root_nonstrict(self) -> None:
        """(x-1)² >= 0 → all reals (trivially true)."""
        with patch_bridge((1, -2, 1)):
            result = try_solve_inequality(_ineq(GREATER_EQUAL), X)
        assert result == [IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))]


class TestQuadIneqNoRoots:
    """Quadratic with no real roots: x² + 1."""

    def test_always_positive(self) -> None:
        """x² + 1 > 0 → all reals."""
        with patch_bridge((1, 0, 1)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result == [IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))]

    def test_never_negative(self) -> None:
        """x² + 1 < 0 → empty list."""
        with patch_bridge((1, 0, 1)):
            result = try_solve_inequality(_ineq(LESS), X)
        assert result == []

    def test_nonstrict_always_true(self) -> None:
        """x² + 1 >= 0 → all reals."""
        with patch_bridge((1, 0, 1)):
            result = try_solve_inequality(_ineq(GREATER_EQUAL), X)
        assert result == [IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))]


class TestHighDegree:
    """Higher-degree polynomials (degrees 3 and 4) use numeric roots."""

    def test_cubic_x3_minus_x_greater(self) -> None:
        """x³ - x = x(x-1)(x+1) > 0 → two positive intervals."""
        # ascending: (0, -1, 0, 1)
        with patch_bridge((0, -1, 0, 1)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result is not None
        # Roots: -1, 0, 1.  Positive on (-1,0) and (1,+∞).
        assert len(result) == 2

    def test_quartic_x4_minus_1_greater(self) -> None:
        """x⁴ - 1 > 0 → two outer intervals."""
        # ascending: (-1, 0, 0, 0, 1)
        with patch_bridge((-1, 0, 0, 0, 1)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result is not None
        assert len(result) == 2

    def test_quartic_boundaries_are_ir_float(self) -> None:
        """x⁴ - 1 > 0: boundaries come from numeric solver → IRFloat."""
        with patch_bridge((-1, 0, 0, 0, 1)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result is not None
        # The outer Less(x, a) condition should have IRFloat boundary.
        lo_cond = result[0]
        assert isinstance(lo_cond, IRApply)
        boundary = lo_cond.args[1]
        assert isinstance(boundary, IRFloat)


# ===========================================================================
# 9. Constant polynomial edge cases
# ===========================================================================


class TestConstantPolynomial:
    """Degree-0 polynomials: always true or always false."""

    def test_positive_constant_greater(self) -> None:
        """f = 5 > 0 → always satisfied → all reals."""
        # Bridge returns a constant (degree 0)
        with patch_bridge((5,)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result == [IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))]

    def test_negative_constant_greater(self) -> None:
        """f = -3 > 0 → never satisfied → []."""
        with patch_bridge((-3,)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result == []

    def test_zero_constant_nonstrict(self) -> None:
        """f = 0 >= 0 → all reals (0 >= 0 is true)."""
        with patch_bridge((0,)):
            result = try_solve_inequality(_ineq(GREATER_EQUAL), X)
        assert result == [IRApply(GREATER_EQUAL, (IRInteger(0), IRInteger(0)))]

    def test_zero_constant_strict(self) -> None:
        """f = 0 > 0 → never satisfied (0 is not strictly positive)."""
        with patch_bridge((0,)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result == []


# ===========================================================================
# 10. Fallthrough / None cases
# ===========================================================================


class TestFallthrough:
    """Cases that should return None (unsupported or unrecognised)."""

    def test_non_ir_apply(self) -> None:
        """Non-IRApply input → None."""
        assert try_solve_inequality(IRInteger(1), X) is None

    def test_equal_head_unsupported(self) -> None:
        """An Equal(...) IR node is not an inequality → None."""
        from symbolic_ir import EQUAL

        eq_node = IRApply(EQUAL, (X, IRInteger(0)))
        assert try_solve_inequality(eq_node, X) is None

    def test_wrong_arg_count(self) -> None:
        """Inequality with 3 args (malformed) → None."""
        node = IRApply(GREATER, (X, IRInteger(0), IRInteger(1)))
        assert try_solve_inequality(node, X) is None

    def test_bridge_unavailable(self) -> None:
        """If polynomial bridge is not installed, return None."""
        # Remove bridge from sys.modules if present.
        old_bridge = sys.modules.pop("symbolic_vm.polynomial_bridge", None)
        old_vm = sys.modules.pop("symbolic_vm", None)
        try:
            result = try_solve_inequality(_ineq(GREATER), X)
            assert result is None
        finally:
            if old_vm is not None:
                sys.modules["symbolic_vm"] = old_vm
            if old_bridge is not None:
                sys.modules["symbolic_vm.polynomial_bridge"] = old_bridge

    def test_bridge_returns_none(self) -> None:
        """If to_rational returns None (non-polynomial), return None."""
        with patch_bridge_none():
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result is None

    def test_degree_above_4(self) -> None:
        """Degree-5 polynomial → None (unsupported)."""
        # ascending: (1, 0, 0, 0, 0, 1) — x^5 + 1
        with patch_bridge((1, 0, 0, 0, 0, 1)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result is None

    def test_lhs_zero_form(self) -> None:
        """0 > x — lhs is 0, rhs is x; direction is flipped internally.

        0 > x  ↔  x < 0, so f_ir = x, want_positive flipped to False.
        Bridge returns ascending (0, 1) for x → root at 0 → Less(x, 0).
        """
        node = IRApply(GREATER, (IRInteger(0), X))
        with patch_bridge((0, 1)):
            result = try_solve_inequality(node, X)
        assert result is not None
        assert len(result) == 1
        assert result[0] == IRApply(LESS, (X, IRInteger(0)))

    def test_irrational_quadratic_via_try_solve(self) -> None:
        """x² - 2 > 0 — irrational roots force numeric path in try_solve_inequality.

        Ascending coeffs: (-2, 0, 1) → irrational roots ±√2.
        Result: two outer intervals with IRFloat boundaries.
        """
        with patch_bridge((-2, 0, 1)):
            result = try_solve_inequality(_ineq(GREATER), X)
        assert result is not None
        assert len(result) == 2
        # Boundaries must be IRFloat (irrational)
        lo_cond = result[0]
        assert isinstance(lo_cond, IRApply) and lo_cond.head is LESS
        assert isinstance(lo_cond.args[1], IRFloat)

    def test_rational_function_rejected(self) -> None:
        """to_rational returning non-trivial denominator → None."""
        # Return a rational function (1+x)/(1+x²)
        mock_mod = types.ModuleType("symbolic_vm.polynomial_bridge")
        mock_mod.to_rational = lambda e, v: (  # noqa: ARG005
            (Fraction(1), Fraction(1)),
            (Fraction(1), Fraction(0), Fraction(1)),
        )
        old_bridge = sys.modules.get("symbolic_vm.polynomial_bridge")
        old_vm = sys.modules.get("symbolic_vm")
        sys.modules["symbolic_vm"] = types.ModuleType("symbolic_vm")
        sys.modules["symbolic_vm.polynomial_bridge"] = mock_mod
        try:
            result = try_solve_inequality(_ineq(GREATER), X)
            assert result is None
        finally:
            if old_vm is None:
                sys.modules.pop("symbolic_vm", None)
            else:
                sys.modules["symbolic_vm"] = old_vm
            if old_bridge is None:
                sys.modules.pop("symbolic_vm.polynomial_bridge", None)
            else:
                sys.modules["symbolic_vm.polynomial_bridge"] = old_bridge
