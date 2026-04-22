"""Phase 8 integration tests — power-of-composite u-substitution.

∫ f(g(x))ⁿ · c·g'(x) dx = c · F(g(x)) + C

Two main families:
  Case A: outer = f(g(x))^n where f is a 1-arg function (SIN, COS, TAN)
  Case B: outer = g(x)^n   where g is a general non-linear expression

Bonus: single-factor (ax+b)^n in the POW branch.

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
    NEG,
    POW,
    SIN,
    SUB,
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
# Shared helpers (same pattern as Phase 7 tests)
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


def _add(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(ADD, (a, b))


def _sub(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(SUB, (a, b))


def _neg(a: IRNode) -> IRNode:
    return IRApply(NEG, (a,))


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


def _pow(base: IRNode, n: int) -> IRNode:
    return IRApply(POW, (base, IRInteger(n)))


def _xn(n: int) -> IRNode:
    """x^n as IR."""
    if n == 1:
        return X
    return IRApply(POW, (X, IRInteger(n)))


def _cx(c: int) -> IRNode:
    """c·x as IR."""
    return IRApply(MUL, (IRInteger(c), X))


def _linear(a: int, b: int) -> IRNode:
    """a·x + b as IR."""
    return _add(_cx(a), IRInteger(b))


# ---------------------------------------------------------------------------
# TestPhase8_CaseA_Trig — POW(f(g(x)), n) · g'(x) for trig/poly inner g
# ---------------------------------------------------------------------------


class TestPhase8_CaseA_Trig:
    """∫ f(g(x))^n · g'(x) dx via u = g(x), inner = Phase 5b/5c."""

    def test_cos_sin2_sin(self):
        """∫ cos(x)·sin²(sin(x)) dx — n=2, g=sin(x)"""
        vm = _make_vm()
        f = _mul(_cos(X), _pow(_sin(_sin(X)), 2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cos_sin3_sin(self):
        """∫ cos(x)·sin³(sin(x)) dx — n=3, g=sin(x)"""
        vm = _make_vm()
        f = _mul(_cos(X), _pow(_sin(_sin(X)), 3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cos_sin4_sin(self):
        """∫ cos(x)·sin⁴(sin(x)) dx — n=4"""
        vm = _make_vm()
        f = _mul(_cos(X), _pow(_sin(_sin(X)), 4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_neg_sin_cos2_cos(self):
        """∫ −sin(x)·cos²(cos(x)) dx — cos outer, g=cos(x), g'=−sin(x)"""
        vm = _make_vm()
        f = _mul(_neg(_sin(X)), _pow(_cos(_cos(X)), 2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cos_cos3_sin(self):
        """∫ cos(x)·cos³(sin(x)) dx — cos outer, sin inner"""
        vm = _make_vm()
        f = _mul(_cos(X), _pow(_cos(_sin(X)), 3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cos_tan2_sin(self):
        """∫ cos(x)·tan²(sin(x)) dx — tan outer, Phase 5c inner integral"""
        vm = _make_vm()
        f = _mul(_cos(X), _pow(_tan(_sin(X)), 2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_2x_sin2_x2(self):
        """∫ 2x·sin²(x²) dx — polynomial inner g=x²"""
        vm = _make_vm()
        f = _mul(_cx(2), _pow(_sin(_xn(2)), 2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_2x_cos2_x2(self):
        """∫ 2x·cos²(x²) dx — polynomial inner g=x²"""
        vm = _make_vm()
        f = _mul(_cx(2), _pow(_cos(_xn(2)), 2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_3x2_sin2_x3(self):
        """∫ 3x²·sin²(x³) dx — polynomial inner g=x³"""
        vm = _make_vm()
        f = _mul(_mul(IRInteger(3), _xn(2)), _pow(_sin(_xn(3)), 2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_2cos2x_sin2_sin2x(self):
        """∫ 2·cos(2x)·sin²(sin(2x)) dx — scaled linear arg, g=sin(2x)"""
        vm = _make_vm()
        two_x = _cx(2)
        f = _mul(_mul(IRInteger(2), _cos(two_x)), _pow(_sin(_sin(two_x)), 2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# TestPhase8_CaseB_Poly — POW(g(x), n) · c·g'(x) for various g
# ---------------------------------------------------------------------------


class TestPhase8_CaseB_Poly:
    """∫ g(x)^n · c·g'(x) dx via u = g(x), inner = Phase 1 power rule."""

    def test_2x_x2p1_cube(self):
        """∫ 2x·(x²+1)³ dx = (x²+1)⁴/4"""
        vm = _make_vm()
        g = _add(_xn(2), IRInteger(1))
        f = _mul(_cx(2), _pow(g, 3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_2x_x2p1_inv(self):
        """∫ 2x·(x²+1)⁻¹ dx = log(x²+1)  [n=−1 special case]"""
        vm = _make_vm()
        g = _add(_xn(2), IRInteger(1))
        f = _mul(_cx(2), _pow(g, -1))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_3x2_x3p5_quad(self):
        """∫ 3x²·(x³+5)⁴ dx = (x³+5)⁵/5"""
        vm = _make_vm()
        g = _add(_xn(3), IRInteger(5))
        f = _mul(_mul(IRInteger(3), _xn(2)), _pow(g, 4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_3x2_x3p5_neg2(self):
        """∫ 3x²·(x³+5)⁻² dx = −1/(x³+5)"""
        vm = _make_vm()
        g = _add(_xn(3), IRInteger(5))
        f = _mul(_mul(IRInteger(3), _xn(2)), _pow(g, -2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_x2p1_cube_half(self):
        """∫ x·(x²+1)³ dx = (x²+1)⁴/8  [c=1/2 rational scale]"""
        vm = _make_vm()
        g = _add(_xn(2), IRInteger(1))
        f = _mul(X, _pow(g, 3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_4x3_x4m1_sq(self):
        """∫ 4x³·(x⁴−1)² dx = (x⁴−1)³/3"""
        vm = _make_vm()
        g = _sub(_xn(4), IRInteger(1))
        f = _mul(_mul(IRInteger(4), _xn(3)), _pow(g, 2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cos_sinp1_cube(self):
        """∫ cos(x)·(sin(x)+1)³ dx = (sin(x)+1)⁴/4  [trig inner g]"""
        vm = _make_vm()
        g = _add(_sin(X), IRInteger(1))
        f = _mul(_cos(X), _pow(g, 3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_expp1_quad(self):
        """∫ exp(x)·(exp(x)+1)⁴ dx = (exp(x)+1)⁵/5  [tests ADD diff extension]"""
        vm = _make_vm()
        g = _add(_exp(X), IRInteger(1))
        f = _mul(_exp(X), _pow(g, 4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_exp_expm1_sq(self):
        """∫ exp(x)·(exp(x)−1)² dx = (exp(x)−1)³/3  [tests SUB diff extension]"""
        vm = _make_vm()
        g = _sub(_exp(X), IRInteger(1))
        f = _mul(_exp(X), _pow(g, 2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_2x_x2p3_fifth(self):
        """∫ 2x·(x²+3)⁵ dx = (x²+3)⁶/6"""
        vm = _make_vm()
        g = _add(_xn(2), IRInteger(3))
        f = _mul(_cx(2), _pow(g, 5))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# TestPhase8_LinearPow — single-factor (ax+b)^n in the POW branch
# ---------------------------------------------------------------------------


class TestPhase8_LinearPow:
    """∫ (ax+b)^n dx — POW branch linear-base extension."""

    def test_2xp1_cube(self):
        """∫ (2x+1)³ dx = (2x+1)⁴/8"""
        vm = _make_vm()
        f = _pow(_linear(2, 1), 3)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_3xm2_quad(self):
        """∫ (3x−2)⁴ dx = (3x−2)⁵/15"""
        vm = _make_vm()
        f = _pow(_sub(_cx(3), IRInteger(2)), 4)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_xp5_inv(self):
        """∫ (x+5)⁻¹ dx = log(x+5)"""
        vm = _make_vm()
        f = _pow(_add(X, IRInteger(5)), -1)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        # Shift test points to avoid x=-5 singularity (0.4, 0.7 are fine)
        _check_antiderivative(f, F)

    def test_2xp1_neg2(self):
        """∫ (2x+1)⁻² dx = −1/(2·(2x+1))"""
        vm = _make_vm()
        f = _pow(_linear(2, 1), -2)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_half_xp1_cube(self):
        """∫ (x/2+1)³ dx = (x/2+1)⁴/2"""
        vm = _make_vm()
        half_x_p1 = _add(IRApply(MUL, (IRRational(1, 2), X)), IRInteger(1))
        f = _pow(half_x_p1, 3)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_const_factor_linear_pow(self):
        """∫ 3·(2x+1)² dx — constant factor rule + linear POW branch"""
        vm = _make_vm()
        lin_pow = _pow(_linear(2, 1), 2)
        f = _mul(IRInteger(3), lin_pow)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# TestPhase8_Fallthrough — cases Phase 8 should decline
# ---------------------------------------------------------------------------


class TestPhase8_Fallthrough:
    """Phase 8 must return None for these cases; earlier phases may handle them."""

    def test_sin_x_sin_x2_unevaluated(self):
        """∫ sin(x)·sin(x²) dx — different args, no ratio match → unevaluated"""
        vm = _make_vm()
        f = _mul(_sin(X), _sin(_xn(2)))
        F = _integrate_ir(vm, f)
        # No phase handles sin(x)·sin(x²): product-to-sum needs both linear args.
        assert IRApply(INTEGRATE, (f, X)) == F, (
            "Expected unevaluated Integrate for sin(x)·sin(x²)"
        )

    def test_sin_cos_same_arg_phase6(self):
        """∫ sin²(x)·cos(x) dx — Phase 6 (sinⁿ·cosᵐ) handles this, not Phase 8."""
        vm = _make_vm()
        f = _mul(_pow(_sin(X), 2), _cos(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_poly_pow_alone_unevaluated(self):
        """∫ (x²+1)³ dx — no g' factor; POW branch can't simplify → unevaluated."""
        vm = _make_vm()
        g = _add(_xn(2), IRInteger(1))
        f = _pow(g, 3)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) == F, (
            "Expected unevaluated Integrate for (x²+1)³ alone"
        )

    def test_exp_sin3_unevaluated(self):
        """∫ exp(x)·sin³(x) dx — no phase handles exp × sin^n → unevaluated."""
        vm = _make_vm()
        f = _mul(_exp(X), _pow(_sin(X), 3))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) == F, (
            "Expected unevaluated Integrate for exp(x)·sin³(x)"
        )


# ---------------------------------------------------------------------------
# TestPhase8_Regressions — verify earlier phases still work
# ---------------------------------------------------------------------------


class TestPhase8_Regressions:
    """Ensure Phases 1–7 are unaffected by Phase 8 additions."""

    def test_phase7_x_sin_x2(self):
        """Phase 7 regression: ∫ x·sin(x²) dx"""
        vm = _make_vm()
        f = _mul(X, _sin(_xn(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase7_cos_exp_sin(self):
        """Phase 7 regression: ∫ cos(x)·exp(sin(x)) dx"""
        vm = _make_vm()
        f = _mul(_cos(X), _exp(_sin(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase6_sin3_cos2(self):
        """Phase 6 regression: ∫ sin³(x)·cos²(x) dx"""
        vm = _make_vm()
        f = _mul(_pow(_sin(X), 3), _pow(_cos(X), 2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase5_sin2(self):
        """Phase 5 regression: ∫ sin²(x) dx"""
        vm = _make_vm()
        f = _pow(_sin(X), 2)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase5_sin2_linear_arg(self):
        """Phase 5 regression: ∫ sin²(2x+1) dx"""
        vm = _make_vm()
        f = _pow(_sin(_linear(2, 1)), 2)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase3_exp_linear(self):
        """Phase 3 regression: ∫ exp(2x+1) dx"""
        vm = _make_vm()
        f = _exp(_linear(2, 1))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# TestPhase8_Macsyma — end-to-end MACSYMA string tests
# ---------------------------------------------------------------------------


class TestPhase8_Macsyma:
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
        f_ir = integrate_ir.args[0]
        return f_ir, F, integrate_ir

    def test_macsyma_case_a_trig(self):
        """MACSYMA: integrate(cos(x)*sin(sin(x))^2, x)"""
        f, F, _ = self._parse_and_integrate("cos(x)*sin(sin(x))^2")
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_macsyma_case_b_poly(self):
        """MACSYMA: integrate(2*x*(x^2+1)^3, x)"""
        f, F, _ = self._parse_and_integrate("2*x*(x^2+1)^3")
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_macsyma_case_b_exp(self):
        """MACSYMA: integrate(exp(x)*(exp(x)+1)^4, x)"""
        f, F, _ = self._parse_and_integrate("exp(x)*(exp(x)+1)^4")
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_macsyma_linear_pow(self):
        """MACSYMA: integrate((2*x+1)^3, x)"""
        f, F, _ = self._parse_and_integrate("(2*x+1)^3")
        _was_evaluated(f, F)
        _check_antiderivative(f, F)
