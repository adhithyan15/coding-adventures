"""Phase 13 integration tests — hyperbolic functions.

Tests evaluation, differentiation, and integration for:
- sinh, cosh, tanh (forward)
- asinh, acosh, atanh (inverse)

Integration correctness is verified numerically: differentiate the
antiderivative at test points and confirm the result matches the original
integrand.

Test points:
- General: x₀ = 0.3, x₁ = 0.6
- acosh family: x₀ = 1.5, x₁ = 2.0  (requires |ax+b| > 1)
- atanh family: x₀ = 0.3, x₁ = 0.5  (requires |ax+b| < 1)
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ACOSH,
    ADD,
    ASINH,
    ATANH,
    COSH,
    DIV,
    EXP,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
    POW,
    SINH,
    TANH,
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

_TP = (0.3, 0.6)       # general test points
_TP_ACOSH = (1.5, 2.0)  # acosh domain: |ax+b| > 1
_TP_ATANH = (0.3, 0.5)  # atanh domain: |ax+b| < 1


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
        return math.sqrt(abs(_eval_ir(node.args[0], x_val)))
    if head == "Exp":
        return math.exp(_eval_ir(node.args[0], x_val))
    if head == "Sin":
        return math.sin(_eval_ir(node.args[0], x_val))
    if head == "Cos":
        return math.cos(_eval_ir(node.args[0], x_val))
    if head == "Tan":
        return math.tan(_eval_ir(node.args[0], x_val))
    if head == "Asin":
        return math.asin(_eval_ir(node.args[0], x_val))
    if head == "Acos":
        return math.acos(_eval_ir(node.args[0], x_val))
    if head == "Atan":
        return math.atan(_eval_ir(node.args[0], x_val))
    if head == "Sinh":
        return math.sinh(_eval_ir(node.args[0], x_val))
    if head == "Cosh":
        return math.cosh(_eval_ir(node.args[0], x_val))
    if head == "Tanh":
        return math.tanh(_eval_ir(node.args[0], x_val))
    if head == "Asinh":
        return math.asinh(_eval_ir(node.args[0], x_val))
    if head == "Acosh":
        return math.acosh(_eval_ir(node.args[0], x_val))
    if head == "Atanh":
        return math.atanh(_eval_ir(node.args[0], x_val))
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


def _neg(a: IRNode) -> IRNode:
    return IRApply(NEG, (a,))


def _div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _pow(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(POW, (a, b))


def _sinh(arg: IRNode) -> IRNode:
    return IRApply(SINH, (arg,))


def _cosh(arg: IRNode) -> IRNode:
    return IRApply(COSH, (arg,))


def _tanh(arg: IRNode) -> IRNode:
    return IRApply(TANH, (arg,))


def _asinh(arg: IRNode) -> IRNode:
    return IRApply(ASINH, (arg,))


def _acosh(arg: IRNode) -> IRNode:
    return IRApply(ACOSH, (arg,))


def _atanh(arg: IRNode) -> IRNode:
    return IRApply(ATANH, (arg,))


def _lin(a: IRRational | IRInteger, b_val: int = 0) -> IRNode:
    """Build a·x + b as an IR node."""
    ax: IRNode = _mul(a, X) if not (isinstance(a, IRInteger) and a.value == 1) else X
    if b_val == 0:
        return ax
    return _add(ax, _INT(b_val))


# ---------------------------------------------------------------------------
# Class 1: Sinh canonical cases
# ---------------------------------------------------------------------------


class TestPhase13_SinhCanonical:
    """Canonical sinh integrals: ∫ xⁿ·sinh(x) dx."""

    def test_sinh_x(self) -> None:
        """∫ sinh(x) dx = cosh(x)."""
        vm = _make_vm()
        f = _sinh(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_sinh_x(self) -> None:
        """∫ x·sinh(x) dx = x·cosh(x) − sinh(x)."""
        vm = _make_vm()
        f = _mul(X, _sinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_sinh_x(self) -> None:
        """∫ x²·sinh(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(2)), _sinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_cubed_sinh_x(self) -> None:
        """∫ x³·sinh(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(3)), _sinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_fourth_sinh_x(self) -> None:
        """∫ x⁴·sinh(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(4)), _sinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_plus_1_sinh_x(self) -> None:
        """∫ (x+1)·sinh(x) dx."""
        vm = _make_vm()
        f = _mul(_add(X, _INT(1)), _sinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_x_commutativity(self) -> None:
        """∫ sinh(x)·x dx = ∫ x·sinh(x) dx (MUL arg order)."""
        vm = _make_vm()
        f1 = _mul(X, _sinh(X))
        f2 = _mul(_sinh(X), X)
        F1 = _integrate_ir(vm, f1)
        F2 = _integrate_ir(vm, f2)
        _check_antiderivative(f1, F1)
        _check_antiderivative(f2, F2)

    def test_const_factor_sinh_x(self) -> None:
        """∫ 3·sinh(x) dx = 3·cosh(x)."""
        vm = _make_vm()
        f = _mul(_INT(3), _sinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 2: Cosh canonical cases
# ---------------------------------------------------------------------------


class TestPhase13_CoshCanonical:
    """Canonical cosh integrals: ∫ xⁿ·cosh(x) dx."""

    def test_cosh_x(self) -> None:
        """∫ cosh(x) dx = sinh(x)."""
        vm = _make_vm()
        f = _cosh(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_cosh_x(self) -> None:
        """∫ x·cosh(x) dx = x·sinh(x) − cosh(x)."""
        vm = _make_vm()
        f = _mul(X, _cosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_cosh_x(self) -> None:
        """∫ x²·cosh(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(2)), _cosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_cubed_cosh_x(self) -> None:
        """∫ x³·cosh(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(3)), _cosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_plus_1_cosh_x(self) -> None:
        """∫ (x+1)·cosh(x) dx."""
        vm = _make_vm()
        f = _mul(_add(X, _INT(1)), _cosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cosh_x_commutativity(self) -> None:
        """∫ cosh(x)·x dx = ∫ x·cosh(x) dx."""
        vm = _make_vm()
        f = _mul(_cosh(X), X)
        F = _integrate_ir(vm, f)
        _check_antiderivative(f, F)

    def test_const_factor_cosh_x(self) -> None:
        """∫ 2·cosh(x) dx = 2·sinh(x)."""
        vm = _make_vm()
        f = _mul(_INT(2), _cosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 3: Linear argument cases — sinh/cosh(ax+b)
# ---------------------------------------------------------------------------


class TestPhase13_LinearHyp:
    """Hyperbolic integrals with non-trivial linear arguments."""

    def test_sinh_2x(self) -> None:
        """∫ sinh(2x) dx = cosh(2x)/2."""
        vm = _make_vm()
        arg = _mul(_INT(2), X)
        f = _sinh(arg)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_cosh_2x(self) -> None:
        """∫ cosh(2x) dx = sinh(2x)/2."""
        vm = _make_vm()
        arg = _mul(_INT(2), X)
        f = _cosh(arg)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_2x_plus_1(self) -> None:
        """∫ sinh(2x+1) dx."""
        vm = _make_vm()
        arg = _add(_mul(_INT(2), X), _INT(1))
        f = _sinh(arg)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_cosh_half_x(self) -> None:
        """∫ x·cosh(x/2) dx."""
        vm = _make_vm()
        arg = _div(X, _INT(2))
        f = _mul(X, _cosh(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_sinh_2x(self) -> None:
        """∫ x·sinh(2x) dx."""
        vm = _make_vm()
        arg = _mul(_INT(2), X)
        f = _mul(X, _sinh(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_cosh_2x(self) -> None:
        """∫ x²·cosh(2x) dx."""
        vm = _make_vm()
        arg = _mul(_INT(2), X)
        f = _mul(_pow(X, _INT(2)), _cosh(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sinh_neg_x(self) -> None:
        """∫ sinh(−x) dx = −∫ sinh(x) dx = cosh(−x) (a=−1)."""
        vm = _make_vm()
        arg = _neg(X)
        f = _sinh(arg)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_cosh_neg_x(self) -> None:
        """∫ x·cosh(−x) dx (a=−1)."""
        vm = _make_vm()
        arg = _neg(X)
        f = _mul(X, _cosh(arg))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 4: Asinh canonical cases
# ---------------------------------------------------------------------------


class TestPhase13_AsinhCanonical:
    """Canonical asinh integrals: ∫ xⁿ·asinh(x) dx."""

    def test_asinh_x(self) -> None:
        """∫ asinh(x) dx = x·asinh(x) − √(x²+1)."""
        vm = _make_vm()
        f = _asinh(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_asinh_x(self) -> None:
        """∫ x·asinh(x) dx = (x²/2 + 1/4)·asinh(x) − x/4·√(x²+1)."""
        vm = _make_vm()
        f = _mul(X, _asinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_asinh_x(self) -> None:
        """∫ x²·asinh(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(2)), _asinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_cubed_asinh_x(self) -> None:
        """∫ x³·asinh(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(3)), _asinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_fourth_asinh_x(self) -> None:
        """∫ x⁴·asinh(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(4)), _asinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_asinh_x_commutativity(self) -> None:
        """∫ asinh(x)·x dx = ∫ x·asinh(x) dx."""
        vm = _make_vm()
        f = _mul(_asinh(X), X)
        F = _integrate_ir(vm, f)
        _check_antiderivative(f, F)

    def test_const_factor_asinh_x(self) -> None:
        """∫ 2·asinh(x) dx."""
        vm = _make_vm()
        f = _mul(_INT(2), _asinh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_asinh_2x(self) -> None:
        """∫ asinh(2x) dx (a=2)."""
        vm = _make_vm()
        arg = _mul(_INT(2), X)
        f = _asinh(arg)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 5: Acosh canonical cases
# ---------------------------------------------------------------------------


class TestPhase13_AcoshCanonical:
    """Canonical acosh integrals: ∫ xⁿ·acosh(x) dx.

    Test points x₀=1.5, x₁=2.0 since acosh requires |x|>1.
    """

    def test_acosh_x(self) -> None:
        """∫ acosh(x) dx = x·acosh(x) − √(x²−1)."""
        vm = _make_vm()
        f = _acosh(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_ACOSH)

    def test_x_acosh_x(self) -> None:
        """∫ x·acosh(x) dx."""
        vm = _make_vm()
        f = _mul(X, _acosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_ACOSH)

    def test_x_squared_acosh_x(self) -> None:
        """∫ x²·acosh(x) dx."""
        vm = _make_vm()
        f = _mul(_pow(X, _INT(2)), _acosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_ACOSH)

    def test_acosh_x_commutativity(self) -> None:
        """∫ acosh(x)·x dx."""
        vm = _make_vm()
        f = _mul(_acosh(X), X)
        F = _integrate_ir(vm, f)
        _check_antiderivative(f, F, test_points=_TP_ACOSH)

    def test_const_factor_acosh_x(self) -> None:
        """∫ 3·acosh(x) dx."""
        vm = _make_vm()
        f = _mul(_INT(3), _acosh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_ACOSH)


# ---------------------------------------------------------------------------
# Class 6: Bare tanh and atanh
# ---------------------------------------------------------------------------


class TestPhase13_BareAtanhTanh:
    """Bare tanh and atanh integrals."""

    def test_tanh_x(self) -> None:
        """∫ tanh(x) dx = log(cosh(x))."""
        vm = _make_vm()
        f = _tanh(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_tanh_2x(self) -> None:
        """∫ tanh(2x) dx = log(cosh(2x))/2."""
        vm = _make_vm()
        arg = _mul(_INT(2), X)
        f = _tanh(arg)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_atanh_x(self) -> None:
        """∫ atanh(x) dx = x·atanh(x) + (1/2)·log(1−x²)."""
        vm = _make_vm()
        f = _atanh(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_ATANH)

    def test_atanh_half_x(self) -> None:
        """∫ atanh(x/2) dx (a=1/2, linear arg)."""
        vm = _make_vm()
        arg = _div(X, _INT(2))
        f = _atanh(arg)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_ATANH)


# ---------------------------------------------------------------------------
# Class 7: Fallthrough cases — expressions that should stay unevaluated
# ---------------------------------------------------------------------------


class TestPhase13_Fallthrough:
    """Integrals that Phase 13 cannot evaluate — should remain Integrate(...)."""

    def test_sinh_x_squared(self) -> None:
        """∫ sinh(x²) dx — non-linear argument, unevaluated."""
        vm = _make_vm()
        f = _sinh(_pow(X, _INT(2)))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_tanh_times_x(self) -> None:
        """∫ x·tanh(x) dx — poly×tanh deferred, unevaluated."""
        vm = _make_vm()
        f = _mul(X, _tanh(X))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_atanh_times_x(self) -> None:
        """∫ x·atanh(x) dx — now closed-form via Phase 14c IBP."""
        vm = _make_vm()
        f = _mul(X, _atanh(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_ATANH)


# ---------------------------------------------------------------------------
# Class 8: Regression tests — earlier phases must still work
# ---------------------------------------------------------------------------


class TestPhase13_Regressions:
    """Verify that Phase 13 additions do not break earlier phase integrals."""

    def test_phase12_asin(self) -> None:
        """Phase 12 regression: ∫ x·asin(x) dx."""
        from symbolic_ir import ASIN

        vm = _make_vm()
        f = _mul(X, IRApply(ASIN, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase11_atan(self) -> None:
        """Phase 11 regression: ∫ x·atan(x) dx."""
        from symbolic_ir import ATAN

        vm = _make_vm()
        f = _mul(X, IRApply(ATAN, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase4a_sin(self) -> None:
        """Phase 4a regression: ∫ x·sin(x) dx = sin(x) − x·cos(x)."""
        from symbolic_ir import SIN

        vm = _make_vm()
        f = _mul(X, IRApply(SIN, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase3e_log(self) -> None:
        """Phase 3e regression: ∫ log(x) dx = x·log(x) − x."""
        vm = _make_vm()
        f = IRApply(LOG, (X,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.5, 1.5))

    def test_phase1_exp(self) -> None:
        """Phase 1 regression: ∫ exp(x) dx = exp(x)."""
        vm = _make_vm()
        f = IRApply(EXP, (X,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_power_rule(self) -> None:
        """Phase 1 regression: ∫ x² dx = x³/3."""
        vm = _make_vm()
        f = _pow(X, _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 9: Macsyma string interface end-to-end tests
# ---------------------------------------------------------------------------


class TestPhase13_Macsyma:
    """End-to-end tests via the Macsyma string interface."""

    def _make_macsyma_vm(self) -> VM:
        from symbolic_vm.backends import SymbolicBackend

        return VM(SymbolicBackend())

    def _macsyma_integrate(self, integrand: str, var: str = "x") -> IRNode:
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        vm = self._make_macsyma_vm()
        ast = parse_macsyma(f"integrate({integrand}, {var});")
        ir = compile_macsyma(ast)[0]
        return vm.eval(ir)

    def test_integrate_sinh_x(self) -> None:
        """integrate(sinh(x), x) produces a closed form."""
        result = self._macsyma_integrate("sinh(x)")
        unevaluated = IRApply(
            INTEGRATE, (IRApply(SINH, (IRSymbol("x"),)), IRSymbol("x"))
        )
        assert result != unevaluated

    def test_integrate_x_cosh_x(self) -> None:
        """integrate(x*cosh(x), x) produces a closed form."""
        result = self._macsyma_integrate("x*cosh(x)")
        unevaluated = IRApply(
            INTEGRATE,
            (
                IRApply(MUL, (IRSymbol("x"), IRApply(COSH, (IRSymbol("x"),)))),
                IRSymbol("x"),
            ),
        )
        assert result != unevaluated

    def test_integrate_asinh_x(self) -> None:
        """integrate(asinh(x), x) produces a closed form."""
        result = self._macsyma_integrate("asinh(x)")
        unevaluated = IRApply(
            INTEGRATE, (IRApply(ASINH, (IRSymbol("x"),)), IRSymbol("x"))
        )
        assert result != unevaluated

    def test_integrate_tanh_2x(self) -> None:
        """integrate(tanh(2*x), x) produces a closed form."""
        result = self._macsyma_integrate("tanh(2*x)")
        unevaluated = IRApply(
            INTEGRATE,
            (
                IRApply(
                    TANH,
                    (IRApply(MUL, (IRInteger(2), IRSymbol("x"))),),
                ),
                IRSymbol("x"),
            ),
        )
        assert result != unevaluated

    def test_integrate_atanh_x(self) -> None:
        """integrate(atanh(x), x) produces a closed form."""
        result = self._macsyma_integrate("atanh(x)")
        unevaluated = IRApply(
            INTEGRATE, (IRApply(ATANH, (IRSymbol("x"),)), IRSymbol("x"))
        )
        assert result != unevaluated
