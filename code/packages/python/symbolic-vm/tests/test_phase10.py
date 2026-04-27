"""Phase 10 integration tests — generalized partial-fraction integration.

Three families:

  1. ∫ N(x) / (Q₁(x)·Q₂(x)·Q₃(x)) dx  — three distinct irreducible quadratics,
     degree-6 denominator, no linear factors.

  2. ∫ N(x) / (L(x)·Q₁(x)·Q₂(x)) dx  — rational linear factors × two irreducible
     quadratics; degree 5 (one linear) or 6 (two linears).

  3. RT performance guard — degree ≥ 6 denominators no longer hang; degree ≤ 5
     still route through Rothstein–Trager when appropriate.

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
# Shared helpers (mirror Phase 9 test helpers)
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


def _pow(base: IRNode, n: int) -> IRNode:
    return IRApply(POW, (base, IRInteger(n)))


def _atan(arg: IRNode) -> IRNode:
    return IRApply(ATAN, (arg,))


def _log(arg: IRNode) -> IRNode:
    return IRApply(LOG, (arg,))


def _c(n: int) -> IRNode:
    return IRInteger(n)


def _r(p: int, q: int) -> IRNode:
    return IRRational(p, q)


def _x2pk(k: int) -> IRNode:
    """Builds (x² + k)."""
    return _add(_pow(X, 2), _c(k))


def _quad(a: int, b: int) -> IRNode:
    """Builds (x² + ax + b)."""
    return _add(_add(_pow(X, 2), _mul(_c(a), X)), _c(b))


def _linear(r_numer: int, r_denom: int = 1) -> IRNode:
    """Builds (x − r_numer/r_denom)."""
    if r_denom == 1:
        return _sub(X, _c(r_numer))
    return _sub(X, _r(r_numer, r_denom))


# ---------------------------------------------------------------------------
# Class 1: Three distinct irreducible quadratics  Q₁·Q₂·Q₃  (degree 6)
# ---------------------------------------------------------------------------


class TestPhase10_ThreeQuadratic:
    """∫ N/((x²+α)(x²+β)(x²+γ)) dx — diagonal and non-diagonal cases."""

    def test_one_over_x2p1_x2p4_x2p9(self) -> None:
        """∫ 1/((x²+1)(x²+4)(x²+9)) dx — canonical three-diagonal case."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_x2pk(1), _mul(_x2pk(4), _x2pk(9))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x_over_three_quad(self) -> None:
        """∫ x/((x²+1)(x²+4)(x²+9)) dx."""
        vm = _make_vm()
        denom = _mul(_x2pk(1), _mul(_x2pk(4), _x2pk(9)))
        f = _div(X, denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_xp1_over_three_quad(self) -> None:
        """∫ (x+1)/((x²+1)(x²+4)(x²+9)) dx."""
        vm = _make_vm()
        denom = _mul(_x2pk(1), _mul(_x2pk(4), _x2pk(9)))
        f = _div(_add(X, _c(1)), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_x2_over_three_quad(self) -> None:
        """∫ x²/((x²+1)(x²+4)(x²+9)) dx."""
        vm = _make_vm()
        denom = _mul(_x2pk(1), _mul(_x2pk(4), _x2pk(9)))
        f = _div(_pow(X, 2), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_one_over_x2p1_x2p4_x2p16(self) -> None:
        """∫ 1/((x²+1)(x²+4)(x²+16)) dx."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_x2pk(1), _mul(_x2pk(4), _x2pk(16))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_one_over_x2p1_x2p9_x2p16(self) -> None:
        """∫ 1/((x²+1)(x²+9)(x²+16)) dx."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_x2pk(1), _mul(_x2pk(9), _x2pk(16))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_one_over_x2p4_x2p9_x2p16(self) -> None:
        """∫ 1/((x²+4)(x²+9)(x²+16)) dx."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_x2pk(4), _mul(_x2pk(9), _x2pk(16))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_one_over_x2p1_x2p4_x2p25(self) -> None:
        """∫ 1/((x²+1)(x²+4)(x²+25)) dx."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_x2pk(1), _mul(_x2pk(4), _x2pk(25))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_non_diagonal_first_factor(self) -> None:
        """∫ 1/((x²+2x+2)(x²+4)(x²+9)) dx — non-diagonal Q₁."""
        vm = _make_vm()
        # (x²+2x+2): disc = 4-8 = -4 < 0, irreducible
        denom = _mul(_quad(2, 2), _mul(_x2pk(4), _x2pk(9)))
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_non_diagonal_x2p2xp5(self) -> None:
        """∫ 1/((x²+2x+5)(x²+4)(x²+9)) dx — non-diagonal Q₁."""
        vm = _make_vm()
        # (x²+2x+5): disc = 4-20 = -16 < 0, irreducible
        denom = _mul(_quad(2, 5), _mul(_x2pk(4), _x2pk(9)))
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_non_diagonal_numerator_xp1(self) -> None:
        """∫ (x+1)/((x²+2x+2)(x²+4)(x²+9)) dx."""
        vm = _make_vm()
        denom = _mul(_quad(2, 2), _mul(_x2pk(4), _x2pk(9)))
        f = _div(_add(X, _c(1)), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_two_non_diagonal_factors(self) -> None:
        """∫ 1/((x²+2x+2)(x²+4x+5)(x²+9)) dx — two non-diagonal quadratics."""
        vm = _make_vm()
        # (x²+2x+2): disc = -4 < 0, (x²+4x+5): disc = 16-20 = -4 < 0
        denom = _mul(_quad(2, 2), _mul(_quad(4, 5), _x2pk(9)))
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 2: Linear factors × two irreducible quadratics  (degree 5 and 6)
# ---------------------------------------------------------------------------


class TestPhase10_LinearPlusTwoQuad:
    """∫ N / (Lᵐ·Q₁·Q₂) dx — one or two linear factors with two quadratics."""

    def test_one_over_L1_Q1_Q4(self) -> None:
        """∫ 1/((x−1)(x²+1)(x²+4)) dx — degree-5 canonical case."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_linear(1), _mul(_x2pk(1), _x2pk(4))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.8))

    def test_one_over_Lm1_Q1_Q4(self) -> None:
        """∫ 1/((x+1)(x²+1)(x²+4)) dx — negative linear root."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_add(X, _c(1)), _mul(_x2pk(1), _x2pk(4))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.5))

    def test_x_over_L1_Q1_Q4(self) -> None:
        """∫ x/((x−1)(x²+1)(x²+4)) dx."""
        vm = _make_vm()
        denom = _mul(_linear(1), _mul(_x2pk(1), _x2pk(4)))
        f = _div(X, denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.8))

    def test_xp1_over_L1_Q1_Q4(self) -> None:
        """∫ (x+1)/((x−1)(x²+1)(x²+4)) dx."""
        vm = _make_vm()
        denom = _mul(_linear(1), _mul(_x2pk(1), _x2pk(4)))
        f = _div(_add(X, _c(1)), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.8))

    def test_one_over_L2_Q1_Q4(self) -> None:
        """∫ 1/((x−2)(x²+1)(x²+4)) dx."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_linear(2), _mul(_x2pk(1), _x2pk(4))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.5))

    def test_one_over_L1_Q1_Q9(self) -> None:
        """∫ 1/((x−1)(x²+1)(x²+9)) dx."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_linear(1), _mul(_x2pk(1), _x2pk(9))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.8))

    def test_one_over_L1_L2_Q1_Q4(self) -> None:
        """∫ 1/((x−1)(x−2)(x²+1)(x²+4)) dx — degree-6 two-linear case."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_linear(1), _mul(_linear(2), _mul(_x2pk(1), _x2pk(4)))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.5))

    def test_one_over_L1_L3_Q1_Q4(self) -> None:
        """∫ 1/((x−1)(x−3)(x²+1)(x²+4)) dx."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_linear(1), _mul(_linear(3), _mul(_x2pk(1), _x2pk(4)))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.5))

    def test_x_over_L1_L2_Q1_Q4(self) -> None:
        """∫ x/((x−1)(x−2)(x²+1)(x²+4)) dx."""
        vm = _make_vm()
        denom = _mul(_linear(1), _mul(_linear(2), _mul(_x2pk(1), _x2pk(4))))
        f = _div(X, denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.5))

    def test_one_over_L1_L2_Q4_Q9(self) -> None:
        """∫ 1/((x−1)(x−2)(x²+4)(x²+9)) dx."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_linear(1), _mul(_linear(2), _mul(_x2pk(4), _x2pk(9)))))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.5))

    def test_xp1_over_L1_L2_Q1_Q4(self) -> None:
        """∫ (x+1)/((x−1)(x−2)(x²+1)(x²+4)) dx."""
        vm = _make_vm()
        denom = _mul(_linear(1), _mul(_linear(2), _mul(_x2pk(1), _x2pk(4))))
        f = _div(_add(X, _c(1)), denom)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.5))

    def test_one_over_Lm1_Lp1_Q1_Q4(self) -> None:
        """∫ 1/((x+1)(x−1)(x²+1)(x²+4)) dx — roots ±1."""
        vm = _make_vm()
        f = _div(
            _c(1),
            _mul(
                _add(X, _c(1)),
                _mul(_linear(1), _mul(_x2pk(1), _x2pk(4))),
            ),
        )
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.5))


# ---------------------------------------------------------------------------
# Class 3: RT performance guard
# ---------------------------------------------------------------------------


class TestPhase10_RTGuard:
    """Verify the degree-6 Rothstein–Trager guard: fast completion and correctness."""

    def test_three_quad_completes_without_hanging(self) -> None:
        """∫ 1/((x²+1)(x²+4)(x²+9)) must complete (formerly hung 26 ks)."""
        import time

        vm = _make_vm()
        f = _div(_c(1), _mul(_x2pk(1), _mul(_x2pk(4), _x2pk(9))))
        t0 = time.monotonic()
        F = _integrate_ir(vm, f)
        elapsed = time.monotonic() - t0
        _was_evaluated(f, F)
        assert elapsed < 5.0, f"Phase 10 took {elapsed:.2f}s — expected < 5s"

    def test_degree5_three_factor_completes_fast(self) -> None:
        """∫ 1/((x−1)(x²+1)(x²+4)) must also complete quickly."""
        import time

        vm = _make_vm()
        f = _div(_c(1), _mul(_linear(1), _mul(_x2pk(1), _x2pk(4))))
        t0 = time.monotonic()
        F = _integrate_ir(vm, f)
        elapsed = time.monotonic() - t0
        _was_evaluated(f, F)
        assert elapsed < 5.0, f"Phase 10 took {elapsed:.2f}s — expected < 5s"

    def test_degree4_still_routes_to_phase9(self) -> None:
        """∫ 1/((x²+1)(x²+4)) — degree 4, Phase 9 handles it (not Phase 10)."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_x2pk(1), _x2pk(4)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_degree3_linear_plus_quad_routes_to_phase2f(self) -> None:
        """∫ 1/((x−1)(x²+1)) — degree 3, Phase 2f handles it (not Phase 10)."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_linear(1), _x2pk(1)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.8))


# ---------------------------------------------------------------------------
# Class 4: Fallthrough — cases Phase 10 cannot evaluate
# ---------------------------------------------------------------------------


class TestPhase10_Fallthrough:
    """Cases that remain unevaluated after Phase 10 (degree > 6, or unfactorable)."""

    def test_degree7_linear_three_quad_unevaluated(self) -> None:
        """∫ 1/((x−1)(x²+1)(x²+4)(x²+9)) dx — degree 7, beyond Phase 10 scope."""
        vm = _make_vm()
        denom = _mul(_linear(1), _mul(_x2pk(1), _mul(_x2pk(4), _x2pk(9))))
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_degree8_four_quad_unevaluated(self) -> None:
        """∫ 1/((x²+1)(x²+4)(x²+9)(x²+16)) dx — degree 8, beyond Phase 10 scope."""
        vm = _make_vm()
        denom = _mul(_x2pk(1), _mul(_x2pk(4), _mul(_x2pk(9), _x2pk(16))))
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_degree5_unfactorable_quartic_unevaluated(self) -> None:
        """∫ 1/((x−1)(x⁴+2x²+2)) dx — degree-4 remainder x⁴+2x²+2 unfactorable.

        x⁴+2x²+2 treated as biquadratic z²+2z+2: disc = 4−8 = −4 < 0,
        no real quadratic factorization over Q → Phase 10 returns None.
        """
        vm = _make_vm()
        # Build x⁴+2x²+2 = x⁴ + 2x² + 2
        x4 = _pow(X, 4)
        quartic = _add(_add(x4, _mul(_c(2), _pow(X, 2))), _c(2))
        denom = _mul(_linear(1), quartic)
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_degree6_two_linear_three_quad_unevaluated(self) -> None:
        """∫ 1/((x−1)(x−2)(x²+1)(x²+4)(x²+9)) dx — degree 7, unevaluated."""
        vm = _make_vm()
        denom = _mul(
            _linear(1),
            _mul(_linear(2), _mul(_x2pk(1), _mul(_x2pk(4), _x2pk(9)))),
        )
        f = _div(_c(1), denom)
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)

    def test_irrational_biquadratic_unevaluated(self) -> None:
        """∫ 1/(x⁴+1) dx — factors over Q(√2) only, not over Q; unevaluated."""
        vm = _make_vm()
        # x⁴+1 = (x²+x√2+1)(x²−x√2+1) — no factorization with rational coefficients
        x4 = _pow(X, 4)
        f = _div(_c(1), _add(x4, _c(1)))
        F = _integrate_ir(vm, f)
        _is_unevaluated(f, F)


# ---------------------------------------------------------------------------
# Class 5: Regressions — earlier phases must still work
# ---------------------------------------------------------------------------


class TestPhase10_Regressions:
    """Confirm Phase 10 additions haven't broken any earlier integration phase."""

    def test_phase2d_log_regression(self) -> None:
        """∫ 1/(x²−1) dx — Rothstein–Trager (Phase 2d) still works."""
        vm = _make_vm()
        f = _div(_c(1), _sub(_pow(X, 2), _c(1)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.5))

    def test_phase2e_arctan_regression(self) -> None:
        """∫ 1/(x²+1) dx — single quadratic (Phase 2e) still works."""
        vm = _make_vm()
        f = _div(_c(1), _x2pk(1))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase2f_linear_quad_regression(self) -> None:
        """∫ 1/((x−1)(x²+1)) dx — Phase 2f still works."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_linear(1), _x2pk(1)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F, test_points=(0.3, 0.8))

    def test_phase9_two_quad_regression(self) -> None:
        """∫ 1/((x²+1)(x²+4)) dx — Phase 9 still works."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_x2pk(1), _x2pk(4)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase9_non_diagonal_quad_regression(self) -> None:
        """∫ 1/((x²+1)(x²+2x+5)) dx — Phase 9 non-diagonal still works."""
        vm = _make_vm()
        f = _div(_c(1), _mul(_x2pk(1), _quad(2, 5)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase7_polynomial_regression(self) -> None:
        """∫ x³ dx — polynomial integration (Phase 7) still works."""
        vm = _make_vm()
        f = _pow(X, 3)
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase3_exp_regression(self) -> None:
        """∫ exp(x) dx — elementary function (Phase 3) still works."""
        from symbolic_ir import EXP

        vm = _make_vm()
        f = IRApply(EXP, (X,))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)

    def test_phase5_product_regression(self) -> None:
        """∫ x·exp(x) dx — IBP (Phase 5) still works."""
        from symbolic_ir import EXP

        vm = _make_vm()
        f = _mul(X, IRApply(EXP, (X,)))
        F = _integrate_ir(vm, f)
        _was_evaluated(f, F)
        _check_antiderivative(f, F)


# ---------------------------------------------------------------------------
# Class 6: End-to-end MACSYMA string tests
# ---------------------------------------------------------------------------


class TestPhase10_Macsyma:
    """End-to-end: parse MACSYMA source → compile → evaluate on SymbolicVM."""

    @staticmethod
    def _eval_macsyma(src: str) -> IRNode:
        from macsyma_compiler import compile_macsyma
        from macsyma_parser import parse_macsyma

        ast = parse_macsyma(src + ";")
        integrate_ir = compile_macsyma(ast)[0]
        vm = _make_vm()
        return vm.eval(integrate_ir)

    def test_three_quad_macsyma(self) -> None:
        """integrate(1/((x^2+1)*(x^2+4)*(x^2+9)), x)."""
        result = self._eval_macsyma(
            "integrate(1/((x^2+1)*(x^2+4)*(x^2+9)), x)"
        )
        assert not isinstance(result, IRApply) or result.head.name != "Integrate"
        f = _div(_c(1), _mul(_x2pk(1), _mul(_x2pk(4), _x2pk(9))))
        _check_antiderivative(f, result)

    def test_linear_two_quad_macsyma(self) -> None:
        """integrate(1/((x-1)*(x^2+1)*(x^2+4)), x)."""
        result = self._eval_macsyma(
            "integrate(1/((x-1)*(x^2+1)*(x^2+4)), x)"
        )
        assert not isinstance(result, IRApply) or result.head.name != "Integrate"
        f = _div(_c(1), _mul(_linear(1), _mul(_x2pk(1), _x2pk(4))))
        _check_antiderivative(f, result, test_points=(0.3, 0.5))

    def test_two_linear_two_quad_macsyma(self) -> None:
        """integrate(1/((x-1)*(x-2)*(x^2+1)*(x^2+4)), x)."""
        result = self._eval_macsyma(
            "integrate(1/((x-1)*(x-2)*(x^2+1)*(x^2+4)), x)"
        )
        assert not isinstance(result, IRApply) or result.head.name != "Integrate"
        f = _div(_c(1), _mul(_linear(1), _mul(_linear(2), _mul(_x2pk(1), _x2pk(4)))))
        _check_antiderivative(f, result, test_points=(0.3, 0.5))

    def test_x_over_three_quad_macsyma(self) -> None:
        """integrate(x/((x^2+1)*(x^2+4)*(x^2+9)), x)."""
        result = self._eval_macsyma(
            "integrate(x/((x^2+1)*(x^2+4)*(x^2+9)), x)"
        )
        assert not isinstance(result, IRApply) or result.head.name != "Integrate"
        f = _div(X, _mul(_x2pk(1), _mul(_x2pk(4), _x2pk(9))))
        _check_antiderivative(f, result)

    def test_xp1_linear_two_quad_macsyma(self) -> None:
        """integrate((x+1)/((x-1)*(x^2+1)*(x^2+4)), x)."""
        result = self._eval_macsyma(
            "integrate((x+1)/((x-1)*(x^2+1)*(x^2+4)), x)"
        )
        assert not isinstance(result, IRApply) or result.head.name != "Integrate"
        denom = _mul(_linear(1), _mul(_x2pk(1), _x2pk(4)))
        f = _div(_add(X, _c(1)), denom)
        _check_antiderivative(f, result, test_points=(0.3, 0.5))
