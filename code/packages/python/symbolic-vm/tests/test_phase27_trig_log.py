"""Tests for Phase 27: sin(log(x)) and cos(log(x)) integration.

Phase 27 adds two new patterns to the ``Integrate`` handler:

1. **Pure trig-of-log** — ``∫ sin(log(x)) dx`` and ``∫ cos(log(x)) dx``.
   Uses the substitution u = log(x) which converts the integral to the
   standard exp×trig form, giving:

       ∫ sin(log x) dx = x/2 · (sin(log x) − cos(log x))
       ∫ cos(log x) dx = x/2 · (sin(log x) + cos(log x))

2. **Polynomial × trig(log(x))** — ``∫ Q(x) · sin(log(x)) dx`` and
   ``∫ Q(x) · cos(log(x)) dx``.
   Applies the term-by-term formula:

       ∫ xᵏ sin(log x) dx = x^(k+1) · ((k+1) sin(log x) − cos(log x)) / ((k+1)² + 1)
       ∫ xᵏ cos(log x) dx = x^(k+1) · ((k+1) cos(log x) + sin(log x)) / ((k+1)² + 1)

Both patterns recurse down to k = 0 from the polynomial degree, all handled
by the same ``_trig_log_integral`` kernel.
"""

from __future__ import annotations

import math

import pytest
from symbolic_ir import (
    ADD,
    COS,
    INTEGRATE,
    LOG,
    MUL,
    NEG,
    POW,
    SIN,
    SUB,
    DIV,
    IRApply,
    IRInteger,
    IRRational,
    IRSymbol,
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


def _eval_ir(node: object, x_val: float) -> float:
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
    if h == SIN:
        return math.sin(_eval_ir(node.args[0], x_val))
    if h == COS:
        return math.cos(_eval_ir(node.args[0], x_val))
    raise ValueError(f"unsupported head {h}")


def _numerical_definite(integrand_fn: object, a: float, b: float, n: int = 10_000) -> float:
    """Simple trapezoidal rule for ground-truth comparison."""
    assert callable(integrand_fn)
    h = (b - a) / n
    total = 0.5 * (integrand_fn(a) + integrand_fn(b))
    for i in range(1, n):
        total += integrand_fn(a + i * h)
    return total * h


def _sin_log_x(x: float) -> float:
    return math.sin(math.log(x))


def _cos_log_x(x: float) -> float:
    return math.cos(math.log(x))


# ---------------------------------------------------------------------------
# Test 1 — ∫ sin(log(x)) dx returns a closed form
# ---------------------------------------------------------------------------


def test_sin_log_x_is_closed(vm: VM) -> None:
    """``∫ sin(log(x)) dx`` must return a closed form without ``Integrate``."""
    integrand = IRApply(SIN, (IRApply(LOG, (X,)),))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    # Result must involve both SIN and COS of log(x).
    assert _contains_head(out, SIN), f"expected SIN in result, got: {out}"
    assert _contains_head(out, COS), f"expected COS in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 2 — ∫ sin(log(x)) dx numerical correctness
# ---------------------------------------------------------------------------


def test_sin_log_x_numeric(vm: VM) -> None:
    """``∫₁^3 sin(log(x)) dx``: antiderivative difference matches trapezoidal.

    Closed form: F(x) = x/2 · (sin(log x) − cos(log x)).
    """
    integrand = IRApply(SIN, (IRApply(LOG, (X,)),))
    out = vm.eval(_integrate(integrand))
    f3 = _eval_ir(out, 3.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f3 - f1

    numerical = _numerical_definite(_sin_log_x, 1.0, 3.0)
    assert abs(antideriv_diff - numerical) < 1e-5, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 3 — ∫ cos(log(x)) dx returns a closed form
# ---------------------------------------------------------------------------


def test_cos_log_x_is_closed(vm: VM) -> None:
    """``∫ cos(log(x)) dx`` must return a closed form without ``Integrate``."""
    integrand = IRApply(COS, (IRApply(LOG, (X,)),))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, SIN), f"expected SIN in result, got: {out}"
    assert _contains_head(out, COS), f"expected COS in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 4 — ∫ cos(log(x)) dx numerical correctness
# ---------------------------------------------------------------------------


def test_cos_log_x_numeric(vm: VM) -> None:
    """``∫₁^3 cos(log(x)) dx``: antiderivative difference matches trapezoidal.

    Closed form: F(x) = x/2 · (sin(log x) + cos(log x)).
    """
    integrand = IRApply(COS, (IRApply(LOG, (X,)),))
    out = vm.eval(_integrate(integrand))
    f3 = _eval_ir(out, 3.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f3 - f1

    numerical = _numerical_definite(_cos_log_x, 1.0, 3.0)
    assert abs(antideriv_diff - numerical) < 1e-5, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 5 — ∫ x·sin(log(x)) dx is closed
# ---------------------------------------------------------------------------


def test_x_sin_log_x_is_closed(vm: VM) -> None:
    """``∫ x·sin(log(x)) dx`` must return a closed form."""
    integrand = IRApply(MUL, (X, IRApply(SIN, (IRApply(LOG, (X,)),))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, SIN), f"expected SIN in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 6 — ∫ x·sin(log(x)) dx numerical correctness
# ---------------------------------------------------------------------------


def test_x_sin_log_x_numeric(vm: VM) -> None:
    """``∫₁^2 x·sin(log(x)) dx``.

    Closed form: x²/5 · (2 sin(log x) − cos(log x)).
    """
    integrand = IRApply(MUL, (X, IRApply(SIN, (IRApply(LOG, (X,)),))))
    out = vm.eval(_integrate(integrand))
    f2 = _eval_ir(out, 2.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f2 - f1

    numerical = _numerical_definite(lambda xv: xv * _sin_log_x(xv), 1.0, 2.0)
    assert abs(antideriv_diff - numerical) < 1e-5, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 7 — ∫ x·cos(log(x)) dx is closed
# ---------------------------------------------------------------------------


def test_x_cos_log_x_is_closed(vm: VM) -> None:
    """``∫ x·cos(log(x)) dx`` must return a closed form."""
    integrand = IRApply(MUL, (X, IRApply(COS, (IRApply(LOG, (X,)),))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, COS), f"expected COS in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 8 — ∫ x·cos(log(x)) dx numerical correctness
# ---------------------------------------------------------------------------


def test_x_cos_log_x_numeric(vm: VM) -> None:
    """``∫₁^2 x·cos(log(x)) dx``.

    Closed form: x²/5 · (2 cos(log x) + sin(log x)).
    """
    integrand = IRApply(MUL, (X, IRApply(COS, (IRApply(LOG, (X,)),))))
    out = vm.eval(_integrate(integrand))
    f2 = _eval_ir(out, 2.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f2 - f1

    numerical = _numerical_definite(lambda xv: xv * _cos_log_x(xv), 1.0, 2.0)
    assert abs(antideriv_diff - numerical) < 1e-5, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 9 — ∫ x²·sin(log(x)) dx numerical correctness
# ---------------------------------------------------------------------------


def test_x_sq_sin_log_x_numeric(vm: VM) -> None:
    """``∫₁^2 x²·sin(log(x)) dx``.

    Closed form: x³/10 · (3 sin(log x) − cos(log x)).
    """
    x_sq = IRApply(POW, (X, IRInteger(2)))
    integrand = IRApply(MUL, (x_sq, IRApply(SIN, (IRApply(LOG, (X,)),))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    f2 = _eval_ir(out, 2.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f2 - f1

    numerical = _numerical_definite(lambda xv: xv**2 * _sin_log_x(xv), 1.0, 2.0)
    assert abs(antideriv_diff - numerical) < 1e-4, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 10 — ∫ x²·cos(log(x)) dx numerical correctness
# ---------------------------------------------------------------------------


def test_x_sq_cos_log_x_numeric(vm: VM) -> None:
    """``∫₁^2 x²·cos(log(x)) dx``.

    Closed form: x³/10 · (3 cos(log x) + sin(log x)).
    """
    x_sq = IRApply(POW, (X, IRInteger(2)))
    integrand = IRApply(MUL, (x_sq, IRApply(COS, (IRApply(LOG, (X,)),))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    f2 = _eval_ir(out, 2.0)
    f1 = _eval_ir(out, 1.0)
    antideriv_diff = f2 - f1

    numerical = _numerical_definite(lambda xv: xv**2 * _cos_log_x(xv), 1.0, 2.0)
    assert abs(antideriv_diff - numerical) < 1e-4, (
        f"antiderivative diff {antideriv_diff:.8f} != numerical {numerical:.8f}"
    )


# ---------------------------------------------------------------------------
# Test 11 — polynomial × sin(log(x)) is closed
# ---------------------------------------------------------------------------


def test_poly_sin_log_x_is_closed(vm: VM) -> None:
    """``∫ (x^2 + 2·x + 1)·sin(log(x)) dx`` must return a closed form."""
    # (x^2 + 2x + 1) = (x+1)^2
    x_sq = IRApply(POW, (X, IRInteger(2)))
    two_x = IRApply(MUL, (IRInteger(2), X))
    from symbolic_ir import ADD as _ADD
    poly = IRApply(_ADD, (IRApply(_ADD, (x_sq, two_x)), IRInteger(1)))
    integrand = IRApply(MUL, (poly, IRApply(SIN, (IRApply(LOG, (X,)),))))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    assert _contains_head(out, SIN), f"expected SIN in result, got: {out}"


# ---------------------------------------------------------------------------
# Test 12 — regression: ∫ sin(x) dx still works (no Phase 27 interference)
# ---------------------------------------------------------------------------


def test_regression_sin_x(vm: VM) -> None:
    """``∫ sin(x) dx = −cos(x)`` still correct after Phase 27 is added."""
    integrand = IRApply(SIN, (X,))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    # Numerical check: ∫₀^(π/2) sin(x) dx = 1
    f_pi2 = _eval_ir(out, math.pi / 2)
    f_0 = _eval_ir(out, 0.0)
    assert abs((f_pi2 - f_0) - 1.0) < 1e-9, (
        f"expected 1.0, got {f_pi2 - f_0}"
    )


# ---------------------------------------------------------------------------
# Test 13 — regression: ∫ cos(x) dx still works
# ---------------------------------------------------------------------------


def test_regression_cos_x(vm: VM) -> None:
    """``∫ cos(x) dx = sin(x)`` still correct after Phase 27 is added."""
    integrand = IRApply(COS, (X,))
    out = vm.eval(_integrate(integrand))
    assert not _contains_head(out, INTEGRATE), f"expected closed form, got: {out}"
    # Numerical: ∫₀^(π/2) cos(x) dx = 1
    f_pi2 = _eval_ir(out, math.pi / 2)
    f_0 = _eval_ir(out, 0.0)
    assert abs((f_pi2 - f_0) - 1.0) < 1e-9, (
        f"expected 1.0, got {f_pi2 - f_0}"
    )
