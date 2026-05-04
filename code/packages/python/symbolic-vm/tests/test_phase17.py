"""Phase 17 integration tests — tanh^n power integrals.

Tests the identity-based reduction formula for:
- tanh^n (hyperbolic tangent to integer power n)

Key identity: tanh²(t) = 1 − sech²(t)

Reduction formula:
  I_n = I_{n-2} − tanh^(n-1)(ax+b) / ((n-1)·a)

Base cases:
  I_0 = x
  I_1 = log(cosh(ax+b)) / a

Key identities verified:
  ∫ tanh²(x) dx = x − tanh(x)
  ∫ tanh³(x) dx = log(cosh(x)) − tanh²(x)/2
  ∫ tanh⁴(x) dx = x − tanh(x) − tanh³(x)/3

Higher-order reductions verified numerically: F'(x) ≈ f(x).

Test points: x₀ = 0.3, x₁ = 0.8  (strictly away from 0)
"""

from __future__ import annotations

import math

from symbolic_ir import (
    COSH,
    INTEGRATE,
    MUL,
    POW,
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

# Test points strictly away from 0.
_TP = (0.3, 0.8)
# Shifted test points for ax+b arguments with b ≠ 0
_TP_SHIFT = (0.4, 0.9)


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
    if head == "Pow":
        return _eval_ir(node.args[0], x_val) ** _eval_ir(node.args[1], x_val)
    if head == "Log":
        return math.log(abs(_eval_ir(node.args[0], x_val)))
    if head == "Exp":
        return math.exp(_eval_ir(node.args[0], x_val))
    if head == "Sinh":
        return math.sinh(_eval_ir(node.args[0], x_val))
    if head == "Cosh":
        return math.cosh(_eval_ir(node.args[0], x_val))
    if head == "Tanh":
        return math.tanh(_eval_ir(node.args[0], x_val))
    if head == "Sech":
        return 1.0 / math.cosh(_eval_ir(node.args[0], x_val))
    if head == "Csch":
        return 1.0 / math.sinh(_eval_ir(node.args[0], x_val))
    if head == "Coth":
        v = _eval_ir(node.args[0], x_val)
        return math.cosh(v) / math.sinh(v)
    if head == "Atan":
        return math.atan(_eval_ir(node.args[0], x_val))
    if head == "Asinh":
        return math.asinh(_eval_ir(node.args[0], x_val))
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


# Shorthand builders
def _tanh(arg: IRNode) -> IRNode:
    return IRApply(TANH, (arg,))


def _pow(base: IRNode, exp: IRNode) -> IRNode:
    return IRApply(POW, (base, exp))


def _has_tanh(node: IRNode) -> bool:
    """Return True if the IR tree contains a TANH node."""
    if isinstance(node, IRApply):
        if node.head == TANH:
            return True
        return any(_has_tanh(c) for c in node.args)
    return False


def _has_log_cosh(node: IRNode) -> bool:
    """Return True if the IR tree contains log(cosh(...))."""
    if not isinstance(node, IRApply):
        return False
    if (
        node.head.name == "Log"
        and len(node.args) == 1
        and isinstance(node.args[0], IRApply)
        and node.args[0].head == COSH
    ):
        return True
    return any(_has_log_cosh(c) for c in node.args)


# ---------------------------------------------------------------------------
# TestPhase17_TanhPowers
# ---------------------------------------------------------------------------


class TestPhase17_TanhPowers:
    """∫ tanh^n(ax+b) dx via identity reduction tanh² = 1 − sech².

    Reduction formula:
        I_n = I_{n-2} − tanh^(n-1) / ((n-1)·a)
    Bases:
        I_0 = x
        I_1 = log(cosh(ax+b)) / a
    """

    def test_tanh_squared(self) -> None:
        """∫ tanh²(x) dx = x − tanh(x) — the fundamental tanh² identity."""
        vm = _make_vm()
        f = _pow(_tanh(X), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)
        # Structure: contains tanh
        assert _has_tanh(F), f"Expected Tanh in {F!r}"

    def test_tanh_cubed(self) -> None:
        """∫ tanh³(x) dx = log(cosh(x)) − tanh²(x)/2."""
        vm = _make_vm()
        f = _pow(_tanh(X), _INT(3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)
        # Structure: contains log(cosh)
        assert _has_log_cosh(F), f"Expected log(cosh(...)) in {F!r}"

    def test_tanh_fourth(self) -> None:
        """∫ tanh⁴(x) dx = x − tanh(x) − tanh³(x)/3."""
        vm = _make_vm()
        f = _pow(_tanh(X), _INT(4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_tanh_fifth(self) -> None:
        """∫ tanh⁵(x) dx — three reduction steps."""
        vm = _make_vm()
        f = _pow(_tanh(X), _INT(5))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_tanh_squared_a2(self) -> None:
        """∫ tanh²(2x) dx = x/2 − tanh(2x)/2 — chain-rule factor 1/2."""
        vm = _make_vm()
        inner = IRApply(MUL, (_INT(2), X))
        f = _pow(_tanh(inner), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_tanh_cubed_a2(self) -> None:
        """∫ tanh³(2x) dx — a=2 chain-rule scaling."""
        vm = _make_vm()
        inner = IRApply(MUL, (_INT(2), X))
        f = _pow(_tanh(inner), _INT(3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_tanh_squared_with_shift(self) -> None:
        """∫ tanh²(x+1) dx — non-zero b."""
        vm = _make_vm()
        from symbolic_ir import ADD, IRInteger  # noqa: PLC0415

        inner = IRApply(ADD, (X, IRInteger(1)))
        f = _pow(_tanh(inner), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_SHIFT)

    def test_tanh_fourth_a_half(self) -> None:
        """∫ tanh⁴(x/2) dx — a=1/2 rational chain-rule factor."""
        vm = _make_vm()
        inner = IRApply(MUL, (_RAT(1, 2), X))
        f = _pow(_tanh(inner), _INT(4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_tanh_squared_structure_contains_tanh(self) -> None:
        """Structural check: tanh² antiderivative contains TANH node."""
        vm = _make_vm()
        f = _pow(_tanh(X), _INT(2))
        F = _integrate_ir(vm, f)
        assert _has_tanh(F), f"Expected Tanh in result: {F!r}"

    def test_tanh_cubed_structure_contains_log_cosh(self) -> None:
        """Structural check: tanh³ antiderivative contains log(cosh(...))."""
        vm = _make_vm()
        f = _pow(_tanh(X), _INT(3))
        F = _integrate_ir(vm, f)
        assert _has_log_cosh(F), f"Expected log(cosh(...)) in result: {F!r}"


# ---------------------------------------------------------------------------
# TestPhase17_Fallthrough
# ---------------------------------------------------------------------------


class TestPhase17_Fallthrough:
    """Cases that must stay unevaluated — fallthrough to the user."""

    def test_poly_times_tanh_squared_unevaluated(self) -> None:
        """∫ x·tanh²(x) dx — poly×tanh^n is non-elementary (polylogarithm)."""
        from symbolic_ir import MUL  # noqa: PLC0415

        vm = _make_vm()
        f = IRApply(MUL, (X, _pow(_tanh(X), _INT(2))))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_tanh_nonlinear_arg_unevaluated(self) -> None:
        """∫ tanh²(x²) dx — non-linear argument, unevaluated."""
        from symbolic_ir import POW  # noqa: PLC0415

        vm = _make_vm()
        x_sq = IRApply(POW, (X, _INT(2)))
        f = _pow(_tanh(x_sq), _INT(2))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_poly_times_tanh_unevaluated_still(self) -> None:
        """∫ x·tanh(x) dx — poly×tanh (Phase 13 regression) still unevaluated."""
        from symbolic_ir import MUL  # noqa: PLC0415

        vm = _make_vm()
        f = IRApply(MUL, (X, _tanh(X)))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)


# ---------------------------------------------------------------------------
# TestPhase17_Regressions
# ---------------------------------------------------------------------------


class TestPhase17_Regressions:
    """Regression tests — Phase 16, 15, 14, and 13 results must be unaffected."""

    def test_phase16_sech_squared_still_works(self) -> None:
        """∫ sech²(x) dx still evaluates (Phase 16 regression)."""
        from symbolic_ir import SECH  # noqa: PLC0415

        vm = _make_vm()
        f = _pow(IRApply(SECH, (X,)), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase13_tanh_bare_still_works(self) -> None:
        """∫ tanh(x) dx = log(cosh(x)) still evaluates (Phase 13 regression)."""
        vm = _make_vm()
        f = _tanh(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase14_sinh_fourth_still_works(self) -> None:
        """∫ sinh⁴(x) dx still evaluates (Phase 14 regression)."""
        from symbolic_ir import SINH  # noqa: PLC0415

        vm = _make_vm()
        f = _pow(IRApply(SINH, (X,)), _INT(4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)
