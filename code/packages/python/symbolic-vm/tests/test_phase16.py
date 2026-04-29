"""Phase 16 integration tests — reciprocal hyperbolic power integrals.

Tests the IBP reduction formulas for:
- sech^n (hyperbolic secant to integer power n)
- csch^n (hyperbolic cosecant to integer power n)
- coth^n (hyperbolic cotangent to integer power n)

Key identities verified:
  ∫ sech²(ax+b) dx = tanh(ax+b) / a
  ∫ csch²(ax+b) dx = -coth(ax+b) / a
  ∫ coth²(ax+b) dx = x - coth(ax+b) / a

Higher-order reduction formulas verified numerically: F'(x) ≈ f(x).

Test points: x₀ = 0.5, x₁ = 1.2  (strictly away from 0; coth/csch undefined at 0)
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    COTH,
    CSCH,
    DIV,
    INTEGRATE,
    MUL,
    POW,
    SECH,
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

# Test points strictly away from 0 so coth/csch are well-defined.
_TP = (0.5, 1.2)
# Shifted test points for ax+b arguments with b ≠ 0
_TP_SHIFT = (0.8, 1.5)


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
    if head == "Coth":
        v = _eval_ir(node.args[0], x_val)
        return math.cosh(v) / math.sinh(v)
    if head == "Sech":
        return 1.0 / math.cosh(_eval_ir(node.args[0], x_val))
    if head == "Csch":
        return 1.0 / math.sinh(_eval_ir(node.args[0], x_val))
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
def _sech(arg: IRNode) -> IRNode:
    return IRApply(SECH, (arg,))


def _csch(arg: IRNode) -> IRNode:
    return IRApply(CSCH, (arg,))


def _coth(arg: IRNode) -> IRNode:
    return IRApply(COTH, (arg,))


def _pow(base: IRNode, exp: IRNode) -> IRNode:
    return IRApply(POW, (base, exp))


# ---------------------------------------------------------------------------
# TestPhase16_SechPowers
# ---------------------------------------------------------------------------


class TestPhase16_SechPowers:
    """∫ sech^n(ax+b) dx via IBP reduction.

    Reduction formula:
        I_n = sech^(n-2)·tanh / ((n-1)·a)  +  (n-2)/(n-1) · I_{n-2}
    Bases:
        I_1 = atan(sinh) / a
        I_2 = tanh / a          ← the most important identity
    """

    def test_sech_squared(self) -> None:
        """∫ sech²(x) dx = tanh(x) — the fundamental sech² identity."""
        vm = _make_vm()
        f = _pow(_sech(X), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sech_cubed(self) -> None:
        """∫ sech³(x) dx = sech(x)·tanh(x)/2 + (1/2)·atan(sinh(x))."""
        vm = _make_vm()
        f = _pow(_sech(X), _INT(3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sech_fourth(self) -> None:
        """∫ sech⁴(x) dx — applies reduction twice (n=4 → n=2 → done)."""
        vm = _make_vm()
        f = _pow(_sech(X), _INT(4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sech_fifth(self) -> None:
        """∫ sech⁵(x) dx — applies reduction three times (n=5 → n=3 → n=1)."""
        vm = _make_vm()
        f = _pow(_sech(X), _INT(5))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sech_squared_a2(self) -> None:
        """∫ sech²(2x) dx = tanh(2x)/2 — coefficient a=2."""
        vm = _make_vm()
        inner = IRApply(MUL, (_INT(2), X))
        f = _pow(_sech(inner), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sech_squared_shift(self) -> None:
        """∫ sech²(x+1) dx = tanh(x+1) — shifted argument."""
        vm = _make_vm()
        inner = IRApply(ADD, (X, _INT(1)))
        f = _pow(_sech(inner), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_SHIFT)

    def test_sech_squared_fractional_a(self) -> None:
        """∫ sech²(x/2) dx = 2·tanh(x/2) — fractional coefficient."""
        vm = _make_vm()
        inner = IRApply(DIV, (X, _INT(2)))
        f = _pow(_sech(inner), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_sech_squared_result_contains_tanh(self) -> None:
        """The antiderivative of sech²(x) explicitly contains Tanh."""
        vm = _make_vm()
        f = _pow(_sech(X), _INT(2))
        F = _integrate_ir(vm, f)

        def _has_tanh(node: IRNode) -> bool:
            if isinstance(node, IRApply):
                if node.head == TANH:
                    return True
                return any(_has_tanh(c) for c in node.args)
            return False

        assert _has_tanh(F), f"Expected Tanh in {F!r}"


# ---------------------------------------------------------------------------
# TestPhase16_CschPowers
# ---------------------------------------------------------------------------


class TestPhase16_CschPowers:
    """∫ csch^n(ax+b) dx via IBP reduction.

    Reduction formula:
        I_n = -csch^(n-2)·coth / ((n-1)·a)  -  (n-2)/(n-1) · I_{n-2}
    Bases:
        I_1 = log(tanh(half)) / a
        I_2 = -coth / a         ← note the negative sign
    """

    def test_csch_squared(self) -> None:
        """∫ csch²(x) dx = -coth(x)."""
        vm = _make_vm()
        f = _pow(_csch(X), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_csch_cubed(self) -> None:
        """∫ csch³(x) dx — one reduction step."""
        vm = _make_vm()
        f = _pow(_csch(X), _INT(3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_csch_fourth(self) -> None:
        """∫ csch⁴(x) dx — two reduction steps."""
        vm = _make_vm()
        f = _pow(_csch(X), _INT(4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_csch_fifth(self) -> None:
        """∫ csch⁵(x) dx — three reduction steps."""
        vm = _make_vm()
        f = _pow(_csch(X), _INT(5))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_csch_squared_a2(self) -> None:
        """∫ csch²(2x) dx = -coth(2x)/2."""
        vm = _make_vm()
        inner = IRApply(MUL, (_INT(2), X))
        f = _pow(_csch(inner), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_csch_squared_shift(self) -> None:
        """∫ csch²(x+1) dx — shifted argument."""
        vm = _make_vm()
        inner = IRApply(ADD, (X, _INT(1)))
        f = _pow(_csch(inner), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_SHIFT)

    def test_csch_squared_fractional_a(self) -> None:
        """∫ csch²(x/2) dx = -2·coth(x/2)."""
        vm = _make_vm()
        inner = IRApply(DIV, (X, _INT(2)))
        f = _pow(_csch(inner), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_csch_squared_result_contains_coth(self) -> None:
        """The antiderivative of csch²(x) explicitly contains Coth (negated)."""
        vm = _make_vm()
        f = _pow(_csch(X), _INT(2))
        F = _integrate_ir(vm, f)

        def _has_coth(node: IRNode) -> bool:
            if isinstance(node, IRApply):
                if node.head == COTH:
                    return True
                return any(_has_coth(c) for c in node.args)
            return False

        assert _has_coth(F), f"Expected Coth in {F!r}"


# ---------------------------------------------------------------------------
# TestPhase16_CothPowers
# ---------------------------------------------------------------------------


class TestPhase16_CothPowers:
    """∫ coth^n(ax+b) dx via identity coth² = 1 + csch².

    Reduction formula:
        I_n = I_{n-2} - coth^(n-1) / ((n-1)·a)
    Bases:
        I_0 = x
        I_1 = log(sinh) / a
    """

    def test_coth_squared(self) -> None:
        """∫ coth²(x) dx = x - coth(x)."""
        vm = _make_vm()
        f = _pow(_coth(X), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_coth_cubed(self) -> None:
        """∫ coth³(x) dx = log(sinh(x)) - coth²(x)/2."""
        vm = _make_vm()
        f = _pow(_coth(X), _INT(3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_coth_fourth(self) -> None:
        """∫ coth⁴(x) dx — two reductions n=4→2→0, telescoping to x - coth - coth³/3."""
        vm = _make_vm()
        f = _pow(_coth(X), _INT(4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_coth_fifth(self) -> None:
        """∫ coth⁵(x) dx — three reduction steps."""
        vm = _make_vm()
        f = _pow(_coth(X), _INT(5))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_coth_squared_a2(self) -> None:
        """∫ coth²(2x) dx = x - coth(2x)/2."""
        vm = _make_vm()
        inner = IRApply(MUL, (_INT(2), X))
        f = _pow(_coth(inner), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_coth_squared_shift(self) -> None:
        """∫ coth²(x+1) dx — shifted argument."""
        vm = _make_vm()
        inner = IRApply(ADD, (X, _INT(1)))
        f = _pow(_coth(inner), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_SHIFT)

    def test_coth_squared_fractional_a(self) -> None:
        """∫ coth²(x/2) dx = x - 2·coth(x/2)."""
        vm = _make_vm()
        inner = IRApply(DIV, (X, _INT(2)))
        f = _pow(_coth(inner), _INT(2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_coth_squared_result_contains_coth(self) -> None:
        """The antiderivative of coth²(x) explicitly contains Coth."""
        vm = _make_vm()
        f = _pow(_coth(X), _INT(2))
        F = _integrate_ir(vm, f)

        def _has_coth(node: IRNode) -> bool:
            if isinstance(node, IRApply):
                if node.head == COTH:
                    return True
                return any(_has_coth(c) for c in node.args)
            return False

        assert _has_coth(F), f"Expected Coth in {F!r}"


# ---------------------------------------------------------------------------
# TestPhase16_Fallthrough
# ---------------------------------------------------------------------------


class TestPhase16_Fallthrough:
    """Inputs that must return unevaluated Integrate — deferred cases."""

    def test_poly_times_sech_squared_unevaluated(self) -> None:
        """∫ x·sech²(x) dx — poly×power not yet implemented."""
        vm = _make_vm()
        f = IRApply(MUL, (X, _pow(_sech(X), _INT(2))))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_sech_nonlinear_arg_squared_unevaluated(self) -> None:
        """∫ sech²(x²) dx — non-linear argument returns unevaluated."""
        vm = _make_vm()
        f = _pow(_sech(IRApply(POW, (X, _INT(2)))), _INT(2))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_sech_times_csch_unevaluated(self) -> None:
        """∫ sech(x)·csch(x) dx — mixed product returns unevaluated."""
        vm = _make_vm()
        f = IRApply(MUL, (_sech(X), _csch(X)))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)


# ---------------------------------------------------------------------------
# TestPhase16_Regressions
# ---------------------------------------------------------------------------


class TestPhase16_Regressions:
    """Regression tests — Phase 15, 14, 13, and 3 results must be unaffected."""

    def test_phase15_sech_bare_still_works(self) -> None:
        """∫ sech(x) dx still evaluates via Phase 15 (not power reduction)."""
        vm = _make_vm()
        f = _sech(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        # sech(x) bare antiderivative is atan(sinh(x))
        for xv in _TP:
            expected = _eval_ir(f, xv)
            actual = _numerical_deriv(F, xv)
            assert abs(actual - expected) < 1e-6

    def test_phase14_sinh_fourth_still_works(self) -> None:
        """∫ sinh⁴(x) dx still evaluates (Phase 14 regression)."""
        from symbolic_ir import SINH  # noqa: PLC0415

        vm = _make_vm()
        f = _pow(IRApply(SINH, (X,)), _INT(4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        for xv in _TP:
            expected = _eval_ir(f, xv)
            actual = _numerical_deriv(F, xv)
            assert abs(actual - expected) < 1e-6

    def test_phase3_exp_2x_still_works(self) -> None:
        """∫ exp(2x) dx still evaluates (Phase 3 regression)."""
        from symbolic_ir import EXP  # noqa: PLC0415

        vm = _make_vm()
        inner = IRApply(MUL, (_INT(2), X))
        f = IRApply(EXP, (inner,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        for xv in _TP:
            expected = _eval_ir(f, xv)
            actual = _numerical_deriv(F, xv)
            assert abs(actual - expected) < 1e-6


# ---------------------------------------------------------------------------
# TestPhase16_Macsyma
# ---------------------------------------------------------------------------


class TestPhase16_Macsyma:
    """End-to-end tests via the MACSYMA string interface."""

    def _run(self, source: str) -> IRNode:
        from macsyma_compiler import compile_macsyma  # noqa: PLC0415
        from macsyma_parser import parse_macsyma  # noqa: PLC0415

        vm = _make_vm()
        ast = parse_macsyma(source + ";")
        stmts = compile_macsyma(ast)
        result = None
        for stmt in stmts:
            result = vm.eval(stmt)
        return result  # type: ignore[return-value]

    def test_sech_squared_via_macsyma(self) -> None:
        """integrate(sech(x)^2, x) = tanh(x) via MACSYMA syntax."""
        result = self._run("integrate(sech(x)^2, x)")
        f = _pow(_sech(X), _INT(2))
        _was_evaluated(f, result)
        _check_antiderivative(f, result)

    def test_csch_squared_via_macsyma(self) -> None:
        """integrate(csch(x)^2, x) = -coth(x) via MACSYMA syntax."""
        result = self._run("integrate(csch(x)^2, x)")
        f = _pow(_csch(X), _INT(2))
        _was_evaluated(f, result)
        _check_antiderivative(f, result)

    def test_coth_squared_via_macsyma(self) -> None:
        """integrate(coth(x)^2, x) = x - coth(x) via MACSYMA syntax."""
        result = self._run("integrate(coth(x)^2, x)")
        f = _pow(_coth(X), _INT(2))
        _was_evaluated(f, result)
        _check_antiderivative(f, result)

    def test_sech_cubed_via_macsyma(self) -> None:
        """integrate(sech(x)^3, x) returns a closed form."""
        result = self._run("integrate(sech(x)^3, x)")
        f = _pow(_sech(X), _INT(3))
        _was_evaluated(f, result)
        _check_antiderivative(f, result)
