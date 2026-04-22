"""Tests for the Phase 1 ``Integrate`` handler on :class:`SymbolicBackend`.

Phase 1 is the reverse-derivative-table integrator. It handles
constants, the power rule, linearity, constant factors, and the direct
elementary functions. Anything else stays as ``Integrate(f, x)``
unevaluated — the CAS never claims to have integrated what it didn't.
"""

from __future__ import annotations

import pytest
from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EXP,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SQRT,
    SUB,
    IRApply,
    IRInteger,
    IRRational,
    IRSymbol,
)

from symbolic_vm import VM, SymbolicBackend


@pytest.fixture
def vm() -> VM:
    return VM(SymbolicBackend())


X = IRSymbol("x")
Y = IRSymbol("y")


def _integrate(f):
    return IRApply(INTEGRATE, (f, X))


# ---------------------------------------------------------------------------
# Constants and linearity
# ---------------------------------------------------------------------------


def test_integrate_integer_constant(vm: VM) -> None:
    # ∫ 5 dx = 5·x
    assert vm.eval(_integrate(IRInteger(5))) == IRApply(MUL, (IRInteger(5), X))


def test_integrate_free_variable(vm: VM) -> None:
    # ∫ y dx = y·x (y independent of x)
    assert vm.eval(_integrate(Y)) == IRApply(MUL, (Y, X))


def test_integrate_self(vm: VM) -> None:
    # ∫ x dx = (1/2)·x^2
    expected = IRApply(
        MUL, (IRRational(1, 2), IRApply(POW, (X, IRInteger(2))))
    )
    assert vm.eval(_integrate(X)) == expected


def test_integrate_zero(vm: VM) -> None:
    # ∫ 0 dx = 0·x — simplifies via Mul's zero law.
    assert vm.eval(_integrate(IRInteger(0))) == IRInteger(0)


# ---------------------------------------------------------------------------
# Power rule
# ---------------------------------------------------------------------------


def test_integrate_power_positive(vm: VM) -> None:
    # ∫ x^2 dx = (1/3)·x^3
    expr = _integrate(IRApply(POW, (X, IRInteger(2))))
    assert vm.eval(expr) == IRApply(
        MUL, (IRRational(1, 3), IRApply(POW, (X, IRInteger(3))))
    )


def test_integrate_power_literal_minus_one(vm: VM) -> None:
    # ∫ x^(-1) dx = log(x)
    expr = _integrate(IRApply(POW, (X, IRInteger(-1))))
    assert vm.eval(expr) == IRApply(LOG, (X,))


def test_integrate_power_neg_one_form(vm: VM) -> None:
    # ∫ x^(-(1)) dx = log(x) — same rule, just through the Neg form.
    expr = _integrate(IRApply(POW, (X, IRApply(NEG, (IRInteger(1),)))))
    assert vm.eval(expr) == IRApply(LOG, (X,))


def test_integrate_reciprocal(vm: VM) -> None:
    # ∫ 1/x dx = log(x)  — Mul(1, Log(x)) collapses via the identity law.
    expr = _integrate(IRApply(DIV, (IRInteger(1), X)))
    assert vm.eval(expr) == IRApply(LOG, (X,))


def test_integrate_constant_over_x(vm: VM) -> None:
    # ∫ 3/x dx = 3·log(x)
    expr = _integrate(IRApply(DIV, (IRInteger(3), X)))
    assert vm.eval(expr) == IRApply(MUL, (IRInteger(3), IRApply(LOG, (X,))))


def test_integrate_exponential_constant_base(vm: VM) -> None:
    # ∫ 2^x dx = 2^x / log(2). The Log handler folds ``log(2)`` to a
    # float (existing behaviour), so that's what we get in the result.
    import math

    from symbolic_ir import IRFloat

    expr = _integrate(IRApply(POW, (IRInteger(2), X)))
    expected = IRApply(
        DIV,
        (
            IRApply(POW, (IRInteger(2), X)),
            IRFloat(math.log(2)),
        ),
    )
    assert vm.eval(expr) == expected


# ---------------------------------------------------------------------------
# Linearity (Add, Sub, Neg)
# ---------------------------------------------------------------------------


def _half_x_squared():
    return IRApply(
        MUL, (IRRational(1, 2), IRApply(POW, (X, IRInteger(2))))
    )


def test_integrate_sum(vm: VM) -> None:
    # ∫ (x + 3) dx = (1/2)·x^2 + 3·x
    expr = _integrate(IRApply(ADD, (X, IRInteger(3))))
    expected = IRApply(
        ADD,
        (_half_x_squared(), IRApply(MUL, (IRInteger(3), X))),
    )
    assert vm.eval(expr) == expected


def test_integrate_difference(vm: VM) -> None:
    # ∫ (x - 1) dx = (1/2)·x^2 - x  (Mul(1, x) collapses to x)
    expr = _integrate(IRApply(SUB, (X, IRInteger(1))))
    expected = IRApply(SUB, (_half_x_squared(), X))
    assert vm.eval(expr) == expected


def test_integrate_negation(vm: VM) -> None:
    # ∫ -x dx = -((1/2)·x^2)
    expr = _integrate(IRApply(NEG, (X,)))
    expected = IRApply(NEG, (_half_x_squared(),))
    assert vm.eval(expr) == expected


# ---------------------------------------------------------------------------
# Constant factor
# ---------------------------------------------------------------------------


def test_integrate_constant_factor_left(vm: VM) -> None:
    # ∫ 5·x dx = 5 · (1/2)·x^2
    expr = _integrate(IRApply(MUL, (IRInteger(5), X)))
    expected = IRApply(MUL, (IRInteger(5), _half_x_squared()))
    assert vm.eval(expr) == expected


def test_integrate_constant_factor_right(vm: VM) -> None:
    # ∫ x·7 dx = 7 · (1/2)·x^2 — handler pulls the x-free factor left.
    expr = _integrate(IRApply(MUL, (X, IRInteger(7))))
    expected = IRApply(MUL, (IRInteger(7), _half_x_squared()))
    assert vm.eval(expr) == expected


def test_integrate_free_symbol_factor(vm: VM) -> None:
    # ∫ y·x^2 dx = y · (1/3)·x^3
    inner = IRApply(MUL, (Y, IRApply(POW, (X, IRInteger(2)))))
    expr = _integrate(inner)
    expected = IRApply(
        MUL,
        (
            Y,
            IRApply(
                MUL,
                (IRRational(1, 3), IRApply(POW, (X, IRInteger(3)))),
            ),
        ),
    )
    assert vm.eval(expr) == expected


# ---------------------------------------------------------------------------
# Elementary functions at x
# ---------------------------------------------------------------------------


def test_integrate_sin(vm: VM) -> None:
    # ∫ sin(x) dx = -cos(x)
    assert vm.eval(_integrate(IRApply(SIN, (X,)))) == IRApply(
        NEG, (IRApply(COS, (X,)),)
    )


def test_integrate_cos(vm: VM) -> None:
    # ∫ cos(x) dx = sin(x)
    assert vm.eval(_integrate(IRApply(COS, (X,)))) == IRApply(SIN, (X,))


def test_integrate_exp(vm: VM) -> None:
    # ∫ exp(x) dx = exp(x)
    assert vm.eval(_integrate(IRApply(EXP, (X,)))) == IRApply(EXP, (X,))


def test_integrate_log(vm: VM) -> None:
    # ∫ log(x) dx = x·log(x) - x  (the hard-coded by-parts case)
    expected = IRApply(
        SUB,
        (IRApply(MUL, (X, IRApply(LOG, (X,)))), X),
    )
    assert vm.eval(_integrate(IRApply(LOG, (X,)))) == expected


def test_integrate_sqrt(vm: VM) -> None:
    # ∫ sqrt(x) dx = (2/3)·x^(3/2)
    expected = IRApply(
        MUL,
        (
            IRRational(2, 3),
            IRApply(POW, (X, IRRational(3, 2))),
        ),
    )
    assert vm.eval(_integrate(IRApply(SQRT, (X,)))) == expected


# ---------------------------------------------------------------------------
# Unevaluated fallback — Phase 1 limits
# ---------------------------------------------------------------------------


def test_integrate_poly_times_sin_now_closed_by_phase4(vm: VM) -> None:
    # ∫ x·sin(x) dx = sin(x) − x·cos(x)  (Phase 4a closes this).
    inner = IRApply(MUL, (X, IRApply(SIN, (X,))))
    result = vm.eval(_integrate(inner))
    assert result != IRApply(INTEGRATE, (inner, X)), (
        "Phase 4a should close x·sin(x) — got unevaluated result"
    )


def test_integrate_composed_trig_now_closed_by_phase3(vm: VM) -> None:
    # ∫ sin(2·x) dx = −cos(2x)/2  (Phase 3b closes this).
    inner = IRApply(SIN, (IRApply(MUL, (IRInteger(2), X)),))
    result = vm.eval(_integrate(inner))
    # Must NOT be unevaluated.
    assert not (isinstance(result, IRApply) and result.head.name == "Integrate")


def test_integrate_unknown_function_unevaluated(vm: VM) -> None:
    # ∫ g(x) dx for an unknown symbolic head stays unevaluated.
    unknown = IRApply(IRSymbol("g"), (X,))
    expr = _integrate(unknown)
    assert vm.eval(expr) == IRApply(INTEGRATE, (unknown, X))


def test_integrate_non_symbol_variable_unevaluated(vm: VM) -> None:
    # Integration variable must be a bare symbol; otherwise pass through.
    bad = IRApply(INTEGRATE, (X, IRInteger(1)))
    assert vm.eval(bad) == bad


def test_integrate_wrong_arity_raises(vm: VM) -> None:
    with pytest.raises(TypeError, match="Integrate expects 2 arguments"):
        vm.eval(IRApply(INTEGRATE, (X,)))


# ---------------------------------------------------------------------------
# Diff ∘ Integrate sanity check — the fundamental theorem holds for any
# antiderivative whose derivative simplifies cleanly. Polynomials don't
# fully round-trip yet (the simplifier doesn't cancel common factors
# inside ``Div``); that's Phase 2 territory. Elementary-function cases
# that don't involve rational-coefficient cancellation do work today.
# ---------------------------------------------------------------------------


def test_diff_then_integrate_sin(vm: VM) -> None:
    # d/dx (∫ sin(x) dx) = d/dx (-cos(x)) = sin(x)
    from symbolic_ir import D

    sin_x = IRApply(SIN, (X,))
    integral = IRApply(INTEGRATE, (sin_x, X))
    roundtrip = IRApply(D, (integral, X))
    assert vm.eval(roundtrip) == sin_x


def test_diff_then_integrate_exp(vm: VM) -> None:
    # d/dx (∫ exp(x) dx) = exp(x)
    from symbolic_ir import D

    exp_x = IRApply(EXP, (X,))
    integral = IRApply(INTEGRATE, (exp_x, X))
    roundtrip = IRApply(D, (integral, X))
    assert vm.eval(roundtrip) == exp_x


# ---------------------------------------------------------------------------
# Phase 2c — Hermite reduction end-to-end
# ---------------------------------------------------------------------------
#
# The rational-function route fires before Phase 1 when the integrand is
# a rational function of x over Q with a non-constant denominator. The
# universal correctness check is the same one the Hermite unit tests
# use: differentiating the integrator's output must give back the
# original integrand — but here we check it at the IR level via the
# ``D`` handler on :class:`SymbolicBackend`.


def _contains_head(node, head):
    """True iff the IR subtree contains an ``IRApply`` with ``head``."""
    if isinstance(node, IRApply):
        if node.head == head:
            return True
        return any(_contains_head(a, head) for a in (node.head, *node.args))
    return False


def test_hermite_pure_rational_antiderivative(vm: VM) -> None:
    # ∫ 1/(x - 1)² dx = -1/(x - 1). Pure rational antiderivative — the
    # log residual is zero, so the output must not contain an inner
    # Integrate node. (Structural equality against the "expected" IR
    # is fragile — different canonicalisations of the same rational
    # are all correct — so we assert the property that matters:
    # Hermite closed the integral, no residual was left behind.)
    xm1 = IRApply(SUB, (X, IRInteger(1)))
    integrand = IRApply(DIV, (IRInteger(1), IRApply(POW, (xm1, IRInteger(2)))))
    out = vm.eval(IRApply(INTEGRATE, (integrand, X)))
    assert not _contains_head(out, INTEGRATE), (
        f"Hermite should have closed ∫ 1/(x-1)² dx, got {out}"
    )


def test_hermite_higher_power(vm: VM) -> None:
    # ∫ 1/(x - 1)³ dx — multiplicity 3, still closed-form rational.
    xm1 = IRApply(SUB, (X, IRInteger(1)))
    integrand = IRApply(DIV, (IRInteger(1), IRApply(POW, (xm1, IRInteger(3)))))
    out = vm.eval(IRApply(INTEGRATE, (integrand, X)))
    assert not _contains_head(out, INTEGRATE)


def test_hermite_plus_rt_closes_mixed_case(vm: VM) -> None:
    # ∫ 1 / ((x-1)² (x+1)) dx — Hermite extracts the rational piece,
    # RT closes the log residual (roots of the RT resultant are in Q).
    # The combined pipeline emits a closed form with *no* residual
    # Integrate.
    xm1 = IRApply(SUB, (X, IRInteger(1)))
    xp1 = IRApply(ADD, (X, IRInteger(1)))
    den = IRApply(MUL, (IRApply(POW, (xm1, IRInteger(2))), xp1))
    integrand = IRApply(DIV, (IRInteger(1), den))
    out = vm.eval(IRApply(INTEGRATE, (integrand, X)))
    assert not _contains_head(out, INTEGRATE), (
        f"Hermite + RT should close ∫ 1/((x-1)²(x+1)) dx, got {out}"
    )
    # Expect log terms in the output.
    assert _contains_head(out, LOG)


def test_rt_closes_squarefree_one_over_linear(vm: VM) -> None:
    # ∫ 1/(x - 1) dx = log(x - 1). Hermite does nothing (squarefree
    # denom); RT's resultant has single rational root α = 1 with
    # v = x − 1. Output is the bare Log node.
    xm1 = IRApply(SUB, (X, IRInteger(1)))
    integrand = IRApply(DIV, (IRInteger(1), xm1))
    out = vm.eval(IRApply(INTEGRATE, (integrand, X)))
    assert not _contains_head(out, INTEGRATE)
    assert _contains_head(out, LOG)


def test_hermite_with_polynomial_part(vm: VM) -> None:
    # ∫ (x^3 + 1) / (x - 1)^2 dx — polynomial part + rational part +
    # log part. With RT in place every piece resolves; the output has
    # no residual Integrate.
    xm1 = IRApply(SUB, (X, IRInteger(1)))
    den = IRApply(POW, (xm1, IRInteger(2)))
    num = IRApply(ADD, (IRApply(POW, (X, IRInteger(3))), IRInteger(1)))
    integrand = IRApply(DIV, (num, den))
    out = vm.eval(IRApply(INTEGRATE, (integrand, X)))
    assert not _contains_head(out, INTEGRATE)
    assert _contains_head(out, LOG)


def test_hermite_does_not_touch_sin(vm: VM) -> None:
    # ``to_rational`` rejects transcendentals, so Phase 1 still wins on
    # sin / cos / exp.
    integrand = IRApply(SIN, (X,))
    out = vm.eval(IRApply(INTEGRATE, (integrand, X)))
    assert out == IRApply(NEG, (IRApply(COS, (X,)),))


def test_hermite_does_not_touch_exp(vm: VM) -> None:
    integrand = IRApply(EXP, (X,))
    out = vm.eval(IRApply(INTEGRATE, (integrand, X)))
    assert out == IRApply(EXP, (X,))


# ---------------------------------------------------------------------------
# Phase 2d — Rothstein–Trager end-to-end
# ---------------------------------------------------------------------------


def test_rt_closes_partial_fractions(vm: VM) -> None:
    # ∫ 1 / ((x-1)(x+1)) dx — textbook partial-fractions case. Both
    # residues are in Q, RT emits the log sum. No Integrate residual.
    xm1 = IRApply(SUB, (X, IRInteger(1)))
    xp1 = IRApply(ADD, (X, IRInteger(1)))
    den = IRApply(MUL, (xm1, xp1))
    integrand = IRApply(DIV, (IRInteger(1), den))
    out = vm.eval(IRApply(INTEGRATE, (integrand, X)))
    assert not _contains_head(out, INTEGRATE)
    assert _contains_head(out, LOG)


def test_rt_escape_q_now_closed_by_phase2e(vm: VM) -> None:
    # ∫ 1/(x² + 1) dx — Phase 2d (RT) returns None (roots ±i/2 not in Q),
    # but Phase 2e (arctan formula) closes it: result is Atan(x).
    from symbolic_ir import ATAN
    x2 = IRApply(POW, (X, IRInteger(2)))
    den = IRApply(ADD, (x2, IRInteger(1)))
    integrand = IRApply(DIV, (IRInteger(1), den))
    out = vm.eval(IRApply(INTEGRATE, (integrand, X)))
    assert isinstance(out, IRApply) and out.head == ATAN


def test_rt_coefficient_minus_one(vm: VM) -> None:
    # ∫ 1/(x² - 1) dx produces coefficients ±1/2, exercising the
    # IRRational rendering branch (not the ±1 shortcuts). The output
    # must contain a Log and no Integrate.
    x2 = IRApply(POW, (X, IRInteger(2)))
    den = IRApply(SUB, (x2, IRInteger(1)))
    integrand = IRApply(DIV, (IRInteger(1), den))
    out = vm.eval(IRApply(INTEGRATE, (integrand, X)))
    assert not _contains_head(out, INTEGRATE)
    assert _contains_head(out, LOG)


def test_rt_coefficient_minus_one_explicit(vm: VM) -> None:
    # ∫ x/((x-1)(x-2)) dx = -log(x-1) + 2·log(x-2). Exercises the
    # c = -1 → Neg(Log(·)) rendering shortcut explicitly (one of the
    # pairs has coefficient −1).
    xm1 = IRApply(SUB, (X, IRInteger(1)))
    xm2 = IRApply(SUB, (X, IRInteger(2)))
    den = IRApply(MUL, (xm1, xm2))
    integrand = IRApply(DIV, (X, den))
    out = vm.eval(IRApply(INTEGRATE, (integrand, X)))
    assert not _contains_head(out, INTEGRATE)
    assert _contains_head(out, LOG)
    # One of the terms is a Neg(Log(·)) — verify by structural search.
    def _has_neg_of_log(node):
        if isinstance(node, IRApply):
            if node.head == NEG and len(node.args) == 1:
                inner = node.args[0]
                if isinstance(inner, IRApply) and inner.head == LOG:
                    return True
            return any(_has_neg_of_log(a) for a in node.args)
        return False
    assert _has_neg_of_log(out), f"expected Neg(Log(·)) in {out}"
