"""Phase 5 integration tests — higher trig powers and tan.

Phase 5a: ∫ tan(ax+b) dx  =  −log(cos(ax+b)) / a
Phase 5b: ∫ sinⁿ(ax+b) dx  and  ∫ cosⁿ(ax+b) dx  (n ≥ 2, reduction formula)
Phase 5c: ∫ tanⁿ(ax+b) dx  (n ≥ 2, Pythagorean reduction)

Correctness is verified numerically: differentiate the antiderivative at
test points and confirm the result matches the original integrand.

Test points are chosen safely away from poles (tan has poles at π/2 + kπ ≈ 1.57):
  x₀ = 0.5  — used for all tests
  x₁ = 0.3  — second check, avoids cancellation artefacts

Combined tolerance: atol=1e-6, rtol=1e-6·|expected|.
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    COS,
    INTEGRATE,
    MUL,
    NEG,
    POW,
    SIN,
    TAN,
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
# Shared helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")


def _make_vm() -> VM:
    return VM(SymbolicBackend())


def _eval_ir(node: IRNode, x_val: float) -> float:  # noqa: PLR0911
    """Numerically evaluate an IR tree at x = x_val."""
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRFloat):
        return node.value
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
    if head == "Tan":
        return math.tan(_eval_ir(node.args[0], x_val))
    if head == "Pow":
        return _eval_ir(node.args[0], x_val) ** _eval_ir(node.args[1], x_val)
    if head == "Sqrt":
        return math.sqrt(_eval_ir(node.args[0], x_val))
    if head == "Atan":
        return math.atan(_eval_ir(node.args[0], x_val))
    raise ValueError(f"Unhandled head: {head}")


def _numerical_deriv(node: IRNode, x_val: float, h: float = 1e-7) -> float:
    return (_eval_ir(node, x_val + h) - _eval_ir(node, x_val - h)) / (2 * h)


def _check_antiderivative(
    integrand: IRNode,
    antideriv: IRNode,
    test_points: tuple[float, ...] = (0.5, 0.3),
    atol: float = 1e-6,
    rtol: float = 1e-6,
) -> None:
    """Verify F'(x) ≈ f(x) numerically at each test point."""
    for x_val in test_points:
        expected = _eval_ir(integrand, x_val)
        actual = _numerical_deriv(antideriv, x_val)
        tol = atol + rtol * abs(expected)
        assert abs(actual - expected) < tol, (
            f"At x={x_val}: F'={actual:.8f}, f={expected:.8f}, "
            f"diff={abs(actual-expected):.2e}"
        )


def _integrate_ir(vm: VM, integrand_ir: IRNode) -> IRNode:
    """Construct and evaluate Integrate(f, x)."""
    return vm.eval(IRApply(INTEGRATE, (integrand_ir, X)))


# ---------------------------------------------------------------------------
# Phase 5a — tan(ax+b)
# ---------------------------------------------------------------------------


class TestTanIntegral:
    """∫ tan(ax+b) dx = −log(cos(ax+b)) / a"""

    def test_tan_x_basic(self):
        """∫ tan(x) dx = −log(cos(x))"""
        vm = _make_vm()
        f = IRApply(TAN, (X,))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F, "Should be closed form"
        _check_antiderivative(f, F)

    def test_tan_2x(self):
        """∫ tan(2x) dx = −log(cos(2x)) / 2"""
        vm = _make_vm()
        arg = IRApply(MUL, (IRInteger(2), X))
        f = IRApply(TAN, (arg,))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_tan_3x_plus_1(self):
        """∫ tan(3x+1) dx = −log(cos(3x+1)) / 3"""
        vm = _make_vm()
        arg = IRApply(ADD, (IRApply(MUL, (IRInteger(3), X)), IRInteger(1)))
        f = IRApply(TAN, (arg,))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_tan_half_x(self):
        """∫ tan(x/2) dx = −2 log(cos(x/2))"""
        vm = _make_vm()
        arg = IRApply(MUL, (IRRational(1, 2), X))
        f = IRApply(TAN, (arg,))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_tan_neg_x(self):
        """∫ tan(−x) dx = log(cos(x)) — negative coefficient."""
        vm = _make_vm()
        arg = IRApply(NEG, (X,))
        f = IRApply(TAN, (arg,))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_tan_via_macsyma(self):
        """End-to-end: integrate(tan(x), x) via MACSYMA source."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = _make_vm()
        ir = compile_macsyma(parse_macsyma("integrate(tan(x), x);"))[0]
        result = vm.eval(ir)
        assert result != IRApply(INTEGRATE, (IRApply(TAN, (X,)), X))


# ---------------------------------------------------------------------------
# Phase 5b — sinⁿ(ax+b) reduction
# ---------------------------------------------------------------------------


class TestSinPower:
    """∫ sinⁿ(ax+b) dx via reduction formula for n ≥ 2."""

    def test_sin2_x(self):
        """∫ sin²(x) dx = −sin(x)cos(x)/2 + x/2"""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(SIN, (X,)), IRInteger(2)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin3_x(self):
        """∫ sin³(x) dx = −sin²(x)cos(x)/3 − 2cos(x)/3"""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(SIN, (X,)), IRInteger(3)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin4_x(self):
        """∫ sin⁴(x) dx — reduction formula verified numerically."""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(SIN, (X,)), IRInteger(4)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin5_x(self):
        """∫ sin⁵(x) dx — odd power, deeper recursion."""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(SIN, (X,)), IRInteger(5)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin6_x(self):
        """∫ sin⁶(x) dx — even power, terminates at constant."""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(SIN, (X,)), IRInteger(6)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin2_2x(self):
        """∫ sin²(2x) dx — linear argument with a=2."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRInteger(2), X))
        f = IRApply(POW, (IRApply(SIN, (arg,)), IRInteger(2)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin3_3x_plus_1(self):
        """∫ sin³(3x+1) dx — linear argument with a=3, b=1."""
        vm = _make_vm()
        arg = IRApply(ADD, (IRApply(MUL, (IRInteger(3), X)), IRInteger(1)))
        f = IRApply(POW, (IRApply(SIN, (arg,)), IRInteger(3)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin4_half_x(self):
        """∫ sin⁴(x/2) dx — fractional coefficient a=1/2."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRRational(1, 2), X))
        f = IRApply(POW, (IRApply(SIN, (arg,)), IRInteger(4)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin2_via_macsyma(self):
        """End-to-end: integrate(sin(x)^2, x) via MACSYMA source."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = _make_vm()
        ir = compile_macsyma(parse_macsyma("integrate(sin(x)^2, x);"))[0]
        result = vm.eval(ir)
        f_ir = IRApply(POW, (IRApply(SIN, (X,)), IRInteger(2)))
        assert result != IRApply(INTEGRATE, (f_ir, X))


# ---------------------------------------------------------------------------
# Phase 5b — cosⁿ(ax+b) reduction
# ---------------------------------------------------------------------------


class TestCosPower:
    """∫ cosⁿ(ax+b) dx via reduction formula for n ≥ 2."""

    def test_cos2_x(self):
        """∫ cos²(x) dx = cos(x)sin(x)/2 + x/2"""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(COS, (X,)), IRInteger(2)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_cos3_x(self):
        """∫ cos³(x) dx = cos²(x)sin(x)/3 + 2sin(x)/3"""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(COS, (X,)), IRInteger(3)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_cos4_x(self):
        """∫ cos⁴(x) dx — even power reduction."""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(COS, (X,)), IRInteger(4)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_cos5_x(self):
        """∫ cos⁵(x) dx — odd power reduction."""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(COS, (X,)), IRInteger(5)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_cos2_2x(self):
        """∫ cos²(2x) dx — linear argument a=2."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRInteger(2), X))
        f = IRApply(POW, (IRApply(COS, (arg,)), IRInteger(2)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_cos3_2x_plus_1(self):
        """∫ cos³(2x+1) dx — linear argument a=2, b=1."""
        vm = _make_vm()
        arg = IRApply(ADD, (IRApply(MUL, (IRInteger(2), X)), IRInteger(1)))
        f = IRApply(POW, (IRApply(COS, (arg,)), IRInteger(3)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_cos4_half_x(self):
        """∫ cos⁴(x/2) dx — fractional coefficient a=1/2."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRRational(1, 2), X))
        f = IRApply(POW, (IRApply(COS, (arg,)), IRInteger(4)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_cos3_via_macsyma(self):
        """End-to-end: integrate(cos(x)^3, x) via MACSYMA source."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = _make_vm()
        ir = compile_macsyma(parse_macsyma("integrate(cos(x)^3, x);"))[0]
        result = vm.eval(ir)
        f_ir = IRApply(POW, (IRApply(COS, (X,)), IRInteger(3)))
        assert result != IRApply(INTEGRATE, (f_ir, X))


# ---------------------------------------------------------------------------
# Phase 5c — tanⁿ(ax+b) reduction
# ---------------------------------------------------------------------------


class TestTanPower:
    """∫ tanⁿ(ax+b) dx via Pythagorean reduction for n ≥ 2."""

    def test_tan2_x(self):
        """∫ tan²(x) dx = tan(x) − x"""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(TAN, (X,)), IRInteger(2)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_tan3_x(self):
        """∫ tan³(x) dx = tan²(x)/2 + log(cos(x))"""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(TAN, (X,)), IRInteger(3)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_tan4_x(self):
        """∫ tan⁴(x) dx = tan³(x)/3 − tan(x) + x"""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(TAN, (X,)), IRInteger(4)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_tan5_x(self):
        """∫ tan⁵(x) dx — depth-3 recursion."""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(TAN, (X,)), IRInteger(5)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_tan2_2x(self):
        """∫ tan²(2x) dx = tan(2x)/2 − x"""
        vm = _make_vm()
        arg = IRApply(MUL, (IRInteger(2), X))
        f = IRApply(POW, (IRApply(TAN, (arg,)), IRInteger(2)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_tan3_3x_plus_1(self):
        """∫ tan³(3x+1) dx — linear argument with a=3, b=1."""
        vm = _make_vm()
        arg = IRApply(ADD, (IRApply(MUL, (IRInteger(3), X)), IRInteger(1)))
        f = IRApply(POW, (IRApply(TAN, (arg,)), IRInteger(3)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_tan4_half_x(self):
        """∫ tan⁴(x/2) dx — fractional coefficient a=1/2."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRRational(1, 2), X))
        f = IRApply(POW, (IRApply(TAN, (arg,)), IRInteger(4)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_tan2_via_macsyma(self):
        """End-to-end: integrate(tan(x)^2, x) via MACSYMA source."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = _make_vm()
        ir = compile_macsyma(parse_macsyma("integrate(tan(x)^2, x);"))[0]
        result = vm.eval(ir)
        f_ir = IRApply(POW, (IRApply(TAN, (X,)), IRInteger(2)))
        assert result != IRApply(INTEGRATE, (f_ir, X))


# ---------------------------------------------------------------------------
# Fall-through guards
# ---------------------------------------------------------------------------


class TestFallsThrough:
    """Integrands Phase 5 should NOT handle."""

    def test_sin_negative_power_unevaluated(self):
        """sin(x)^(−1) = csc(x) — no rule, stays unevaluated."""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(SIN, (X,)), IRInteger(-1)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) == F

    def test_tan_nonlinear_arg_unevaluated(self):
        """tan(x²) — non-linear argument, stays unevaluated."""
        vm = _make_vm()
        x_sq = IRApply(POW, (X, IRInteger(2)))
        f = IRApply(TAN, (x_sq,))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) == F

    def test_sin_float_exponent_unevaluated(self):
        """sin(x)^2.5 — non-integer exponent, stays unevaluated."""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(SIN, (X,)), IRFloat(2.5)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) == F

    def test_cos_power_zero_trivial(self):
        """cos(x)^0 = 1; ∫1 dx should not stay as an unevaluated Integrate."""
        vm = _make_vm()
        # The VM simplifies POW(..., 0) → 1 before integration, so this
        # is effectively integrate(1, x) = x.  Verify it closes.
        f_zero = IRApply(POW, (IRApply(COS, (X,)), IRInteger(0)))
        F = _integrate_ir(vm, f_zero)
        assert IRApply(INTEGRATE, (f_zero, X)) != F


# ---------------------------------------------------------------------------
# Derivative handler tests (d/dx tan)
# ---------------------------------------------------------------------------


class TestTanDerivative:
    """d/dx tan(u) = u' / cos²(u)."""

    def test_diff_tan_x(self):
        """diff(tan(x), x) evaluates to sec²(x) = 1/cos²(x) numerically."""
        from symbolic_ir import D

        vm = _make_vm()
        tan_x = IRApply(TAN, (X,))
        result = vm.eval(IRApply(D, (tan_x, X)))
        # Verify: derivative of tan at x=0.5 is sec²(0.5)
        expected = 1.0 / math.cos(0.5) ** 2
        actual = _eval_ir(result, 0.5)
        assert abs(actual - expected) < 1e-9

    def test_diff_tan_2x(self):
        """diff(tan(2x), x) = 2 / cos²(2x) at x=0.5."""
        from symbolic_ir import D

        vm = _make_vm()
        arg = IRApply(MUL, (IRInteger(2), X))
        tan_2x = IRApply(TAN, (arg,))
        result = vm.eval(IRApply(D, (tan_2x, X)))
        expected = 2.0 / math.cos(1.0) ** 2
        actual = _eval_ir(result, 0.5)
        assert abs(actual - expected) < 1e-9


# ---------------------------------------------------------------------------
# Regression tests — phase interactions
# ---------------------------------------------------------------------------


class TestRegressions:
    """Ensure Phase 5 does not break earlier phases."""

    def test_sin_x_still_works(self):
        """∫ sin(x) dx = −cos(x) — Phase 3b unchanged."""
        vm = _make_vm()
        f = IRApply(SIN, (X,))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_cos_x_still_works(self):
        """∫ cos(x) dx = sin(x) — Phase 3c unchanged."""
        vm = _make_vm()
        f = IRApply(COS, (X,))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin_times_sin_phase4b_unchanged(self):
        """∫ sin(x)·sin(x) dx — Phase 4b product-to-sum unchanged."""
        vm = _make_vm()
        sin_x = IRApply(SIN, (X,))
        f = IRApply(MUL, (sin_x, sin_x))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_roundtrip_tan(self):
        """diff(integrate(tan(x), x), x) ≈ tan(x) numerically."""
        from symbolic_ir import D

        vm = _make_vm()
        tan_x = IRApply(TAN, (X,))
        F = vm.eval(IRApply(INTEGRATE, (tan_x, X)))
        dF = vm.eval(IRApply(D, (F, X)))
        for x_val in (0.3, 0.5):
            expected = math.tan(x_val)
            actual = _eval_ir(dF, x_val)
            assert abs(actual - expected) < 1e-6

    def test_roundtrip_sin2(self):
        """diff(integrate(sin(x)^2, x), x) ≈ sin(x)^2 numerically."""
        from symbolic_ir import D

        vm = _make_vm()
        sin2 = IRApply(POW, (IRApply(SIN, (X,)), IRInteger(2)))
        F = vm.eval(IRApply(INTEGRATE, (sin2, X)))
        dF = vm.eval(IRApply(D, (F, X)))
        for x_val in (0.3, 0.5):
            expected = math.sin(x_val) ** 2
            actual = _eval_ir(dF, x_val)
            assert abs(actual - expected) < 1e-6

    def test_x_squared_still_works(self):
        """∫ x² dx = x³/3 — Phase 1 power rule unchanged."""
        vm = _make_vm()
        f = IRApply(POW, (X, IRInteger(2)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        # Numerical check
        x_val = 0.5
        expected = x_val ** 2
        actual = _numerical_deriv(F, x_val)
        assert abs(actual - expected) < 1e-6

    def test_polynomial_times_sin_phase4a_unchanged(self):
        """∫ x·sin(x) dx — Phase 4a tabular IBP unchanged."""
        vm = _make_vm()
        f = IRApply(MUL, (X, IRApply(SIN, (X,))))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)
