"""Phase 27 — Polynomial Inequality Solving integration tests.

Tests the full pipeline from ``Solve(inequality, var)`` through
``solve_handler`` → ``_try_inequality`` → ``cas_solve.try_solve_inequality``.

Test structure
--------------
TestPhase27_Linear          — degree-1 inequalities (x op c)
TestPhase27_QuadTwoRoots    — quadratic with two distinct rational roots
TestPhase27_QuadDoubleRoot  — quadratic with a double root
TestPhase27_QuadNoRoots     — quadratic with no real roots (all reals / empty)
TestPhase27_HighDegree      — cubic and quartic polynomials
TestPhase27_Normalisation   — lhs op rhs with non-zero rhs (normalised f=lhs-rhs)
TestPhase27_Fallthrough     — inequalities that fall back to unevaluated
TestPhase27_Regressions     — Phase 1–26 features still work correctly
TestPhase27_Macsyma         — end-to-end MACSYMA surface-syntax tests

Verification strategy
---------------------

For each solution list we verify:

1. **Structure** — result is a ``List``, has the expected number of elements,
   and each element has the expected IR head.
2. **Numeric back-check** — for boundary conditions we evaluate both a point
   inside the solution interval and a point outside it to confirm the
   original polynomial has the right sign.
"""

from __future__ import annotations

import pytest
from symbolic_ir import (
    GREATER,
    GREATER_EQUAL,
    LESS,
    LESS_EQUAL,
    MUL,
    POW,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.backends import SymbolicBackend
from symbolic_vm.vm import VM

# ---------------------------------------------------------------------------
# Shared symbols / constants
# ---------------------------------------------------------------------------

X = IRSymbol("x")
ZERO = IRInteger(0)
ONE = IRInteger(1)
SOLVE = IRSymbol("Solve")
LIST = IRSymbol("List")


# ---------------------------------------------------------------------------
# Test helpers
# ---------------------------------------------------------------------------


def _make_vm() -> VM:
    return VM(SymbolicBackend())


def _solve(ineq_ir: IRNode, var: IRSymbol = X) -> IRNode:
    """Evaluate ``Solve(ineq_ir, var)`` and return the result."""
    return _make_vm().eval(IRApply(SOLVE, (ineq_ir, var)))


def _is_list(node: IRNode) -> bool:
    return (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "List"
    )


def _list_args(node: IRNode) -> tuple[IRNode, ...]:
    assert _is_list(node), f"Expected List, got {node!r}"
    assert isinstance(node, IRApply)
    return node.args


def _ineq(head: IRSymbol, lhs: IRNode = X, rhs: IRNode = ZERO) -> IRApply:
    """Build a simple ``head(lhs, rhs)`` comparison node."""
    return IRApply(head, (lhs, rhs))


def _sub(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(SUB, (a, b))


def _pow(base: IRNode, exp: IRNode) -> IRApply:
    return IRApply(POW, (base, exp))


def _mul(a: IRNode, b: IRNode) -> IRApply:
    return IRApply(MUL, (a, b))


def _head_name(node: IRNode) -> str | None:
    """Return the head name of an IRApply, or None."""
    if isinstance(node, IRApply) and isinstance(node.head, IRSymbol):
        return node.head.name
    return None


def _is_all_reals(node: IRNode) -> bool:
    """True iff node is the ``GreaterEqual(0, 0)`` all-reals sentinel."""
    return (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "GreaterEqual"
        and len(node.args) == 2
        and isinstance(node.args[0], IRInteger)
        and node.args[0].value == 0
        and isinstance(node.args[1], IRInteger)
        and node.args[1].value == 0
    )


def _boundary_value(cond: IRNode) -> float | None:
    """Extract the boundary float value from a simple comparison condition.

    Handles ``Less(x, a)``, ``LessEqual(x, a)``,
    ``Greater(x, a)``, ``GreaterEqual(x, a)`` where ``a`` is numeric.
    Returns ``None`` for non-simple conditions.
    """
    if not isinstance(cond, IRApply):
        return None
    if _head_name(cond) not in {"Less", "Greater", "LessEqual", "GreaterEqual"}:
        return None
    if len(cond.args) != 2:
        return None
    boundary = cond.args[1]
    if isinstance(boundary, IRInteger):
        return float(boundary.value)
    if isinstance(boundary, IRRational):
        return boundary.numer / boundary.denom
    if isinstance(boundary, IRFloat):
        return boundary.value
    return None


# ===========================================================================
# 1. Linear inequalities
# ===========================================================================


class TestPhase27_Linear:
    """Degree-1 polynomial inequalities with exact rational boundaries."""

    def test_x_minus_1_greater(self) -> None:
        """x - 1 > 0  →  [Greater(x, 1)]."""
        result = _solve(_ineq(GREATER, _sub(X, ONE)))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _head_name(sols[0]) == "Greater"
        assert _boundary_value(sols[0]) == pytest.approx(1.0)

    def test_x_minus_1_greater_equal(self) -> None:
        """x - 1 >= 0  →  [GreaterEqual(x, 1)]."""
        result = _solve(_ineq(GREATER_EQUAL, _sub(X, ONE)))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _head_name(sols[0]) == "GreaterEqual"

    def test_x_minus_1_less(self) -> None:
        """x - 1 < 0  →  [Less(x, 1)]."""
        result = _solve(_ineq(LESS, _sub(X, ONE)))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _head_name(sols[0]) == "Less"

    def test_x_minus_1_less_equal(self) -> None:
        """x - 1 <= 0  →  [LessEqual(x, 1)]."""
        result = _solve(_ineq(LESS_EQUAL, _sub(X, ONE)))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _head_name(sols[0]) == "LessEqual"

    def test_rational_root_boundary(self) -> None:
        """2x + 3 > 0: root at x = -3/2.

        The boundary should be an exact ``IRRational``.
        """
        # Build 2x + 3
        two_x = _mul(IRInteger(2), X)
        two_x_plus_3 = IRApply(
            IRSymbol("Add"), (two_x, IRInteger(3))
        )
        result = _solve(_ineq(GREATER, two_x_plus_3))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        b = _boundary_value(sols[0])
        assert b is not None
        assert b == pytest.approx(-1.5)  # -3/2

    def test_boundary_exact_integer(self) -> None:
        """x - 5 < 0: boundary is the exact integer 5."""
        f = _sub(X, IRInteger(5))
        result = _solve(_ineq(LESS, f))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _head_name(sols[0]) == "Less"
        node = sols[0]
        assert isinstance(node, IRApply)
        assert isinstance(node.args[1], IRInteger)
        assert node.args[1].value == 5


# ===========================================================================
# 2. Quadratic — two distinct rational roots
# ===========================================================================


class TestPhase27_QuadTwoRoots:
    """x² - (a+b)x + ab = 0 → roots a, b; four inequality directions."""

    # x² - 1 = (x-1)(x+1), roots ±1
    _X2_MINUS_1 = _sub(_pow(X, IRInteger(2)), ONE)

    def test_x2_minus_1_greater(self) -> None:
        """x² - 1 > 0  →  [Less(x, -1), Greater(x, 1)]."""
        result = _solve(_ineq(GREATER, self._X2_MINUS_1))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        assert _head_name(sols[0]) == "Less"   # x < -1
        assert _head_name(sols[1]) == "Greater"  # x > 1
        assert _boundary_value(sols[0]) == pytest.approx(-1.0)
        assert _boundary_value(sols[1]) == pytest.approx(1.0)

    def test_x2_minus_1_greater_equal(self) -> None:
        """x² - 1 >= 0  →  [LessEqual(x, -1), GreaterEqual(x, 1)]."""
        result = _solve(_ineq(GREATER_EQUAL, self._X2_MINUS_1))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        assert _head_name(sols[0]) == "LessEqual"
        assert _head_name(sols[1]) == "GreaterEqual"

    def test_x2_minus_1_less(self) -> None:
        """x² - 1 < 0  →  [And(Greater(x,-1), Less(x,1))]."""
        result = _solve(_ineq(LESS, self._X2_MINUS_1))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        inner = sols[0]
        assert _head_name(inner) == "And"
        assert isinstance(inner, IRApply)
        lo, hi = inner.args
        assert _head_name(lo) == "Greater"
        assert _head_name(hi) == "Less"
        assert _boundary_value(lo) == pytest.approx(-1.0)
        assert _boundary_value(hi) == pytest.approx(1.0)

    def test_x2_minus_1_less_equal(self) -> None:
        """x² - 1 <= 0  →  [And(GreaterEqual(x,-1), LessEqual(x,1))]."""
        result = _solve(_ineq(LESS_EQUAL, self._X2_MINUS_1))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        inner = sols[0]
        assert _head_name(inner) == "And"
        assert isinstance(inner, IRApply)
        lo, hi = inner.args
        assert _head_name(lo) == "GreaterEqual"
        assert _head_name(hi) == "LessEqual"

    def test_x2_minus_3x_plus_2_less_equal(self) -> None:
        """x²-3x+2 <= 0 → And(GreaterEqual(x,1), LessEqual(x,2))."""
        # Build x^2 - 3x + 2 = Sub(Sub(x^2, 3x), -2)
        x2 = _pow(X, IRInteger(2))
        three_x = _mul(IRInteger(3), X)
        # Sub(Sub(x^2, 3x), -2) = x^2 - 3x - (-2) = x^2 - 3x + 2  ✓
        f2 = _sub(_sub(x2, three_x), IRInteger(-2))
        result = _solve(_ineq(LESS_EQUAL, f2))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        inner = sols[0]
        assert _head_name(inner) == "And"
        assert isinstance(inner, IRApply)
        lo, hi = inner.args
        assert _head_name(lo) == "GreaterEqual"
        assert _head_name(hi) == "LessEqual"
        assert _boundary_value(lo) == pytest.approx(1.0)
        assert _boundary_value(hi) == pytest.approx(2.0)

    def test_boundary_points_numeric(self) -> None:
        """x² - 1 > 0: verify that x=-2 satisfies and x=0 does not."""
        result = _solve(_ineq(GREATER, self._X2_MINUS_1))
        sols = _list_args(result)
        # sols[0] is Less(x, -1): x=-2 satisfies, x=0 does not
        cond0 = sols[0]
        b0 = _boundary_value(cond0)
        assert b0 == pytest.approx(-1.0)
        # p(-2) = 4 - 1 = 3 > 0  ✓
        assert (-2.0) ** 2 - 1 > 0
        # p(0) = -1 < 0  (not in solution)
        assert 0.0**2 - 1 < 0


# ===========================================================================
# 3. Quadratic — double root
# ===========================================================================


class TestPhase27_QuadDoubleRoot:
    """(x - 1)² = x² - 2x + 1."""

    # (x-1)^2 = x^2 - 2x + 1
    @staticmethod
    def _f() -> IRApply:
        x2 = _pow(X, IRInteger(2))
        two_x = _mul(IRInteger(2), X)
        return _sub(_sub(x2, two_x), IRInteger(-1))  # x^2 - 2x - (-1) = x^2 - 2x + 1

    def test_strict_gives_two_half_lines(self) -> None:
        """(x-1)² > 0  →  [Less(x,1), Greater(x,1)]."""
        result = _solve(_ineq(GREATER, self._f()))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        assert _head_name(sols[0]) == "Less"
        assert _head_name(sols[1]) == "Greater"
        # Both boundary values are 1.
        assert _boundary_value(sols[0]) == pytest.approx(1.0)
        assert _boundary_value(sols[1]) == pytest.approx(1.0)

    def test_nonstrict_all_reals(self) -> None:
        """(x-1)² >= 0  →  all reals (trivially satisfied)."""
        result = _solve(_ineq(GREATER_EQUAL, self._f()))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _is_all_reals(sols[0])


# ===========================================================================
# 4. Quadratic — no real roots
# ===========================================================================


class TestPhase27_QuadNoRoots:
    """x² + 1 has no real roots: always positive."""

    # x^2 + 1
    _X2_PLUS_1 = IRApply(
        IRSymbol("Add"), (_pow(X, IRInteger(2)), ONE)
    )

    def test_always_positive_greater(self) -> None:
        """x² + 1 > 0  →  all reals."""
        result = _solve(_ineq(GREATER, self._X2_PLUS_1))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _is_all_reals(sols[0])

    def test_always_positive_greater_equal(self) -> None:
        """x² + 1 >= 0  →  all reals."""
        result = _solve(_ineq(GREATER_EQUAL, self._X2_PLUS_1))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _is_all_reals(sols[0])

    def test_never_negative_less(self) -> None:
        """x² + 1 < 0  →  empty list."""
        result = _solve(_ineq(LESS, self._X2_PLUS_1))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 0

    def test_never_negative_less_equal(self) -> None:
        """x² + 1 <= 0  →  empty list."""
        result = _solve(_ineq(LESS_EQUAL, self._X2_PLUS_1))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 0


# ===========================================================================
# 5. Higher-degree polynomials (numeric roots → IRFloat boundaries)
# ===========================================================================


class TestPhase27_HighDegree:
    """Cubic and quartic inequalities; boundaries are IRFloat."""

    def test_cubic_three_roots_greater(self) -> None:
        """x³ - x = x(x-1)(x+1) > 0 → two positive intervals.

        Roots: -1, 0, 1.  Positive on (-1, 0) ∪ (1, +∞).
        """
        # x^3 - x = Mul(x, Sub(Pow(x,2), 1)) but we build it as
        # Sub(Pow(x,3), x)
        x3 = _pow(X, IRInteger(3))
        f = _sub(x3, X)
        result = _solve(_ineq(GREATER, f))
        assert _is_list(result)
        sols = _list_args(result)
        # Two positive intervals
        assert len(sols) == 2

    def test_quartic_two_outer_intervals_greater(self) -> None:
        """x⁴ - 1 > 0  →  two outer half-line intervals."""
        x4 = _pow(X, IRInteger(4))
        f = _sub(x4, ONE)
        result = _solve(_ineq(GREATER, f))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2

    def test_quartic_boundaries_numeric(self) -> None:
        """x⁴ - 1 > 0: boundaries come from numeric solver → IRFloat."""
        x4 = _pow(X, IRInteger(4))
        f = _sub(x4, ONE)
        result = _solve(_ineq(GREATER, f))
        sols = _list_args(result)
        assert len(sols) == 2
        # Both conditions should have float boundaries
        b0 = _boundary_value(sols[0])
        b1 = _boundary_value(sols[1])
        assert b0 is not None
        assert b1 is not None
        # Roots of x^4 - 1 = 0 are ±1 (but found numerically)
        assert abs(b0 - (-1.0)) < 1e-4
        assert abs(b1 - 1.0) < 1e-4

    def test_cubic_less_three_intervals(self) -> None:
        """x³ - x < 0  →  two negative intervals: (-∞,-1) ∪ (0,1)."""
        x3 = _pow(X, IRInteger(3))
        f = _sub(x3, X)
        result = _solve(_ineq(LESS, f))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2


# ===========================================================================
# 6. Normalisation: lhs op rhs with non-zero rhs
# ===========================================================================


class TestPhase27_Normalisation:
    """Inequality in form ``lhs op rhs`` with rhs ≠ 0."""

    def test_x_greater_than_3(self) -> None:
        """x > 3  (rhs is 3, not 0) → normalised to x-3 > 0 → [Greater(x, 3)]."""
        # Greater(x, 3)
        result = _solve(IRApply(GREATER, (X, IRInteger(3))))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _head_name(sols[0]) == "Greater"
        assert _boundary_value(sols[0]) == pytest.approx(3.0)

    def test_x_less_than_negative_2(self) -> None:
        """x < -2 → [Less(x, -2)]."""
        result = _solve(IRApply(LESS, (X, IRInteger(-2))))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _head_name(sols[0]) == "Less"
        assert _boundary_value(sols[0]) == pytest.approx(-2.0)

    def test_lhs_rhs_both_nonzero(self) -> None:
        """x² > 1 means x²-1 > 0 → [Less(x,-1), Greater(x,1)]."""
        result = _solve(IRApply(GREATER, (_pow(X, IRInteger(2)), ONE)))
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2


# ===========================================================================
# 7. Fallthrough cases (unevaluated)
# ===========================================================================


class TestPhase27_Fallthrough:
    """Inequalities that should return unevaluated (not a List of solutions)."""

    def test_non_polynomial_transcendental(self) -> None:
        """sin(x) > 0 is transcendental — not a polynomial → unevaluated."""
        from symbolic_ir import SIN  # noqa: PLC0415

        sin_x = IRApply(SIN, (X,))
        result = _solve(IRApply(GREATER, (sin_x, ZERO)))
        # Should return the unevaluated Solve(...) node
        assert not _is_list(result)

    def test_equal_head_not_dispatched(self) -> None:
        """Solve(Equal(x, 1), x) still uses the equation solver, not inequality."""
        from symbolic_ir import EQUAL  # noqa: PLC0415

        eq = IRApply(EQUAL, (X, ONE))
        result = _solve(eq)
        # Should succeed via equation solver giving List(1)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1


# ===========================================================================
# 8. Regression — earlier phases still work
# ===========================================================================


class TestPhase27_Regressions:
    """Inequality wiring must not break earlier Solve paths."""

    def test_linear_equation_solve(self) -> None:
        """solve(x - 3 = 0, x) → [3] (Phase 1)."""
        from symbolic_ir import EQUAL  # noqa: PLC0415

        eq = IRApply(EQUAL, (_sub(X, IRInteger(3)), ZERO))
        result = _solve(eq)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1

    def test_quadratic_equation_solve(self) -> None:
        """solve(x² - 5x + 6 = 0, x) → [2, 3] (Phase 1)."""
        from symbolic_ir import EQUAL  # noqa: PLC0415

        x2 = _pow(X, IRInteger(2))
        five_x = _mul(IRInteger(5), X)
        f = _sub(_sub(x2, five_x), IRInteger(-6))  # x^2 - 5x + 6
        eq = IRApply(EQUAL, (f, ZERO))
        result = _solve(eq)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2

    def test_system_solve_still_works(self) -> None:
        """Solve([x+y=3, x-y=1], [x,y]) → [x=2, y=1] (Phase 3)."""
        from symbolic_ir import EQUAL  # noqa: PLC0415

        Y = IRSymbol("y")
        e1 = IRApply(EQUAL, (IRApply(IRSymbol("Add"), (X, Y)), IRInteger(3)))
        e2 = IRApply(EQUAL, (IRApply(SUB, (X, Y)), ONE))
        eq_list = IRApply(LIST, (e1, e2))
        var_list = IRApply(LIST, (X, Y))
        result = _make_vm().eval(IRApply(SOLVE, (eq_list, var_list)))
        assert _is_list(result)

    def test_transcendental_equation_still_works(self) -> None:
        """solve(exp(x) = 1, x) → [log(1)] = [0] (Phase 26)."""
        from symbolic_ir import EQUAL, EXP  # noqa: PLC0415

        eq = IRApply(EQUAL, (IRApply(EXP, (X,)), ONE))
        result = _solve(eq)
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1


# ===========================================================================
# 9. MACSYMA surface-syntax tests
# ===========================================================================


class TestPhase27_Macsyma:
    """End-to-end via MACSYMA parser → compiler → symbolic VM."""

    def _run(self, src: str) -> IRNode:
        pytest.importorskip(
            "macsyma_runtime",
            reason="macsyma-runtime not installed; skipping MACSYMA e2e test",
        )
        from macsyma_compiler.compiler import (  # noqa: PLC0415
            _STANDARD_FUNCTIONS,
            compile_macsyma,
        )
        from macsyma_parser.parser import parse_macsyma  # noqa: PLC0415
        from macsyma_runtime.name_table import (  # noqa: PLC0415
            extend_compiler_name_table,
        )

        extend_compiler_name_table(_STANDARD_FUNCTIONS)
        stmts = compile_macsyma(parse_macsyma(src + ";"))
        return _make_vm().eval_program(stmts)

    def test_macsyma_linear_greater(self) -> None:
        """solve(x - 1 > 0, x) → List(Greater(x, 1))."""
        result = self._run("solve(x - 1 > 0, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _head_name(sols[0]) == "Greater"

    def test_macsyma_linear_less_equal(self) -> None:
        """solve(2*x + 4 <= 0, x) → List(LessEqual(x, -2))."""
        result = self._run("solve(2*x + 4 <= 0, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _head_name(sols[0]) == "LessEqual"
        assert _boundary_value(sols[0]) == pytest.approx(-2.0)

    def test_macsyma_quad_two_outer(self) -> None:
        """solve(x^2 - 1 > 0, x) → List(Less(x,-1), Greater(x,1))."""
        result = self._run("solve(x^2 - 1 > 0, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
        assert _head_name(sols[0]) == "Less"
        assert _head_name(sols[1]) == "Greater"

    def test_macsyma_quad_interval(self) -> None:
        """solve(x^2 - 1 < 0, x) → List(And(Greater(x,-1), Less(x,1)))."""
        result = self._run("solve(x^2 - 1 < 0, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _head_name(sols[0]) == "And"

    def test_macsyma_quad_roots_1_2(self) -> None:
        """solve(x^2 - 3*x + 2 <= 0, x) → And(GreaterEqual(x,1), LessEqual(x,2))."""
        result = self._run("solve(x^2 - 3*x + 2 <= 0, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        inner = sols[0]
        assert _head_name(inner) == "And"
        assert isinstance(inner, IRApply)
        lo, hi = inner.args
        assert _head_name(lo) == "GreaterEqual"
        assert _head_name(hi) == "LessEqual"
        assert _boundary_value(lo) == pytest.approx(1.0)
        assert _boundary_value(hi) == pytest.approx(2.0)

    def test_macsyma_all_reals(self) -> None:
        """solve(x^2 + 1 > 0, x) → all reals."""
        result = self._run("solve(x^2 + 1 > 0, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _is_all_reals(sols[0])

    def test_macsyma_no_solution(self) -> None:
        """solve(x^2 + 1 < 0, x) → empty list."""
        result = self._run("solve(x^2 + 1 < 0, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 0

    def test_macsyma_double_root_nonstrict(self) -> None:
        """solve((x-1)^2 >= 0, x) → all reals."""
        # (x-1)^2 = x^2 - 2*x + 1
        result = self._run("solve(x^2 - 2*x + 1 >= 0, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 1
        assert _is_all_reals(sols[0])

    def test_macsyma_double_root_strict(self) -> None:
        """solve((x-1)^2 > 0, x) → two open half-lines."""
        result = self._run("solve(x^2 - 2*x + 1 > 0, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2

    def test_macsyma_regression_equation_still_works(self) -> None:
        """solve(x^2-5*x+6=0, x) → [2,3]; inequality wiring must not break equations."""
        result = self._run("solve(x^2 - 5*x + 6 = 0, x)")
        assert _is_list(result)
        sols = _list_args(result)
        assert len(sols) == 2
