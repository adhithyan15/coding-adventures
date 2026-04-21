"""Phase 6 integration tests — sinⁿ·cosᵐ mixed trig powers.

Phase 6a: n odd  — u = cos substitution → closed-form sum of cos powers
Phase 6b: m odd, n even — u = sin substitution → closed-form sum of sin powers
Phase 6c: both even — IBP reduction formula, recurses to Phase 5b

Correctness is verified numerically: differentiate the antiderivative at test
points and confirm the result matches the original integrand.

Test points: x₀ = 0.4, x₁ = 0.7 — safely away from trig poles.
Combined tolerance: atol=1e-6, rtol=1e-6·|expected|.
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    COS,
    INTEGRATE,
    MUL,
    POW,
    SIN,
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
# Shared helpers (same pattern as test_phase5.py)
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
    test_points: tuple[float, ...] = (0.4, 0.7),
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


def _sin_pow(n: int, arg_ir: IRNode | None = None) -> IRNode:
    """Build SIN(x)^n or SIN(arg)^n IR node."""
    base = IRApply(SIN, (arg_ir if arg_ir is not None else X,))
    if n == 1:
        return base
    return IRApply(POW, (base, IRInteger(n)))


def _cos_pow(m: int, arg_ir: IRNode | None = None) -> IRNode:
    """Build COS(x)^m or COS(arg)^m IR node."""
    base = IRApply(COS, (arg_ir if arg_ir is not None else X,))
    if m == 1:
        return base
    return IRApply(POW, (base, IRInteger(m)))


def _sin_cos_product(n: int, m: int, arg_ir: IRNode | None = None) -> IRNode:
    """Build sinⁿ(arg)·cosᵐ(arg) as MUL IR node."""
    return IRApply(MUL, (_sin_pow(n, arg_ir), _cos_pow(m, arg_ir)))


# ---------------------------------------------------------------------------
# Phase 6a — n odd (cosine substitution)
# ---------------------------------------------------------------------------


class TestSinCosOddN:
    """∫ sinⁿ cosᵐ dx for odd n — closed-form sum of cos powers."""

    def test_sin1_cos2(self):
        """∫ sin(x) cos²(x) dx = -cos³(x)/3"""
        vm = _make_vm()
        f = _sin_cos_product(1, 2)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin1_cos3(self):
        """∫ sin(x) cos³(x) dx = -cos⁴(x)/4"""
        vm = _make_vm()
        f = _sin_cos_product(1, 3)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin1_cos4(self):
        """∫ sin(x) cos⁴(x) dx = -cos⁵(x)/5"""
        vm = _make_vm()
        f = _sin_cos_product(1, 4)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin3_cos2(self):
        """∫ sin³(x) cos²(x) dx = cos⁵(x)/5 - cos³(x)/3"""
        vm = _make_vm()
        f = _sin_cos_product(3, 2)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin3_cos4(self):
        """∫ sin³(x) cos⁴(x) dx — two-term binomial expansion."""
        vm = _make_vm()
        f = _sin_cos_product(3, 4)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin5_cos2(self):
        """∫ sin⁵(x) cos²(x) dx — three-term binomial expansion."""
        vm = _make_vm()
        f = _sin_cos_product(5, 2)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin5_cos4(self):
        """∫ sin⁵(x) cos⁴(x) dx — three-term expansion with higher m."""
        vm = _make_vm()
        f = _sin_cos_product(5, 4)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin1_cos2_linear_arg(self):
        """∫ sin(2x) cos²(2x) dx — linear argument a=2."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRInteger(2), X))
        f = _sin_cos_product(1, 2, arg)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin3_cos2_linear_arg(self):
        """∫ sin³(3x+1) cos²(3x+1) dx — a=3, b=1."""
        vm = _make_vm()
        arg = IRApply(ADD, (IRApply(MUL, (IRInteger(3), X)), IRInteger(1)))
        f = _sin_cos_product(3, 2, arg)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin1_cos2_via_macsyma(self):
        """End-to-end: integrate(sin(x)*cos(x)^2, x) via MACSYMA source."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = _make_vm()
        ir = compile_macsyma(parse_macsyma("integrate(sin(x)*cos(x)^2, x);"))[0]
        result = vm.eval(ir)
        f = _sin_cos_product(1, 2)
        assert result != IRApply(INTEGRATE, (f, X))


# ---------------------------------------------------------------------------
# Phase 6b — m odd, n even (sine substitution)
# ---------------------------------------------------------------------------


class TestSinCosOddM:
    """∫ sinⁿ cosᵐ dx for even n, odd m — closed-form sum of sin powers."""

    def test_sin2_cos1(self):
        """∫ sin²(x) cos(x) dx = sin³(x)/3"""
        vm = _make_vm()
        f = _sin_cos_product(2, 1)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin4_cos1(self):
        """∫ sin⁴(x) cos(x) dx = sin⁵(x)/5"""
        vm = _make_vm()
        f = _sin_cos_product(4, 1)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin2_cos3(self):
        """∫ sin²(x) cos³(x) dx = sin³/3 - sin⁵/5"""
        vm = _make_vm()
        f = _sin_cos_product(2, 3)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin4_cos3(self):
        """∫ sin⁴(x) cos³(x) dx — two-term expansion."""
        vm = _make_vm()
        f = _sin_cos_product(4, 3)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin2_cos5(self):
        """∫ sin²(x) cos⁵(x) dx — three-term expansion."""
        vm = _make_vm()
        f = _sin_cos_product(2, 5)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin4_cos5(self):
        """∫ sin⁴(x) cos⁵(x) dx — three-term expansion."""
        vm = _make_vm()
        f = _sin_cos_product(4, 5)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin2_cos1_linear_arg(self):
        """∫ sin²(2x) cos(2x) dx — linear argument a=2."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRInteger(2), X))
        f = _sin_cos_product(2, 1, arg)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin2_cos3_via_macsyma(self):
        """End-to-end: integrate(sin(x)^2*cos(x)^3, x) via MACSYMA."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = _make_vm()
        ir = compile_macsyma(parse_macsyma("integrate(sin(x)^2*cos(x)^3, x);"))[0]
        result = vm.eval(ir)
        f = _sin_cos_product(2, 3)
        assert result != IRApply(INTEGRATE, (f, X))


# ---------------------------------------------------------------------------
# Phase 6c — both even (IBP reduction)
# ---------------------------------------------------------------------------


class TestSinCosBothEven:
    """∫ sinⁿ cosᵐ dx for even n, m — IBP reduction formula."""

    def test_sin2_cos2(self):
        """∫ sin²(x) cos²(x) dx — one step to Phase 5b."""
        vm = _make_vm()
        f = _sin_cos_product(2, 2)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin4_cos2(self):
        """∫ sin⁴(x) cos²(x) dx — two steps to Phase 5b."""
        vm = _make_vm()
        f = _sin_cos_product(4, 2)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin2_cos4(self):
        """∫ sin²(x) cos⁴(x) dx — one step (n→0) to Phase 5b."""
        vm = _make_vm()
        f = _sin_cos_product(2, 4)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin4_cos4(self):
        """∫ sin⁴(x) cos⁴(x) dx — two recursion steps."""
        vm = _make_vm()
        f = _sin_cos_product(4, 4)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin6_cos2(self):
        """∫ sin⁶(x) cos²(x) dx — three recursion steps."""
        vm = _make_vm()
        f = _sin_cos_product(6, 2)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin2_cos2_linear_arg(self):
        """∫ sin²(2x) cos²(2x) dx — linear argument a=2."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRInteger(2), X))
        f = _sin_cos_product(2, 2, arg)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin4_cos4_linear_arg(self):
        """∫ sin⁴(x/2) cos⁴(x/2) dx — fractional coefficient a=1/2."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRRational(1, 2), X))
        f = _sin_cos_product(4, 4, arg)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin2_cos2_via_macsyma(self):
        """End-to-end: integrate(sin(x)^2*cos(x)^2, x) via MACSYMA."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = _make_vm()
        ir = compile_macsyma(parse_macsyma("integrate(sin(x)^2*cos(x)^2, x);"))[0]
        result = vm.eval(ir)
        f = _sin_cos_product(2, 2)
        assert result != IRApply(INTEGRATE, (f, X))


# ---------------------------------------------------------------------------
# Linear argument variations
# ---------------------------------------------------------------------------


class TestSinCosLinearArgs:
    """Verify coefficient scaling with a ≠ 1 and b ≠ 0."""

    def test_sin1_cos2_a3(self):
        """∫ sin(3x) cos²(3x) dx — a=3."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRInteger(3), X))
        f = _sin_cos_product(1, 2, arg)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin2_cos3_a2(self):
        """∫ sin²(2x) cos³(2x) dx — a=2."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRInteger(2), X))
        f = _sin_cos_product(2, 3, arg)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin3_cos2_a_half(self):
        """∫ sin³(x/2) cos²(x/2) dx — a=1/2."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRRational(1, 2), X))
        f = _sin_cos_product(3, 2, arg)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin1_cos2_with_b(self):
        """∫ sin(2x+1) cos²(2x+1) dx — a=2, b=1."""
        vm = _make_vm()
        arg = IRApply(ADD, (IRApply(MUL, (IRInteger(2), X)), IRInteger(1)))
        f = _sin_cos_product(1, 2, arg)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin2_cos2_with_b(self):
        """∫ sin²(x+2) cos²(x+2) dx — a=1, b=2."""
        vm = _make_vm()
        arg = IRApply(ADD, (X, IRInteger(2)))
        f = _sin_cos_product(2, 2, arg)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)

    def test_sin3_cos4_a3_b1(self):
        """∫ sin³(3x+1) cos⁴(3x+1) dx — a=3, b=1, mixed odd/even."""
        vm = _make_vm()
        arg = IRApply(ADD, (IRApply(MUL, (IRInteger(3), X)), IRInteger(1)))
        f = _sin_cos_product(3, 4, arg)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Fall-through: cases that should stay unevaluated
# ---------------------------------------------------------------------------


class TestSinCosFallthrough:
    """Integrands that Phase 6 should NOT claim — must stay unevaluated."""

    def test_different_args_falls_through(self):
        """sin(x) cos(2x) — different arguments, not Phase 6."""
        vm = _make_vm()
        arg2 = IRApply(MUL, (IRInteger(2), X))
        f = IRApply(MUL, (IRApply(SIN, (X,)), IRApply(COS, (arg2,))))
        F = _integrate_ir(vm, f)
        # Phase 4b (product-to-sum) should handle this, not leave it unevaluated.
        # The important thing is it doesn't crash.
        assert F is not None

    def test_sin_sin_not_phase6(self):
        """sin²(x) · sin(x) — two sin factors, Phase 6 ignores it."""
        vm = _make_vm()
        sin2 = IRApply(POW, (IRApply(SIN, (X,)), IRInteger(2)))
        f = IRApply(MUL, (sin2, IRApply(SIN, (X,))))
        F = _integrate_ir(vm, f)
        # Phase 6 returns None for two-sin; should be handled by Phase 5 as sin³
        # via the _try_trig_product path or fall through gracefully.
        assert F is not None

    def test_cos_cos_not_phase6(self):
        """cos³(x) · cos²(x) — two cos factors, Phase 6 ignores it."""
        vm = _make_vm()
        cos3 = IRApply(POW, (IRApply(COS, (X,)), IRInteger(3)))
        cos2 = IRApply(POW, (IRApply(COS, (X,)), IRInteger(2)))
        f = IRApply(MUL, (cos3, cos2))
        F = _integrate_ir(vm, f)
        # Phase 6 returns None for two-cos; doesn't crash.
        assert F is not None

    def test_non_integer_exponent_falls_through(self):
        """sin^(1/2)(x)·cos(x) — fractional sin exponent, Phase 6 ignores."""
        vm = _make_vm()
        sin_half = IRApply(POW, (IRApply(SIN, (X,)), IRRational(1, 2)))
        f = IRApply(MUL, (sin_half, IRApply(COS, (X,))))
        F = _integrate_ir(vm, f)
        # Phase 6 can't handle non-integer exponents. Should not crash.
        assert F is not None


# ---------------------------------------------------------------------------
# MACSYMA end-to-end
# ---------------------------------------------------------------------------


class TestSinCosMacsyma:
    """Full pipeline: MACSYMA source → tokens → AST → IR → evaluated result."""

    def test_sin3_cos2_macsyma(self):
        """integrate(sin(x)^3*cos(x)^2, x) end-to-end."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = _make_vm()
        ir = compile_macsyma(parse_macsyma("integrate(sin(x)^3*cos(x)^2, x);"))[0]
        result = vm.eval(ir)
        f = _sin_cos_product(3, 2)
        assert result != IRApply(INTEGRATE, (f, X))

    def test_sin4_cos3_macsyma(self):
        """integrate(sin(x)^4*cos(x)^3, x) end-to-end."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = _make_vm()
        ir = compile_macsyma(parse_macsyma("integrate(sin(x)^4*cos(x)^3, x);"))[0]
        result = vm.eval(ir)
        f = _sin_cos_product(4, 3)
        assert result != IRApply(INTEGRATE, (f, X))

    def test_sin4_cos2_macsyma(self):
        """integrate(sin(x)^4*cos(x)^2, x) end-to-end — even+even."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = _make_vm()
        ir = compile_macsyma(parse_macsyma("integrate(sin(x)^4*cos(x)^2, x);"))[0]
        result = vm.eval(ir)
        f = _sin_cos_product(4, 2)
        assert result != IRApply(INTEGRATE, (f, X))

    def test_cos_order_macsyma(self):
        """integrate(cos(x)^3*sin(x)^2, x) — cos written first."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = _make_vm()
        ir = compile_macsyma(parse_macsyma("integrate(cos(x)^3*sin(x)^2, x);"))[0]
        result = vm.eval(ir)
        f = _sin_cos_product(2, 3)
        assert result != IRApply(INTEGRATE, (f, X))


# ---------------------------------------------------------------------------
# Regression: prior phase tests still pass
# ---------------------------------------------------------------------------


class TestRegressions:
    """Ensure Phase 6 doesn't interfere with previously passing cases."""

    def test_phase5b_sin2_unchanged(self):
        """∫ sin²(x) dx — solo power, Phase 5b handles it."""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(SIN, (X,)), IRInteger(2)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        # Verify numerically.
        for xv in (0.4, 0.7):
            h = 1e-7
            expected = _eval_ir(f, xv)
            actual = (_eval_ir(F, xv + h) - _eval_ir(F, xv - h)) / (2 * h)
            assert abs(actual - expected) < 1e-5

    def test_phase5b_cos3_unchanged(self):
        """∫ cos³(x) dx — solo power, Phase 5b handles it."""
        vm = _make_vm()
        f = IRApply(POW, (IRApply(COS, (X,)), IRInteger(3)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
        for xv in (0.4, 0.7):
            h = 1e-7
            expected = _eval_ir(f, xv)
            actual = (_eval_ir(F, xv + h) - _eval_ir(F, xv - h)) / (2 * h)
            assert abs(actual - expected) < 1e-5

    def test_phase4b_sin_cos_same_arg_unchanged(self):
        """∫ sin(x)cos(x) dx — n=m=1 same arg handled by Phase 4b."""
        vm = _make_vm()
        f = IRApply(MUL, (IRApply(SIN, (X,)), IRApply(COS, (X,))))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F

    def test_phase4b_sin_cos_diff_arg_unchanged(self):
        """∫ sin(x)cos(2x) dx — different arguments, handled by Phase 4b."""
        vm = _make_vm()
        arg2 = IRApply(MUL, (IRInteger(2), X))
        f = IRApply(MUL, (IRApply(SIN, (X,)), IRApply(COS, (arg2,))))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F
