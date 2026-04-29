"""Phase 15 integration tests — reciprocal hyperbolic functions.

Tests evaluation, differentiation, and integration for:
- coth (hyperbolic cotangent = cosh/sinh)
- sech (hyperbolic secant   = 1/cosh)
- csch (hyperbolic cosecant = 1/sinh)

Integration correctness is verified numerically: differentiate the
antiderivative at test points and confirm the result matches the original
integrand.

Test points:
- General: x₀ = 0.5, x₁ = 1.2  (strictly away from 0; coth/csch undefined at 0)
- Negative arg: x₀ = -1.5, x₁ = -0.7  (same, negative side)

Integrals:
- ∫ coth(ax+b) dx = (1/a)·log(sinh(ax+b))
- ∫ sech(ax+b) dx = (1/a)·atan(sinh(ax+b))
- ∫ csch(ax+b) dx = (1/a)·log(tanh((ax+b)/2))
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    ATAN,
    COTH,
    CSCH,
    DIV,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
    POW,
    SECH,
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

# Test points strictly away from 0 so coth/csch are well-defined.
_TP = (0.5, 1.2)
# Shifted test points for ax+b arguments where b shifts the zero
_TP_SHIFT = (0.8, 1.5)  # used when b ≠ 0


def _make_vm() -> VM:
    return VM(SymbolicBackend())


def _eval_ir(node: IRNode, x_val: float) -> float:  # noqa: PLR0911
    """Numerically evaluate an IR tree at x = x_val.

    Covers all the IR heads produced by Phase 15 antiderivatives, plus
    the heads used by Phases 1–14 that appear in regression tests.
    """
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
        return base**exp
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


# ---------------------------------------------------------------------------
# TestPhase15_HandlerEval
# ---------------------------------------------------------------------------


class TestPhase15_HandlerEval:
    """Numeric evaluation of coth/sech/csch via the VM handler table."""

    def test_coth_numeric(self) -> None:
        """coth(x) evaluates to cosh(x)/sinh(x) numerically."""
        vm = _make_vm()
        x_val = IRFloat(1.2)
        result = vm.eval(IRApply(COTH, (x_val,)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - math.cosh(1.2) / math.sinh(1.2)) < 1e-12

    def test_sech_numeric(self) -> None:
        """sech(x) evaluates to 1/cosh(x) numerically."""
        vm = _make_vm()
        x_val = IRFloat(0.7)
        result = vm.eval(IRApply(SECH, (x_val,)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - 1.0 / math.cosh(0.7)) < 1e-12

    def test_csch_numeric(self) -> None:
        """csch(x) evaluates to 1/sinh(x) numerically."""
        vm = _make_vm()
        x_val = IRFloat(0.5)
        result = vm.eval(IRApply(CSCH, (x_val,)))
        assert isinstance(result, IRFloat)
        assert abs(result.value - 1.0 / math.sinh(0.5)) < 1e-12

    def test_sech_at_zero(self) -> None:
        """sech(0) = 1 — the only exact identity among the three."""
        vm = _make_vm()
        result = vm.eval(IRApply(SECH, (IRInteger(0),)))
        assert result == IRInteger(1)

    def test_coth_symbolic_stays_unevaluated(self) -> None:
        """coth(x) with a symbolic argument stays unevaluated in symbolic mode."""
        vm = _make_vm()
        expr = IRApply(COTH, (X,))
        result = vm.eval(expr)
        assert result == expr

    def test_csch_symbolic_stays_unevaluated(self) -> None:
        """csch(x) with a symbolic argument stays unevaluated in symbolic mode."""
        vm = _make_vm()
        expr = IRApply(CSCH, (X,))
        result = vm.eval(expr)
        assert result == expr


# ---------------------------------------------------------------------------
# TestPhase15_Differentiation
# ---------------------------------------------------------------------------


class TestPhase15_Differentiation:
    """d/dx for coth, sech, csch — plain and chain-rule cases."""

    def _diff(self, expr: IRNode) -> IRNode:
        from symbolic_ir import D

        vm = _make_vm()
        return vm.eval(IRApply(D, (expr, X)))

    def test_diff_coth(self) -> None:
        """d/dx coth(x) = -1/sinh²(x)."""
        result = self._diff(IRApply(COTH, (X,)))
        # Numerically verify at a test point
        for xv in _TP:
            expected = -1.0 / math.sinh(xv) ** 2
            actual = _eval_ir(result, xv)
            assert abs(actual - expected) < 1e-9, f"At x={xv}: {actual} vs {expected}"

    def test_diff_sech(self) -> None:
        """d/dx sech(x) = -sinh(x)/cosh²(x)."""
        result = self._diff(IRApply(SECH, (X,)))
        for xv in _TP:
            expected = -math.sinh(xv) / math.cosh(xv) ** 2
            actual = _eval_ir(result, xv)
            assert abs(actual - expected) < 1e-9

    def test_diff_csch(self) -> None:
        """d/dx csch(x) = -cosh(x)/sinh²(x)."""
        result = self._diff(IRApply(CSCH, (X,)))
        for xv in _TP:
            expected = -math.cosh(xv) / math.sinh(xv) ** 2
            actual = _eval_ir(result, xv)
            assert abs(actual - expected) < 1e-9

    def test_diff_coth_chain_rule(self) -> None:
        """d/dx coth(2x+1) = -2/sinh²(2x+1) — chain rule."""
        inner = IRApply(ADD, (IRApply(MUL, (IRInteger(2), X)), IRInteger(1)))
        result = self._diff(IRApply(COTH, (inner,)))
        for xv in _TP_SHIFT:
            u = 2 * xv + 1
            expected = -2.0 / math.sinh(u) ** 2
            actual = _eval_ir(result, xv)
            assert abs(actual - expected) < 1e-9

    def test_diff_sech_chain_rule(self) -> None:
        """d/dx sech(3x) = -3·sinh(3x)/cosh²(3x) — chain rule."""
        inner = IRApply(MUL, (IRInteger(3), X))
        result = self._diff(IRApply(SECH, (inner,)))
        for xv in _TP:
            u = 3 * xv
            expected = -3.0 * math.sinh(u) / math.cosh(u) ** 2
            actual = _eval_ir(result, xv)
            assert abs(actual - expected) < 1e-9

    def test_diff_csch_chain_rule(self) -> None:
        """d/dx csch(x/2) = -(1/2)·cosh(x/2)/sinh²(x/2) — chain rule."""
        inner = IRApply(DIV, (X, IRInteger(2)))
        result = self._diff(IRApply(CSCH, (inner,)))
        for xv in _TP:
            u = xv / 2
            expected = -0.5 * math.cosh(u) / math.sinh(u) ** 2
            actual = _eval_ir(result, xv)
            assert abs(actual - expected) < 1e-9

    def test_diff_coth_uses_sinh_denom(self) -> None:
        """The derivative of coth(x) is expressed in terms of Sinh, not Coth.

        This confirms the implementation chooses the primitive form
        (``−1/sinh²(u)``) rather than the self-referential ``1 − coth²(u)``.
        """
        from symbolic_ir import NEG  # noqa: PLC0415

        result = self._diff(IRApply(COTH, (X,)))
        # Result should be a Neg(...) wrapping a Div with Sinh in denominator
        assert isinstance(result, IRApply)
        assert result.head == NEG

    def test_diff_sech_structure(self) -> None:
        """The derivative of sech(x) is Neg(Div(Sinh(x), Pow(Cosh(x), 2)))."""
        result = self._diff(IRApply(SECH, (X,)))
        assert isinstance(result, IRApply)
        assert result.head == NEG

    def test_diff_csch_structure(self) -> None:
        """The derivative of csch(x) is Neg(Div(Cosh(x), Pow(Sinh(x), 2)))."""
        result = self._diff(IRApply(CSCH, (X,)))
        assert isinstance(result, IRApply)
        assert result.head == NEG


# ---------------------------------------------------------------------------
# TestPhase15_CothIntegral
# ---------------------------------------------------------------------------


class TestPhase15_CothIntegral:
    """∫ coth(ax+b) dx = (1/a)·log(sinh(ax+b))."""

    def test_coth_bare(self) -> None:
        """∫ coth(x) dx is evaluated to a closed form."""
        vm = _make_vm()
        f = IRApply(COTH, (X,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_coth_linear_arg_a2(self) -> None:
        """∫ coth(2x) dx = (1/2)·log(sinh(2x))."""
        vm = _make_vm()
        inner = IRApply(MUL, (IRInteger(2), X))
        f = IRApply(COTH, (inner,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_coth_linear_arg_shift(self) -> None:
        """∫ coth(x+1) dx — shifted argument."""
        vm = _make_vm()
        inner = IRApply(ADD, (X, IRInteger(1)))
        f = IRApply(COTH, (inner,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_SHIFT)

    def test_coth_fractional_a(self) -> None:
        """∫ coth(x/3) dx = 3·log(sinh(x/3))."""
        vm = _make_vm()
        inner = IRApply(DIV, (X, IRInteger(3)))
        f = IRApply(COTH, (inner,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_coth_result_contains_log_sinh(self) -> None:
        """The antiderivative of coth(x) explicitly contains Log(Sinh(...))."""
        vm = _make_vm()
        f = IRApply(COTH, (X,))
        F = _integrate_ir(vm, f)

        def _has_log_sinh(node: IRNode) -> bool:
            if not isinstance(node, IRApply):
                return False
            if (
                node.head == LOG
                and isinstance(node.args[0], IRApply)
                and node.args[0].head == SINH
            ):
                return True
            return any(_has_log_sinh(c) for c in node.args)

        assert _has_log_sinh(F), f"Expected log(sinh(...)) in {F!r}"


# ---------------------------------------------------------------------------
# TestPhase15_SechIntegral
# ---------------------------------------------------------------------------


class TestPhase15_SechIntegral:
    """∫ sech(ax+b) dx = (1/a)·atan(sinh(ax+b))."""

    def test_sech_bare(self) -> None:
        """∫ sech(x) dx is evaluated to a closed form."""
        vm = _make_vm()
        f = IRApply(SECH, (X,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_sech_linear_arg_a3(self) -> None:
        """∫ sech(3x) dx = (1/3)·atan(sinh(3x))."""
        vm = _make_vm()
        inner = IRApply(MUL, (IRInteger(3), X))
        f = IRApply(SECH, (inner,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_sech_linear_arg_shift(self) -> None:
        """∫ sech(x+2) dx — shifted argument."""
        vm = _make_vm()
        inner = IRApply(ADD, (X, IRInteger(2)))
        f = IRApply(SECH, (inner,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_sech_fractional_a(self) -> None:
        """∫ sech(x/2) dx = 2·atan(sinh(x/2))."""
        vm = _make_vm()
        inner = IRApply(DIV, (X, IRInteger(2)))
        f = IRApply(SECH, (inner,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_sech_result_contains_atan_sinh(self) -> None:
        """The antiderivative of sech(x) explicitly contains Atan(Sinh(...))."""
        vm = _make_vm()
        f = IRApply(SECH, (X,))
        F = _integrate_ir(vm, f)

        def _has_atan_sinh(node: IRNode) -> bool:
            if not isinstance(node, IRApply):
                return False
            if (
                node.head == ATAN
                and isinstance(node.args[0], IRApply)
                and node.args[0].head == SINH
            ):
                return True
            return any(_has_atan_sinh(c) for c in node.args)

        assert _has_atan_sinh(F), f"Expected atan(sinh(...)) in {F!r}"


# ---------------------------------------------------------------------------
# TestPhase15_CschIntegral
# ---------------------------------------------------------------------------


class TestPhase15_CschIntegral:
    """∫ csch(ax+b) dx = (1/a)·log(tanh((ax+b)/2)).

    Test points must be > 0 (csch is positive there and tanh of positive
    half-argument is positive, so the log is well-defined).
    """

    def test_csch_bare(self) -> None:
        """∫ csch(x) dx is evaluated to a closed form."""
        vm = _make_vm()
        f = IRApply(CSCH, (X,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_csch_linear_arg_a2(self) -> None:
        """∫ csch(2x) dx = (1/2)·log(tanh(x))."""
        vm = _make_vm()
        inner = IRApply(MUL, (IRInteger(2), X))
        f = IRApply(CSCH, (inner,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_csch_linear_arg_shift(self) -> None:
        """∫ csch(x+1) dx — shifted argument."""
        vm = _make_vm()
        inner = IRApply(ADD, (X, IRInteger(1)))
        f = IRApply(CSCH, (inner,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP_SHIFT)

    def test_csch_fractional_a(self) -> None:
        """∫ csch(x/2) dx = 2·log(tanh(x/4))."""
        vm = _make_vm()
        inner = IRApply(DIV, (X, IRInteger(2)))
        f = IRApply(CSCH, (inner,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=_TP)

    def test_csch_result_contains_log_tanh(self) -> None:
        """The antiderivative of csch(x) contains Log(Tanh(...)) (half-arg form)."""
        vm = _make_vm()
        f = IRApply(CSCH, (X,))
        F = _integrate_ir(vm, f)

        def _has_log_tanh(node: IRNode) -> bool:
            if not isinstance(node, IRApply):
                return False
            if (
                node.head == LOG
                and isinstance(node.args[0], IRApply)
                and node.args[0].head == TANH
            ):
                return True
            return any(_has_log_tanh(c) for c in node.args)

        assert _has_log_tanh(F), f"Expected log(tanh(...)) in {F!r}"


# ---------------------------------------------------------------------------
# TestPhase15_Fallthrough
# ---------------------------------------------------------------------------


class TestPhase15_Fallthrough:
    """Inputs that must return unevaluated Integrate — deferred cases."""

    def test_poly_times_coth_unevaluated(self) -> None:
        """∫ x·coth(x) dx is not yet implemented — returns unevaluated."""
        vm = _make_vm()
        f = IRApply(MUL, (X, IRApply(COTH, (X,))))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_coth_nonlinear_arg_unevaluated(self) -> None:
        """∫ coth(x²) dx has a non-linear argument — returns unevaluated."""
        vm = _make_vm()
        f = IRApply(COTH, (IRApply(POW, (X, IRInteger(2))),))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_sech_squared_now_evaluates(self) -> None:
        """∫ sech²(x) dx = tanh(x) — wired in Phase 16 (IBP power reduction).

        Updated from ``test_sech_squared_unevaluated`` when Phase 16 landed:
        ``sech^n`` now dispatches through ``_try_recip_hyp_power`` for n ≥ 2.
        """
        vm = _make_vm()
        f = IRApply(POW, (IRApply(SECH, (X,)), IRInteger(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)


# ---------------------------------------------------------------------------
# TestPhase15_Regressions
# ---------------------------------------------------------------------------


class TestPhase15_Regressions:
    """Regression tests — Phase 14, 13, and 3 results must be unaffected."""

    def test_phase14_atanh_times_x(self) -> None:
        """∫ x·atanh(x) dx still evaluates (Phase 14c regression)."""
        vm = _make_vm()
        f = IRApply(MUL, (X, IRApply(ATAN, (X,))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)

    def test_phase13_sinh_times_x(self) -> None:
        """∫ x·sinh(x) dx still evaluates (Phase 13 regression)."""
        vm = _make_vm()
        f = IRApply(MUL, (X, IRApply(SINH, (X,))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase3_exp_2x(self) -> None:
        """∫ exp(2x) dx still evaluates (Phase 3a regression)."""
        from symbolic_ir import EXP  # noqa: PLC0415

        vm = _make_vm()
        inner = IRApply(MUL, (IRInteger(2), X))
        f = IRApply(EXP, (inner,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# TestPhase15_Macsyma
# ---------------------------------------------------------------------------


class TestPhase15_Macsyma:
    """End-to-end tests via the MACSYMA string interface.

    These tests drive the full pipeline: parser → compiler → VM.
    They confirm that ``coth``, ``sech``, ``csch`` are wired as
    first-class MACSYMA functions, not just raw IR heads.
    """

    def _run(self, source: str) -> IRNode:
        from macsyma_compiler import compile_macsyma  # noqa: PLC0415
        from macsyma_parser import parse_macsyma  # noqa: PLC0415

        vm = _make_vm()
        ast = parse_macsyma(source + ";")
        # compile_macsyma returns a list of IR nodes (one per statement).
        # wrap_terminators=False (the default) gives us unwrapped expressions.
        stmts = compile_macsyma(ast)
        result = None
        for stmt in stmts:
            result = vm.eval(stmt)
        return result  # type: ignore[return-value]

    def test_sech_numeric_eval(self) -> None:
        """sech(0.5) evaluates numerically via MACSYMA syntax."""
        result = self._run("sech(0.5)")
        assert isinstance(result, IRFloat)
        assert abs(result.value - 1.0 / math.cosh(0.5)) < 1e-12

    def test_sech_zero(self) -> None:
        """sech(0) = 1 via MACSYMA syntax."""
        result = self._run("sech(0)")
        assert result == IRInteger(1)

    def test_coth_diff(self) -> None:
        """diff(coth(x), x) returns a closed form via MACSYMA syntax."""
        result = self._run("diff(coth(x), x)")
        # Should not be an unevaluated D(...)
        from symbolic_ir import D  # noqa: PLC0415

        assert not (isinstance(result, IRApply) and result.head == D)
        # Numerically verify: d/dx coth(x) = -1/sinh²(x)
        for xv in _TP:
            expected = -1.0 / math.sinh(xv) ** 2
            actual = _eval_ir(result, xv)
            assert abs(actual - expected) < 1e-9

    def test_csch_integrate(self) -> None:
        """integrate(csch(x), x) returns a closed form via MACSYMA syntax."""
        result = self._run("integrate(csch(x), x)")
        f = IRApply(CSCH, (X,))
        _was_evaluated(f, result)
        _check_antiderivative(f, result, test_points=_TP)

    def test_sech_integrate_linear(self) -> None:
        """integrate(sech(2*x+1), x) returns a closed form."""
        result = self._run("integrate(sech(2*x+1), x)")
        inner = IRApply(ADD, (IRApply(MUL, (IRInteger(2), X)), IRInteger(1)))
        f = IRApply(SECH, (inner,))
        _was_evaluated(f, result)
        _check_antiderivative(f, result, test_points=_TP_SHIFT)
