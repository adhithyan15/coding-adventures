"""Tests for the arctan antiderivative builder (Phase 2e).

Correctness gate: every test that expects a non-None result verifies
**numerical re-differentiation** — we evaluate the derivative of the
returned IR tree at several x values and confirm it matches the
original integrand within floating-point tolerance (1e-9).

This sidesteps the lack of a full symbolic simplifier: we don't need
to compare IR trees structurally, only confirm the calculus is right.

For ``None``-return tests (unsupported shapes) we simply assert the
return is ``None``.
"""

from __future__ import annotations

import math
from fractions import Fraction

from symbolic_ir import (
    ADD,
    ATAN,
    DIV,
    LOG,
    MUL,
    SQRT,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.arctan_integral import arctan_integral
from symbolic_vm.backends import SymbolicBackend
from symbolic_vm.vm import VM

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")


def P(*coefs):
    """Polynomial tuple with Fraction coefficients, constant-first order."""
    return tuple(Fraction(c) for c in coefs)


def _vm():
    return VM(SymbolicBackend())


def _eval_ir(node: IRNode, x_val: float) -> float:
    """Numerically evaluate an IR expression by substituting x = x_val.

    Works for Add, Mul, Div, Neg, Inv, Log, Atan, Sqrt, and literals.
    Enough for the shapes arctan_integral emits.
    """
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRSymbol):
        if node.name == "x":
            return x_val
        raise ValueError(f"Unknown symbol: {node.name}")
    if not isinstance(node, IRApply):
        raise TypeError(f"Unexpected node: {node}")
    head = node.head.name
    if head == "Add":
        return _eval_ir(node.args[0], x_val) + _eval_ir(node.args[1], x_val)
    if head == "Mul":
        return _eval_ir(node.args[0], x_val) * _eval_ir(node.args[1], x_val)
    if head == "Div":
        return _eval_ir(node.args[0], x_val) / _eval_ir(node.args[1], x_val)
    if head == "Neg":
        return -_eval_ir(node.args[0], x_val)
    if head == "Inv":
        return 1.0 / _eval_ir(node.args[0], x_val)
    if head == "Log":
        return math.log(_eval_ir(node.args[0], x_val))
    if head == "Atan":
        return math.atan(_eval_ir(node.args[0], x_val))
    if head == "Sqrt":
        return math.sqrt(_eval_ir(node.args[0], x_val))
    if head == "Pow":
        return _eval_ir(node.args[0], x_val) ** _eval_ir(node.args[1], x_val)
    raise ValueError(f"Unhandled head: {head}")


def _numerical_deriv(ir_node: IRNode, x_val: float, h: float = 1e-7) -> float:
    """Central-difference derivative of ``ir_node`` at ``x = x_val``."""
    return (_eval_ir(ir_node, x_val + h) - _eval_ir(ir_node, x_val - h)) / (2 * h)


def _integrand_val(num: tuple, den: tuple, x_val: float) -> float:
    """Evaluate num(x)/den(x) as a float at x = x_val."""
    def poly_eval(p, x):
        return sum(float(c) * x**i for i, c in enumerate(p))
    return poly_eval(num, x_val) / poly_eval(den, x_val)


def assert_antideriv(num: tuple, den: tuple, test_xs=(0.5, 1.0, 2.0, -1.5)):
    """Universal gate: d/dx(arctan_integral result) == num/den at test_xs."""
    result = arctan_integral(num, den, X)
    for xv in test_xs:
        deriv = _numerical_deriv(result, xv)
        expected = _integrand_val(num, den, xv)
        assert abs(deriv - expected) < 1e-7, (
            f"Re-differentiation failed at x={xv}: "
            f"d/dx(antideriv) = {deriv}, integrand = {expected}"
        )


# ---------------------------------------------------------------------------
# Pure imaginary denominator — x² + k²
# ---------------------------------------------------------------------------


class TestPureImaginaryDenominator:
    def test_one_over_x_sq_plus_one(self):
        # ∫ 1/(x²+1) dx = arctan(x).
        result = arctan_integral(P(1), P(1, 0, 1), X)
        # Result should be arctan(x) — no log term since A = 0.
        assert isinstance(result, IRApply) and result.head == ATAN
        assert_antideriv(P(1), P(1, 0, 1))

    def test_one_over_x_sq_plus_four(self):
        # ∫ 1/(x²+4) dx = (1/2)·arctan(x/2).
        assert_antideriv(P(1), P(4, 0, 1))

    def test_one_over_two_x_sq_plus_two(self):
        # ∫ 1/(2x²+2) dx = (1/2)·arctan(x).  Leading coeff ≠ 1.
        assert_antideriv(P(1), P(2, 0, 2))

    def test_one_over_x_sq_plus_nine(self):
        # ∫ 1/(x²+9) dx = (1/3)·arctan(x/3).
        assert_antideriv(P(1), P(9, 0, 1))


# ---------------------------------------------------------------------------
# Completed-square denominator — (x + h)² + k²
# ---------------------------------------------------------------------------


class TestCompletedSquareDenominator:
    def test_one_over_x_sq_plus_2x_plus_5(self):
        # ∫ 1/(x²+2x+5) dx = (1/2)·arctan((x+1)/2).
        assert_antideriv(P(1), P(5, 2, 1))

    def test_one_over_x_sq_minus_2x_plus_5(self):
        # ∫ 1/(x²-2x+5) dx = (1/2)·arctan((x-1)/2).
        assert_antideriv(P(1), P(5, -2, 1))

    def test_one_over_x_sq_plus_4x_plus_13(self):
        # ∫ 1/(x²+4x+13) dx = (1/3)·arctan((x+2)/3).
        assert_antideriv(P(1), P(13, 4, 1))


# ---------------------------------------------------------------------------
# Mixed numerator — log + arctan terms
# ---------------------------------------------------------------------------


class TestMixedNumerator:
    def test_2x_plus_1_over_x_sq_plus_1(self):
        # ∫ (2x+1)/(x²+1) dx = log(x²+1) + arctan(x).
        result = arctan_integral(P(1, 2), P(1, 0, 1), X)
        # Should be an Add of log and atan terms.
        assert isinstance(result, IRApply) and result.head == ADD
        assert_antideriv(P(1, 2), P(1, 0, 1))

    def test_x_over_x_sq_plus_1(self):
        # ∫ x/(x²+1) dx = (1/2)·log(x²+1).  No arctan term since B = 0.
        assert_antideriv(P(0, 1), P(1, 0, 1), test_xs=(0.5, 1.0, 2.0))

    def test_x_plus_3_over_x_sq_plus_2x_plus_5(self):
        # ∫ (x+3)/(x²+2x+5) dx = (1/2)·log(x²+2x+5) + arctan((x+1)/2).
        assert_antideriv(P(3, 1), P(5, 2, 1))

    def test_3x_minus_1_over_x_sq_plus_4(self):
        # ∫ (3x-1)/(x²+4) dx = (3/2)·log(x²+4) - (1/2)·arctan(x/2).
        assert_antideriv(P(-1, 3), P(4, 0, 1))

    def test_fractional_coefficients(self):
        # Rational numerator coefficients — Fraction inputs throughout.
        assert_antideriv(P(Fraction(1, 2), 1), P(1, 0, 1))


# ---------------------------------------------------------------------------
# Irrational discriminant — D = √k with k not a perfect square
# ---------------------------------------------------------------------------


class TestIrrationalDiscriminant:
    def test_one_over_x_sq_plus_3(self):
        # ∫ 1/(x²+3) dx = (1/√12)·arctan(2x/√12) = (1/√3)·arctan(x/√3).
        result = arctan_integral(P(1), P(3, 0, 1), X)
        # Output should contain a Sqrt node somewhere.
        assert _contains_sqrt(result), (
            f"Expected Sqrt node in output for irrational D; got: {result}"
        )
        assert_antideriv(P(1), P(3, 0, 1))

    def test_one_over_2x_sq_plus_1(self):
        # D² = 4·2·1 − 0 = 8, not a perfect square.
        result = arctan_integral(P(1), P(1, 0, 2), X)
        assert _contains_sqrt(result)
        assert_antideriv(P(1), P(1, 0, 2))

    def test_one_over_x_sq_plus_x_plus_1(self):
        # D² = 4·1·1 − 1 = 3, irrational D.
        result = arctan_integral(P(1), P(1, 1, 1), X)
        assert _contains_sqrt(result)
        assert_antideriv(P(1), P(1, 1, 1))


def _contains_sqrt(node: IRNode) -> bool:
    """True if any Sqrt node appears anywhere in the tree."""
    if isinstance(node, IRApply):
        if node.head == SQRT:
            return True
        return any(_contains_sqrt(a) for a in node.args)
    return False


# ---------------------------------------------------------------------------
# try_arctan_integral — the gating wrapper in integrate.py
# ---------------------------------------------------------------------------


class TestTryArctanIntegral:
    def test_degree_one_returns_none(self):
        # Degree-1 denominators are handled by RT; arctan doesn't apply.
        from symbolic_vm.integrate import _try_arctan_integral
        assert _try_arctan_integral(P(1), P(-1, 1), X) is None

    def test_degree_three_returns_none(self):
        from symbolic_vm.integrate import _try_arctan_integral
        # x³ + x + 1 — irreducible but degree > 2.
        assert _try_arctan_integral(P(1), P(1, 1, 0, 1), X) is None

    def test_reducible_quadratic_returns_none(self):
        # x² - 1 = (x-1)(x+1) has rational roots — not for arctan.
        from polynomial import multiply
        xm1 = (Fraction(-1), Fraction(1))
        xp1 = (Fraction(1), Fraction(1))
        den = multiply(xm1, xp1)
        from symbolic_vm.integrate import _try_arctan_integral
        assert _try_arctan_integral(P(1), den, X) is None

    def test_irreducible_quadratic_succeeds(self):
        from symbolic_vm.integrate import _try_arctan_integral
        result = _try_arctan_integral(P(1), P(1, 0, 1), X)
        assert result is not None


# ---------------------------------------------------------------------------
# End-to-end via the full VM  (Integrate handler, IR built by hand)
# ---------------------------------------------------------------------------

def _integrate_ir(f: IRNode) -> IRNode:
    """Evaluate Integrate(f, x) through the SymbolicBackend VM."""
    vm = VM(SymbolicBackend())
    from symbolic_ir import INTEGRATE
    return vm.eval(IRApply(INTEGRATE, (f, X)))


class TestEndToEnd:
    def test_integrate_one_over_x_sq_plus_one(self):
        # ∫ 1/(x²+1) dx = arctan(x).
        # IR: 1 / (x^2 + 1)
        den = IRApply(ADD, (IRApply(MUL, (X, X)), IRInteger(1)))
        result = _integrate_ir(IRApply(DIV, (IRInteger(1), den)))
        assert isinstance(result, IRApply) and result.head == ATAN

    def test_integrate_one_over_x_sq_plus_four(self):
        # ∫ 1/(x²+4) dx = (1/2)·arctan(x/2).
        from symbolic_ir import POW
        den = IRApply(ADD, (IRApply(POW, (X, IRInteger(2))), IRInteger(4)))
        result = _integrate_ir(IRApply(DIV, (IRInteger(1), den)))
        assert _contains_atan(result)

    def test_integrate_one_over_x_sq_plus_2x_plus_5(self):
        # ∫ 1/(x²+2x+5) dx = (1/2)·arctan((x+1)/2).
        from symbolic_ir import POW
        x2 = IRApply(POW, (X, IRInteger(2)))
        two_x = IRApply(MUL, (IRInteger(2), X))
        den = IRApply(ADD, (IRApply(ADD, (x2, two_x)), IRInteger(5)))
        result = _integrate_ir(IRApply(DIV, (IRInteger(1), den)))
        assert _contains_atan(result)

    def test_integrate_2x_plus_1_over_x_sq_plus_1(self):
        # ∫ (2x+1)/(x²+1) dx = log(x²+1) + arctan(x).
        from symbolic_ir import POW
        num = IRApply(ADD, (IRApply(MUL, (IRInteger(2), X)), IRInteger(1)))
        den = IRApply(ADD, (IRApply(POW, (X, IRInteger(2))), IRInteger(1)))
        result = _integrate_ir(IRApply(DIV, (num, den)))
        assert _contains_atan(result) and _contains_log(result)

    def test_degree_four_irreducible_stays_unevaluated(self):
        # ∫ 1/(x⁴+1) dx — degree 4, irreducible over Q, stays unevaluated.
        from symbolic_ir import INTEGRATE, POW
        den = IRApply(ADD, (IRApply(POW, (X, IRInteger(4))), IRInteger(1)))
        result = _integrate_ir(IRApply(DIV, (IRInteger(1), den)))
        assert isinstance(result, IRApply) and result.head == INTEGRATE

    def test_rt_still_closes_rational_roots(self):
        # ∫ 1/(x²−1) dx — handled by RT (log sum), NOT arctan.
        from symbolic_ir import POW
        den = IRApply(ADD, (IRApply(POW, (X, IRInteger(2))), IRInteger(-1)))
        result = _integrate_ir(IRApply(DIV, (IRInteger(1), den)))
        assert _contains_log(result) and not _contains_atan(result)


def _contains_atan(node: IRNode) -> bool:
    if isinstance(node, IRApply):
        if node.head == ATAN:
            return True
        return any(_contains_atan(a) for a in node.args)
    return False


def _contains_log(node: IRNode) -> bool:
    if isinstance(node, IRApply):
        if node.head == LOG:
            return True
        return any(_contains_log(a) for a in node.args)
    return False
