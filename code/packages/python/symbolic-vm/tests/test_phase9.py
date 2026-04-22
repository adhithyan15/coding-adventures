"""Phase 9 integration tests — multi-quadratic partial-fraction integration.

Two families:

  1. ∫ N(x)/((Q₁(x)·Q₂(x)) dx where Q₁, Q₂ are distinct irreducible quadratics.
     Split into partial fractions, integrate each via Phase 2e (arctan formula).

  2. ∫ atan(ax+b) dx — direct IBP table entry (bonus).

Correctness is verified numerically: differentiate the antiderivative at test
points and confirm the result matches the original integrand.

Test points: x₀ = 0.4, x₁ = 0.7 — safely away from singularities.
"""

from __future__ import annotations

import math

from symbolic_ir import (
    ADD,
    ATAN,
    COS,
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
# Shared helpers (same pattern as Phase 7 and Phase 8 tests)
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


def _div(a: IRNode, b: IRNode) -> IRNode:
    return IRApply(DIV, (a, b))


def _pow(base: IRNode, n: int) -> IRNode:
    return IRApply(POW, (base, IRInteger(n)))


def _sin(arg: IRNode) -> IRNode:
    return IRApply(SIN, (arg,))


def _cos(arg: IRNode) -> IRNode:
    return IRApply(COS, (arg,))


def _exp(arg: IRNode) -> IRNode:
    return IRApply(EXP, (arg,))


def _log(arg: IRNode) -> IRNode:
    return IRApply(LOG, (arg,))


def _atan(arg: IRNode) -> IRNode:
    return IRApply(ATAN, (arg,))


def _c(n: int) -> IRNode:
    return IRInteger(n)


def _r(p: int, q: int) -> IRNode:
    return IRRational(p, q)


# Builds (x² + k) as an IRNode.
def _x2pk(k: int) -> IRNode:
    return _add(_pow(X, 2), _c(k))


# Builds (x² + ax + b).
def _quad(a: int, b: int) -> IRNode:
    return _add(_add(_pow(X, 2), _mul(_c(a), X)), _c(b))


# Builds 1/((x²+a)(x²+b)).
def _inv_two_quad(a: int, b: int) -> IRNode:
    return _div(_c(1), _mul(_x2pk(a), _x2pk(b)))


# ---------------------------------------------------------------------------
# Class 1: Two distinct diagonal irreducible quadratics in denominator
# ---------------------------------------------------------------------------


class TestPhase9_TwoDistinctQuad:
    """∫ N/((x²+α)(x²+β)) dx  (diagonal quadratics, α≠β)."""

    def test_one_over_x2p1_x2p4(self) -> None:
        """∫ 1/((x²+1)(x²+4)) dx  →  (1/3)·atan(x) − (1/6)·atan(x/2)."""
        vm = _make_vm()
        f = _inv_two_quad(1, 4)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_one_over_x2p1_x2p9(self) -> None:
        """∫ 1/((x²+1)(x²+9)) dx  →  (1/8)·atan(x) − (1/24)·atan(x/3)."""
        vm = _make_vm()
        f = _inv_two_quad(1, 9)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_one_over_x2p4_x2p9(self) -> None:
        """∫ 1/((x²+4)(x²+9)) dx  →  (1/10)·atan(x/2) − (1/15)·atan(x/3)."""
        vm = _make_vm()
        f = _inv_two_quad(4, 9)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_one_over_x2p2_x2p3(self) -> None:
        """∫ 1/((x²+2)(x²+3)) dx — small non-unit coefficients."""
        vm = _make_vm()
        f = _inv_two_quad(2, 3)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_plus_one_over_two_quad(self) -> None:
        """∫ (x+1)/((x²+1)(x²+4)) dx — mixed log+atan output."""
        vm = _make_vm()
        num = _add(X, _c(1))
        denom = _mul(_x2pk(1), _x2pk(4))
        f = _div(num, denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_two_x_plus_one_over_two_quad(self) -> None:
        """∫ (2x+1)/((x²+1)(x²+4)) dx."""
        vm = _make_vm()
        num = _add(_mul(_c(2), X), _c(1))
        denom = _mul(_x2pk(1), _x2pk(4))
        f = _div(num, denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_squared_over_two_quad(self) -> None:
        """∫ x²/((x²+1)(x²+4)) dx — degree-2 numerator."""
        vm = _make_vm()
        num = _pow(X, 2)
        denom = _mul(_x2pk(1), _x2pk(4))
        f = _div(num, denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_cubed_over_two_quad(self) -> None:
        """∫ x³/((x²+1)(x²+4)) dx — degree-3 numerator (still proper)."""
        vm = _make_vm()
        num = _pow(X, 3)
        denom = _mul(_x2pk(1), _x2pk(4))
        f = _div(num, denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_plus_two_over_two_quad(self) -> None:
        """∫ (x+2)/((x²+1)(x²+4)) dx."""
        vm = _make_vm()
        num = _add(X, _c(2))
        denom = _mul(_x2pk(1), _x2pk(4))
        f = _div(num, denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_scaled_by_three(self) -> None:
        """∫ 3/((x²+1)(x²+4)) dx — constant-factor scaling."""
        vm = _make_vm()
        f = _mul(_c(3), _inv_two_quad(1, 4))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_one_over_x2p1_x2p16(self) -> None:
        """∫ 1/((x²+1)(x²+16)) dx."""
        vm = _make_vm()
        f = _inv_two_quad(1, 16)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x2_plus_x_over_two_quad(self) -> None:
        """∫ (x²+x)/((x²+1)(x²+4)) dx — degree-2 + degree-1 numerator."""
        vm = _make_vm()
        num = _add(_pow(X, 2), X)
        denom = _mul(_x2pk(1), _x2pk(4))
        f = _div(num, denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 2: Non-diagonal irreducible quadratics (b·x term present)
# ---------------------------------------------------------------------------


class TestPhase9_NonDiagonal:
    """∫ N/((x²+ax+b)(x²+cx+d)) dx — non-diagonal quadratic factors."""

    def test_one_over_x2p1_and_x2p2xp5(self) -> None:
        """∫ 1/((x²+1)(x²+2x+5)) dx — second quadratic has linear term."""
        vm = _make_vm()
        denom = _mul(_x2pk(1), _quad(2, 5))
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_one_over_x2p2xp2_and_x2p4xp5(self) -> None:
        """∫ 1/((x²+2x+2)(x²+4x+5)) dx — both non-diagonal."""
        vm = _make_vm()
        denom = _mul(_quad(2, 2), _quad(4, 5))
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_over_x2p1_and_x2p2xp5(self) -> None:
        """∫ x/((x²+1)(x²+2x+5)) dx."""
        vm = _make_vm()
        denom = _mul(_x2pk(1), _quad(2, 5))
        f = _div(X, denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_one_over_x2p2xp2_and_x2p1(self) -> None:
        """∫ 1/((x²+2x+2)(x²+1)) dx."""
        vm = _make_vm()
        denom = _mul(_quad(2, 2), _x2pk(1))
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 3: Direct ∫ atan(ax+b) dx (bonus table entry)
# ---------------------------------------------------------------------------


class TestPhase9_ArcTanDirect:
    """∫ atan(ax+b) dx = x·atan(ax+b) − (1/(2a))·log((ax+b)²+1)."""

    def test_atan_x(self) -> None:
        """∫ atan(x) dx  →  x·atan(x) − (1/2)·log(x²+1)."""
        vm = _make_vm()
        f = _atan(X)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_atan_2x(self) -> None:
        """∫ atan(2x) dx  →  x·atan(2x) − (1/4)·log(4x²+1)."""
        vm = _make_vm()
        f = _atan(_mul(_c(2), X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_atan_x_plus_1(self) -> None:
        """∫ atan(x+1) dx  →  x·atan(x+1) − (1/2)·log((x+1)²+1)."""
        vm = _make_vm()
        f = _atan(_add(X, _c(1)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_atan_2x_plus_1(self) -> None:
        """∫ atan(2x+1) dx."""
        vm = _make_vm()
        f = _atan(_add(_mul(_c(2), X), _c(1)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_atan_x_over_2(self) -> None:
        """∫ atan(x/2) dx — fractional coefficient a=1/2."""
        vm = _make_vm()
        f = _atan(_div(X, _c(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_atan_3x_minus_1(self) -> None:
        """∫ atan(3x−1) dx."""
        vm = _make_vm()
        f = _atan(_sub(_mul(_c(3), X), _c(1)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_two_times_atan_x(self) -> None:
        """∫ 2·atan(x) dx — constant factor via linearity."""
        vm = _make_vm()
        f = _mul(_c(2), _atan(X))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_atan_x_plus_x_squared(self) -> None:
        """∫ (atan(x) + x²) dx — linearity through ADD branch."""
        vm = _make_vm()
        f = _add(_atan(X), _pow(X, 2))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 4: Fallthrough — cases that must remain unevaluated
# ---------------------------------------------------------------------------


class TestPhase9_Fallthrough:
    """Integrals that Phase 9 cannot handle — must return unevaluated."""

    def test_atan_nonlinear_arg_unevaluated(self) -> None:
        """∫ atan(x²) dx — non-linear argument: atan table entry doesn't fire."""
        vm = _make_vm()
        f = _atan(_pow(X, 2))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) == F

    def test_irreducible_degree4_x4p1_unevaluated(self) -> None:
        """∫ 1/(x⁴+1) dx — irrational quadratic factors: unevaluated."""
        vm = _make_vm()
        # x⁴+1 = (x²+√2·x+1)(x²−√2·x+1) — irrational, not handled.
        f = _div(_c(1), _add(_pow(X, 4), _c(1)))
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) == F

    def test_irreducible_degree3_unevaluated(self) -> None:
        """∫ 1/(x³+x+1) dx — irreducible degree-3 denominator: unevaluated."""
        vm = _make_vm()
        denom = _add(_add(_pow(X, 3), X), _c(1))
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) == F

    def test_mixed_linear_two_quad_evaluated_by_phase10(self) -> None:
        """∫ 1/((x−1)(x²+1)(x²+4)) dx — Phase 10 handles L·Q₁·Q₂.

        Phase 9 handles Q₁·Q₂ only; Phase 10 generalizes to L·Q₁·Q₂.
        """
        vm = _make_vm()
        linear = _sub(X, _c(1))
        denom = _mul(linear, _mul(_x2pk(1), _x2pk(4)))
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        assert IRApply(INTEGRATE, (f, X)) != F

    def test_repeated_quadratic_unevaluated(self) -> None:
        """∫ 1/(x²+1)² dx — repeated quadratic: Hermite squarefree removes it,
        leaving an unevaluated residual."""
        vm = _make_vm()
        f = _div(_c(1), _pow(_x2pk(1), 2))
        F = _integrate_ir(vm, f)
        # Hermite will extract a rational part from the repeated factor,
        # but the log residual will be unevaluated (degree-2 arctan handled
        # by Phase 2e on the squarefree remainder — so this DOES evaluate).
        # Just verify it returns something and check correctness numerically.
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 5: Regressions — earlier phases must still work
# ---------------------------------------------------------------------------


class TestPhase9_Regressions:
    """Verify that adding Phase 9 did not break Phases 2d–2f, 3, 5, 7, 8."""

    def test_phase2d_log_sum(self) -> None:
        """Phase 2d: ∫ 1/((x−1)(x−2)) dx  →  log terms via RT."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_sub(X, _c(1)), _sub(X, _c(2))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase2e_single_arctan(self) -> None:
        """Phase 2e: ∫ 1/(x²+1) dx  →  atan(x)."""
        vm = _make_vm()
        f = _div(_c(1), _x2pk(1))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase2e_nontrivial_arctan(self) -> None:
        """Phase 2e: ∫ 1/(x²+2x+5) dx."""
        vm = _make_vm()
        f = _div(_c(1), _quad(2, 5))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase2f_mixed(self) -> None:
        """Phase 2f: ∫ 1/((x−1)(x²+1)) dx."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_sub(X, _c(1)), _x2pk(1)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase7_u_sub(self) -> None:
        """Phase 7: ∫ cos(x)·exp(sin(x)) dx."""
        vm = _make_vm()
        f = _mul(_cos(X), _exp(_sin(X)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase8_pow_composite(self) -> None:
        """Phase 8: ∫ 2x·(x²+1)³ dx  →  (x²+1)⁴/4."""
        vm = _make_vm()
        f = _mul(_mul(_c(2), X), _pow(_x2pk(1), 3))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase5_trig_power(self) -> None:
        """Phase 5: ∫ sin²(x) dx."""
        from symbolic_ir import SIN as _SIN
        vm = _make_vm()
        f = IRApply(POW, (IRApply(_SIN, (X,)), IRInteger(2)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase3_transcendental(self) -> None:
        """Phase 3: ∫ exp(2x+1) dx."""
        from symbolic_ir import EXP as _EXP
        vm = _make_vm()
        f = IRApply(_EXP, (_add(_mul(_c(2), X), _c(1)),))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 6: End-to-end Macsyma string tests
# ---------------------------------------------------------------------------


class TestPhase9_Macsyma:
    """End-to-end: parse MACSYMA source → compile → evaluate on SymbolicVM."""

    @staticmethod
    def _eval_macsyma(src: str) -> IRNode:
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        ast = parse_macsyma(src + ";")
        integrate_ir = compile_macsyma(ast)[0]
        vm = _make_vm()
        return vm.eval(integrate_ir)

    def test_two_quad_pure_atan(self) -> None:
        """integrate(1/((x^2+1)*(x^2+4)), x) — pure atan output."""
        result = self._eval_macsyma("integrate(1/((x^2+1)*(x^2+4)), x)")
        assert not isinstance(result, IRApply) or result.head.name != "Integrate"
        _check_antiderivative(
            _inv_two_quad(1, 4), result
        )

    def test_two_quad_mixed_log_atan(self) -> None:
        """integrate((x+1)/((x^2+1)*(x^2+4)), x) — mixed log+atan."""
        result = self._eval_macsyma(
            "integrate((x+1)/((x^2+1)*(x^2+4)), x)"
        )
        assert not isinstance(result, IRApply) or result.head.name != "Integrate"
        num = _add(X, _c(1))
        denom = _mul(_x2pk(1), _x2pk(4))
        f = _div(num, denom)
        _check_antiderivative(f, result)

    def test_atan_x_macsyma(self) -> None:
        """integrate(atan(x), x)  →  x·atan(x) − (1/2)·log(x²+1)."""
        result = self._eval_macsyma("integrate(atan(x), x)")
        assert not isinstance(result, IRApply) or result.head.name != "Integrate"
        _check_antiderivative(_atan(X), result)

    def test_atan_linear_arg_macsyma(self) -> None:
        """integrate(atan(2*x+1), x)."""
        result = self._eval_macsyma("integrate(atan(2*x+1), x)")
        assert not isinstance(result, IRApply) or result.head.name != "Integrate"
        f = _atan(_add(_mul(_c(2), X), _c(1)))
        _check_antiderivative(f, result)

    def test_non_diagonal_two_quad(self) -> None:
        """integrate(1/((x^2+1)*(x^2+2*x+5)), x)."""
        result = self._eval_macsyma(
            "integrate(1/((x^2+1)*(x^2+2*x+5)), x)"
        )
        assert not isinstance(result, IRApply) or result.head.name != "Integrate"
        denom = _mul(_x2pk(1), _quad(2, 5))
        f = _div(_c(1), denom)
        _check_antiderivative(f, result)
