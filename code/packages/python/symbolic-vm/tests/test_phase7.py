"""Phase 7 integration tests — u-substitution (chain-rule reversal).

∫ f(g(x)) · g'(x) dx = F(g(x)) + C

Three families:
  - Power inner:     g = polynomial (x², x³, x²+1, …)
  - Trig inner:      g = sin(x), cos(x)
  - Exp inner:       g = exp(x), exp(2x), …

Correctness is verified numerically: differentiate the antiderivative at test
points and confirm the result matches the original integrand.

Test points: x₀ = 0.4, x₁ = 0.7 — safely away from trig/log singularities.
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    COS,
    EXP,
    INTEGRATE,
    LOG,
    MUL,
    POW,
    SIN,
    SQRT,
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
        return math.sqrt(abs(_eval_ir(node.args[0], x_val)))
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
    return vm.eval(IRApply(INTEGRATE, (integrand_ir, X)))


def _was_evaluated(f: IRNode, F: IRNode) -> None:
    """Assert the integral was not left unevaluated."""
    assert IRApply(INTEGRATE, (f, X)) != F, (
        "Expected a closed-form antiderivative, got an unevaluated Integrate"
    )


# ---------------------------------------------------------------------------
# Small IR builder helpers
# ---------------------------------------------------------------------------


def _mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _sin(arg: IRNode) -> IRNode:
    return IRApply(SIN, (arg,))


def _cos(arg: IRNode) -> IRNode:
    return IRApply(COS, (arg,))


def _exp(arg: IRNode) -> IRNode:
    return IRApply(EXP, (arg,))


def _log(arg: IRNode) -> IRNode:
    return IRApply(LOG, (arg,))


def _tan(arg: IRNode) -> IRNode:
    return IRApply(TAN, (arg,))


def _sqrt(arg: IRNode) -> IRNode:
    return IRApply(SQRT, (arg,))


def _pow(base: IRNode, n: int) -> IRNode:
    return IRApply(POW, (base, IRInteger(n)))


def _xn(n: int) -> IRNode:
    """x^n as IR."""
    if n == 1:
        return X
    return IRApply(POW, (X, IRInteger(n)))


# ---------------------------------------------------------------------------
# TestUSub_PowerInner — g(x) is a polynomial, g' is also a polynomial
# ---------------------------------------------------------------------------


class TestUSub_PowerInner:
    """∫ p(x)·f(x^n or poly) dx via u = polynomial substitution."""

    def test_x_sin_x2(self):
        """∫ x·sin(x²) dx = −cos(x²)/2"""
        vm = _make_vm()
        f = _mul(X, _sin(_xn(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_cos_x2(self):
        """∫ x·cos(x²) dx = sin(x²)/2"""
        vm = _make_vm()
        f = _mul(X, _cos(_xn(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_exp_x2(self):
        """∫ x·exp(x²) dx = exp(x²)/2"""
        vm = _make_vm()
        f = _mul(X, _exp(_xn(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x2_sin_x3(self):
        """∫ x²·sin(x³) dx = −cos(x³)/3"""
        vm = _make_vm()
        f = _mul(_xn(2), _sin(_xn(3)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x2_cos_x3(self):
        """∫ x²·cos(x³) dx = sin(x³)/3"""
        vm = _make_vm()
        f = _mul(_xn(2), _cos(_xn(3)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x2_exp_x3(self):
        """∫ x²·exp(x³) dx = exp(x³)/3"""
        vm = _make_vm()
        f = _mul(_xn(2), _exp(_xn(3)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x3_exp_x4(self):
        """∫ x³·exp(x⁴) dx = exp(x⁴)/4"""
        vm = _make_vm()
        f = _mul(_xn(3), _exp(_xn(4)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_sin_x2_plus1(self):
        """∫ x·sin(x²+1) dx = −cos(x²+1)/2"""
        vm = _make_vm()
        g = IRApply(ADD, (_xn(2), IRInteger(1)))
        f = _mul(X, _sin(g))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_exp_x2_minus1(self):
        """∫ x·exp(x²−1) dx = exp(x²−1)/2"""
        from symbolic_ir import SUB
        vm = _make_vm()
        g = IRApply(SUB, (_xn(2), IRInteger(1)))
        f = _mul(X, _exp(g))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_3x2_sin_x3_minus1(self):
        """∫ 3x²·sin(x³−1) dx = −cos(x³−1)"""
        from symbolic_ir import SUB
        vm = _make_vm()
        g = IRApply(SUB, (_xn(3), IRInteger(1)))
        f = _mul(_mul(IRInteger(3), _xn(2)), _sin(g))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# TestUSub_TrigInner — g(x) = sin(x) or cos(x)
# ---------------------------------------------------------------------------


class TestUSub_TrigInner:
    """∫ trig(x)·f(sin(x) or cos(x)) dx via trig substitution."""

    def test_cos_exp_sin(self):
        """∫ cos(x)·exp(sin(x)) dx = exp(sin(x))"""
        vm = _make_vm()
        f = _mul(_cos(X), _exp(_sin(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sin_exp_cos(self):
        """∫ sin(x)·exp(cos(x)) dx = −exp(cos(x))"""
        vm = _make_vm()
        f = _mul(_sin(X), _exp(_cos(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cos_sin_of_sin(self):
        """∫ cos(x)·sin(sin(x)) dx = −cos(sin(x))"""
        vm = _make_vm()
        f = _mul(_cos(X), _sin(_sin(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sin_cos_of_cos(self):
        """∫ sin(x)·cos(cos(x)) dx = −sin(cos(x))"""
        vm = _make_vm()
        f = _mul(_sin(X), _cos(_cos(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cos_tan_of_sin(self):
        """∫ cos(x)·tan(sin(x)) dx = −log(cos(sin(x)))"""
        vm = _make_vm()
        f = _mul(_cos(X), _tan(_sin(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.4, 0.7))

    def test_cos_log_sin(self):
        """∫ cos(x)·log(sin(x)) dx = sin(x)·log(sin(x)) − sin(x)"""
        vm = _make_vm()
        f = _mul(_cos(X), _log(_sin(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.4, 0.7))

    def test_sin_log_cos(self):
        """∫ sin(x)·log(cos(x)) dx = −cos(x)·log(cos(x)) + cos(x)"""
        vm = _make_vm()
        f = _mul(_sin(X), _log(_cos(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.4, 0.7))

    def test_cos_exp_sin_scaled(self):
        """∫ cos(2x)·exp(sin(2x)) dx = exp(sin(2x))/2"""
        vm = _make_vm()
        arg2x = IRApply(MUL, (IRInteger(2), X))
        f = _mul(_cos(arg2x), _exp(_sin(arg2x)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# TestUSub_ExpInner — g(x) = exp(linear)
# ---------------------------------------------------------------------------


class TestUSub_ExpInner:
    """∫ exp(x)·f(exp(x)) dx via u = exp(x) substitution."""

    def test_exp_sin_exp(self):
        """∫ exp(x)·sin(exp(x)) dx = −cos(exp(x))"""
        vm = _make_vm()
        f = _mul(_exp(X), _sin(_exp(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_cos_exp(self):
        """∫ exp(x)·cos(exp(x)) dx = sin(exp(x))"""
        vm = _make_vm()
        f = _mul(_exp(X), _cos(_exp(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_exp_exp(self):
        """∫ exp(x)·exp(exp(x)) dx = exp(exp(x))"""
        vm = _make_vm()
        f = _mul(_exp(X), _exp(_exp(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp2x_sin_exp2x(self):
        """∫ exp(2x)·sin(exp(2x)) dx = −cos(exp(2x))/2"""
        vm = _make_vm()
        arg2x = IRApply(MUL, (IRInteger(2), X))
        f = _mul(_exp(arg2x), _sin(_exp(arg2x)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_2exp2x_sin_exp2x(self):
        """∫ 2·exp(2x)·sin(exp(2x)) dx = −cos(exp(2x))"""
        vm = _make_vm()
        arg2x = IRApply(MUL, (IRInteger(2), X))
        fa = IRApply(MUL, (IRInteger(2), _exp(arg2x)))
        f = _mul(fa, _sin(_exp(arg2x)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_tan_exp(self):
        """∫ exp(x)·tan(exp(x)) dx = −log(cos(exp(x)))"""
        vm = _make_vm()
        f = _mul(_exp(X), _tan(_exp(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.5))


# ---------------------------------------------------------------------------
# TestUSub_LinearArgBypass — linear inner args must NOT be stolen from earlier phases
# ---------------------------------------------------------------------------


class TestUSub_LinearArgBypass:
    """Phase 7 must not fire for f(ax+b) — those are handled by Phases 3–5."""

    def test_x_sin_x_goes_to_phase4a(self):
        """∫ x·sin(x) dx — Phase 4a (poly×trig), not Phase 7."""
        vm = _make_vm()
        f = _mul(X, _sin(X))
        F = _integrate_ir(vm, f)
        # Must be evaluated (by Phase 4a)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_sin_2x_goes_to_phase4a(self):
        """∫ x·sin(2x) dx — Phase 4a, not Phase 7 (g=2x is linear)."""
        vm = _make_vm()
        arg = IRApply(MUL, (IRInteger(2), X))
        f = _mul(X, _sin(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_linear_times_sin_linear_phase4c(self):
        """∫ exp(x)·sin(x) dx — Phase 4c, not Phase 7."""
        vm = _make_vm()
        f = _mul(_exp(X), _sin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sin_x_cos_x_phase4b(self):
        """∫ sin(x)·cos(x) dx — Phase 4b (product-to-sum), not Phase 7."""
        vm = _make_vm()
        f = _mul(_sin(X), _cos(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# TestUSub_Fallthrough — inputs Phase 7 must NOT integrate (return None)
# ---------------------------------------------------------------------------


class TestUSub_Fallthrough:
    """Inputs that Phase 7 should decline (no matching u-sub pattern)."""

    def test_sin_x_times_sin_x2_no_match(self):
        """∫ sin(x)·sin(x²) dx — no g such that one factor is c·g'(x)."""
        vm = _make_vm()
        f = _mul(_sin(X), _sin(_xn(2)))
        F = _integrate_ir(vm, f)
        # Should remain unevaluated (no phase handles this)
        assert IRApply(INTEGRATE, (f, X)) == F, (
            f"Expected unevaluated Integrate, got {F!r}"
        )

    def test_different_arg_trig_no_match(self):
        """∫ sin(x)·exp(cos(2x)) dx — g=cos(2x) but g'=-2sin(2x)≠sin(x)."""
        vm = _make_vm()
        arg2x = IRApply(MUL, (IRInteger(2), X))
        f = _mul(_sin(X), _exp(_cos(arg2x)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) == F

    def test_x2_sin_x_no_match(self):
        """∫ x²·sin(x) dx — g=x, but g=x is skipped (Phase 7 guard)."""
        vm = _make_vm()
        f = _mul(_xn(2), _sin(X))
        F = _integrate_ir(vm, f)
        # Phase 4a handles this — must NOT be left unevaluated
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sin_x3_no_matching_factor(self):
        """∫ sin(x³) dx alone — no product, left unevaluated."""
        vm = _make_vm()
        f = _sin(_xn(3))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) == F


# ---------------------------------------------------------------------------
# TestUSub_Regressions — earlier phases still work
# ---------------------------------------------------------------------------


class TestUSub_Regressions:
    """Verify that Phase 7 does not break earlier integrations."""

    def test_sin3_cos2_phase6(self):
        """∫ sin³(x)·cos²(x) dx — Phase 6 still handles this."""
        vm = _make_vm()
        sin3 = _pow(_sin(X), 3)
        cos2 = _pow(_cos(X), 2)
        f = _mul(sin3, cos2)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sin2_phase5b(self):
        """∫ sin²(x) dx — Phase 5b still works."""
        vm = _make_vm()
        f = _pow(_sin(X), 2)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_x_sin_x_phase4c(self):
        """∫ exp(x)·sin(x) dx — Phase 4c still works."""
        vm = _make_vm()
        f = _mul(_exp(X), _sin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_poly_trig_phase4a(self):
        """∫ x²·cos(x) dx — Phase 4a still works."""
        vm = _make_vm()
        f = _mul(_xn(2), _cos(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_log_x_phase3(self):
        """∫ x·log(x) dx — Phase 3 (log×poly) still works."""
        vm = _make_vm()
        f = _mul(X, _log(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.5, 1.2))


# ---------------------------------------------------------------------------
# TestUSub_Macsyma — end-to-end string parsing tests
# ---------------------------------------------------------------------------


class TestUSub_Macsyma:
    """MACSYMA string → IR → Integrate → numerical verification."""

    def _parse_and_integrate(
        self, expr: str
    ) -> tuple[IRNode, IRNode, IRNode]:
        """Parse ``integrate(expr, x);``, evaluate, return (f_ir, F, integrate_ir)."""
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma
        vm = _make_vm()
        src = f"integrate({expr}, x);"
        integrate_ir = compile_macsyma(parse_macsyma(src))[0]
        F = vm.eval(integrate_ir)
        # Reconstruct integrand IR from the compiled Integrate node.
        f_ir = integrate_ir.args[0]
        return f_ir, F, integrate_ir

    def test_macsyma_x_sin_x2(self):
        """MACSYMA: integrate(x*sin(x^2), x)"""
        f, F, _ = self._parse_and_integrate("x*sin(x^2)")
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_macsyma_x2_exp_x3(self):
        """MACSYMA: integrate(x^2*exp(x^3), x)"""
        f, F, _ = self._parse_and_integrate("x^2*exp(x^3)")
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_macsyma_cos_exp_sin(self):
        """MACSYMA: integrate(cos(x)*exp(sin(x)), x)"""
        f, F, _ = self._parse_and_integrate("cos(x)*exp(sin(x))")
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_macsyma_sin_exp_cos(self):
        """MACSYMA: integrate(sin(x)*exp(cos(x)), x)"""
        f, F, _ = self._parse_and_integrate("sin(x)*exp(cos(x))")
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_macsyma_exp_sin_exp(self):
        """MACSYMA: integrate(exp(x)*sin(exp(x)), x)"""
        f, F, _ = self._parse_and_integrate("exp(x)*sin(exp(x))")
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_macsyma_cos_log_sin(self):
        """MACSYMA: integrate(cos(x)*log(sin(x)), x)"""
        f, F, _ = self._parse_and_integrate("cos(x)*log(sin(x))")
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.4, 0.7))
