"""Tests for cas_solve.transcendental — Phase 26 transcendental solver.

These tests cover the helper functions, internal dispatchers, and the
public ``try_solve_transcendental`` API directly within the ``cas-solve``
package boundary, without depending on ``symbolic_vm``.

Strategy
--------

``transcendental.py`` contains two categories of helper that need the
optional ``symbolic_vm.polynomial_bridge.to_rational`` function:

* ``_extract_linear``   — parses linear arguments like ``ax + b``
* ``_poly_coeffs``      — extracts polynomial coefficients for compound
                          substitution

For the tests that exercise these paths we use one of two strategies:

1. **mock_bridge fixture** — injects a lightweight fake
   ``symbolic_vm.polynomial_bridge`` into ``sys.modules`` so that
   ``_extract_linear`` can handle bare ``x`` and ``n*x`` patterns.

2. **unittest.mock.patch.object** — patches the private helper directly
   with a pre-cooked return value for tests that need complex polynomial
   coefficients (compound substitution).

All other helpers (``_frac_ir``, ``_is_const_wrt``, ``_split_equal``,
``_solve_linear_for_val``, ``_subst_walk``, ``_substitute_func``,
``_solve_poly``) are tested without any mocking.
"""

from __future__ import annotations

import sys
import types
from fractions import Fraction
from unittest.mock import patch

import pytest
from symbolic_ir import (
    ADD,
    ASINH,
    ATANH,
    COS,
    COSH,
    DIV,
    EQUAL,
    EXP,
    LAMBERT_W,
    LOG,
    MUL,
    SIN,
    SINH,
    SUB,
    TANH,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRSymbol,
)

import cas_solve.transcendental as _mod
from cas_solve.transcendental import (
    _extract_linear,
    _frac_ir,
    _is_const_wrt,
    _poly_coeffs,
    _solve_linear_for_val,
    _solve_poly,
    _split_equal,
    _subst_walk,
    _substitute_func,
    _try_compound,
    _try_func_eq_const,
    _try_lambert,
    try_solve_transcendental,
)

# ---------------------------------------------------------------------------
# Shared symbols and tiny IR builders
# ---------------------------------------------------------------------------

X = IRSymbol("x")
Y = IRSymbol("y")


def _sin(node: IRApply) -> IRApply:
    return IRApply(SIN, (node,))


def _cos(node: IRApply) -> IRApply:
    return IRApply(COS, (node,))


def _exp(node: IRApply) -> IRApply:
    return IRApply(EXP, (node,))


def _log(node: IRApply) -> IRApply:
    return IRApply(LOG, (node,))


def _eq(lhs, rhs) -> IRApply:
    return IRApply(EQUAL, (lhs, rhs))


# ---------------------------------------------------------------------------
# Fixture: lightweight polynomial-bridge mock
# ---------------------------------------------------------------------------


@pytest.fixture
def mock_bridge():
    """Inject a fake ``symbolic_vm.polynomial_bridge`` into ``sys.modules``.

    The fake ``to_rational`` handles only two patterns:

    * ``IRSymbol`` equal to ``var``         →  ``((0, 1), (1,))``  (1·x)
    * ``MUL(IRInteger(n), var)``            →  ``((0, n), (1,))``  (n·x)

    Any other expression returns ``None`` so unexpected patterns still
    fall through to ``None`` just as they would without the bridge.
    """

    def _to_rational(expr, var):
        if isinstance(expr, IRSymbol) and expr == var:
            return (Fraction(0), Fraction(1)), (Fraction(1),)
        if (
            isinstance(expr, IRApply)
            and expr.head is MUL
            and len(expr.args) == 2
        ):
            a_nd, b_nd = expr.args
            if (
                isinstance(a_nd, IRInteger)
                and isinstance(b_nd, IRSymbol)
                and b_nd == var
            ):
                return (Fraction(0), Fraction(a_nd.value)), (Fraction(1),)
            if (
                isinstance(b_nd, IRInteger)
                and isinstance(a_nd, IRSymbol)
                and a_nd == var
            ):
                return (Fraction(0), Fraction(b_nd.value)), (Fraction(1),)
        return None

    mock_mod = types.ModuleType("symbolic_vm.polynomial_bridge")
    mock_mod.to_rational = _to_rational
    mock_vm = types.ModuleType("symbolic_vm")

    old_vm = sys.modules.get("symbolic_vm")
    old_bridge = sys.modules.get("symbolic_vm.polynomial_bridge")

    sys.modules["symbolic_vm"] = mock_vm
    sys.modules["symbolic_vm.polynomial_bridge"] = mock_mod

    yield _to_rational

    if old_vm is None:
        sys.modules.pop("symbolic_vm", None)
    else:
        sys.modules["symbolic_vm"] = old_vm

    if old_bridge is None:
        sys.modules.pop("symbolic_vm.polynomial_bridge", None)
    else:
        sys.modules["symbolic_vm.polynomial_bridge"] = old_bridge


# ===========================================================================
# 1. _frac_ir
# ===========================================================================


class TestFracIr:
    """_frac_ir: convert a Fraction to its canonical IR literal."""

    def test_integer_fraction(self) -> None:
        """Fraction(3) → IRInteger(3)."""
        assert _frac_ir(Fraction(3)) == IRInteger(3)

    def test_negative_integer(self) -> None:
        """Fraction(-7) → IRInteger(-7)."""
        assert _frac_ir(Fraction(-7)) == IRInteger(-7)

    def test_zero(self) -> None:
        """Fraction(0) → IRInteger(0)."""
        assert _frac_ir(Fraction(0)) == IRInteger(0)

    def test_proper_fraction(self) -> None:
        """Fraction(1, 2) → IRRational(1, 2)."""
        assert _frac_ir(Fraction(1, 2)) == IRRational(1, 2)

    def test_negative_fraction(self) -> None:
        """Fraction(-3, 4) → IRRational with numer −3, denom 4."""
        result = _frac_ir(Fraction(-3, 4))
        assert isinstance(result, IRRational)
        assert result.numer == -3
        assert result.denom == 4


# ===========================================================================
# 2. _is_const_wrt
# ===========================================================================


class TestIsConstWrt:
    """_is_const_wrt: true iff a node is free of the given variable."""

    def test_integer_is_const(self) -> None:
        assert _is_const_wrt(IRInteger(5), X)

    def test_float_is_const(self) -> None:
        assert _is_const_wrt(IRFloat(3.14), X)

    def test_rational_is_const(self) -> None:
        assert _is_const_wrt(IRRational(1, 2), X)

    def test_same_symbol_is_not_const(self) -> None:
        assert not _is_const_wrt(X, X)

    def test_same_name_different_object_is_not_const(self) -> None:
        """IRSymbol is not interned — equality, not identity, is used."""
        x2 = IRSymbol("x")
        assert not _is_const_wrt(x2, X)

    def test_different_symbol_is_const(self) -> None:
        assert _is_const_wrt(Y, X)

    def test_apply_containing_var_is_not_const(self) -> None:
        assert not _is_const_wrt(_sin(X), X)

    def test_apply_not_containing_var_is_const(self) -> None:
        assert _is_const_wrt(_sin(Y), X)

    def test_nested_apply_with_var(self) -> None:
        inner = IRApply(ADD, (IRInteger(1), X))
        assert not _is_const_wrt(inner, X)

    def test_nested_apply_without_var(self) -> None:
        inner = IRApply(ADD, (IRInteger(1), Y))
        assert _is_const_wrt(inner, X)


# ===========================================================================
# 3. _split_equal
# ===========================================================================


class TestSplitEqual:
    """_split_equal: unwrap Equal(lhs, rhs) or return None."""

    def test_equal_node(self) -> None:
        """Equal(sin(x), 0) → (sin(x), 0)."""
        lhs, rhs = _sin(X), IRInteger(0)
        result = _split_equal(_eq(lhs, rhs))
        assert result == (lhs, rhs)

    def test_non_equal_apply_returns_none(self) -> None:
        assert _split_equal(_sin(X)) is None

    def test_plain_integer_returns_none(self) -> None:
        assert _split_equal(IRInteger(42)) is None

    def test_one_arg_equal_returns_none(self) -> None:
        """Equal node with only one argument should return None."""
        one_arg = IRApply(EQUAL, (X,))
        assert _split_equal(one_arg) is None

    def test_apply_with_non_symbol_head_returns_none(self) -> None:
        """An IRApply whose head is itself an IRApply should return None."""
        weird_head = IRApply(IRApply(ADD, (X,)), (X, Y))
        assert _split_equal(weird_head) is None


# ===========================================================================
# 4. _solve_linear_for_val
# ===========================================================================


class TestSolveLinearForVal:
    """_solve_linear_for_val: invert a·x + b = val for x."""

    def test_a1_b0_returns_val(self) -> None:
        """a=1, b=0 → val (no wrapping needed)."""
        val = IRInteger(5)
        result = _solve_linear_for_val(Fraction(1), Fraction(0), val, X)
        assert result is val

    def test_a2_b0_returns_div(self) -> None:
        """a=2, b=0 → DIV(val, 2)."""
        val = IRInteger(4)
        result = _solve_linear_for_val(Fraction(2), Fraction(0), val, X)
        assert isinstance(result, IRApply)
        assert result.head is DIV
        assert result.args[1] == IRInteger(2)

    def test_a1_b3_returns_sub(self) -> None:
        """a=1, b=3 → SUB(val, 3)."""
        val = IRInteger(5)
        result = _solve_linear_for_val(Fraction(1), Fraction(3), val, X)
        assert isinstance(result, IRApply)
        assert result.head is SUB
        assert result.args[1] == IRInteger(3)

    def test_a2_b3_returns_div_of_sub(self) -> None:
        """a=2, b=3 → DIV(SUB(val, 3), 2)."""
        val = IRInteger(5)
        result = _solve_linear_for_val(Fraction(2), Fraction(3), val, X)
        assert isinstance(result, IRApply)
        assert result.head is DIV
        inner = result.args[0]
        assert isinstance(inner, IRApply)
        assert inner.head is SUB

    def test_rational_a(self) -> None:
        """a=1/2 → DIV(val, IRRational(1,2))."""
        val = IRInteger(3)
        result = _solve_linear_for_val(Fraction(1, 2), Fraction(0), val, X)
        assert isinstance(result, IRApply)
        assert result.head is DIV
        assert result.args[1] == IRRational(1, 2)


# ===========================================================================
# 5. _subst_walk and _substitute_func
# ===========================================================================


class TestSubstWalk:
    """_subst_walk: replace f(var) with sub inside an IR tree."""

    def setup_method(self) -> None:
        self.sub = IRSymbol("__u__")

    def test_sin_x_replaced(self) -> None:
        """sin(x) with func_name=Sin, var=x → sub."""
        result = _subst_walk(_sin(X), "Sin", X, self.sub)
        assert result == self.sub

    def test_bare_var_returns_none(self) -> None:
        """Bare x (not inside a function) → None (substitution fails)."""
        result = _subst_walk(X, "Sin", X, self.sub)
        assert result is None

    def test_integer_unchanged(self) -> None:
        """IRInteger passes through unchanged."""
        result = _subst_walk(IRInteger(7), "Sin", X, self.sub)
        assert result == IRInteger(7)

    def test_other_symbol_unchanged(self) -> None:
        """Symbols other than var pass through unchanged."""
        result = _subst_walk(Y, "Sin", X, self.sub)
        assert result is Y

    def test_nested_add_both_replaced(self) -> None:
        """ADD(sin(x), sin(x)) → ADD(sub, sub)."""
        expr = IRApply(ADD, (_sin(X), _sin(X)))
        result = _subst_walk(expr, "Sin", X, self.sub)
        assert result == IRApply(ADD, (self.sub, self.sub))

    def test_fails_when_var_appears_bare(self) -> None:
        """ADD(sin(x), x) → None because x appears outside Sin."""
        expr = IRApply(ADD, (_sin(X), X))
        result = _subst_walk(expr, "Sin", X, self.sub)
        assert result is None

    def test_wrong_func_name_recurses_into_var(self) -> None:
        """cos(x) when substituting Sin: x appears bare inside Cos → None."""
        result = _subst_walk(_cos(X), "Sin", X, self.sub)
        assert result is None

    def test_non_symbol_head_returns_none(self) -> None:
        """IRApply whose head is another IRApply → None."""
        weird = IRApply(IRApply(ADD, (X,)), (X,))
        result = _subst_walk(weird, "Sin", X, self.sub)
        assert result is None


class TestSubstituteFunc:
    """_substitute_func: high-level wrapper that also checks var absence."""

    def setup_method(self) -> None:
        self.sub = IRSymbol("__u__")

    def test_sin_x_substituted(self) -> None:
        """sin(x) → sub (x gone after substitution)."""
        result = _substitute_func(_sin(X), "Sin", X, self.sub)
        assert result == self.sub

    def test_fails_when_var_remains_after_substitution(self) -> None:
        """ADD(sin(x), x) → None since x remains bare after substitution."""
        expr = IRApply(ADD, (_sin(X), X))
        result = _substitute_func(expr, "Sin", X, self.sub)
        assert result is None

    def test_expression_without_target_func_returned_if_var_absent(self) -> None:
        """sin(y) — Sin(y) doesn't match Sin(x), but y≠x so x is absent.
        The function returns sin(y) because x never appeared."""
        result = _substitute_func(_sin(Y), "Sin", X, self.sub)
        # x doesn't appear, so no failure on the var-presence check.
        # sin(y) is returned as-is.
        assert result == _sin(Y)


# ===========================================================================
# 6. _solve_poly
# ===========================================================================


class TestSolvePoly:
    """_solve_poly: delegate to degree-specific solvers."""

    def test_degree_0_returns_none(self) -> None:
        """Constant polynomial (degree 0) → None."""
        assert _solve_poly((Fraction(5),)) is None

    def test_degree_1_simple(self) -> None:
        """u + 2 = 0 → [−2]."""
        # (b=2, a=1) → 1·u + 2 = 0 → u = −2
        result = _solve_poly((Fraction(2), Fraction(1)))
        assert result == [IRInteger(-2)]

    def test_degree_1_zero_constant(self) -> None:
        """1·u + 0 = 0 → [0]."""
        result = _solve_poly((Fraction(0), Fraction(1)))
        assert result == [IRInteger(0)]

    def test_degree_1_all_returns_none(self) -> None:
        """0·u + 0 = 0 → trivially true → None."""
        result = _solve_poly((Fraction(0), Fraction(0)))
        assert result is None

    def test_degree_2_two_distinct_roots(self) -> None:
        """u² − u − 2 = 0 → roots {2, −1}."""
        # (c=−2, b=−1, a=1) ascending
        result = _solve_poly((Fraction(-2), Fraction(-1), Fraction(1)))
        assert result is not None
        assert len(result) == 2
        values = {repr(r) for r in result}
        assert repr(IRInteger(2)) in values or repr(IRInteger(-1)) in values

    def test_degree_2_double_root(self) -> None:
        """u² − 2u + 1 = 0 → [1]."""
        result = _solve_poly((Fraction(1), Fraction(-2), Fraction(1)))
        assert result is not None
        assert IRInteger(1) in result

    def test_degree_3_runs_without_error(self) -> None:
        """Degree-3 polynomial delegates to cubic solver (result may vary)."""
        # u³ − u = 0: (d=0, c=−1, b=0, a=1)
        result = _solve_poly((Fraction(0), Fraction(-1), Fraction(0), Fraction(1)))
        assert result is None or isinstance(result, list)

    def test_degree_4_runs_without_error(self) -> None:
        """Degree-4 polynomial delegates to quartic solver."""
        # u⁴ − 1 = 0: (e=−1, d=0, c=0, b=0, a=1)
        result = _solve_poly(
            (Fraction(-1), Fraction(0), Fraction(0), Fraction(0), Fraction(1))
        )
        assert result is None or isinstance(result, list)

    def test_degree_5_returns_none(self) -> None:
        """Degree > 4 (unsupported) → None."""
        result = _solve_poly(tuple(Fraction(i) for i in range(7)))
        assert result is None


# ===========================================================================
# 7. _try_func_eq_const — early-return paths (no bridge)
# ===========================================================================


class TestTryFuncEqConstEarlyReturns:
    """Paths that return None before reaching the polynomial bridge."""

    def test_const_side_contains_var_returns_none(self) -> None:
        """If const_side is not constant w.r.t. var, return None."""
        assert _try_func_eq_const(_sin(X), X, X) is None

    def test_func_side_not_irapply_returns_none(self) -> None:
        """Plain symbol as func_side → None."""
        assert _try_func_eq_const(X, IRInteger(0), X) is None

    def test_func_side_non_symbol_head_returns_none(self) -> None:
        """IRApply with IRApply head (not IRSymbol) → None."""
        weird = IRApply(IRApply(ADD, (X,)), (X,))
        assert _try_func_eq_const(weird, IRInteger(0), X) is None


# ===========================================================================
# 8. _try_func_eq_const — solve paths (with mock bridge)
# ===========================================================================


class TestTryFuncEqConst:
    """Test all function families using the mock polynomial bridge."""

    def test_multi_arg_func_returns_none(self, mock_bridge) -> None:
        """f(x, y) — two args — → None."""
        two_arg = IRApply(IRSymbol("Foo"), (X, Y))
        assert _try_func_eq_const(two_arg, IRInteger(0), X) is None

    def test_extract_linear_fails_returns_none(self, mock_bridge) -> None:
        """Argument not recognised by mock bridge → None."""
        # sin(sin(x)) has non-linear argument
        nested = IRApply(SIN, (_sin(X),))
        assert _try_func_eq_const(nested, IRInteger(0), X) is None

    def test_sin_equals_zero(self, mock_bridge) -> None:
        """sin(x) = 0 → two periodic solution families."""
        result = _try_func_eq_const(_sin(X), IRInteger(0), X)
        assert result is not None
        assert len(result) == 2
        for sol in result:
            assert isinstance(sol, IRApply)

    def test_cos_equals_one(self, mock_bridge) -> None:
        """cos(x) = 1 → two periodic solution families."""
        result = _try_func_eq_const(_cos(X), IRInteger(1), X)
        assert result is not None
        assert len(result) == 2

    def test_tan_equals_zero(self, mock_bridge) -> None:
        """tan(x) = 0 → one periodic family."""
        result = _try_func_eq_const(
            IRApply(IRSymbol("Tan"), (X,)), IRInteger(0), X
        )
        assert result is not None
        assert len(result) == 1

    def test_exp_equals_one(self, mock_bridge) -> None:
        """exp(x) = 1 → unique solution involving Log."""
        result = _try_func_eq_const(_exp(X), IRInteger(1), X)
        assert result is not None
        assert len(result) == 1
        # Solution = log(1); the outermost node wraps LOG
        sol = result[0]
        assert isinstance(sol, IRApply)
        assert sol.head is LOG

    def test_log_equals_zero(self, mock_bridge) -> None:
        """log(x) = 0 → unique solution involving Exp."""
        result = _try_func_eq_const(_log(X), IRInteger(0), X)
        assert result is not None
        assert len(result) == 1
        sol = result[0]
        assert isinstance(sol, IRApply)
        assert sol.head is EXP

    def test_sinh_equals_zero(self, mock_bridge) -> None:
        """sinh(x) = 0 → unique solution involving Asinh."""
        result = _try_func_eq_const(IRApply(SINH, (X,)), IRInteger(0), X)
        assert result is not None
        assert len(result) == 1
        sol = result[0]
        assert isinstance(sol, IRApply)
        assert sol.head is ASINH

    def test_cosh_equals_one(self, mock_bridge) -> None:
        """cosh(x) = 1 → two branches (±acosh)."""
        result = _try_func_eq_const(IRApply(COSH, (X,)), IRInteger(1), X)
        assert result is not None
        assert len(result) == 2

    def test_tanh_equals_zero(self, mock_bridge) -> None:
        """tanh(x) = 0 → unique solution involving Atanh."""
        result = _try_func_eq_const(IRApply(TANH, (X,)), IRInteger(0), X)
        assert result is not None
        assert len(result) == 1
        sol = result[0]
        assert isinstance(sol, IRApply)
        assert sol.head is ATANH

    def test_unrecognised_head_returns_none(self, mock_bridge) -> None:
        """Unknown function head → None."""
        result = _try_func_eq_const(
            IRApply(IRSymbol("Zeta"), (X,)), IRInteger(0), X
        )
        assert result is None

    def test_scaled_argument_2x(self, mock_bridge) -> None:
        """sin(2*x) = 0 — mock bridge handles 2*x → two families."""
        two_x = IRApply(MUL, (IRInteger(2), X))
        result = _try_func_eq_const(IRApply(SIN, (two_x,)), IRInteger(0), X)
        assert result is not None
        assert len(result) == 2


# ===========================================================================
# 9. _try_lambert — Lambert W pattern detector
# ===========================================================================


class TestTryLambert:
    """_try_lambert: detect f·exp(f) = c and return W-based solution."""

    def test_x_exp_x_equals_one(self, mock_bridge) -> None:
        """x·exp(x) = 1 → [LambertW(1)]."""
        product = IRApply(MUL, (X, _exp(X)))
        result = _try_lambert(product, IRInteger(1), X)
        assert result is not None
        assert len(result) == 1
        sol = result[0]
        assert isinstance(sol, IRApply)
        assert sol.head is LAMBERT_W

    def test_exp_x_times_x_commutative(self, mock_bridge) -> None:
        """exp(x)·x = 1 also matches (factor order is irrelevant)."""
        product = IRApply(MUL, (_exp(X), X))
        result = _try_lambert(product, IRInteger(1), X)
        assert result is not None
        assert len(result) == 1

    def test_constant_on_left_side(self, mock_bridge) -> None:
        """1 = x·exp(x) — symmetric dispatch handles both orientations."""
        product = IRApply(MUL, (X, _exp(X)))
        result = _try_lambert(IRInteger(1), product, X)
        assert result is not None
        assert len(result) == 1

    def test_not_mul_returns_none(self, mock_bridge) -> None:
        """Non-MUL expression as product side → None."""
        result = _try_lambert(_sin(X), IRInteger(0), X)
        assert result is None

    def test_mismatched_inner_args_returns_none(self, mock_bridge) -> None:
        """x·exp(y) — linear parts don't match — → None."""
        product = IRApply(MUL, (X, _exp(Y)))
        result = _try_lambert(product, IRInteger(1), X)
        assert result is None

    def test_rhs_not_constant_returns_none(self, mock_bridge) -> None:
        """x·exp(x) = x — rhs is not constant w.r.t. var → None."""
        product = IRApply(MUL, (X, _exp(X)))
        result = _try_lambert(product, X, X)
        assert result is None


# ===========================================================================
# 10. _try_compound — polynomial-in-transcendental substitution
# ===========================================================================


class TestTryCompound:
    """_try_compound: substitute u = f(x) to reduce to a polynomial."""

    def test_degree1_sin_poly(self) -> None:
        """u = sin(x), degree-1 poly u = 0 → x = arcsin(0) family."""
        # Patch poly_coeffs to return (0, 1) → linear in u: u + 0 = 0 → u=0
        # Patch _extract_linear so the inner sin(x)=0 solve succeeds
        with patch.object(
            _mod, "_poly_coeffs", return_value=(Fraction(0), Fraction(1))
        ), patch.object(
            _mod,
            "_extract_linear",
            return_value=(Fraction(1), Fraction(0)),
        ):
            result = _try_compound(_sin(X), IRInteger(0), X)

        assert result is not None
        assert len(result) >= 1

    def test_degree2_quadratic_sin(self) -> None:
        """u² + u = 0 (u = sin(x)) → two sub-solutions {0, −1} → solutions."""
        sin_x = _sin(X)
        # ADD(MUL(sin_x, sin_x), sin_x) models sin²(x) + sin(x)
        lhs = IRApply(ADD, (IRApply(MUL, (sin_x, sin_x)), sin_x))

        # (0, 1, 1) means 0 + 1·u + 1·u² → u² + u = 0 → {0, −1}
        with patch.object(
            _mod, "_poly_coeffs", return_value=(Fraction(0), Fraction(1), Fraction(1))
        ), patch.object(
            _mod,
            "_extract_linear",
            return_value=(Fraction(1), Fraction(0)),
        ):
            result = _try_compound(lhs, IRInteger(0), X)

        assert result is not None
        assert len(result) >= 2

    def test_bare_var_returns_none(self) -> None:
        """Bare x can't be substituted → None."""
        result = _try_compound(X, IRInteger(0), X)
        assert result is None

    def test_poly_coeffs_none_falls_through_to_none(self) -> None:
        """When _poly_coeffs returns None for all heads, return None."""
        with patch.object(_mod, "_poly_coeffs", return_value=None):
            result = _try_compound(_sin(X), IRInteger(0), X)
        assert result is None


# ===========================================================================
# 11. _extract_linear — graceful fallback without bridge
# ===========================================================================


class TestExtractLinear:
    """_extract_linear returns None gracefully when the bridge is absent."""

    def test_returns_none_without_bridge(self) -> None:
        """Without symbolic_vm installed, _extract_linear always returns None."""
        sys.modules.pop("symbolic_vm", None)
        sys.modules.pop("symbolic_vm.polynomial_bridge", None)
        assert _extract_linear(X, X) is None


# ===========================================================================
# 12. _poly_coeffs — graceful fallback without bridge
# ===========================================================================


class TestPolyCoeffs:
    """_poly_coeffs returns None gracefully when the bridge is absent."""

    def test_returns_none_without_bridge(self) -> None:
        """Without symbolic_vm installed, _poly_coeffs always returns None."""
        sys.modules.pop("symbolic_vm", None)
        sys.modules.pop("symbolic_vm.polynomial_bridge", None)
        assert _poly_coeffs(_sin(X), X) is None


# ===========================================================================
# 13. try_solve_transcendental — public API
# ===========================================================================


class TestTrySolveTranscendental:
    """Integration tests for the public entry point."""

    def test_returns_none_without_bridge(self) -> None:
        """Without the polynomial bridge, all sub-dispatchers return None."""
        sys.modules.pop("symbolic_vm", None)
        sys.modules.pop("symbolic_vm.polynomial_bridge", None)
        result = try_solve_transcendental(_eq(_sin(X), IRInteger(0)), X)
        assert result is None

    def test_bare_expression_treated_as_eq_zero(self, mock_bridge) -> None:
        """A bare expression (no Equal wrapper) is treated as expr = 0."""
        result = try_solve_transcendental(_sin(X), X)
        assert result is not None
        assert len(result) == 2  # sin(x) = 0 has two periodic families

    def test_equal_node_dispatched(self, mock_bridge) -> None:
        """Equal(sin(x), 0) → solutions returned."""
        result = try_solve_transcendental(_eq(_sin(X), IRInteger(0)), X)
        assert result is not None

    def test_reversed_equal_also_solved(self, mock_bridge) -> None:
        """Equal(0, sin(x)) — reversed orientation — still returns solutions."""
        result = try_solve_transcendental(_eq(IRInteger(0), _sin(X)), X)
        assert result is not None

    def test_exp_eq_const(self, mock_bridge) -> None:
        """exp(x) = 2 → unique solution involving Log."""
        result = try_solve_transcendental(_eq(_exp(X), IRInteger(2)), X)
        assert result is not None
        assert len(result) == 1

    def test_lambert_w_dispatch(self, mock_bridge) -> None:
        """x·exp(x) = 1 → Lambert-W solution."""
        lhs = IRApply(MUL, (X, _exp(X)))
        result = try_solve_transcendental(_eq(lhs, IRInteger(1)), X)
        assert result is not None
        assert len(result) == 1
        sol = result[0]
        assert isinstance(sol, IRApply)
        assert sol.head is LAMBERT_W

    def test_unrecognised_equation_returns_none(self, mock_bridge) -> None:
        """sin(sin(x)) = 0 — nested composition not handled → None."""
        eq = _eq(IRApply(SIN, (_sin(X),)), IRInteger(0))
        result = try_solve_transcendental(eq, X)
        assert result is None

    def test_equation_free_of_var_returns_none(self, mock_bridge) -> None:
        """sin(y) = 0, solving for x — x never appears → None."""
        result = try_solve_transcendental(_eq(_sin(Y), IRInteger(0)), X)
        assert result is None
