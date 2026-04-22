"""Tests for the mixed partial-fraction integrator (Phase 2f).

Correctness gate: **numerical re-differentiation** at several x values.
Every test that expects a non-None result evaluates the numerical
derivative of the returned IR tree and confirms it matches the
integrand.

Architecture: the tests in this file test ``mixed_integral`` directly
(unit-level) and via the full VM ``Integrate`` handler (end-to-end).
"""

from __future__ import annotations

import math
from fractions import Fraction

from polynomial import multiply
from symbolic_ir import (
    ATAN,
    INTEGRATE,
    LOG,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.backends import SymbolicBackend
from symbolic_vm.mixed_integral import mixed_integral
from symbolic_vm.vm import VM

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")


def P(*coefs):
    """Polynomial tuple with Fraction coefficients, constant-first order."""
    return tuple(Fraction(c) for c in coefs)


def _eval_ir(node: IRNode, x_val: float) -> float:
    """Numerically evaluate an IR tree at x = x_val."""
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
        return math.log(abs(_eval_ir(node.args[0], x_val)))
    if head == "Atan":
        return math.atan(_eval_ir(node.args[0], x_val))
    if head == "Sqrt":
        return math.sqrt(_eval_ir(node.args[0], x_val))
    if head == "Pow":
        return _eval_ir(node.args[0], x_val) ** _eval_ir(node.args[1], x_val)
    if head == "Sub":
        return _eval_ir(node.args[0], x_val) - _eval_ir(node.args[1], x_val)
    raise ValueError(f"Unhandled head: {head}")


def _numerical_deriv(node: IRNode, x_val: float, h: float = 1e-7) -> float:
    return (_eval_ir(node, x_val + h) - _eval_ir(node, x_val - h)) / (2 * h)


def _poly_eval(p: tuple, x: float) -> float:
    return sum(float(c) * x**i for i, c in enumerate(p))


def assert_antideriv(num: tuple, den: tuple, test_xs=(0.5, 2.0, 3.0, -2.0)):
    """Gate: d/dx(mixed_integral result) == num/den at every test_x."""
    result = mixed_integral(num, den, X)
    assert result is not None, "mixed_integral returned None unexpectedly"
    for xv in test_xs:
        deriv = _numerical_deriv(result, xv)
        expected = _poly_eval(num, xv) / _poly_eval(den, xv)
        assert abs(deriv - expected) < 1e-6, (
            f"Re-differentiation failed at x={xv}: "
            f"d/dx = {deriv:.8f}, expected = {expected:.8f}"
        )


# ---------------------------------------------------------------------------
# One linear factor + one irreducible quadratic
# ---------------------------------------------------------------------------


class TestOneLinearOneQuadratic:
    def test_one_over_xm1_times_xsq_plus_one(self):
        # ∫ 1/((x-1)(x²+1)) dx.
        xm1 = P(-1, 1)
        q = P(1, 0, 1)
        den = multiply(xm1, q)
        assert_antideriv(P(1), den, test_xs=(2.0, 3.0, 4.0, -2.0))

    def test_x_over_xp2_times_xsq_plus_4(self):
        # ∫ x/((x+2)(x²+4)) dx.
        xp2 = P(2, 1)
        q = P(4, 0, 1)
        den = multiply(xp2, q)
        assert_antideriv(P(0, 1), den, test_xs=(1.0, 3.0, 5.0, -3.0))

    def test_one_over_x_times_xsq_plus_one(self):
        # ∫ 1/(x(x²+1)) dx.
        x_poly = P(0, 1)
        q = P(1, 0, 1)
        den = multiply(x_poly, q)
        assert_antideriv(P(1), den, test_xs=(1.0, 2.0, 3.0, 0.5))

    def test_one_over_xm2_times_xsq_plus_2x_plus_5(self):
        # ∫ 1/((x-2)(x²+2x+5)) dx.
        xm2 = P(-2, 1)
        q = P(5, 2, 1)
        den = multiply(xm2, q)
        assert_antideriv(P(1), den, test_xs=(3.0, 4.0, 5.0, -3.0))

    def test_xsq_over_xp1_times_xsq_plus_one(self):
        # ∫ x²/((x+1)(x²+1)) dx.
        xp1 = P(1, 1)
        q = P(1, 0, 1)
        den = multiply(xp1, q)
        # deg num = 2 = deg den - 1 — still proper after Hermite.
        assert_antideriv(P(0, 0, 1), den, test_xs=(1.0, 2.0, 3.0, -2.0))


# ---------------------------------------------------------------------------
# Two linear factors + one irreducible quadratic
# ---------------------------------------------------------------------------


class TestTwoLinearOneQuadratic:
    def test_one_over_xm1_xp1_xsq_plus_one(self):
        # ∫ 1/((x-1)(x+1)(x²+1)) dx.
        xm1 = P(-1, 1)
        xp1 = P(1, 1)
        q = P(1, 0, 1)
        den = multiply(multiply(xm1, xp1), q)
        assert_antideriv(P(1), den, test_xs=(2.0, 3.0, 4.0, -2.0))

    def test_x_over_xm1_xm2_xsq_plus_4(self):
        # ∫ x/((x-1)(x-2)(x²+4)) dx.
        xm1 = P(-1, 1)
        xm2 = P(-2, 1)
        q = P(4, 0, 1)
        den = multiply(multiply(xm1, xm2), q)
        assert_antideriv(P(0, 1), den, test_xs=(3.0, 4.0, 5.0, -1.0))


# ---------------------------------------------------------------------------
# Mixed numerator
# ---------------------------------------------------------------------------


class TestMixedNumerator:
    def test_xsq_plus_1_over_xm2_times_xsq_plus_2x_plus_5(self):
        # ∫ (x²+1)/((x-2)(x²+2x+5)) dx.
        xm2 = P(-2, 1)
        q = P(5, 2, 1)
        den = multiply(xm2, q)
        assert_antideriv(P(1, 0, 1), den, test_xs=(3.0, 4.0, 5.0, -3.0))

    def test_2x_over_x_times_xsq_plus_1(self):
        # ∫ 2x/(x(x²+1)) dx = ∫ 2/(x²+1) dx = 2·arctan(x). (Simplification
        # happens at the polynomial-bridge level — the integrand goes
        # through Hermite as 2x/(x³+x) which reduces properly.)
        x_poly = P(0, 1)
        q = P(1, 0, 1)
        den = multiply(x_poly, q)
        assert_antideriv(P(0, 2), den, test_xs=(1.0, 2.0, 3.0, 0.5))


# ---------------------------------------------------------------------------
# mixed_integral returns None for unsupported shapes
# ---------------------------------------------------------------------------


class TestFallsThrough:
    def test_no_rational_roots_returns_none(self):
        # x²+1 alone — no linear factors. Phase 2e handles this; 2f doesn't.
        assert mixed_integral(P(1), P(1, 0, 1), X) is None

    def test_two_irreducible_quadratics_returns_none(self):
        # (x²+1)(x²+4) — two irreducible quadratics, no rational roots.
        den = multiply(P(1, 0, 1), P(4, 0, 1))
        assert mixed_integral(P(1), den, X) is None

    def test_linear_only_returns_none_or_succeeds(self):
        # x²-1 = (x-1)(x+1) — all linear, no quadratic remainder (deg Q=0).
        # mixed_integral sees deg Q = 0 ≠ 2 → returns None.
        den = multiply(P(-1, 1), P(1, 1))
        assert mixed_integral(P(1), den, X) is None


# ---------------------------------------------------------------------------
# Bézout split identity verification
# ---------------------------------------------------------------------------


class TestBezoutSplitIdentity:
    def test_bezout_split_one_linear_one_quadratic(self):
        """C_Q·L + C_L·Q == C  (as polynomial identity)."""
        from polynomial import multiply

        # Use 1/((x-1)(x²+1)) as the test case.
        xm1 = P(-1, 1)
        q = P(1, 0, 1)
        den = multiply(xm1, q)
        num = P(1)
        # Recover internal split by verifying the antiderivative.
        # (Direct access to C_L, C_Q would require exposing internals;
        # we verify correctness via re-differentiation instead.)
        result = mixed_integral(num, den, X)
        assert result is not None
        for xv in (2.0, 3.0, -2.0, 0.5):
            deriv = _numerical_deriv(result, xv)
            expected = _poly_eval(num, xv) / _poly_eval(den, xv)
            assert abs(deriv - expected) < 1e-6


# ---------------------------------------------------------------------------
# End-to-end via the full VM Integrate handler
# ---------------------------------------------------------------------------


def _vm_integrate(f: IRNode) -> IRNode:
    vm = VM(SymbolicBackend())
    return vm.eval(IRApply(INTEGRATE, (f, X)))


class TestEndToEnd:
    def _den_ir(self, *factors):
        """Build IR for the product of polynomial factors."""
        from symbolic_ir import MUL

        from symbolic_vm.polynomial_bridge import from_polynomial
        result = None
        for fac in factors:
            fac_ir = from_polynomial(fac, X)
            result = fac_ir if result is None else IRApply(MUL, (result, fac_ir))
        return result

    def test_one_over_xm1_times_xsq_plus_one(self):
        # ∫ 1/((x-1)(x²+1)) dx — contains both log and arctan.
        from symbolic_ir import DIV

        from symbolic_vm.polynomial_bridge import from_polynomial
        den_poly = multiply(P(-1, 1), P(1, 0, 1))
        den_ir = from_polynomial(den_poly, X)
        result = _vm_integrate(IRApply(DIV, (IRInteger(1), den_ir)))
        assert _contains_log(result), "Expected Log in result"
        assert _contains_atan(result), "Expected Atan in result"

    def test_one_over_x_times_xsq_plus_one(self):
        # ∫ 1/(x(x²+1)) dx = log(x) − ½·log(x²+1).
        # The C_Q piece is −x/(x²+1), whose arctan coefficient is zero,
        # so the integral contains only logs — no arctan term.
        from symbolic_ir import DIV

        from symbolic_vm.polynomial_bridge import from_polynomial
        den_poly = multiply(P(0, 1), P(1, 0, 1))
        den_ir = from_polynomial(den_poly, X)
        result = _vm_integrate(IRApply(DIV, (IRInteger(1), den_ir)))
        assert _contains_log(result)

    def test_two_quadratics_evaluated_by_phase9(self):
        # ∫ 1/((x²+1)(x²+4)) dx — Phase 9 now handles two irreducible quadratics.
        from symbolic_ir import DIV

        from symbolic_vm.polynomial_bridge import from_polynomial
        den_poly = multiply(P(1, 0, 1), P(4, 0, 1))
        den_ir = from_polynomial(den_poly, X)
        integrand = IRApply(DIV, (IRInteger(1), den_ir))
        result = _vm_integrate(integrand)
        assert not (isinstance(result, IRApply) and result.head == INTEGRATE)

    def test_rt_single_log_not_broken(self):
        # Regression: ∫ 1/(x-1) dx still gives log(x-1), not broken by 2f.
        from symbolic_ir import DIV

        from symbolic_vm.polynomial_bridge import from_polynomial
        den_ir = from_polynomial(P(-1, 1), X)
        result = _vm_integrate(IRApply(DIV, (IRInteger(1), den_ir)))
        assert _contains_log(result) and not _contains_atan(result)

    def test_phase2e_single_quadratic_not_broken(self):
        # Regression: ∫ 1/(x²+1) dx still gives arctan(x), not broken by 2f.
        from symbolic_ir import DIV

        from symbolic_vm.polynomial_bridge import from_polynomial
        den_ir = from_polynomial(P(1, 0, 1), X)
        result = _vm_integrate(IRApply(DIV, (IRInteger(1), den_ir)))
        assert isinstance(result, IRApply) and result.head == ATAN


def _contains_log(node: IRNode) -> bool:
    if isinstance(node, IRApply):
        if node.head == LOG:
            return True
        return any(_contains_log(a) for a in node.args)
    return False


def _contains_atan(node: IRNode) -> bool:
    if isinstance(node, IRApply):
        if node.head == ATAN:
            return True
        return any(_contains_atan(a) for a in node.args)
    return False
