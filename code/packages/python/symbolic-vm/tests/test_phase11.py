"""Phase 11 integration tests — polynomial × arctan(linear) integration via IBP.

Tests the formula:

    ∫ P(x) · atan(ax+b) dx
        = Q(x) · atan(ax+b) − a · T(x) − a · arctan_integral(R, D)

where Q = ∫P dx, D = (ax+b)²+1, Q = S·D + R, T = ∫S dx.

Correctness is verified numerically: differentiate the antiderivative at test
points and confirm the result matches the original integrand.

Test points: x₀ = 0.3, x₁ = 0.8 — safely away from singularities.
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    ATAN,
    DIV,
    EXP,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
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
# Shared helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")
_INT = IRInteger
_RAT = lambda n, d: IRRational(n, d)  # noqa: E731


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
    if head == "Log":
        return math.log(abs(_eval_ir(node.args[0], x_val)))
    if head == "Pow":
        return _eval_ir(node.args[0], x_val) ** _eval_ir(node.args[1], x_val)
    if head == "Atan":
        return math.atan(_eval_ir(node.args[0], x_val))
    if head == "Exp":
        return math.exp(_eval_ir(node.args[0], x_val))
    if head == "Sin":
        return math.sin(_eval_ir(node.args[0], x_val))
    if head == "Cos":
        return math.cos(_eval_ir(node.args[0], x_val))
    raise ValueError(f"Unhandled head: {head}")


def _numerical_deriv(node: IRNode, x_val: float, h: float = 1e-7) -> float:
    return (_eval_ir(node, x_val + h) - _eval_ir(node, x_val - h)) / (2 * h)


def _check_antiderivative(
    integrand: IRNode,
    antideriv: IRNode,
    test_points: tuple[float, ...] = (0.3, 0.8),
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
    assert IRApply(INTEGRATE, (f, X)) != F, (
        "Expected a closed-form antiderivative, got an unevaluated Integrate"
    )


def _is_unevaluated(f: IRNode, F: IRNode) -> None:
    assert IRApply(INTEGRATE, (f, X)) == F, (
        "Expected an unevaluated Integrate, got a closed form"
    )


# ---------------------------------------------------------------------------
# IR builder helpers
# ---------------------------------------------------------------------------


def _mul(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(MUL, (a, b))


def _add(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(ADD, (a, b))


def _sub(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(SUB, (a, b))


def _neg(a: IRNode) -> IRNode:
    return IRApply(NEG, (a,))


def _div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _pow(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(POW, (a, b))


def _atan(arg: IRNode) -> IRNode:
    return IRApply(ATAN, (arg,))


# ---------------------------------------------------------------------------
# Class 1: Canonical cases — ∫ xⁿ · atan(x) dx  (a=1, b=0)
# ---------------------------------------------------------------------------


class TestPhase11_Canonical:
    """Canonical cases with a=1, b=0: ∫ xⁿ · atan(x) dx."""

    def test_x_atan_x(self) -> None:
        """∫ x·atan(x) dx = (x²+1)/2 · atan(x) − x/2."""
        vm = _make_vm()
        f = _mul(X, _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_atan_x(self) -> None:
        """∫ x²·atan(x) dx = x³/3 · atan(x) − x²/6 + log(x²+1)/6."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(2)), _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_cubed_atan_x(self) -> None:
        """∫ x³·atan(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(3)), _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_fourth_atan_x(self) -> None:
        """∫ x⁴·atan(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(4)), _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_fifth_atan_x(self) -> None:
        """∫ x⁵·atan(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(5)), _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_const_atan_x_consistency(self) -> None:
        """∫ 1·atan(x) dx should match the Phase 9 bare arctan result."""
        vm = _make_vm()
        f = _atan(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_two_atan_x(self) -> None:
        """∫ 2·atan(x) dx = 2·(x·atan(x) − log(x²+1)/2)."""
        vm = _make_vm()
        f = _mul(_INT(2), _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_atan_order_commutes(self) -> None:
        """∫ atan(x)·x dx should equal ∫ x·atan(x) dx (commutativity of Mul)."""
        vm = _make_vm()
        f1 = _mul(X, _atan(X))
        f2 = _mul(_atan(X), X)
        F1 = _integrate_ir(vm, f1)
        F2 = _integrate_ir(vm, f2)
        _was_evaluated(f1, F1)
        _was_evaluated(f2, F2)
        # Both antiderivatives must satisfy F'(x) = x·atan(x)
        _check_antiderivative(f1, F1)
        _check_antiderivative(f2, F2)

    def test_linear_poly_atan_x(self) -> None:
        """∫ (x+1)·atan(x) dx — P has degree 1 with constant term."""
        vm = _make_vm()
        f = _mul(_add(X, _INT(1)), _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_quadratic_poly_atan_x(self) -> None:
        """∫ (x²+x+1)·atan(x) dx."""
        vm = _make_vm()
        f = _mul(_add(_add(_pow(X, _INT(2)), X), _INT(1)), _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cubic_poly_atan_x(self) -> None:
        """∫ (x³+x)·atan(x) dx."""
        vm = _make_vm()
        f = _mul(_add(_pow(X, _INT(3)), X), _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_negative_coeff_poly(self) -> None:
        """∫ (x² − 3x)·atan(x) dx."""
        vm = _make_vm()
        p = _sub(_pow(X, _INT(2)), _mul(_INT(3), X))
        f = _mul(p, _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 2: Non-trivial linear arguments atan(ax+b) for various a,b
# ---------------------------------------------------------------------------


class TestPhase11_LinearArg:
    """Cases with ax+b where a ≠ 1 or b ≠ 0."""

    def test_x_atan_2x(self) -> None:
        """∫ x·atan(2x) dx."""
        vm = _make_vm()
        f = _mul(X, _atan(_mul(_INT(2), X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_atan_2x_plus1(self) -> None:
        """∫ x·atan(2x+1) dx."""
        vm = _make_vm()
        arg = _add(_mul(_INT(2), X), _INT(1))
        f = _mul(X, _atan(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_atan_2x_plus1(self) -> None:
        """∫ x²·atan(2x+1) dx."""
        vm = _make_vm()
        arg = _add(_mul(_INT(2), X), _INT(1))
        f = _mul(_pow(X, _INT(2)), _atan(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_atan_3x_minus1(self) -> None:
        """∫ x·atan(3x−1) dx."""
        vm = _make_vm()
        arg = _sub(_mul(_INT(3), X), _INT(1))
        f = _mul(X, _atan(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_atan_half_x(self) -> None:
        """∫ x·atan(x/2) dx  (a=1/2, b=0)."""
        vm = _make_vm()
        arg = _mul(_RAT(1, 2), X)
        f = _mul(X, _atan(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_atan_half_x_plus2(self) -> None:
        """∫ x²·atan(x/2 + 2) dx."""
        vm = _make_vm()
        arg = _add(_mul(_RAT(1, 2), X), _INT(2))
        f = _mul(_pow(X, _INT(2)), _atan(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_const_atan_2x_plus1(self) -> None:
        """∫ atan(2x+1) dx — bare shifted/scaled arctan (P=1)."""
        vm = _make_vm()
        arg = _add(_mul(_INT(2), X), _INT(1))
        f = _atan(arg)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_atan_neg_x(self) -> None:
        """∫ x·atan(−x) dx  (a=−1, b=0)."""
        vm = _make_vm()
        f = _mul(X, _atan(_neg(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_rational_a_b(self) -> None:
        """∫ x·atan(x/3 − 1/2) dx."""
        vm = _make_vm()
        arg = _sub(_mul(_RAT(1, 3), X), _RAT(1, 2))
        f = _mul(X, _atan(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 3: Higher-degree polynomials
# ---------------------------------------------------------------------------


class TestPhase11_HigherDegree:
    """Polynomials of degree 3, 4, 5 multiplied by atan(linear)."""

    def test_degree4_poly_atan_x(self) -> None:
        """∫ x⁴·atan(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(4)), _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_degree5_poly_atan_x(self) -> None:
        """∫ x⁵·atan(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(5)), _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cubic_sum_atan_2x(self) -> None:
        """∫ (x³+x²+x)·atan(2x) dx."""
        vm = _make_vm()
        p = _add(_add(_pow(X, _INT(3)), _pow(X, _INT(2))), X)
        f = _mul(p, _atan(_mul(_INT(2), X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_quartic_atan_x_plus1(self) -> None:
        """∫ x⁴·atan(x+1) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(4)), _atan(_add(X, _INT(1))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_mixed_degree3_atan_3x_minus1(self) -> None:
        """∫ (x³ − x + 2)·atan(3x−1) dx."""
        vm = _make_vm()
        p = _add(_sub(_pow(X, _INT(3)), X), _INT(2))
        arg = _sub(_mul(_INT(3), X), _INT(1))
        f = _mul(p, _atan(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 4: Fallthrough cases — Phase 11 must NOT evaluate these
# ---------------------------------------------------------------------------


class TestPhase11_Fallthrough:
    """Integrands that Phase 11 cannot handle — should remain unevaluated."""

    def test_nonlinear_atan_arg(self) -> None:
        """∫ atan(x²+1) dx — non-linear arctan arg, should be unevaluated."""
        vm = _make_vm()
        f = _atan(_add(_pow(X, _INT(2)), _INT(1)))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_atan_squared(self) -> None:
        """∫ atan(x)² dx — n≥2 power of arctan, should be unevaluated."""
        vm = _make_vm()
        f = _pow(_atan(X), _INT(2))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_nonlinear_atan_trig(self) -> None:
        """∫ atan(sin(x)) dx — transcendental arg, should be unevaluated."""
        from symbolic_ir import SIN
        from symbolic_ir import IRApply as _A
        vm = _make_vm()
        f = _atan(_A(SIN, (X,)))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_rational_times_atan(self) -> None:
        """∫ (1/x)·atan(x) dx — rational (non-polynomial) coefficient, unevaluated."""
        vm = _make_vm()
        f = _mul(_div(_INT(1), X), _atan(X))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_atan_nonlinear_quadratic(self) -> None:
        """∫ atan(x²+1) dx — non-linear arg, should be unevaluated."""
        vm = _make_vm()
        f = _atan(_add(_pow(X, _INT(2)), _INT(1)))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)


# ---------------------------------------------------------------------------
# Class 5: Regressions — verify earlier phases still work
# ---------------------------------------------------------------------------


class TestPhase11_Regressions:
    """Verify that Phase 11 does not break earlier integration phases."""

    def test_phase9_bare_atan_x(self) -> None:
        """Phase 9: ∫ atan(x) dx."""
        vm = _make_vm()
        f = _atan(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase9_bare_atan_shifted(self) -> None:
        """Phase 9: ∫ atan(2x+1) dx."""
        vm = _make_vm()
        f = _atan(_add(_mul(_INT(2), X), _INT(1)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase10_three_quadratics(self) -> None:
        """Phase 10: ∫ 1/((x²+1)(x²+4)(x²+9)) dx."""
        vm = _make_vm()
        q1 = _add(_pow(X, _INT(2)), _INT(1))
        q2 = _add(_pow(X, _INT(2)), _INT(4))
        q3 = _add(_pow(X, _INT(2)), _INT(9))
        denom = _mul(_mul(q1, q2), q3)
        f = _div(_INT(1), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase3e_poly_log(self) -> None:
        """Phase 3e: ∫ x·log(x) dx = x²/2·log(x) − x²/4."""
        vm = _make_vm()
        f = _mul(X, IRApply(LOG, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.5, 1.5))

    def test_phase2e_arctan_rational(self) -> None:
        """Phase 2e: ∫ 1/(x²+1) dx = atan(x)."""
        vm = _make_vm()
        f = _div(_INT(1), _add(_pow(X, _INT(2)), _INT(1)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase1_polynomial(self) -> None:
        """Phase 1: ∫ x³ dx = x⁴/4."""
        vm = _make_vm()
        f = _pow(X, _INT(3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase4a_poly_exp(self) -> None:
        """Phase 4a: ∫ x·eˣ dx = eˣ(x−1)."""
        vm = _make_vm()
        f = _mul(X, IRApply(EXP, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase2d_rational_log(self) -> None:
        """Phase 2d: ∫ 1/(x−1) dx = log(|x−1|)."""
        vm = _make_vm()
        f = _div(_INT(1), _sub(X, _INT(1)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(1.3, 2.5))


# ---------------------------------------------------------------------------
# Class 6: Macsyma end-to-end string tests
# ---------------------------------------------------------------------------


class TestPhase11_Macsyma:
    """End-to-end tests via the Macsyma string interface."""

    @staticmethod
    def _integrate_macsyma(integrand: str, var: str = "x") -> IRNode:
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma
        vm = _make_vm()
        ast = parse_macsyma(f"integrate({integrand}, {var});")
        integrate_ir = compile_macsyma(ast)[0]
        return vm.eval(integrate_ir)

    def test_x_atan_x_macsyma(self) -> None:
        """integrate(x*atan(x), x) via Macsyma."""
        F = self._integrate_macsyma("x*atan(x)")
        f = _mul(X, _atan(X))
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_atan_x_macsyma(self) -> None:
        """integrate(x^2*atan(x), x) via Macsyma."""
        F = self._integrate_macsyma("x^2*atan(x)")
        f = _mul(_pow(X, _INT(2)), _atan(X))
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_atan_2x_plus1_macsyma(self) -> None:
        """integrate(x*atan(2*x+1), x) via Macsyma."""
        F = self._integrate_macsyma("x*atan(2*x+1)")
        arg = _add(_mul(_INT(2), X), _INT(1))
        f = _mul(X, _atan(arg))
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cubic_atan_x_macsyma(self) -> None:
        """integrate((x^3+x)*atan(x), x) via Macsyma."""
        F = self._integrate_macsyma("(x^3+x)*atan(x)")
        f = _mul(_add(_pow(X, _INT(3)), X), _atan(X))
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_atan_shifted_bare_macsyma(self) -> None:
        """integrate(atan(3*x-1), x) via Macsyma."""
        F = self._integrate_macsyma("atan(3*x-1)")
        arg = _sub(_mul(_INT(3), X), _INT(1))
        f = _atan(arg)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)
