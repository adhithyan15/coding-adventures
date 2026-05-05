"""Tests for Phase 3 transcendental integration.

Correctness gate: **numerical re-differentiation** at several x values.
Every test that expects a closed-form result differentiates the returned
IR tree numerically and confirms it matches the integrand.

Architecture:
- ``TestExpIntegral``       — unit tests for exp_integral directly.
- ``TestLogIntegral``       — unit tests for log_poly_integral directly.
- ``TestLinearArgTrig``     — sin/cos with linear arguments via VM.
- ``TestFallsThrough``      — expressions that should stay unevaluated.
- ``TestEndToEnd``          — full VM Integrate handler, various shapes.
- ``TestRegressions``       — Phase 1 cases unchanged by Phase 3.
"""

from __future__ import annotations

import math
from fractions import Fraction

from symbolic_ir import (
    ATAN,
    COS,
    EXP,
    INTEGRATE,
    LOG,
    SIN,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.backends import SymbolicBackend
from symbolic_vm.exp_integral import exp_integral
from symbolic_vm.log_integral import log_poly_integral
from symbolic_vm.vm import VM

# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")


def P(*coefs: int | float) -> tuple:
    """Polynomial tuple with Fraction coefficients, constant-first."""
    return tuple(Fraction(c) for c in coefs)


def _eval_ir(node: IRNode, x_val: float) -> float:  # noqa: PLR0911
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
        raise TypeError(f"Unexpected node: {node!r}")
    head = node.head.name
    if head == "Add":
        return _eval_ir(node.args[0], x_val) + _eval_ir(node.args[1], x_val)
    if head == "Sub":
        return _eval_ir(node.args[0], x_val) - _eval_ir(node.args[1], x_val)
    if head == "Mul":
        return _eval_ir(node.args[0], x_val) * _eval_ir(node.args[1], x_val)
    if head == "Div":
        return _eval_ir(node.args[0], x_val) / _eval_ir(node.args[1], x_val)
    if head == "Neg":
        return -_eval_ir(node.args[0], x_val)
    if head == "Inv":
        return 1.0 / _eval_ir(node.args[0], x_val)
    if head == "Exp":
        return math.exp(_eval_ir(node.args[0], x_val))
    if head == "Log":
        return math.log(abs(_eval_ir(node.args[0], x_val)))
    if head == "Sin":
        return math.sin(_eval_ir(node.args[0], x_val))
    if head == "Cos":
        return math.cos(_eval_ir(node.args[0], x_val))
    if head == "Pow":
        return _eval_ir(node.args[0], x_val) ** _eval_ir(node.args[1], x_val)
    if head == "Sqrt":
        return math.sqrt(_eval_ir(node.args[0], x_val))
    raise ValueError(f"Unhandled head: {head}")


def _numerical_deriv(node: IRNode, x_val: float, h: float = 1e-7) -> float:
    return (_eval_ir(node, x_val + h) - _eval_ir(node, x_val - h)) / (2 * h)


def _integrand_eval(f: IRNode, x_val: float) -> float:
    return _eval_ir(f, x_val)


def assert_antideriv(
    integrand: IRNode,
    antideriv: IRNode,
    test_xs: tuple = (0.5, 1.0, 2.0, 3.0),
    rtol: float = 1e-5,
    atol: float = 1e-5,
) -> None:
    """Assert d/dx[antideriv] == integrand at every test x.

    Uses a combined absolute + relative tolerance so that tests at large
    x values (where exp(a·x) is big) don't fail on rounding.
    """
    for xv in test_xs:
        d = _numerical_deriv(antideriv, xv)
        expected = _integrand_eval(integrand, xv)
        tol = atol + rtol * abs(expected)
        assert abs(d - expected) < tol, (
            f"Re-differentiation failed at x={xv}: "
            f"d/dx = {d:.8f}, expected = {expected:.8f}"
        )


def _vm_integrate(f: IRNode) -> IRNode:
    vm = VM(SymbolicBackend())
    return vm.eval(IRApply(INTEGRATE, (f, X)))


def _mul(a: IRNode, b: IRNode) -> IRNode:
    from symbolic_ir import MUL
    return IRApply(MUL, (a, b))


def _exp(arg: IRNode) -> IRNode:
    return IRApply(EXP, (arg,))


def _sin(arg: IRNode) -> IRNode:
    return IRApply(SIN, (arg,))


def _cos(arg: IRNode) -> IRNode:
    return IRApply(COS, (arg,))


def _log(arg: IRNode) -> IRNode:
    return IRApply(LOG, (arg,))


def _add(a: IRNode, b: IRNode) -> IRNode:
    from symbolic_ir import ADD
    return IRApply(ADD, (a, b))


# ---------------------------------------------------------------------------
# Unit tests — exp_integral
# ---------------------------------------------------------------------------


class TestExpIntegral:
    def test_exp_x(self):
        # ∫ eˣ dx = eˣ  (a=1, b=0, p=(1,))
        result = exp_integral(P(1), Fraction(1), Fraction(0), X)
        assert_antideriv(_exp(X), result)

    def test_exp_2x(self):
        # ∫ e^(2x) dx = e^(2x)/2
        a = Fraction(2)
        arg = _mul(IRInteger(2), X)
        result = exp_integral(P(1), a, Fraction(0), X)
        assert_antideriv(_exp(arg), result)

    def test_exp_3x_plus_1(self):
        # ∫ e^(3x+1) dx = e^(3x+1)/3
        from symbolic_ir import ADD
        arg = IRApply(ADD, (_mul(IRInteger(3), X), IRInteger(1)))
        result = exp_integral(P(1), Fraction(3), Fraction(1), X)
        assert_antideriv(_exp(arg), result)

    def test_exp_neg_x(self):
        # ∫ e^(-x) dx = -e^(-x)
        from symbolic_ir import NEG
        arg = IRApply(NEG, (X,))
        result = exp_integral(P(1), Fraction(-1), Fraction(0), X)
        assert_antideriv(_exp(arg), result)

    def test_x_times_exp_x(self):
        # ∫ x·eˣ dx = (x-1)·eˣ
        integrand = _mul(X, _exp(X))
        result = exp_integral(P(0, 1), Fraction(1), Fraction(0), X)
        assert_antideriv(integrand, result)

    def test_xsq_times_exp_x(self):
        # ∫ x²·eˣ dx = (x²-2x+2)·eˣ
        from symbolic_ir import POW
        xsq = IRApply(POW, (X, IRInteger(2)))
        integrand = _mul(xsq, _exp(X))
        result = exp_integral(P(0, 0, 1), Fraction(1), Fraction(0), X)
        assert_antideriv(integrand, result)

    def test_poly_times_exp_2x(self):
        # ∫ (x²+2x+3)·e^(2x) dx
        from symbolic_ir import POW
        xsq = IRApply(POW, (X, IRInteger(2)))
        poly_ir = _add(_add(xsq, _mul(IRInteger(2), X)), IRInteger(3))
        arg2x = _mul(IRInteger(2), X)
        integrand = _mul(poly_ir, _exp(arg2x))
        result = exp_integral(P(3, 2, 1), Fraction(2), Fraction(0), X)
        assert_antideriv(integrand, result)

    def test_poly_times_exp_neg_x(self):
        # ∫ (x²+2x+3)·e^(-x) dx
        from symbolic_ir import NEG, POW
        xsq = IRApply(POW, (X, IRInteger(2)))
        poly_ir = _add(_add(xsq, _mul(IRInteger(2), X)), IRInteger(3))
        neg_x = IRApply(NEG, (X,))
        integrand = _mul(poly_ir, _exp(neg_x))
        result = exp_integral(P(3, 2, 1), Fraction(-1), Fraction(0), X)
        assert_antideriv(integrand, result)


# ---------------------------------------------------------------------------
# Unit tests — log_poly_integral
# ---------------------------------------------------------------------------


class TestLogIntegral:
    def test_log_x(self):
        # ∫ log(x) dx = x·log(x) - x  (regression: matches Phase 1 result)
        result = log_poly_integral(P(1), Fraction(1), Fraction(0), X)
        assert_antideriv(_log(X), result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_log_2x_plus_1(self):
        # ∫ log(2x+1) dx = (x + 1/2)·log(2x+1) - x
        from symbolic_ir import ADD
        arg = IRApply(ADD, (_mul(IRInteger(2), X), IRInteger(1)))
        result = log_poly_integral(P(1), Fraction(2), Fraction(1), X)
        assert_antideriv(_log(arg), result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_x_times_log_x(self):
        # ∫ x·log(x) dx = (x²/2)·log(x) - x²/4
        integrand = _mul(X, _log(X))
        result = log_poly_integral(P(0, 1), Fraction(1), Fraction(0), X)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_xsq_times_log_x(self):
        # ∫ x²·log(x) dx = (x³/3)·log(x) - x³/9
        from symbolic_ir import POW
        xsq = IRApply(POW, (X, IRInteger(2)))
        integrand = _mul(xsq, _log(X))
        result = log_poly_integral(P(0, 0, 1), Fraction(1), Fraction(0), X)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_x_times_log_2x_plus_1(self):
        # ∫ x·log(2x+1) dx
        from symbolic_ir import ADD
        arg = IRApply(ADD, (_mul(IRInteger(2), X), IRInteger(1)))
        integrand = _mul(X, _log(arg))
        result = log_poly_integral(P(0, 1), Fraction(2), Fraction(1), X)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_const_times_log_x(self):
        # ∫ 3·log(x) dx = 3·(x·log(x) - x)
        integrand = _mul(IRInteger(3), _log(X))
        result = _vm_integrate(integrand)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))


# ---------------------------------------------------------------------------
# Linear-argument trig via the VM
# ---------------------------------------------------------------------------


class TestLinearArgTrig:
    def test_sin_2x(self):
        # ∫ sin(2x) dx = -cos(2x)/2
        arg = _mul(IRInteger(2), X)
        integrand = _sin(arg)
        result = _vm_integrate(integrand)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_cos_3x(self):
        # ∫ cos(3x) dx = sin(3x)/3
        arg = _mul(IRInteger(3), X)
        integrand = _cos(arg)
        result = _vm_integrate(integrand)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_sin_x_plus_1(self):
        # ∫ sin(x+1) dx = -cos(x+1)
        from symbolic_ir import ADD
        arg = IRApply(ADD, (X, IRInteger(1)))
        integrand = _sin(arg)
        result = _vm_integrate(integrand)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_cos_half_x(self):
        # ∫ cos(x/2) dx = 2·sin(x/2)
        arg = _mul(IRRational(1, 2), X)
        integrand = _cos(arg)
        result = _vm_integrate(integrand)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_sin_neg_x(self):
        # ∫ sin(-x) dx = cos(-x) = cos(x)
        from symbolic_ir import NEG
        arg = IRApply(NEG, (X,))
        integrand = _sin(arg)
        result = _vm_integrate(integrand)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))


# ---------------------------------------------------------------------------
# Fall-through: integrands that should remain unevaluated
# ---------------------------------------------------------------------------


class TestFallsThrough:
    def test_rational_times_exp_stays_unevaluated(self):
        # ∫ (1/x)·eˣ dx — rational × exp, not polynomial × exp.
        from symbolic_ir import DIV
        integrand = _mul(IRApply(DIV, (IRInteger(1), X)), _exp(X))
        result = _vm_integrate(integrand)
        assert isinstance(result, IRApply) and result.head == INTEGRATE

    def test_exp_xsq_evaluates_via_phase23(self):
        # ∫ exp(x²) dx — Phase 23 now evaluates this as (√π/2)·erfi(x).
        # Previously unevaluated; updated when Phase 23 special functions landed.
        from symbolic_ir import POW
        integrand = _exp(IRApply(POW, (X, IRInteger(2))))
        result = _vm_integrate(integrand)
        # Must not stay unevaluated — Phase 23 provides the Erfi fallback.
        assert not (isinstance(result, IRApply) and result.head == INTEGRATE), (
            "Expected Phase 23 to evaluate ∫ exp(x²) dx via erfi, got unevaluated"
        )
        assert "Erfi" in repr(result), f"Expected Erfi in result, got {result!r}"

    def test_exp_times_log_stays_unevaluated(self):
        # ∫ exp(x)·log(x) dx — two transcendentals, no rule.
        integrand = _mul(_exp(X), _log(X))
        result = _vm_integrate(integrand)
        assert isinstance(result, IRApply) and result.head == INTEGRATE


# ---------------------------------------------------------------------------
# End-to-end via the full VM
# ---------------------------------------------------------------------------


class TestEndToEnd:
    def test_xp1_times_exp_x(self):
        # ∫ (x+1)·eˣ dx — both factors depend on x.
        from symbolic_ir import ADD
        integrand = _mul(IRApply(ADD, (X, IRInteger(1))), _exp(X))
        result = _vm_integrate(integrand)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_xsq_times_log_x(self):
        # ∫ x²·log(x) dx  (both arguments depend on x).
        from symbolic_ir import POW
        xsq = IRApply(POW, (X, IRInteger(2)))
        integrand = _mul(xsq, _log(X))
        result = _vm_integrate(integrand)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_3_times_sin_2x(self):
        # ∫ 3·sin(2x) dx = −(3/2)·cos(2x).
        arg = _mul(IRInteger(2), X)
        integrand = _mul(IRInteger(3), _sin(arg))
        result = _vm_integrate(integrand)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_exp_2x_via_vm(self):
        # ∫ e^(2x) dx = e^(2x)/2 via the VM route (arg is not bare x).
        arg = _mul(IRInteger(2), X)
        integrand = _exp(arg)
        result = _vm_integrate(integrand)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_x_times_exp_2x_via_vm(self):
        # ∫ x·e^(2x) dx — MUL with both factors depending on x.
        arg = _mul(IRInteger(2), X)
        integrand = _mul(X, _exp(arg))
        result = _vm_integrate(integrand)
        assert_antideriv(integrand, result, test_xs=(0.5, 1.0, 2.0, 3.0))

    def test_log_x_via_vm(self):
        # ∫ log(x) dx = x·log(x) - x — regression via VM.
        result = _vm_integrate(_log(X))
        assert_antideriv(_log(X), result, test_xs=(0.5, 1.0, 2.0, 3.0))


# ---------------------------------------------------------------------------
# Regression: Phase 1 cases unchanged
# ---------------------------------------------------------------------------


class TestRegressions:
    def test_exp_x_unchanged(self):
        # ∫ exp(x) dx = exp(x) — Phase 1 result, not broken by Phase 3.
        result = _vm_integrate(_exp(X))
        assert result == _exp(X)

    def test_sin_x_unchanged(self):
        # ∫ sin(x) dx = −cos(x).
        from symbolic_ir import NEG
        result = _vm_integrate(_sin(X))
        assert result == IRApply(NEG, (_cos(X),))

    def test_cos_x_unchanged(self):
        # ∫ cos(x) dx = sin(x).
        result = _vm_integrate(_cos(X))
        assert result == _sin(X)

    def test_log_x_unchanged(self):
        # ∫ log(x) dx = x·log(x) − x.
        from symbolic_ir import MUL, SUB
        result = _vm_integrate(_log(X))
        expected = IRApply(SUB, (IRApply(MUL, (X, _log(X))), X))
        assert result == expected

    def test_rational_route_unchanged(self):
        # ∫ 1/(x²+1) dx = atan(x) — Phase 2e, not broken by Phase 3.
        from symbolic_ir import ADD, DIV, POW
        xsq_p1 = IRApply(ADD, (IRApply(POW, (X, IRInteger(2))), IRInteger(1)))
        integrand = IRApply(DIV, (IRInteger(1), xsq_p1))
        result = _vm_integrate(integrand)
        assert isinstance(result, IRApply) and result.head == ATAN
