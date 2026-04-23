"""Phase 12 integration tests — polynomial × asin/acos(linear) via IBP.

Tests the formulae:

    ∫ P(x) · asin(ax+b) dx
        = [Q(x) − B(ax+b)] · asin(ax+b) − A(ax+b) · √(1−(ax+b)²)

    ∫ P(x) · acos(ax+b) dx
        = Q(x) · acos(ax+b) + A(ax+b) · √(1−(ax+b)²) + B(ax+b) · asin(ax+b)

where Q = ∫P dx and A, B are computed from the residual integral
    ∫ Q̃(t)/√(1−t²) dt = A(t)·√(1−t²) + B(t)·asin(t).

Correctness is verified numerically: differentiate the antiderivative at test
points and confirm the result matches the original integrand.

Test points: x₀ = 0.3, x₁ = 0.6 — safely inside |ax+b| < 1 for all
linear arguments used here.
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ACOS,
    ADD,
    ASIN,
    ATAN,
    DIV,
    EXP,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
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

# Test points safely inside |x| < 1 and |ax+b| < 1 for all test cases.
_TP = (0.3, 0.6)


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
        base = _eval_ir(node.args[0], x_val)
        exp = _eval_ir(node.args[1], x_val)
        return base ** exp
    if head == "Sqrt":
        return math.sqrt(_eval_ir(node.args[0], x_val))
    if head == "Atan":
        return math.atan(_eval_ir(node.args[0], x_val))
    if head == "Asin":
        return math.asin(_eval_ir(node.args[0], x_val))
    if head == "Acos":
        return math.acos(_eval_ir(node.args[0], x_val))
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
    test_points: tuple[float, ...] = _TP,
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
            f"diff={abs(actual - expected):.2e}"
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


def _asin(arg: IRNode) -> IRNode:
    return IRApply(ASIN, (arg,))


def _acos(arg: IRNode) -> IRNode:
    return IRApply(ACOS, (arg,))


# ---------------------------------------------------------------------------
# Class 1: Asin canonical cases — ∫ xⁿ · asin(x) dx  (a=1, b=0)
# ---------------------------------------------------------------------------


class TestPhase12_AsinCanonical:
    """Canonical asin cases: ∫ xⁿ · asin(x) dx and small combinations."""

    def test_asin_x(self) -> None:
        """∫ asin(x) dx = x·asin(x) + √(1−x²)."""
        vm = _make_vm()
        f = _asin(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_asin_x(self) -> None:
        """∫ x·asin(x) dx = (x²/2 − 1/4)·asin(x) + x/4·√(1−x²)."""
        vm = _make_vm()
        f = _mul(X, _asin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_asin_x(self) -> None:
        """∫ x²·asin(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(2)), _asin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_cubed_asin_x(self) -> None:
        """∫ x³·asin(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(3)), _asin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_fourth_asin_x(self) -> None:
        """∫ x⁴·asin(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(4)), _asin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_plus1_asin_x(self) -> None:
        """∫ (x+1)·asin(x) dx."""
        vm = _make_vm()
        f = _mul(_add(X, _INT(1)), _asin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_plus_x_asin_x(self) -> None:
        """∫ (x²+x)·asin(x) dx."""
        vm = _make_vm()
        f = _mul(_add(_pow(X, _INT(2)), X), _asin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_asin_x_times_x_commutativity(self) -> None:
        """∫ asin(x)·x dx = ∫ x·asin(x) dx (commutativity of MUL args)."""
        vm = _make_vm()
        f1 = _mul(X, _asin(X))
        f2 = _mul(_asin(X), X)
        F1 = _integrate_ir(vm, f1)
        F2 = _integrate_ir(vm, f2)
        _check_antiderivative(f1, F1)
        _check_antiderivative(f2, F2)

    def test_negative_coeff_poly_asin_x(self) -> None:
        """∫ (−x+2)·asin(x) dx."""
        vm = _make_vm()
        f = _mul(_add(_neg(X), _INT(2)), _asin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_fifth_asin_x(self) -> None:
        """∫ x⁵·asin(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(5)), _asin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cubic_sum_asin_x(self) -> None:
        """∫ (x³+x²+x)·asin(x) dx."""
        vm = _make_vm()
        p = _add(_add(_pow(X, _INT(3)), _pow(X, _INT(2))), X)
        f = _mul(p, _asin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_const_times_asin_x(self) -> None:
        """∫ 3·asin(x) dx = 3·(x·asin(x) + √(1−x²))."""
        vm = _make_vm()
        f = _mul(_INT(3), _asin(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 2: Acos canonical cases — ∫ xⁿ · acos(x) dx  (a=1, b=0)
# ---------------------------------------------------------------------------


class TestPhase12_AcosCanonical:
    """Canonical acos cases: ∫ xⁿ · acos(x) dx."""

    def test_acos_x(self) -> None:
        """∫ acos(x) dx = x·acos(x) − √(1−x²)."""
        vm = _make_vm()
        f = _acos(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_acos_x(self) -> None:
        """∫ x·acos(x) dx = x²/2·acos(x) − x/4·√(1−x²) + 1/4·asin(x)."""
        vm = _make_vm()
        f = _mul(X, _acos(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_acos_x(self) -> None:
        """∫ x²·acos(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(2)), _acos(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_cubed_acos_x(self) -> None:
        """∫ x³·acos(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(3)), _acos(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_plus1_acos_x(self) -> None:
        """∫ (x+1)·acos(x) dx."""
        vm = _make_vm()
        f = _mul(_add(X, _INT(1)), _acos(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_const_factor_acos_x(self) -> None:
        """∫ 2·acos(x) dx = 2·(x·acos(x) − √(1−x²))."""
        vm = _make_vm()
        f = _mul(_INT(2), _acos(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_acos_x_times_x_commutativity(self) -> None:
        """∫ acos(x)·x dx = ∫ x·acos(x) dx (commutativity of MUL args)."""
        vm = _make_vm()
        f1 = _mul(X, _acos(X))
        f2 = _mul(_acos(X), X)
        F1 = _integrate_ir(vm, f1)
        F2 = _integrate_ir(vm, f2)
        _check_antiderivative(f1, F1)
        _check_antiderivative(f2, F2)

    def test_x_fourth_acos_x(self) -> None:
        """∫ x⁴·acos(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(4)), _acos(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 3: Linear argument cases — asin/acos(ax+b) × polynomial
# ---------------------------------------------------------------------------


class TestPhase12_LinearArg:
    """Cases with a ≠ 1 or b ≠ 0 in the asin/acos argument."""

    def test_x_asin_2x(self) -> None:
        """∫ x·asin(2x) dx — a=2, b=0."""
        vm = _make_vm()
        arg = _mul(_INT(2), X)
        f = _mul(X, _asin(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.4))

    def test_x_asin_half_x(self) -> None:
        """∫ x·asin(x/2) dx — a=1/2, b=0."""
        vm = _make_vm()
        arg = _div(X, _INT(2))
        f = _mul(X, _asin(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_bare_asin_2x_plus1(self) -> None:
        """∫ asin(2x+1) dx — a=2, b=1. Test points where |2x+1| < 1."""
        vm = _make_vm()
        arg = _add(_mul(_INT(2), X), _INT(1))
        f = _asin(arg)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        # |2*0.1+1| = 1.2 > 1 — choose points in (-1, 0).
        _check_antiderivative(f, F, test_points=(-0.4, -0.1))

    def test_x_squared_asin_half_x(self) -> None:
        """∫ x²·asin(x/2) dx — a=1/2, b=0."""
        vm = _make_vm()
        arg = _div(X, _INT(2))
        f = _mul(_pow(X, _INT(2)), _asin(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_acos_2x(self) -> None:
        """∫ x·acos(2x) dx — a=2, b=0."""
        vm = _make_vm()
        arg = _mul(_INT(2), X)
        f = _mul(X, _acos(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.4))

    def test_bare_acos_3x_minus1(self) -> None:
        """∫ acos(3x−1) dx — a=3, b=−1. Test points where |3x−1| < 1."""
        vm = _make_vm()
        arg = _sub(_mul(_INT(3), X), _INT(1))
        f = _acos(arg)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        # |3*0.1−1| = 0.7 < 1, |3*0.3−1| = 0.1 < 1.
        _check_antiderivative(f, F, test_points=(0.1, 0.3))

    def test_x_asin_neg_x(self) -> None:
        """∫ x·asin(−x) dx — a=−1, b=0."""
        vm = _make_vm()
        arg = _neg(X)
        f = _mul(X, _asin(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_acos_half_x(self) -> None:
        """∫ x·acos(x/2) dx — a=1/2, b=0."""
        vm = _make_vm()
        arg = _div(X, _INT(2))
        f = _mul(X, _acos(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 4: Fallthrough — Phase 12 must NOT evaluate these
# ---------------------------------------------------------------------------


class TestPhase12_Fallthrough:
    """Integrands Phase 12 cannot handle — should remain unevaluated."""

    def test_asin_nonlinear_arg(self) -> None:
        """∫ asin(x²) dx — non-linear arg, should be unevaluated."""
        vm = _make_vm()
        f = _asin(_pow(X, _INT(2)))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_asin_squared(self) -> None:
        """∫ asin(x)² dx — power of asin, should be unevaluated."""
        vm = _make_vm()
        f = _pow(_asin(X), _INT(2))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_rational_times_asin(self) -> None:
        """∫ (1/x)·asin(x) dx — rational (non-polynomial) factor, unevaluated."""
        vm = _make_vm()
        f = _mul(_div(_INT(1), X), _asin(X))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_acos_nonlinear_arg(self) -> None:
        """∫ acos(x²+1) dx — non-linear arg, should be unevaluated."""
        vm = _make_vm()
        f = _acos(_add(_pow(X, _INT(2)), _INT(1)))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)


# ---------------------------------------------------------------------------
# Class 5: Regressions — earlier phases must still work
# ---------------------------------------------------------------------------


class TestPhase12_Regressions:
    """Verify Phase 12 does not break earlier integration phases."""

    def test_phase11_x_atan_x(self) -> None:
        """Phase 11: ∫ x·atan(x) dx."""
        vm = _make_vm()
        f = _mul(X, IRApply(ATAN, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.8))

    def test_phase3e_poly_log(self) -> None:
        """Phase 3e: ∫ x·log(x) dx = x²/2·log(x) − x²/4."""
        vm = _make_vm()
        f = _mul(X, IRApply(LOG, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.5, 1.5))

    def test_phase4a_poly_sin(self) -> None:
        """Phase 4a: ∫ x·sin(x) dx = sin(x) − x·cos(x)."""
        vm = _make_vm()
        f = _mul(X, IRApply(SIN, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.5, 1.2))

    def test_phase1_polynomial(self) -> None:
        """Phase 1: ∫ x³ dx = x⁴/4."""
        vm = _make_vm()
        f = _pow(X, _INT(3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.5, 1.5))

    def test_phase2e_arctan_rational(self) -> None:
        """Phase 2e: ∫ 1/(x²+1) dx = atan(x)."""
        vm = _make_vm()
        f = _div(_INT(1), _add(_pow(X, _INT(2)), _INT(1)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.8))

    def test_phase4a_poly_exp(self) -> None:
        """Phase 4a: ∫ x·eˣ dx = eˣ(x−1)."""
        vm = _make_vm()
        f = _mul(X, IRApply(EXP, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.5, 1.0))


# ---------------------------------------------------------------------------
# Class 6: Macsyma end-to-end string tests
# ---------------------------------------------------------------------------


class TestPhase12_Macsyma:
    """End-to-end tests via the Macsyma string interface."""

    @staticmethod
    def _integrate_macsyma(integrand: str, var: str = "x") -> IRNode:
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma
        vm = _make_vm()
        ast = parse_macsyma(f"integrate({integrand}, {var});")
        integrate_ir = compile_macsyma(ast)[0]
        return vm.eval(integrate_ir)

    def test_x_asin_x_macsyma(self) -> None:
        """integrate(x*asin(x), x)."""
        F = self._integrate_macsyma("x*asin(x)")
        f = _mul(X, _asin(X))
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_acos_x_macsyma(self) -> None:
        """integrate(acos(x), x)."""
        F = self._integrate_macsyma("acos(x)")
        f = _acos(X)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_asin_x_macsyma(self) -> None:
        """integrate(x^2*asin(x), x)."""
        F = self._integrate_macsyma("x^2*asin(x)")
        f = _mul(_pow(X, _INT(2)), _asin(X))
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_acos_2x_macsyma(self) -> None:
        """integrate(x*acos(2*x), x)."""
        F = self._integrate_macsyma("x*acos(2*x)")
        arg = _mul(_INT(2), X)
        f = _mul(X, _acos(arg))
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.4))

    def test_asin_x_macsyma(self) -> None:
        """integrate(asin(x), x)."""
        F = self._integrate_macsyma("asin(x)")
        f = _asin(X)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)
