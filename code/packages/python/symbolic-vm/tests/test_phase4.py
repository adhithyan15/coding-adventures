"""Tests for Phase 4 trigonometric integration.

Three sub-phases under test:
- Phase 4a: polynomial × sin/cos via tabular IBP
- Phase 4b: trig products/squares via product-to-sum identities
- Phase 4c: exp × sin/cos via double-IBP closed form

Correctness gate: numerical re-differentiation.  Every closed-form result
is differentiated numerically and compared to the original integrand.

Architecture:
- ``TestTrigPolyIntegral``  — unit tests for trig_sin_integral /
                              trig_cos_integral directly.
- ``TestTrigProducts``      — sin², cos², sin·cos, sin·sin, cos·cos
                              via the full Integrate handler.
- ``TestExpTrig``           — exp × sin/cos via the full Integrate handler.
- ``TestFallsThrough``      — tan(x), sin(x)/x → unevaluated.
- ``TestEndToEnd``          — one case per sub-phase through the full VM.
- ``TestRegressions``       — Phase 3 and earlier cases unchanged.
"""

from __future__ import annotations

import math
from fractions import Fraction

from symbolic_ir import (
    COS,
    EXP,
    INTEGRATE,
    MUL,
    SIN,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.backends import SymbolicBackend
from symbolic_vm.exp_trig_integral import exp_cos_integral, exp_sin_integral
from symbolic_vm.trig_poly_integral import trig_cos_integral, trig_sin_integral
from symbolic_vm.vm import VM

# ---------------------------------------------------------------------------
# Shared helpers (identical pattern to test_phase3.py)
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
    """Assert d/dx[antideriv] == integrand at several x values.

    Combined tolerance: ``tol = atol + rtol * |expected|``.  Prevents
    failures at large x where trig/exp values can be large.
    """
    for xv in test_xs:
        d = _numerical_deriv(antideriv, xv)
        expected = _integrand_eval(integrand, xv)
        tol = atol + rtol * abs(expected)
        assert abs(d - expected) < tol, (
            f"Re-differentiation failed at x={xv}: "
            f"d/dx = {d:.8f}, expected = {expected:.8f}"
        )


def _make_vm() -> VM:
    return VM(SymbolicBackend())


def _integrate_ir(integrand: IRNode, vm: VM | None = None) -> IRNode:
    """Run Integrate(integrand, x) through the symbolic VM."""
    if vm is None:
        vm = _make_vm()
    expr = IRApply(IRSymbol("Integrate"), (integrand, X))
    return vm.eval(expr)


# ---------------------------------------------------------------------------
# Phase 4a — Polynomial × sin/cos (unit tests on trig_poly_integral)
# ---------------------------------------------------------------------------


class TestTrigPolyIntegral:
    """Unit tests for trig_sin_integral and trig_cos_integral."""

    def test_x_times_sin_x(self):
        # ∫ x·sin(x) dx = sin(x) − x·cos(x)
        integrand = IRApply(MUL, (X, IRApply(SIN, (X,))))
        result = trig_sin_integral(P(0, 1), Fraction(1), Fraction(0), X)
        assert_antideriv(integrand, result)

    def test_x_sq_times_sin_x(self):
        # ∫ x²·sin(x) dx = 2x·sin(x) + (2 − x²)·cos(x)
        integrand = IRApply(MUL, (IRApply(IRSymbol("Pow"), (X, IRInteger(2))),
                                  IRApply(SIN, (X,))))
        result = trig_sin_integral(P(0, 0, 1), Fraction(1), Fraction(0), X)
        assert_antideriv(integrand, result)

    def test_x_times_sin_2x_plus_1(self):
        # ∫ x·sin(2x+1) dx
        integrand = IRApply(MUL, (X, IRApply(SIN, (
            IRApply(IRSymbol("Add"), (
                IRApply(IRSymbol("Mul"), (IRInteger(2), X)),
                IRInteger(1),
            )),
        ))))
        result = trig_sin_integral(P(0, 1), Fraction(2), Fraction(1), X)
        assert_antideriv(integrand, result)

    def test_x_times_cos_x(self):
        # ∫ x·cos(x) dx = x·sin(x) + cos(x)
        integrand = IRApply(MUL, (X, IRApply(COS, (X,))))
        result = trig_cos_integral(P(0, 1), Fraction(1), Fraction(0), X)
        assert_antideriv(integrand, result)

    def test_x_sq_times_cos_x(self):
        # ∫ x²·cos(x) dx = (x²−2)·sin(x) + 2x·cos(x)
        integrand = IRApply(MUL, (IRApply(IRSymbol("Pow"), (X, IRInteger(2))),
                                  IRApply(COS, (X,))))
        result = trig_cos_integral(P(0, 0, 1), Fraction(1), Fraction(0), X)
        assert_antideriv(integrand, result)

    def test_x_times_cos_2x_plus_3(self):
        # ∫ x·cos(2x+3) dx
        integrand = IRApply(MUL, (X, IRApply(COS, (
            IRApply(IRSymbol("Add"), (
                IRApply(IRSymbol("Mul"), (IRInteger(2), X)),
                IRInteger(3),
            )),
        ))))
        result = trig_cos_integral(P(0, 1), Fraction(2), Fraction(3), X)
        assert_antideriv(integrand, result)

    def test_x_cubed_times_sin_x(self):
        # ∫ x³·sin(x) dx = (3x²−6)·sin(x) − (x³−6x)·cos(x)
        integrand = IRApply(MUL, (IRApply(IRSymbol("Pow"), (X, IRInteger(3))),
                                  IRApply(SIN, (X,))))
        result = trig_sin_integral(P(0, 0, 0, 1), Fraction(1), Fraction(0), X)
        assert_antideriv(integrand, result)

    def test_constant_poly_sin(self):
        # ∫ 3·sin(2x) dx = −3cos(2x)/2  (degree-0 polynomial)
        integrand = IRApply(MUL, (IRInteger(3), IRApply(SIN, (
            IRApply(IRSymbol("Mul"), (IRInteger(2), X)),
        ))))
        result = trig_sin_integral(P(3), Fraction(2), Fraction(0), X)
        assert_antideriv(integrand, result)

    def test_zero_poly_returns_zero(self):
        # Empty polynomial → IRInteger(0)
        from symbolic_ir import IRInteger as I
        result = trig_sin_integral((), Fraction(1), Fraction(0), X)
        assert result == I(0)

    def test_a_equals_3(self):
        # ∫ x·sin(3x) dx  — checks non-unit a
        integrand = IRApply(MUL, (X, IRApply(SIN, (
            IRApply(IRSymbol("Mul"), (IRInteger(3), X)),
        ))))
        result = trig_sin_integral(P(0, 1), Fraction(3), Fraction(0), X)
        assert_antideriv(integrand, result)


# ---------------------------------------------------------------------------
# Phase 4a via full Integrate handler
# ---------------------------------------------------------------------------


class TestTrigPolyViaHandler:
    """End-to-end: x·sin / x·cos through vm.eval(Integrate(...))."""

    def test_x_times_sin_x_via_handler(self):
        # ∫ x·sin(x) dx — uses Phase 4a _try_trig_product.
        integrand = IRApply(MUL, (X, IRApply(SIN, (X,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_sin_x_times_x_reversed(self):
        # sin(x)·x — factor order swapped; still hits Phase 4a.
        integrand = IRApply(MUL, (IRApply(SIN, (X,)), X))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_x_times_cos_x_via_handler(self):
        integrand = IRApply(MUL, (X, IRApply(COS, (X,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)


# ---------------------------------------------------------------------------
# Phase 4b — Trig products and squares
# ---------------------------------------------------------------------------


class TestTrigProducts:
    """sin², cos², sin·sin, sin·cos, cos·cos via the Integrate handler."""

    def test_sin_sq(self):
        # ∫ sin²(x) dx = x/2 − sin(2x)/4
        integrand = IRApply(MUL, (IRApply(SIN, (X,)), IRApply(SIN, (X,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_cos_sq(self):
        # ∫ cos²(x) dx = x/2 + sin(2x)/4
        integrand = IRApply(MUL, (IRApply(COS, (X,)), IRApply(COS, (X,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_sin_times_cos_same_arg(self):
        # ∫ sin(x)·cos(x) dx = sin²(x)/2 (or −cos(2x)/4)
        integrand = IRApply(MUL, (IRApply(SIN, (X,)), IRApply(COS, (X,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_cos_times_sin_reversed(self):
        # cos(x)·sin(x) — reversed order; same result.
        integrand = IRApply(MUL, (IRApply(COS, (X,)), IRApply(SIN, (X,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_sin_sq_linear_arg(self):
        # ∫ sin²(3x+1) dx
        arg = IRApply(IRSymbol("Add"), (
            IRApply(IRSymbol("Mul"), (IRInteger(3), X)), IRInteger(1)
        ))
        integrand = IRApply(MUL, (IRApply(SIN, (arg,)), IRApply(SIN, (arg,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_cos_sq_linear_arg(self):
        # ∫ cos²(2x) dx
        arg = IRApply(IRSymbol("Mul"), (IRInteger(2), X))
        integrand = IRApply(MUL, (IRApply(COS, (arg,)), IRApply(COS, (arg,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_sin_x_times_sin_2x(self):
        # ∫ sin(x)·sin(2x) dx = sin(x)/2 − sin(3x)/6
        arg2x = IRApply(IRSymbol("Mul"), (IRInteger(2), X))
        integrand = IRApply(MUL, (IRApply(SIN, (X,)), IRApply(SIN, (arg2x,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_cos_x_times_cos_2x(self):
        # ∫ cos(x)·cos(2x) dx
        arg2x = IRApply(IRSymbol("Mul"), (IRInteger(2), X))
        integrand = IRApply(MUL, (IRApply(COS, (X,)), IRApply(COS, (arg2x,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_sin_x_times_cos_2x(self):
        # ∫ sin(x)·cos(2x) dx = −cos(3x)/6 + cos(x)/2
        arg2x = IRApply(IRSymbol("Mul"), (IRInteger(2), X))
        integrand = IRApply(MUL, (IRApply(SIN, (X,)), IRApply(COS, (arg2x,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_cos_2x_times_sin_x(self):
        # cos(2x)·sin(x) — reversed order of previous.
        arg2x = IRApply(IRSymbol("Mul"), (IRInteger(2), X))
        integrand = IRApply(MUL, (IRApply(COS, (arg2x,)), IRApply(SIN, (X,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)


# ---------------------------------------------------------------------------
# Phase 4c — Exp × trig (unit tests on exp_sin/cos_integral)
# ---------------------------------------------------------------------------


class TestExpTrig:
    """Unit tests for exp_sin_integral and exp_cos_integral, plus handler tests."""

    def test_exp_x_sin_x_unit(self):
        # ∫ eˣ·sin(x) dx = eˣ·(sin x − cos x)/2
        integrand = IRApply(MUL, (IRApply(EXP, (X,)), IRApply(SIN, (X,))))
        result = exp_sin_integral(
            Fraction(1), Fraction(0), Fraction(1), Fraction(0), X
        )
        assert_antideriv(integrand, result)

    def test_exp_x_cos_x_unit(self):
        # ∫ eˣ·cos(x) dx = eˣ·(sin x + cos x)/2
        integrand = IRApply(MUL, (IRApply(EXP, (X,)), IRApply(COS, (X,))))
        result = exp_cos_integral(
            Fraction(1), Fraction(0), Fraction(1), Fraction(0), X
        )
        assert_antideriv(integrand, result)

    def test_exp_2x_sin_3x_unit(self):
        # ∫ e^(2x)·sin(3x) dx = e^(2x)·(2sin(3x)−3cos(3x))/13
        arg2x = IRApply(IRSymbol("Mul"), (IRInteger(2), X))
        arg3x = IRApply(IRSymbol("Mul"), (IRInteger(3), X))
        integrand = IRApply(MUL, (IRApply(EXP, (arg2x,)), IRApply(SIN, (arg3x,))))
        result = exp_sin_integral(
            Fraction(2), Fraction(0), Fraction(3), Fraction(0), X
        )
        assert_antideriv(integrand, result)

    def test_exp_2x_cos_3x_unit(self):
        # ∫ e^(2x)·cos(3x) dx = e^(2x)·(2cos(3x)+3sin(3x))/13
        arg2x = IRApply(IRSymbol("Mul"), (IRInteger(2), X))
        arg3x = IRApply(IRSymbol("Mul"), (IRInteger(3), X))
        integrand = IRApply(MUL, (IRApply(EXP, (arg2x,)), IRApply(COS, (arg3x,))))
        result = exp_cos_integral(
            Fraction(2), Fraction(0), Fraction(3), Fraction(0), X
        )
        assert_antideriv(integrand, result)

    def test_exp_x_plus_1_sin_2x_plus_3_unit(self):
        # ∫ e^(x+1)·sin(2x+3) dx
        arg_exp = IRApply(IRSymbol("Add"), (X, IRInteger(1)))
        arg_sin = IRApply(IRSymbol("Add"), (
            IRApply(IRSymbol("Mul"), (IRInteger(2), X)), IRInteger(3)
        ))
        integrand = IRApply(MUL, (IRApply(EXP, (arg_exp,)), IRApply(SIN, (arg_sin,))))
        result = exp_sin_integral(
            Fraction(1), Fraction(1), Fraction(2), Fraction(3), X
        )
        assert_antideriv(integrand, result)

    def test_exp_x_sin_x_via_handler(self):
        # Same as unit test but through the full VM Integrate handler.
        integrand = IRApply(MUL, (IRApply(EXP, (X,)), IRApply(SIN, (X,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_exp_x_cos_x_via_handler(self):
        integrand = IRApply(MUL, (IRApply(EXP, (X,)), IRApply(COS, (X,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_sin_x_times_exp_x_reversed(self):
        # sin(x)·exp(x) — factor order swapped; hits swapped-order call.
        integrand = IRApply(MUL, (IRApply(SIN, (X,)), IRApply(EXP, (X,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_exp_2x_sin_3x_via_handler(self):
        arg2x = IRApply(IRSymbol("Mul"), (IRInteger(2), X))
        arg3x = IRApply(IRSymbol("Mul"), (IRInteger(3), X))
        integrand = IRApply(MUL, (IRApply(EXP, (arg2x,)), IRApply(SIN, (arg3x,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_a_equals_c(self):
        # ∫ eˣ·sin(x) dx — a = c = 1, D = 2. Result = eˣ(sin x − cos x)/2.
        integrand = IRApply(MUL, (IRApply(EXP, (X,)), IRApply(SIN, (X,))))
        result = _integrate_ir(integrand)
        # Numerical spot-check at x=1: d/dx should be e¹·sin(1)
        expected_at_1 = math.e * math.sin(1)
        d_at_1 = _numerical_deriv(result, 1.0)
        assert abs(d_at_1 - expected_at_1) < 1e-5


# ---------------------------------------------------------------------------
# Fall-through guard — expressions that should stay unevaluated
# ---------------------------------------------------------------------------


class TestFallsThrough:
    """Patterns outside Phase 4 scope stay as unevaluated Integrate."""

    def test_tan_x_unevaluated(self):
        # tan(x) = sin(x)/cos(x) — not yet handled.
        from symbolic_ir import DIV
        integrand = IRApply(DIV, (IRApply(SIN, (X,)), IRApply(COS, (X,))))
        result = _integrate_ir(integrand)
        assert result == IRApply(INTEGRATE, (integrand, X))

    def test_sin_over_x_unevaluated(self):
        # sin(x)/x — Sine integral Si(x); non-elementary.
        from symbolic_ir import DIV
        integrand = IRApply(DIV, (IRApply(SIN, (X,)), X))
        result = _integrate_ir(integrand)
        assert result == IRApply(INTEGRATE, (integrand, X))


# ---------------------------------------------------------------------------
# Regression — Phase 3 and Phase 1 cases unchanged
# ---------------------------------------------------------------------------


class TestRegressions:
    """Phase 4 must not break any earlier integration rules."""

    def test_sin_x_still_works(self):
        # ∫ sin(x) dx = −cos(x)  (Phase 1).
        integrand = IRApply(SIN, (X,))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))

    def test_exp_x_still_works(self):
        # ∫ exp(x) dx = exp(x)  (Phase 1).
        integrand = IRApply(EXP, (X,))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))

    def test_x_exp_x_still_works(self):
        # ∫ x·eˣ dx = (x−1)·eˣ  (Phase 3d).
        integrand = IRApply(MUL, (X, IRApply(EXP, (X,))))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)

    def test_sin_2x_still_works(self):
        # ∫ sin(2x) dx = −cos(2x)/2  (Phase 3b).
        arg = IRApply(IRSymbol("Mul"), (IRInteger(2), X))
        integrand = IRApply(SIN, (arg,))
        result = _integrate_ir(integrand)
        assert result != IRApply(INTEGRATE, (integrand, X))
        assert_antideriv(integrand, result)
