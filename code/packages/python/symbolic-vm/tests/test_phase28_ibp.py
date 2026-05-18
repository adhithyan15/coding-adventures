"""Tests for Phase 28 General IBP: poly×log(Q(x)) and poly×atan(Q(x)).

Phase 28 extends two earlier IBP patterns to *non-linear* polynomial arguments:

1. ``∫ P(x)·log(Q(x)) dx = R(x)·log(Q(x)) − ∫ R(x)·Q′(x)/Q(x) dx``
   where R = ∫P and the residual is a rational function handled by the
   Hermite + Rothstein–Trager pipeline.

2. ``∫ P(x)·atan(Q(x)) dx = R(x)·atan(Q(x)) − ∫ R(x)·Q′(x)/(1+Q(x)²) dx``
   Same structure; the residual denominator is 1+Q(x)².

Classic examples:

- ``∫ log(x²+1) dx  =  x·log(x²+1) − 2x + 2·atan(x)``
- ``∫ x·log(x²+1) dx  =  (x²+1)/2·log(x²+1) − x²/2``
  (residual R·Q′/Q = x²/(x²+1), partial-fraction → 1 − 1/(x²+1),
   ∫ = x − atan(x), so full result = x²/2·log(x²+1) + atan(x) − x²/2 + C)

Wait — let me recheck ∫ x·log(x²+1) dx by parts:
  u = log(x²+1), dv = x dx
  v = x²/2,  du = 2x/(x²+1) dx
  result = x²/2·log(x²+1) − ∫ x²/2·(2x/(x²+1)) dx
         = x²/2·log(x²+1) − ∫ x³/(x²+1) dx
  ∫ x³/(x²+1) dx: polynomial division gives x − x/(x²+1),
  so ∫ = x²/2 − (1/2)·log(x²+1)
  Overall: x²/2·log(x²+1) − x²/2 + (1/2)·log(x²+1)

- ``∫ x·atan(x²) dx  =  x²/2·atan(x²) − (1/4)·log(1+x⁴)``
  (R = x²/2, Q′ = 2x, 1+Q² = 1+x⁴,
   residual = x²/2·2x/(1+x⁴) = x³/(1+x⁴),
   ∫ x³/(1+x⁴) dx = (1/4)·log(1+x⁴))

All tests use a numerical ground-truth check (trapezoidal rule) to avoid
hard-coding the exact algebraic form of the result.
"""

from __future__ import annotations

import math

import pytest
from symbolic_ir import (
    ADD,
    ATAN,
    DIV,
    INTEGRATE,
    IRApply,
    IRInteger,
    IRRational,
    IRSymbol,
    LOG,
    MUL,
    NEG,
    POW,
    SUB,
)

from symbolic_vm import VM, SymbolicBackend

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

X = IRSymbol("x")


@pytest.fixture
def vm() -> VM:
    return VM(SymbolicBackend())


def _integrate(f: IRApply) -> IRApply:
    return IRApply(INTEGRATE, (f, X))


def _contains_head(node: object, head: IRSymbol) -> bool:
    """Recursively check whether any sub-node has the given head."""
    if isinstance(node, IRApply):
        if node.head == head:
            return True
        return any(_contains_head(a, head) for a in node.args)
    return False


def _eval_ir(node: object, x_val: float) -> float:  # noqa: PLR0912
    """Numerically evaluate a simple IR tree by substitution."""
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRRational):
        return node.numer / node.denom
    if isinstance(node, IRSymbol):
        if node == X:
            return x_val
        raise ValueError(f"unknown symbol {node}")
    assert isinstance(node, IRApply)
    h = node.head
    if h == ADD:
        return sum(_eval_ir(a, x_val) for a in node.args)
    if h == SUB:
        a, b = node.args
        return _eval_ir(a, x_val) - _eval_ir(b, x_val)
    if h == MUL:
        result = 1.0
        for a in node.args:
            result *= _eval_ir(a, x_val)
        return result
    if h == NEG:
        return -_eval_ir(node.args[0], x_val)
    if h == POW:
        base, exp = node.args
        return _eval_ir(base, x_val) ** _eval_ir(exp, x_val)
    if h == DIV:
        a, b = node.args
        return _eval_ir(a, x_val) / _eval_ir(b, x_val)
    if h == LOG:
        return math.log(_eval_ir(node.args[0], x_val))
    if h == ATAN:
        return math.atan(_eval_ir(node.args[0], x_val))
    raise ValueError(f"unsupported head {h}")


def _numerical_definite(
    integrand_fn: object, a: float, b: float, n: int = 20_000
) -> float:
    """Simple trapezoidal rule for ground-truth comparison."""
    assert callable(integrand_fn)
    h = (b - a) / n
    total = 0.5 * (integrand_fn(a) + integrand_fn(b))
    for i in range(1, n):
        total += integrand_fn(a + i * h)
    return total * h


# ---------------------------------------------------------------------------
# Part 1 — log(non-linear polynomial) patterns
# ---------------------------------------------------------------------------
# Test 1 — ∫ log(x²+1) dx is closed
# ---------------------------------------------------------------------------


def test_log_xsq_plus_1_is_closed(vm: VM) -> None:
    """``∫ log(x²+1) dx`` must return a closed form without ``Integrate``."""
    # Build log(x²+1): LOG(ADD(POW(x,2), 1))
    xsq = IRApply(POW, (X, IRInteger(2)))
    arg = IRApply(ADD, (xsq, IRInteger(1)))
    integrand = IRApply(LOG, (arg,))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    # Result must contain both LOG and ATAN (from the partial-fraction residual).
    assert _contains_head(out, LOG), f"expected LOG in result, got: {out}"
    assert _contains_head(out, ATAN), f"expected ATAN in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 2 — ∫ log(x²+1) dx numerical correctness
# ---------------------------------------------------------------------------


def test_log_xsq_plus_1_numeric(vm: VM) -> None:
    """``∫₀^1 log(x²+1) dx``: antiderivative difference matches trapezoidal.

    Known value: ∫₀¹ log(x²+1) dx = ln 2 + π/2 − 2 ≈ 0.26338...
    """
    xsq = IRApply(POW, (X, IRInteger(2)))
    arg = IRApply(ADD, (xsq, IRInteger(1)))
    integrand = IRApply(LOG, (arg,))
    out = vm.eval(_integrate(integrand))
    f1 = _eval_ir(out, 1.0)
    f0 = _eval_ir(out, 0.0)
    antideriv_diff = f1 - f0

    numerical = _numerical_definite(lambda xv: math.log(xv**2 + 1), 0.0, 1.0)
    assert abs(antideriv_diff - numerical) < 1e-5, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 3 — ∫ x·log(x²+1) dx is closed
# ---------------------------------------------------------------------------


def test_x_log_xsq_plus_1_is_closed(vm: VM) -> None:
    """``∫ x·log(x²+1) dx`` must return a closed form."""
    xsq = IRApply(POW, (X, IRInteger(2)))
    arg = IRApply(ADD, (xsq, IRInteger(1)))
    integrand = IRApply(MUL, (X, IRApply(LOG, (arg,))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, LOG), f"expected LOG in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 4 — ∫ x·log(x²+1) dx numerical correctness
# ---------------------------------------------------------------------------


def test_x_log_xsq_plus_1_numeric(vm: VM) -> None:
    """``∫₁^2 x·log(x²+1) dx``: antiderivative matches trapezoidal."""
    xsq = IRApply(POW, (X, IRInteger(2)))
    arg = IRApply(ADD, (xsq, IRInteger(1)))
    integrand = IRApply(MUL, (X, IRApply(LOG, (arg,))))
    out = vm.eval(_integrate(integrand))
    f2 = _eval_ir(out, 2.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f2 - f1

    numerical = _numerical_definite(lambda xv: xv * math.log(xv**2 + 1), 1.0, 2.0)
    assert abs(antideriv_diff - numerical) < 1e-5, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 5 — ∫ x²·log(x²+1) dx is closed
# ---------------------------------------------------------------------------


def test_x_sq_log_xsq_plus_1_is_closed(vm: VM) -> None:
    """``∫ x²·log(x²+1) dx`` must return a closed form."""
    xsq = IRApply(POW, (X, IRInteger(2)))
    arg = IRApply(ADD, (xsq, IRInteger(1)))
    integrand = IRApply(MUL, (xsq, IRApply(LOG, (arg,))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, LOG), f"expected LOG in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 6 — ∫ x²·log(x²+1) dx numerical correctness
# ---------------------------------------------------------------------------


def test_x_sq_log_xsq_plus_1_numeric(vm: VM) -> None:
    """``∫₁^2 x²·log(x²+1) dx``: antiderivative matches trapezoidal."""
    xsq = IRApply(POW, (X, IRInteger(2)))
    arg = IRApply(ADD, (xsq, IRInteger(1)))
    integrand = IRApply(MUL, (xsq, IRApply(LOG, (arg,))))
    out = vm.eval(_integrate(integrand))
    f2 = _eval_ir(out, 2.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f2 - f1

    numerical = _numerical_definite(lambda xv: xv**2 * math.log(xv**2 + 1), 1.0, 2.0)
    assert abs(antideriv_diff - numerical) < 1e-5, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Part 2 — atan(non-linear polynomial) patterns
# ---------------------------------------------------------------------------
# Test 7 — ∫ atan(x²) dx is NOT closed (expected fallthrough)
# ---------------------------------------------------------------------------


def test_atan_xsq_fallthrough(vm: VM) -> None:
    """``∫ atan(x²) dx`` remains unevaluated — expected behaviour.

    IBP gives residual ``∫ 2x²/(1+x⁴) dx`` which requires irrational partial
    fractions (x⁴+1 factors over ℝ with irrational coefficients) and is outside
    the current Hermite + Rothstein–Trager scope.  The system correctly returns
    the unevaluated ``Integrate`` form rather than producing a wrong answer.
    """
    xsq = IRApply(POW, (X, IRInteger(2)))
    integrand = IRApply(ATAN, (xsq,))
    out = vm.eval(_integrate(integrand))
    # The engine should fall through — result still contains Integrate.
    assert _contains_head(out, INTEGRATE), (
        f"expected unevaluated Integrate, but got a closed form: {out}"
    )


# ---------------------------------------------------------------------------
# Test 8 — ∫ x·atan(x²) dx is closed
# ---------------------------------------------------------------------------


def test_x_atan_xsq_is_closed(vm: VM) -> None:
    """``∫ x·atan(x²) dx`` must return a closed form.

    Expected: x²/2·atan(x²) − (1/4)·log(1+x⁴).
    """
    xsq = IRApply(POW, (X, IRInteger(2)))
    integrand = IRApply(MUL, (X, IRApply(ATAN, (xsq,))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, ATAN), f"expected ATAN in result, got: {out}"
    assert _contains_head(out, LOG), f"expected LOG in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 10 — ∫ x·atan(x²) dx numerical correctness
# ---------------------------------------------------------------------------


def test_x_atan_xsq_numeric(vm: VM) -> None:
    """``∫₀^1 x·atan(x²) dx``: antiderivative difference matches trapezoidal."""
    xsq = IRApply(POW, (X, IRInteger(2)))
    integrand = IRApply(MUL, (X, IRApply(ATAN, (xsq,))))
    out = vm.eval(_integrate(integrand))
    f1 = _eval_ir(out, 1.0)
    f0 = _eval_ir(out, 0.0)
    antideriv_diff = f1 - f0

    numerical = _numerical_definite(lambda xv: xv * math.atan(xv**2), 0.0, 1.0)
    assert abs(antideriv_diff - numerical) < 1e-5, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 11 — ∫ x·atan(x²) dx specific value check
# ---------------------------------------------------------------------------


def test_x_atan_xsq_specific(vm: VM) -> None:
    """Verify ∫ x·atan(x²) dx against known closed form at x=1.

    Known: F(x) = x²/2·atan(x²) − (1/4)·log(1+x⁴), F(0) = 0.
    F(1) = π/8 − (1/4)·log(2) ≈ 0.39269908 − 0.17328679 ≈ 0.21941229.
    """
    xsq = IRApply(POW, (X, IRInteger(2)))
    integrand = IRApply(MUL, (X, IRApply(ATAN, (xsq,))))
    out = vm.eval(_integrate(integrand))
    f1 = _eval_ir(out, 1.0)
    f0 = _eval_ir(out, 0.0)
    antideriv_diff = f1 - f0

    expected = math.pi / 8 - 0.25 * math.log(2)
    assert abs(antideriv_diff - expected) < 1e-9, (
        f"expected {expected:.10f}, got {antideriv_diff:.10f}"
    )


# ---------------------------------------------------------------------------
# Test 12 — ∫ x²·atan(x²) dx is NOT closed (expected fallthrough)
# ---------------------------------------------------------------------------


def test_x_sq_atan_xsq_fallthrough(vm: VM) -> None:
    """``∫ x²·atan(x²) dx`` remains unevaluated — expected behaviour.

    IBP gives residual ``∫ 2x⁴/(3(1+x⁴)) dx``.  After polynomial division:
    ``(2/3)∫ (1 − 1/(1+x⁴)) dx``.  The factor ``1+x⁴`` has irrational roots
    so the partial fraction decomposition requires surds — beyond the current
    rational engine.  Correct behaviour is to return unevaluated.
    """
    xsq = IRApply(POW, (X, IRInteger(2)))
    integrand = IRApply(MUL, (xsq, IRApply(ATAN, (xsq,))))
    out = vm.eval(_integrate(integrand))
    assert _contains_head(out, INTEGRATE), (
        f"expected unevaluated Integrate, but got a closed form: {out}"
    )


# ---------------------------------------------------------------------------
# Part 3 — Regression tests: linear argument forms still work
# ---------------------------------------------------------------------------
# Test 13 — ∫ x·log(x) dx still works (Phase 3 linear log)
# ---------------------------------------------------------------------------


def test_regression_x_log_x(vm: VM) -> None:
    """``∫ x·log(x) dx = x²/2·log(x) − x²/4`` still works after Phase 28."""
    integrand = IRApply(MUL, (X, IRApply(LOG, (X,))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    # Numerical: ∫₁^2 x·log(x) dx
    f2 = _eval_ir(out, 2.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f2 - f1
    numerical = _numerical_definite(lambda xv: xv * math.log(xv), 1.0, 2.0)
    assert abs(antideriv_diff - numerical) < 1e-9, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 14 — ∫ x·atan(x) dx still works (Phase 11 linear atan)
# ---------------------------------------------------------------------------


def test_regression_x_atan_x(vm: VM) -> None:
    """``∫ x·atan(x) dx = x²/2·atan(x) − x/2 + (1/2)·atan(x)`` (or equiv).

    Phase 11 handles linear atan; Phase 28 must not interfere.
    """
    integrand = IRApply(MUL, (X, IRApply(ATAN, (X,))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    # Numerical: ∫₀^1 x·atan(x) dx = π/4 − 1/2 + 1/2·atan(1) − atan(0)/2
    # Known exact: π/4 − 1/2 + π/8 = 3π/8 − 1/2 ... let trapezoidal decide.
    f1 = _eval_ir(out, 1.0)
    f0 = _eval_ir(out, 0.0)
    antideriv_diff = f1 - f0
    numerical = _numerical_definite(lambda xv: xv * math.atan(xv), 0.0, 1.0)
    assert abs(antideriv_diff - numerical) < 1e-9, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 15 — ∫ log(2x+1) dx still works (Phase 3 linear log with offset)
# ---------------------------------------------------------------------------


def test_regression_log_linear(vm: VM) -> None:
    """``∫ log(2x+1) dx`` — Phase 3 linear log must still fire."""
    # 2x+1 = MUL(2, x) + 1 — built as ADD(MUL(2, x), 1)
    lin_arg = IRApply(ADD, (IRApply(MUL, (IRInteger(2), X)), IRInteger(1)))
    integrand = IRApply(LOG, (lin_arg,))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    f2 = _eval_ir(out, 2.0)
    f0 = _eval_ir(out, 0.5)
    antideriv_diff = f2 - f0
    numerical = _numerical_definite(
        lambda xv: math.log(2 * xv + 1), 0.5, 2.0
    )
    assert abs(antideriv_diff - numerical) < 1e-9, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )
